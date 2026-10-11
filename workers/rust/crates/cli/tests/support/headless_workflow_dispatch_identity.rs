use super::agent_support::{LiveAgent, wait_for_lifecycle};
use super::diagnostic_domains::DOMAINS;
use super::domain_diagnostic_cancellation::scan_payload;
use super::domain_diagnostics::{assert_dispatches, assert_ready, request};
use super::domain_workflow_cancellation::place_task;
use super::orchestra_cancel_routing::{HoldMarkers, active, task_result};
use super::orchestra_cancellation::wait_for_task;
use super::orchestra_completion::{assert_bar, bar_task, http_json};
use super::transport_output::cli_with_policy;
use super::transport_research::{assert_retained, workflow};
use super::write_ack_loss::count;
use super::{Scratch, start_live_server_with_state};
use kyuubiki_headless_sdk::{CancelExecutionRequest, HeadlessExecutor, ServiceHeadlessExecutor};
use serde_json::{Value, json};
use std::{
    error::Error,
    fs,
    path::Path,
    thread,
    time::{Duration, Instant},
};

type Result<T> = std::result::Result<T, Box<dyn Error>>;

fn record(root: &Path, task_id: &str) -> Result<Value> {
    let mut records = fs::read_dir(root.join("operator-task-dispatches"))?
        .map(|entry| -> Result<Value> {
            let envelope: Value = serde_json::from_slice(&fs::read(entry?.path())?)?;
            Ok(envelope["payload"].clone())
        })
        .collect::<Result<Vec<_>>>()?
        .into_iter()
        .filter(|record| record["task_id"] == task_id)
        .collect::<Vec<_>>();
    assert_eq!(records.len(), 1, "duplicate workflow dispatch: {records:?}");
    Ok(records.remove(0))
}

fn wait_failed(port: u16, job: &str) -> Result<Value> {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let (_, stored) = http_json(port, &format!("/api/v1/jobs/{job}"), None)?;
        match stored["job"]["status"].as_str() {
            Some("failed") => return Ok(stored),
            Some("completed" | "cancelled") => {
                return Err(
                    format!("exact node cancellation changed parent semantics: {stored}").into(),
                );
            }
            _ if Instant::now() >= deadline => {
                return Err(
                    format!("workflow did not observe cancelled execution: {stored}").into(),
                );
            }
            _ => thread::sleep(Duration::from_millis(10)),
        }
    }
}

#[test]
fn real_parent_bound_workflow_dispatch_inspection_and_exact_cancellation_preserve_same_job_peer()
-> Result<()> {
    let mut owner =
        LiveAgent::start_with_solver_hold("result_diagnostics", "run_operator_task_ir", "1")?;
    let mut peer = LiveAgent::start()?;
    let data = Scratch::new()?;
    let server =
        start_live_server_with_state(Some(owner.port()), Some(peer.port()), Some(&data.0))?;
    let url = format!("http://127.0.0.1:{}", server.port);
    owner.restart_with_orchestrator(&url)?;
    peer.restart_with_orchestrator(&url)?;
    let client = ServiceHeadlessExecutor::try_new(&url).map_err(|error| error.message)?;
    let (_, projects) = http_json(server.port, "/api/v1/projects", None)?;

    for (round, domain) in DOMAINS.into_iter().enumerate() {
        let input = scan_payload(domain);
        let expected =
            kyuubiki_engine::run_extract_operator(domain.diagnostic, input.clone(), json!({}))
                .unwrap();
        thread::scope(|scope| -> Result<()> {
            let markers = HoldMarkers(&owner.hold_path, &peer.hold_path);
            let blocker_job = format!("identity-owner-{}", domain.name);
            fs::write(&owner.hold_path, &blocker_job)?;
            let blocker = place_task(
                domain.task(
                    &blocker_job,
                    domain.diagnostic,
                    "extract",
                    input.clone(),
                    json!({}),
                ),
                "owned-live-owner",
            );
            let blocker = scope.spawn(|| task_result(server.port, blocker));
            wait_for_task(&owner, &blocker_job, Some("result_diagnostics"))?;
            let mut req = serde_json::to_value(request(domain, input.clone(), false, false))?;
            req["graph"]["nodes"][1]["placement_tags"] = json!(["owned-live-owner"]);
            let (status, admitted) =
                http_json(server.port, "/api/v1/workflows/graph/jobs", Some(req))?;
            assert_eq!(status, 202, "{admitted}");
            let job = admitted["job"]["job_id"]
                .as_str()
                .ok_or("missing parent job")?;
            fs::write(&owner.hold_path, job)?;
            let first = blocker.join().map_err(|_| "owner blocker panicked")??;
            assert_retained(&first["result"]["result"], &expected, "owner.summary");
            let held = wait_for_task(&owner, job, Some("result_diagnostics"))?;
            assert_eq!(held["checkpoint"]["completed_steps"], 64);

            // Another execution deliberately shares the parent but not request/process/generation.
            fs::write(&peer.hold_path, job)?;
            let peer_task = bar_task(&format!("peer-child-{job}"), 0.01);
            let peer_params = json!({"task_ir":peer_task, "job_id":job, "mode":"execute"});
            let peer_agent = &peer;
            let bystander = scope.spawn(move || {
                peer_agent
                    .request(
                        "identity-peer-shared-parent",
                        "run_operator_task_ir",
                        peer_params,
                    )
                    .map_err(|error| error.to_string())
            });
            let peer_before = active(&peer, job)?;
            let task_id = format!("{job}:diagnose:{}", domain.diagnostic);
            let retained = record(&data.0, &task_id)?;
            assert_eq!(
                retained["schema_version"],
                "kyuubiki.operator-task-dispatch-record/v2"
            );
            assert_eq!(retained["job_id"], job);
            let digest = retained["task_digest"].as_str().ok_or("missing digest")?;
            let inspected = client
                .inspect_operator_task_dispatch(&task_id, digest)
                .map_err(|error| error.message)?;
            assert_eq!(inspected["retained_attempt_count"], 1);
            let attempt = &inspected["attempts"][0];
            assert_eq!(attempt["job_id"], job);
            assert_eq!(attempt["request_id"], held["request_id"]);
            let target: CancelExecutionRequest =
                serde_json::from_value(attempt["observation"]["execution_target"].clone())?;
            assert_eq!(target.job_id, job);
            assert_ne!(target.job_id, task_id);
            assert_eq!(json!(target.generation), held["generation"]);
            assert_eq!(
                json!(target.process_instance_id),
                held["process_instance_id"]
            );
            let attempt_id = retained["attempt_id"].as_str().ok_or("missing attempt")?;
            let cancelled = client
                .cancel_operator_task_dispatch(&task_id, digest, attempt_id, &target)
                .map_err(|error| error.message)?;
            assert_eq!(cancelled["status"], "requested");
            assert_eq!(cancelled["cancel_registered"], true);
            for field in [
                "job_wide_fallback_performed",
                "execution_terminal_confirmed",
                "automatic_replay_authorized",
                "journal_mutation_performed",
                "publication_performed",
            ] {
                assert_eq!(cancelled[field], false);
            }
            wait_for_lifecycle(&owner, "accepting", 0)?;
            let failed = wait_failed(server.port, job)?;
            for node in ["diagnose", "quality", "objective", "decision"] {
                assert!(
                    failed["result"]["artifacts"]
                        .get(format!("{node}.summary"))
                        .is_none()
                );
            }
            assert_dispatches(&client, &data.0, job, domain, true)?;
            let peer_after = active(&peer, job)?;
            assert_eq!(peer_after["cancel_requested"], false);
            assert_eq!(peer_after["request_id"], peer_before["request_id"]);
            assert_eq!(peer_after["generation"], peer_before["generation"]);
            assert_eq!(count(&owner)? + count(&peer)?, round as u64 * 5 + 3);
            let (status, receipt) = http_json(
                server.port,
                "/api/v1/operator-tasks/fetch-dispatch-result",
                Some(json!({"task_id":task_id, "task_digest":digest, "attempt_id":attempt_id})),
            )?;
            assert_eq!(status, 200, "{receipt}");
            assert_eq!(receipt["status"], "receipt_recovered", "{receipt}");
            assert_eq!(receipt["outcome"], "failed");
            assert_eq!(receipt["generation"], held["generation"]);
            assert_eq!(
                receipt["completion"]["failure_receipt"]["reason_code"],
                "cancelled"
            );
            let mut waiting =
                ServiceHeadlessExecutor::try_new(&url).map_err(|error| error.message)?;
            let error = waiting
                .execute_step("job_wait", 1, &json!({"job_id":job,"timeout_ms":100}))
                .expect_err("failed graph cannot satisfy scientific completion");
            assert!(
                error.message.starts_with("job_terminal_failed:"),
                "{error:?}"
            );

            drop(markers);
            let bystander = bystander.join().map_err(|_| "peer execution panicked")??;
            assert_eq!(bystander["ok"], true);
            assert_bar(&json!({"status":"executed", "result":bystander["result"]}));
            wait_for_lifecycle(&peer, "accepting", 0)?;
            let report = cli_with_policy(
                &workflow(request(domain, input.clone(), false, false)),
                &url,
                true,
                true,
            )?;
            assert_eq!(report.status, "ok");
            let next = report.steps[0].result_preview["job_id"]
                .as_str()
                .ok_or("missing recovery job")?;
            assert_ne!(next, job);
            let (_, result) = http_json(server.port, &format!("/api/v1/results/{next}"), None)?;
            assert_ready(domain, &result["result"]);
            assert_dispatches(&client, &data.0, next, domain, false)?;
            assert_dispatches(&client, &data.0, job, domain, true)?;
            assert_eq!(
                http_json(server.port, &format!("/api/v1/jobs/{job}"), None)?.1,
                failed
            );
            assert_eq!(
                http_json(server.port, "/api/v1/projects", None)?.1,
                projects
            );
            assert_eq!(count(&owner)? + count(&peer)?, (round as u64 + 1) * 5);
            Ok(())
        })?;
    }
    assert_eq!(count(&owner)? + count(&peer)?, 25);
    Ok(())
}
