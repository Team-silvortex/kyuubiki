use super::agent_support::{LiveAgent, wait_for_lifecycle};
use super::diagnostic_domains::DOMAINS;
use super::domain_diagnostic_cancellation::scan_payload;
use super::domain_diagnostics::{assert_dispatches, assert_ready, request};
use super::orchestra_cancel_routing::{HoldMarkers, active, task_result};
use super::orchestra_cancellation::wait_for_task;
use super::orchestra_completion::{assert_bar, bar_task, http_json};
use super::transport_output::cli_with_policy;
use super::transport_research::{assert_retained, workflow};
use super::write_ack_loss::count;
use super::{Scratch, start_live_server_with_state};
use kyuubiki_headless_sdk::{HeadlessExecutor, ServiceHeadlessExecutor};
use kyuubiki_protocol::compute_operator_task_digest;
use serde_json::{Value, json};
use std::{
    error::Error,
    fs,
    path::Path,
    thread,
    time::{Duration, Instant},
};

type Result<T> = std::result::Result<T, Box<dyn Error>>;

pub(super) fn place_task(mut task: Value, tag: &str) -> Value {
    task["runtime_hints"]["placement_tags"] = json!([tag]);
    task["integrity"]["task_digest"] = json!(compute_operator_task_digest(&task).unwrap());
    task
}

fn assert_cancelled_runtime(stored: &Value) {
    assert_eq!(stored["job"]["status"], "cancelled");
    // Graph recovery diagnostics are retained results, not completed scientific outputs.
    assert_eq!(stored["job"]["has_result"], true);
    let runtime = &stored["result"];
    assert_eq!(runtime["recovery"]["state"], "cancelled");
    for node in ["diagnose", "quality", "objective", "decision"] {
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
        assert!(
            !runtime["progress_events"]
                .as_array()
                .unwrap()
                .iter()
                .any(|event| event["node_id"] == node)
        );
    }
}

fn observation_records(root: &Path) -> Result<Option<Vec<Value>>> {
    let paths = fs::read_dir(root.join("operator-task-dispatches"))?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<std::io::Result<Vec<_>>>()?;
    // This live writer uses exclusive .json.next + rename. Do not decode a
    // partial stage or certify the previous published observation while pending.
    if paths
        .iter()
        .any(|path| path.to_string_lossy().ends_with(".json.next"))
    {
        return Ok(None);
    }
    let records = paths
        .into_iter()
        .map(|path| -> Result<Value> {
            if path.extension().is_none_or(|ext| ext != "json") {
                return Err(
                    format!("unexpected dispatch observation file: {}", path.display()).into(),
                );
            }
            let envelope: Value = serde_json::from_slice(&fs::read(&path)?).map_err(|error| {
                format!("published dispatch observation {}: {error}", path.display())
            })?;
            let payload = envelope
                .get("payload")
                .filter(|payload| payload.is_object())
                .ok_or_else(|| {
                    format!(
                        "published dispatch observation has no payload: {}",
                        path.display()
                    )
                })?;
            Ok(payload.clone())
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(Some(records))
}

fn await_observation(root: &Path, job: &str) -> Result<()> {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let snapshot = observation_records(root)?;
        let records = snapshot.as_deref().unwrap_or_default();
        let records = records
            .iter()
            .filter(|record| {
                record["task_id"]
                    .as_str()
                    .is_some_and(|id| id.starts_with(&format!("{job}:")))
            })
            .collect::<Vec<_>>();
        if snapshot.is_some() && cancelled_observation_ready(&records)? {
            return Ok(());
        }
        if Instant::now() >= deadline {
            return Err(format!("cancelled graph observation not retained: {records:?}").into());
        }
        thread::sleep(Duration::from_millis(10));
    }
}

#[test]
fn cancellation_observation_never_decodes_or_certifies_an_unpublished_stage() -> Result<()> {
    let data = Scratch::new()?;
    let root = data.0.join("operator-task-dispatches");
    fs::create_dir(&root)?;
    let published = root.join(format!("{}.json", "a".repeat(32)));
    let stage = root.join(format!("{}.json.next", "a".repeat(32)));
    let record = json!({"task_id":"job:diagnose", "state":"observed_failed"});
    fs::write(&published, serde_json::to_vec(&json!({"payload":record}))?)?;
    for partial in ["", "{", "{\"payload\":"] {
        fs::write(&stage, partial)?;
        assert!(observation_records(&data.0)?.is_none());
    }
    fs::remove_file(stage)?;
    assert_eq!(observation_records(&data.0)?, Some(vec![record]));
    for malformed in ["", "{", "{}", "{\"payload\":null}"] {
        fs::write(&published, malformed)?;
        let error = observation_records(&data.0).unwrap_err().to_string();
        assert!(error.contains("published dispatch observation"), "{error}");
    }
    Ok(())
}

fn cancelled_observation_ready(records: &[&Value]) -> Result<bool> {
    match records {
        [] => Ok(false),
        [record] => match record["state"].as_str() {
            Some("dispatch_boundary_unconfirmed") => Ok(false),
            Some("observed_failed") => Ok(true),
            _ => Err(format!("unexpected cancelled graph observation: {records:?}").into()),
        },
        _ => Err(format!("cancelled graph has extra dispatch attempts: {records:?}").into()),
    }
}

#[test]
fn cancellation_observation_waits_for_unconfirmed_boundary_and_rejects_unknown_outcome()
-> Result<()> {
    assert!(!cancelled_observation_ready(&[])?);
    let pending = json!({"state":"dispatch_boundary_unconfirmed"});
    assert!(!cancelled_observation_ready(&[&pending])?);
    let failed = json!({"state":"observed_failed"});
    assert!(cancelled_observation_ready(&[&failed])?);
    for state in [
        "outcome_unknown",
        "observed_executed",
        "not_dispatched",
        "dispatched",
        "",
    ] {
        assert!(cancelled_observation_ready(&[&json!({"state":state})]).is_err());
    }
    assert!(cancelled_observation_ready(&[&failed, &failed]).is_err());
    Ok(())
}

#[test]
fn real_cross_domain_workflow_job_cancellation_fences_scoring_and_preserves_peer_and_explicit_recovery()
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
    let mut rounds = 0_u64;

    for domain in DOMAINS {
        for skip_error in [false, true] {
            let input = scan_payload(domain);
            let expected =
                kyuubiki_engine::run_extract_operator(domain.diagnostic, input.clone(), json!({}))
                    .unwrap();
            thread::scope(|scope| -> Result<()> {
                // Occupy both slots before admission so the generated parent identity can be held.
                let markers = HoldMarkers(&owner.hold_path, &peer.hold_path);
                let owner_job = format!("graph-owner-{}-{skip_error}", domain.name);
                let peer_job = format!("graph-peer-{}-{skip_error}", domain.name);
                fs::write(&owner.hold_path, &owner_job)?;
                fs::write(&peer.hold_path, &peer_job)?;
                let blocker = place_task(
                    domain.task(
                        &owner_job,
                        domain.diagnostic,
                        "extract",
                        input.clone(),
                        json!({}),
                    ),
                    "owned-live-owner",
                );
                let first = scope.spawn(|| task_result(server.port, blocker));
                wait_for_task(&owner, &owner_job, Some("result_diagnostics"))?;
                let peer_task = place_task(bar_task(&peer_job, 0.01), "owned-live-peer");
                let second = scope.spawn(|| task_result(server.port, peer_task));
                let bystander = active(&peer, &peer_job)?;

                let mut req =
                    serde_json::to_value(request(domain, input.clone(), false, skip_error))?;
                req["graph"]["nodes"][1]["placement_tags"] = json!(["owned-live-owner"]);
                let (status, admitted) =
                    http_json(server.port, "/api/v1/workflows/graph/jobs", Some(req))?;
                assert_eq!(status, 202, "{admitted}");
                let job = admitted["job"]["job_id"]
                    .as_str()
                    .ok_or("missing parent job")?;
                fs::write(&owner.hold_path, job)?;
                let first = first.join().map_err(|_| "owner blocker panicked")??;
                assert_retained(&first["result"]["result"], &expected, "owner.summary");
                let held = wait_for_task(&owner, job, Some("result_diagnostics"))?;
                assert_eq!(held["checkpoint"]["completed_steps"], 64);
                assert_eq!(held["cancel_requested"], false);
                assert_eq!(count(&owner)? + count(&peer)?, rounds * 6 + 3);

                let (status, cancelled) = http_json(
                    server.port,
                    &format!("/api/v1/jobs/{job}/cancel"),
                    Some(json!({})),
                )?;
                assert_eq!(status, 200, "{cancelled}");
                assert_eq!(cancelled["job"]["status"], "cancelled");
                assert_eq!(cancelled["cancellation"]["status"], "requested");
                assert_eq!(cancelled["cancellation"]["target_count"], 1);
                assert_eq!(cancelled["cancellation"]["registered_count"], 1);
                assert_eq!(
                    cancelled["cancellation"]["targets"][0]["agent_id"],
                    "owned-live-agent"
                );
                assert_eq!(
                    cancelled["cancellation"]["execution_terminal_confirmed"],
                    false
                );
                wait_for_lifecycle(&owner, "accepting", 0)?;
                let owner_state =
                    owner.request("graph-cancel-cause", "describe_agent", json!({}))?;
                let failure = owner_state["result"]["watchdog"]["recent_failures"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|failure| failure["request_id"] == held["request_id"])
                    .ok_or("cancelled diagnostic cause missing")?;
                assert_eq!(failure["reason_code"], "cancelled");
                assert_eq!(failure["generation"], held["generation"]);
                assert_eq!(failure["job_id"], job);
                assert_eq!(failure["method"], "run_operator_task_ir");
                assert!(
                    failure["message"]
                        .as_str()
                        .unwrap()
                        .contains("result_diagnostics after 64 steps")
                );
                let peer_after = active(&peer, &peer_job)?;
                assert_eq!(peer_after["cancel_requested"], false);
                assert_eq!(peer_after["generation"], bystander["generation"]);
                assert_eq!(peer_after["request_id"], bystander["request_id"]);
                await_observation(&data.0, job)?;
                assert_dispatches(&client, &data.0, job, domain, true)?;
                let (_, stored) = http_json(server.port, &format!("/api/v1/jobs/{job}"), None)?;
                assert_cancelled_runtime(&stored);
                let mut waiting =
                    ServiceHeadlessExecutor::try_new(&url).map_err(|error| error.message)?;
                let failure = waiting
                    .execute_step("job_wait", 1, &json!({"job_id":job,"timeout_ms":100}))
                    .expect_err(
                        "retained recovery diagnostics cannot satisfy a completed-job wait",
                    );
                assert!(
                    failure.message.starts_with("job_terminal_cancelled:"),
                    "{failure:?}"
                );
                assert_eq!(
                    count(&owner)? + count(&peer)?,
                    rounds * 6 + 3,
                    "cancelled diagnosis must not dispatch scoring or replay"
                );

                drop(markers);
                assert_bar(&second.join().map_err(|_| "peer blocker panicked")??);
                wait_for_lifecycle(&peer, "accepting", 0)?;
                // A job-wide broadcast would poison this same parent identity on the bystander.
                let task = domain.task(
                    &format!("peer-after-{job}"),
                    domain.diagnostic,
                    "extract",
                    input.clone(),
                    json!({}),
                );
                let direct = peer.request(
                    "graph-peer-same-job",
                    "run_operator_task_ir",
                    json!({"task_ir":task,"mode":"execute","job_id":job}),
                )?;
                assert_eq!(direct["ok"], true, "{direct}");
                assert_eq!(direct["result"]["operator_task_ir_status"], "executed");
                assert_retained(&direct["result"]["result"], &expected, "peer.summary");
                wait_for_lifecycle(&peer, "accepting", 0)?;

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
                    .ok_or("missing recovery job")?;
                // CLI previews compact large record arrays; validate the full retained result.
                let (_, result) = http_json(server.port, &format!("/api/v1/results/{next}"), None)?;
                let recovered = &result["result"];
                assert_ready(domain, recovered);
                assert_retained(
                    &recovered["artifacts"]["raw.payload"],
                    &input,
                    "raw.payload",
                );
                assert_ne!(next, job, "recovery must be an explicit new graph job");
                assert_dispatches(&client, &data.0, next, domain, false)?;
                assert_dispatches(&client, &data.0, job, domain, true)?;
                let (_, retained) = http_json(server.port, &format!("/api/v1/jobs/{job}"), None)?;
                assert_cancelled_runtime(&retained);
                assert_eq!(
                    http_json(server.port, "/api/v1/projects", None)?.1,
                    projects
                );
                assert_eq!(
                    count(&owner)? + count(&peer)?,
                    (rounds + 1) * 6,
                    "sixty executions must not replay"
                );
                Ok(())
            })?;
            rounds += 1;
        }
    }
    assert_eq!(rounds, 10);
    assert_eq!(count(&owner)? + count(&peer)?, 60);
    wait_for_lifecycle(&owner, "accepting", 0)?;
    wait_for_lifecycle(&peer, "accepting", 0)?;
    Ok(())
}
