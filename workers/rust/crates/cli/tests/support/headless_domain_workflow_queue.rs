use super::agent_support::{LiveAgent, wait_for_lifecycle};
use super::diagnostic_domains::{DOMAINS, Domain};
use super::domain_diagnostics::{assert_dispatches, assert_ready, request};
use super::domain_workflow_cancellation::place_task;
use super::orchestra_cancel_routing::{HoldMarkers, active, task_result};
use super::orchestra_completion::{assert_bar, bar_task, http_json};
use super::transport_output::cli_with_policy;
use super::transport_research::{assert_retained, workflow};
use super::write_ack_loss::count;
use super::{Scratch, start_live_server_with_state};
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

fn await_gate(port: u16, queued: u64, leases: u64) -> Result<()> {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let (status, health) = http_json(port, "/api/health", None)?;
        assert_eq!(status, 200, "{health}");
        let gate = &health["agent_execution_gate"];
        if gate["queued_request_count"] == queued && gate["active_lease_count"] == leases {
            return Ok(());
        }
        if Instant::now() >= deadline {
            return Err(format!("expected queue {queued}, leases {leases}: {gate}").into());
        }
        thread::sleep(Duration::from_millis(10));
    }
}

fn queued_dispatches(
    client: &ServiceHeadlessExecutor,
    root: &Path,
    job: &str,
    domain: Domain,
    quality_stage: bool,
) -> Result<Vec<Value>> {
    let prefix = format!("{job}:");
    let records = fs::read_dir(root.join("operator-task-dispatches"))?
        .map(|entry| -> Result<Value> {
            let record: Value = serde_json::from_slice(&fs::read(entry?.path())?)?;
            Ok(record["payload"].clone())
        })
        .collect::<Result<Vec<_>>>()?;
    let records = records
        .into_iter()
        .filter(|record| {
            record["task_id"]
                .as_str()
                .is_some_and(|id| id.starts_with(&prefix))
        })
        .collect::<Vec<_>>();
    assert_eq!(
        records.len(),
        usize::from(quality_stage),
        "queued work must not dispatch"
    );
    if quality_stage {
        let record = &records[0];
        assert_eq!(
            record["task_id"],
            format!("{job}:diagnose:{}", domain.diagnostic)
        );
        assert_eq!(record["operator_id"], domain.diagnostic);
        assert_eq!(record["program_id"], domain.diagnostic);
        assert_eq!(record["state"], "observed_executed");
        let inspection = client
            .inspect_operator_task_dispatch(
                record["task_id"].as_str().unwrap(),
                record["task_digest"].as_str().unwrap(),
            )
            .map_err(|error| error.message)?;
        assert_eq!(inspection["attempts"].as_array().unwrap().len(), 1);
        assert_eq!(
            inspection["attempts"][0]["attempt_id"],
            record["attempt_id"]
        );
        assert_eq!(
            inspection["attempts"][0]["request_id"],
            record["request_id"]
        );
        assert_eq!(inspection["attempts"][0]["state"], "observed_executed");
        assert_eq!(inspection["automatic_replay_authorized"], false);
        assert_eq!(inspection["terminal_result_available"], false);
    }
    Ok(records)
}

fn assert_cancelled(stored: &Value, quality_stage: bool) {
    assert_eq!(stored["job"]["status"], "cancelled");
    assert_eq!(stored["job"]["has_result"], true);
    let runtime = &stored["result"];
    assert_eq!(runtime["recovery"]["state"], "cancelled");
    let progress = runtime["progress_events"].as_array().unwrap();
    assert_eq!(
        progress
            .iter()
            .any(|event| { event["node_id"] == "diagnose" && event["status"] == "completed" }),
        quality_stage
    );
    for node in ["quality", "objective", "decision"] {
        assert!(
            !runtime["completed_nodes"]
                .as_array()
                .unwrap()
                .contains(&json!(node))
        );
        assert!(
            runtime["artifacts"]
                .get(format!("{node}.summary"))
                .is_none()
        );
        assert!(!progress.iter().any(|event| event["node_id"] == node));
    }
}

fn run_queued_cases(quality_stage: bool) -> Result<()> {
    let mut owner = LiveAgent::start()?;
    let mut peer = LiveAgent::start()?;
    let data = Scratch::new()?;
    let server =
        start_live_server_with_state(Some(owner.port()), Some(peer.port()), Some(&data.0))?;
    let url = format!("http://127.0.0.1:{}", server.port);
    owner.restart_with_orchestrator(&url)?;
    peer.restart_with_orchestrator(&url)?;
    let client = ServiceHeadlessExecutor::try_new(&url).map_err(|error| error.message)?;
    let (_, projects) = http_json(server.port, "/api/v1/projects", None)?;
    let mut rounds = 0_u64;

    for domain in DOMAINS {
        for skip_error in [false, true] {
            let input = domain.raw();
            thread::scope(|scope| -> Result<()> {
                let markers = HoldMarkers(&owner.hold_path, &peer.hold_path);
                let peer_job = format!("queue-peer-{}-{skip_error}", domain.name);
                fs::write(&peer.hold_path, &peer_job)?;
                let peer_task = place_task(bar_task(&peer_job, 0.01), "owned-live-peer");
                let bystander_task = scope.spawn(|| task_result(server.port, peer_task));
                let bystander = active(&peer, &peer_job)?;
                let owner_job = format!("queue-owner-{}-{skip_error}", domain.name);
                let owner_task = if quality_stage {
                    None
                } else {
                    fs::write(&owner.hold_path, &owner_job)?;
                    let task = place_task(bar_task(&owner_job, 0.01), "owned-live-owner");
                    let held = scope.spawn(|| task_result(server.port, task));
                    active(&owner, &owner_job)?;
                    Some(held)
                };

                let mut req =
                    serde_json::to_value(request(domain, input.clone(), false, skip_error))?;
                for node in req["graph"]["nodes"].as_array_mut().unwrap() {
                    match node["id"].as_str().unwrap() {
                        "diagnose" => node["placement_tags"] = json!(["owned-live-owner"]),
                        "quality" => {
                            node["placement_tags"] = json!(["owned-live-peer"]);
                            if skip_error {
                                node["config"]["on_error"] = json!("skip");
                            }
                        }
                        _ => {}
                    }
                }
                let (status, admitted) =
                    http_json(server.port, "/api/v1/workflows/graph/jobs", Some(req))?;
                assert_eq!(status, 202, "{admitted}");
                let job = admitted["job"]["job_id"]
                    .as_str()
                    .ok_or("missing queued graph job")?;
                let leases = if quality_stage { 1 } else { 2 };
                await_gate(server.port, 1, leases)?;
                assert_eq!(count(&owner)? + count(&peer)?, rounds * 5 + 2);
                let records = queued_dispatches(&client, &data.0, job, domain, quality_stage)?;

                let (status, cancelled) = http_json(
                    server.port,
                    &format!("/api/v1/jobs/{job}/cancel"),
                    Some(json!({})),
                )?;
                assert_eq!(status, 200, "{cancelled}");
                assert_eq!(cancelled["job"]["status"], "cancelled");
                let cancellation = &cancelled["cancellation"];
                assert_eq!(cancellation["status"], "cancelled_before_dispatch");
                assert_eq!(cancellation["queued_cancelled_count"], 1);
                assert_eq!(cancellation["reserved_cancelled_count"], 0);
                assert_eq!(cancellation["target_count"], 0);
                assert_eq!(cancellation["registered_count"], 0);
                assert_eq!(cancellation["targets"], json!([]));
                assert_eq!(cancellation["execution_terminal_confirmed"], false);
                await_gate(server.port, 0, leases)?;
                let after = active(&peer, &peer_job)?;
                assert_eq!(after["request_id"], bystander["request_id"]);
                assert_eq!(after["generation"], bystander["generation"]);
                assert_eq!(after["cancel_requested"], false);
                if !quality_stage {
                    assert_eq!(active(&owner, &owner_job)?["cancel_requested"], false);
                }
                let (_, retained) = http_json(server.port, &format!("/api/v1/jobs/{job}"), None)?;
                assert_cancelled(&retained, quality_stage);
                let mut waiting =
                    ServiceHeadlessExecutor::try_new(&url).map_err(|error| error.message)?;
                let error = waiting
                    .execute_step("job_wait", 1, &json!({"job_id":job,"timeout_ms":100}))
                    .expect_err("retained diagnostics do not complete a cancelled graph");
                assert!(
                    error.message.starts_with("job_terminal_cancelled:"),
                    "{error:?}"
                );
                assert_eq!(count(&owner)? + count(&peer)?, rounds * 5 + 2);

                drop(markers);
                assert_bar(
                    &bystander_task
                        .join()
                        .map_err(|_| "peer holder panicked")??,
                );
                if let Some(task) = owner_task {
                    assert_bar(&task.join().map_err(|_| "owner holder panicked")??);
                }
                await_gate(server.port, 0, 0)?;
                wait_for_lifecycle(&owner, "accepting", 0)?;
                wait_for_lifecycle(&peer, "accepting", 0)?;
                let agent = if quality_stage { &peer } else { &owner };
                let task = domain.task(
                    &format!("identity-after-{job}"),
                    domain.diagnostic,
                    "extract",
                    input.clone(),
                    json!({}),
                );
                let direct = agent.request(
                    "queue-reuse-parent",
                    "run_operator_task_ir",
                    json!({"task_ir":task,"mode":"execute","job_id":job}),
                )?;
                assert_eq!(direct["ok"], true, "{direct}");
                assert_eq!(direct["result"]["operator_task_ir_status"], "executed");
                let expected = kyuubiki_engine::run_extract_operator(
                    domain.diagnostic,
                    input.clone(),
                    json!({}),
                )
                .unwrap();
                assert_retained(&direct["result"]["result"], &expected, "identity.summary");
                wait_for_lifecycle(agent, "accepting", 0)?;

                let report = cli_with_policy(
                    &workflow(request(domain, input.clone(), false, skip_error)),
                    &url,
                    true,
                    true,
                )?;
                assert_eq!(report.status, "ok");
                assert_eq!(report.executed_step_count, 3);
                let next = report.steps[0].result_preview["job_id"]
                    .as_str()
                    .ok_or("missing explicit recovery job")?;
                assert_ne!(next, job);
                let (_, recovered) =
                    http_json(server.port, &format!("/api/v1/results/{next}"), None)?;
                assert_ready(domain, &recovered["result"]);
                assert_retained(
                    &recovered["result"]["artifacts"]["raw.payload"],
                    &input,
                    "raw.payload",
                );
                assert_dispatches(&client, &data.0, next, domain, false)?;
                assert_eq!(
                    queued_dispatches(&client, &data.0, job, domain, quality_stage)?,
                    records
                );
                let (_, unchanged) = http_json(server.port, &format!("/api/v1/jobs/{job}"), None)?;
                assert_eq!(
                    unchanged, retained,
                    "cancelled graph receipt changed after recovery"
                );
                assert_eq!(
                    http_json(server.port, "/api/v1/projects", None)?.1,
                    projects
                );
                assert_eq!(
                    count(&owner)? + count(&peer)?,
                    (rounds + 1) * 5,
                    "queued cancellation must not execute or replay"
                );
                Ok(())
            })?;
            rounds += 1;
        }
    }
    assert_eq!(rounds, 10);
    assert_eq!(count(&owner)? + count(&peer)?, 50);
    await_gate(server.port, 0, 0)?;
    Ok(())
}

#[test]
fn real_cross_domain_queued_diagnostic_cancellation_never_dispatches_and_recovers_explicitly()
-> Result<()> {
    run_queued_cases(false)
}

#[test]
fn real_cross_domain_queued_quality_cancellation_retains_diagnosis_without_dispatch_or_replay()
-> Result<()> {
    run_queued_cases(true)
}
