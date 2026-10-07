use super::ack_loss_proxy::AckLossProxy;
use super::agent_support::{LiveAgent, wait_for_lifecycle};
use super::orchestra_completion::{bar_task, batch, http_json};
use super::{
    Scratch, parse_json_output, run_headless_command, start_live_server_with_state, write_temp_json,
};
use kyuubiki_headless_sdk::{
    HeadlessExecutionBatch, HeadlessExecutor, ServiceHeadlessExecutor, execute_batch_with_executor,
};
use serde_json::{Value, json};
use std::{error::Error, fs};

pub(super) fn document(action: &str, payload: Value) -> HeadlessExecutionBatch {
    let mut document = batch(
        bar_task("unused-write-ack-task", 0.01),
        Some("must-not-follow-lost-ack"),
    );
    document.steps[0].action = action.into();
    document.steps[0].payload = payload;
    document
}

pub(super) fn assert_failure(report: &Value) {
    assert_eq!(report["status"], "failed");
    assert_eq!(report["executed_step_count"], 0);
    assert_eq!(report["steps"].as_array().unwrap().len(), 1);
    let failure = &report["execution_summary"]["failure"];
    assert_eq!(
        failure["error_code"],
        "kyuubiki.headless.service_request_outcome_unknown"
    );
    assert_eq!(failure["stage"], "transport");
    assert_eq!(failure["retryable"], false);
    assert_eq!(failure["retry_strategy"], "none");
    assert!(
        report["steps"][0]["result_preview"]
            .get("project_id")
            .is_none()
    );
    assert!(report["steps"][0]["result_preview"].get("job_id").is_none());
}

pub(super) fn run_cli(
    document: &HeadlessExecutionBatch,
    proxy: &AckLossProxy,
) -> Result<(), Box<dyn Error>> {
    let source = write_temp_json("lost-real-write-ack", &serde_json::to_value(document)?);
    let report_path = source.parent().unwrap().join("report.json");
    let url = format!("http://127.0.0.1:{}", proxy.port);
    let output = run_headless_command(&[
        "run",
        source.to_str().unwrap(),
        "--execute",
        "--executor",
        "service",
        "--api-base-url",
        &url,
        "--json",
        "--report-out",
        report_path.to_str().unwrap(),
    ]);
    assert!(!output.status.success());
    let report = parse_json_output(&output);
    assert_failure(&report);
    assert_eq!(
        report,
        serde_json::from_slice::<Value>(&fs::read(report_path)?)?
    );
    let error: Value = serde_json::from_slice(&output.stderr)?;
    assert_eq!(error["error"]["code"], "headless_execution_failed");
    assert_eq!(error["error"]["retryable"], false);
    Ok(())
}

pub(super) fn count(agent: &LiveAgent) -> Result<u64, Box<dyn Error>> {
    Ok(agent.request("write-ack-count", "describe_agent", json!({}))?
        ["result"]["watchdog"]["total_started_execution_count"].as_u64().unwrap())
}

#[test]
fn real_project_commit_with_lost_ack_is_unknown_without_replay_or_later_writes()
-> Result<(), Box<dyn Error>> {
    let owner = LiveAgent::start()?;
    let peer = LiveAgent::start()?;
    let data = Scratch::new()?;
    let server =
        start_live_server_with_state(Some(owner.port()), Some(peer.port()), Some(&data.0))?;
    let proxy = AckLossProxy::http_post(server.port, "/api/v1/projects")?;
    let mut executor = ServiceHeadlessExecutor::new(&format!("http://127.0.0.1:{}", proxy.port));
    let first = document("project_create", json!({"name":"owned-sdk-ack-loss"}));
    let report = execute_batch_with_executor(&first, &mut executor, false, false);
    assert_failure(&serde_json::to_value(report)?);
    assert_eq!(
        proxy.capture().discarded[0]["project"]["name"],
        "owned-sdk-ack-loss"
    );
    // A distinct explicit CLI experiment is not a replay of the unknown SDK write.
    run_cli(
        &document("project_create", json!({"name":"owned-cli-ack-loss"})),
        &proxy,
    )?;
    let captured = proxy.capture();
    assert!(captured.errors.is_empty(), "{captured:?}");
    assert_eq!(captured.requests.len(), 2);
    assert_eq!(captured.discarded.len(), 2);
    let projects = http_json(server.port, "/api/v1/projects", None)?.1;
    let mut names = projects["projects"]
        .as_array()
        .unwrap()
        .iter()
        .map(|project| project["name"].as_str().unwrap())
        .collect::<Vec<_>>();
    names.sort_unstable();
    assert_eq!(names, ["owned-cli-ack-loss", "owned-sdk-ack-loss"]);
    assert_eq!(
        http_json(server.port, "/api/v1/jobs", None)?.1["jobs"],
        json!([])
    );
    assert_eq!(count(&owner)? + count(&peer)?, 0);
    assert!(!data.0.join("operator-task-dispatches").exists());
    Ok(())
}

#[test]
fn real_job_acceptance_with_lost_ack_computes_once_per_explicit_request()
-> Result<(), Box<dyn Error>> {
    let owner = LiveAgent::start()?;
    let peer = LiveAgent::start()?;
    let data = Scratch::new()?;
    let server =
        start_live_server_with_state(Some(owner.port()), Some(peer.port()), Some(&data.0))?;
    let proxy = AckLossProxy::http_post(server.port, "/api/v1/fem/axial-bar/jobs")?;
    let url = format!("http://127.0.0.1:{}", proxy.port);
    let payload = json!({"model":{"length":1.0,"area":0.01,"youngs_modulus_gpa":210,"elements":4,"tip_force":1000}});
    let original = document("solve_bar_1d", payload.clone());
    let report = execute_batch_with_executor(
        &original,
        &mut ServiceHeadlessExecutor::new(&url),
        false,
        false,
    );
    assert_failure(&serde_json::to_value(report)?);
    // Inspect the real discarded reply only inside this test, not as a recovery API.
    let first_id = proxy.capture().discarded[0]["job"]["job_id"]
        .as_str()
        .unwrap()
        .to_owned();
    let mut direct = ServiceHeadlessExecutor::new(&format!("http://127.0.0.1:{}", server.port));
    let wait = direct
        .execute_step(
            "job_wait",
            1,
            &json!({"job_id":first_id,"timeout_ms":5000,"interval_ms":10}),
        )
        .map_err(|error| error.message)?;
    assert_eq!(wait.result["status"], "completed");
    wait_for_lifecycle(&owner, "accepting", 0)?;
    wait_for_lifecycle(&peer, "accepting", 0)?;
    assert_eq!(count(&owner)? + count(&peer)?, 1);

    let mut second_payload = payload;
    second_payload["model"]["tip_force"] = json!(2000);
    run_cli(&document("solve_bar_1d", second_payload), &proxy)?;
    let captured = proxy.capture();
    assert!(captured.errors.is_empty(), "{captured:?}");
    assert_eq!(captured.requests.len(), 2);
    assert_eq!(captured.discarded.len(), 2);
    let second_id = captured.discarded[1]["job"]["job_id"].as_str().unwrap();
    assert_ne!(first_id, second_id);
    direct
        .execute_step(
            "job_wait",
            1,
            &json!({"job_id":second_id,"timeout_ms":5000,"interval_ms":10}),
        )
        .map_err(|error| error.message)?;
    for (id, force) in [(first_id.as_str(), 1000.0), (second_id, 2000.0)] {
        let result = direct
            .execute_step("result_fetch", 1, &json!({"job_id":id}))
            .map_err(|error| error.message)?;
        let displacement = result.result["result"]["tip_displacement"]
            .as_f64()
            .unwrap();
        assert!((displacement / (force / (210e9 * 0.01)) - 1.0).abs() < 1e-12);
    }
    wait_for_lifecycle(&owner, "accepting", 0)?;
    wait_for_lifecycle(&peer, "accepting", 0)?;
    assert_eq!(
        count(&owner)? + count(&peer)?,
        2,
        "lost job acknowledgement replayed calculation"
    );
    assert_eq!(
        http_json(server.port, "/api/v1/jobs", None)?.1["jobs"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    assert_eq!(
        http_json(server.port, "/api/v1/projects", None)?.1["projects"],
        json!([])
    );
    Ok(())
}
