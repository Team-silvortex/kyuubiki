use super::agent_support::{LiveAgent, wait_for_lifecycle};
use super::diagnostic_domains::DOMAINS;
use super::domain_diagnostic_cancellation::scan_payload;
use super::domain_diagnostics::{assert_dispatches, assert_ready, request};
use super::domain_workflow_cancellation::place_task;
use super::orchestra_cancel_routing::{HoldMarkers, task_result};
use super::orchestra_cancellation::wait_for_task;
use super::orchestra_completion::http_json;
use super::transport_output::cli_with_policy;
use super::transport_research::{assert_retained, workflow};
use super::write_ack_loss::count;
use super::{LiveServer, Scratch, start_live_server_with_result_fixtures};
use kyuubiki_headless_sdk::{HeadlessExecutor, ServiceHeadlessExecutor};
use serde_json::{Value, json};
use std::{
    error::Error,
    fs,
    path::Path,
    thread,
    time::{Duration, Instant},
};

type Result<T> = std::result::Result<T, Box<dyn Error>>;

fn retained(root: &Path, job: &str) -> Result<Value> {
    let records = fs::read_dir(root.join("operator-task-dispatches"))?
        .map(|entry| -> Result<Value> {
            let envelope: Value = serde_json::from_slice(&fs::read(entry?.path())?)?;
            Ok(envelope["payload"].clone())
        })
        .collect::<Result<Vec<_>>>()?;
    let attempts = records
        .iter()
        .filter(|record| record["job_id"] == job)
        .collect::<Vec<_>>();
    assert_eq!(attempts.len(), 1, "unexpected replay: {attempts:?}");
    Ok(attempts[0].clone())
}

fn wait_blocked(server: &LiveServer, job: &str) -> Result<Value> {
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        let (status, stored) = http_json(server.port, &format!("/api/v1/jobs/{job}"), None)?;
        assert_eq!(status, 200, "durable parent lost after restart: {stored}");
        match stored["job"]["status"].as_str() {
            Some("failed") => return Ok(stored),
            Some("completed" | "cancelled") => {
                return Err(format!("ambiguous graph advanced: {stored}").into());
            }
            _ if Instant::now() >= deadline => {
                return Err(format!("recovery did not finish: {stored}\n{}", server.logs()).into());
            }
            _ => thread::sleep(Duration::from_millis(50)),
        }
    }
}

#[test]
fn real_native_workflow_process_restart_fences_unknown_dispatch_with_persisted_sqlite_and_journal()
-> Result<()> {
    for (round, domain) in DOMAINS.into_iter().enumerate() {
        let owner =
            LiveAgent::start_with_solver_hold("result_diagnostics", "run_operator_task_ir", "1")?;
        let peer = LiveAgent::start()?;
        let data = Scratch::new()?;
        let database = data.0.join("restart.sqlite3");
        let server = start_live_server_with_result_fixtures(
            Some(owner.port()),
            Some(peer.port()),
            Some(&data.0),
            false,
            Some(&database),
        )?;
        let input = scan_payload(domain);
        let expected =
            kyuubiki_engine::run_extract_operator(domain.diagnostic, input.clone(), json!({}))
                .unwrap();
        let (_, projects) = http_json(server.port, "/api/v1/projects", None)?;
        let (_, health) = http_json(server.port, "/api/health", None)?;
        let (job, point, original) = thread::scope(|scope| -> Result<(String, Value, Value)> {
            let markers = HoldMarkers(&owner.hold_path, &peer.hold_path);
            let blocker_job = format!("restart-blocker-{}", domain.name);
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
            let port = server.port;
            let blocker = scope.spawn(move || task_result(port, blocker));
            wait_for_task(&owner, &blocker_job, Some("result_diagnostics"))?;
            let mut req = serde_json::to_value(request(domain, input.clone(), false, false))?;
            req["graph"]["nodes"][1]["placement_tags"] = json!(["owned-live-owner"]);
            if round % 2 == 1 {
                req["graph"]["recovery_policy"] = json!({
                    "retry_safety":"checkpointed", "checkpoint":{
                        "operator_task_batch_checkpoint_verification_contract":"kyuubiki.operator_task_batch_checkpoint_verification/v1",
                        "status":"verified", "checkpoint_digest":"b".repeat(64)
                    }
                });
            }
            let (status, admitted) =
                http_json(server.port, "/api/v1/workflows/graph/jobs", Some(req))?;
            assert_eq!(status, 202, "{admitted}");
            let job = admitted["job"]["job_id"]
                .as_str()
                .ok_or("missing parent")?
                .to_string();
            fs::write(&owner.hold_path, &job)?;
            let first = blocker.join().map_err(|_| "blocker panicked")??;
            assert_retained(&first["result"]["result"], &expected, "blocker.summary");
            let point = wait_for_task(&owner, &job, Some("result_diagnostics"))?;
            assert_eq!(point["checkpoint"]["completed_steps"], 64);
            let (_, original) = http_json(server.port, &format!("/api/v1/jobs/{job}"), None)?;
            assert_eq!(original["result"]["recovery"]["state"], "running");
            assert_eq!(count(&owner)?, 2);
            assert_eq!(count(&peer)?, 0);
            // Kill the BEAM process with the native execution still held, not a terminal job.
            drop(server);
            wait_for_lifecycle(&owner, "accepting", 0)?;
            drop(markers);
            Ok((job, point, original))
        })?;
        let record = retained(&data.0, &job)?;
        assert_eq!(record["state"], "dispatch_boundary_unconfirmed");
        assert_eq!(record["request_id"], point["request_id"]);
        let attempt = record["attempt_id"].as_str().ok_or("missing attempt")?;
        let path = data
            .0
            .join("operator-task-dispatches")
            .join(format!("{attempt}.json"));
        let bytes = fs::read(&path)?;
        let restarted = start_live_server_with_result_fixtures(
            Some(owner.port()),
            Some(peer.port()),
            Some(&data.0),
            false,
            Some(&database),
        )?;
        let blocked = wait_blocked(&restarted, &job)?;
        let recovery = &blocked["result"]["recovery"];
        assert_eq!(recovery["state"], "recovery_blocked", "{blocked}");
        for field in [
            "generation",
            "attempt",
            "envelope_sha256",
            "checkpoint_digest",
            "retry_safety",
        ] {
            assert_eq!(
                recovery[field], original["result"]["recovery"][field],
                "{field}"
            );
        }
        assert_eq!(recovery["generation"], 1);
        assert_eq!(recovery["attempt"], 1);
        assert_eq!(recovery["envelope_retained"], true);
        assert_eq!(
            recovery["history"].as_array().unwrap().last().unwrap()["next_action"],
            "inspect_original_dispatch_before_explicit_resubmit"
        );
        assert!(
            blocked["job"]["message"]
                .as_str()
                .unwrap()
                .contains("workflow_dispatch_outcome_unconfirmed")
        );
        for node in ["diagnose", "quality", "objective", "decision"] {
            assert!(
                blocked["result"]["artifacts"]
                    .get(format!("{node}.summary"))
                    .is_none()
            );
        }
        assert_eq!(retained(&data.0, &job)?, record);
        assert_eq!(fs::read(&path)?, bytes);
        assert_eq!(count(&owner)?, 2);
        assert_eq!(count(&peer)?, 0);
        let (_, current_health) = http_json(restarted.port, "/api/health", None)?;
        assert_ne!(
            current_health["workflow_recovery"]["session_id"],
            health["workflow_recovery"]["session_id"]
        );
        assert_eq!(
            current_health["workflow_recovery"]["lease"]["lease_name"],
            health["workflow_recovery"]["lease"]["lease_name"]
        );
        assert_ne!(
            current_health["workflow_recovery"]["lease"]["owner_instance_id"],
            health["workflow_recovery"]["lease"]["owner_instance_id"]
        );
        assert_eq!(
            http_json(restarted.port, "/api/v1/projects", None)?.1,
            projects
        );
        let url = format!("http://127.0.0.1:{}", restarted.port);
        let mut client = ServiceHeadlessExecutor::try_new(&url).map_err(|error| error.message)?;
        let task_id = record["task_id"].as_str().ok_or("missing task")?;
        let digest = record["task_digest"].as_str().ok_or("missing digest")?;
        let inspected = client
            .inspect_operator_task_dispatch(task_id, digest)
            .map_err(|error| error.message)?;
        assert_eq!(inspected["retained_attempt_count"], 1);
        assert_eq!(
            inspected["attempts"][0]["observation"]["outcome"],
            "unknown"
        );
        let (status, receipt) = http_json(
            restarted.port,
            "/api/v1/operator-tasks/fetch-dispatch-result",
            Some(json!({"task_id":task_id,"task_digest":digest,"attempt_id":attempt})),
        )?;
        assert_eq!(status, 200, "{receipt}");
        assert_eq!(receipt["status"], "receipt_recovered", "{receipt}");
        assert_eq!(receipt["outcome"], "failed");
        assert_eq!(receipt["generation"], point["generation"]);
        assert_eq!(
            receipt["completion"]["failure_receipt"]["reason_code"],
            "cancelled"
        );
        assert_eq!(receipt["publication_performed"], false);
        assert_eq!(receipt["automatic_replay_authorized"], false);
        assert_eq!(
            http_json(restarted.port, &format!("/api/v1/jobs/{job}"), None)?.1,
            blocked
        );
        let error = client
            .execute_step("job_wait", 1, &json!({"job_id":job,"timeout_ms":100}))
            .expect_err("blocked graph cannot become a scientific result");
        assert!(
            error.message.starts_with("job_terminal_failed:"),
            "{error:?}"
        );
        assert_eq!(count(&owner)?, 2);
        assert_eq!(count(&peer)?, 0);

        // A new explicit submission succeeds; receipt readback never resumes the old graph.
        let report = cli_with_policy(
            &workflow(request(domain, input, false, false)),
            &url,
            true,
            true,
        )?;
        assert_eq!(report.status, "ok");
        let healthy = report.steps[0].result_preview["job_id"]
            .as_str()
            .ok_or("missing healthy parent")?;
        assert_ne!(healthy, job);
        let (_, fetched) = http_json(restarted.port, &format!("/api/v1/results/{healthy}"), None)?;
        assert_ready(domain, &fetched["result"]);
        assert_dispatches(&client, &data.0, healthy, domain, false)?;
        assert_eq!(count(&owner)? + count(&peer)?, 4);
        assert_eq!(retained(&data.0, &job)?, record);
        assert_eq!(fs::read(&path)?, bytes);
        assert_eq!(
            http_json(restarted.port, &format!("/api/v1/jobs/{job}"), None)?.1,
            blocked
        );
    }
    Ok(())
}
