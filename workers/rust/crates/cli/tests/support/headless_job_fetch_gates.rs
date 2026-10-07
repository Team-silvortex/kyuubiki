use super::ack_loss_proxy::AckLossProxy;
use super::agent_support::{LiveAgent, wait_for_lifecycle};
use super::orchestra_completion::http_json;
use super::write_ack_loss::{count, document};
use super::{
    Scratch, parse_json_output, run_headless_command, start_live_server_with_result_fixtures,
    start_live_server_with_state, write_temp_json,
};
use kyuubiki_headless_sdk::{
    HeadlessExecutionBatch, HeadlessExecutor, ServiceHeadlessExecutor, execute_batch_with_executor,
};
use serde_json::{Value, json};
use std::{error::Error, fs};

type Result<T> = std::result::Result<T, Box<dyn Error>>;

fn assert_failure(report: &Value, context: bool) {
    assert_eq!(report["status"], "failed");
    assert_eq!(report["executed_step_count"], 0);
    assert_eq!(report["steps"].as_array().unwrap().len(), 1);
    let failure = &report["execution_summary"]["failure"];
    assert_eq!(
        failure["category"],
        if context {
            "contract_failure"
        } else {
            "job_receipt_invalid"
        }
    );
    assert_eq!(
        failure["stage"],
        if context {
            "validation"
        } else {
            "job_observation"
        }
    );
    assert_eq!(failure["retryable"], false);
    assert_eq!(failure["retry_strategy"], "none");
    assert!(
        failure["message"]
            .as_str()
            .unwrap()
            .starts_with(if context {
                "job_read_invalid:"
            } else {
                "job_receipt_invalid:"
            })
    );
    for key in ["job_id", "job", "result", "raw"] {
        assert!(report["steps"][0]["result_preview"].get(key).is_none());
    }
}

fn cli_report(batch: &HeadlessExecutionBatch, port: u16, failure: Option<bool>) -> Result<Value> {
    let source = write_temp_json("real-job-observation", &serde_json::to_value(batch)?);
    let report_path = source.parent().unwrap().join("report.json");
    let url = format!("http://127.0.0.1:{port}");
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
    assert_eq!(output.status.success(), failure.is_none());
    let report = parse_json_output(&output);
    assert_eq!(
        report,
        serde_json::from_slice::<Value>(&fs::read(report_path)?)?
    );
    if let Some(context) = failure {
        assert_failure(&report, context);
        let error: Value = serde_json::from_slice(&output.stderr)?;
        assert_eq!(error["error"]["code"], "headless_execution_failed");
        assert_eq!(error["error"]["retryable"], false);
    } else {
        assert_eq!(report["status"], "ok");
        assert_eq!(report["executed_step_count"], 1);
        assert!(output.stderr.is_empty());
    }
    Ok(report)
}

#[test]
fn real_corrupted_job_observations_stop_sdk_and_cli_without_reexecution() -> Result<()> {
    let owner = LiveAgent::start()?;
    let peer = LiveAgent::start()?;
    let data = Scratch::new()?;
    let server =
        start_live_server_with_state(Some(owner.port()), Some(peer.port()), Some(&data.0))?;
    let url = format!("http://127.0.0.1:{}", server.port);
    let mut direct = ServiceHeadlessExecutor::new(&url);
    let project = direct
        .execute_step(
            "project_create",
            1,
            &json!({"name":"job-observation-parent"}),
        )
        .map_err(|e| e.message)?
        .result;
    let model = direct.execute_step("model_create", 1, &json!({"project_id":project["project_id"],
        "name":"job-observation-bar","kind":"axial_bar_1d","payload":{
        "model_schema_version":"kyuubiki.model/v1","kind":"axial_bar_1d","name":"bar","material":"steel",
        "length":1.0,"area":0.01,"youngs_modulus_gpa":210.0,"elements":4,"tip_force":1000.0}}))
        .map_err(|e|e.message)?.result;
    let version = &model["latest_version_id"];
    let completed = direct
        .execute_step(
            "solve_and_wait_from_model_version",
            1,
            &json!({"model_version_id":version,
        "project_id":project["project_id"],"endpoints":[format!("127.0.0.1:{}",owner.port())],
        "timeout_ms":5000,"interval_ms":10}),
        )
        .map_err(|e| e.message)?
        .result;
    let id = completed["job_id"].as_str().unwrap();
    let path = format!("/api/v1/jobs/{id}");
    let result_path = format!("/api/v1/results/{id}");
    let original = http_json(server.port, &path, None)?.1;
    let original_result = http_json(server.port, &result_path, None)?.1;
    let payload = json!({"job_id":id,"projectId":project["project_id"],"modelVersionId":version,
        "simulationCaseId":original["job"]["simulation_case_id"]});
    for missing in [
        "job_id",
        "status",
        "project_id",
        "model_version_id",
        "simulation_case_id",
    ] {
        let context = !matches!(missing, "job_id" | "status");
        let proxy =
            AckLossProxy::http_without_record_identity(server.port, "GET", &path, "job", missing)?;
        let batch = document("job_fetch", payload.clone());
        let report = execute_batch_with_executor(
            &batch,
            &mut ServiceHeadlessExecutor::new(&format!("http://127.0.0.1:{}", proxy.port)),
            false,
            false,
        );
        assert_failure(&serde_json::to_value(report)?, context);
        let cli = matches!(missing, "job_id" | "model_version_id");
        if cli {
            cli_report(&batch, proxy.port, Some(context))?;
        }
        let captured = proxy.capture();
        assert!(captured.errors.is_empty(), "{captured:?}");
        let expected_count = if cli { 2 } else { 1 };
        assert_eq!(captured.requests.len(), expected_count);
        assert_eq!(captured.discarded.len(), expected_count);
        assert_eq!(captured.replies.len(), expected_count);
        for (before, after) in captured.discarded.iter().zip(&captured.replies) {
            assert_eq!(*before, original);
            let mut expected = original.clone();
            expected["job"].as_object_mut().unwrap().remove(missing);
            assert_eq!(*after, expected);
        }
    }
    let observed = direct
        .execute_step("job_fetch", 1, &payload)
        .map_err(|e| e.message)?
        .result;
    assert_eq!(observed["raw"], original);
    assert_eq!(observed["job"]["status"], "completed");
    assert_eq!(observed["result"], completed["result"]["result"]);
    let displacement = observed["result"]["tip_displacement"].as_f64().unwrap();
    assert!((displacement / (1000.0 / (210e9 * 0.01)) - 1.0).abs() < 1e-12);
    let mut read_only = document("job_fetch", payload);
    read_only.steps.truncate(1);
    let report = cli_report(&read_only, server.port, None)?;
    assert_eq!(report["steps"][0]["result_preview"]["status"], "completed");
    assert_eq!(http_json(server.port, &path, None)?.1, original);
    assert_eq!(
        http_json(server.port, &result_path, None)?.1,
        original_result
    );
    wait_for_lifecycle(&owner, "accepting", 0)?;
    wait_for_lifecycle(&peer, "accepting", 0)?;
    assert_eq!(count(&owner)? + count(&peer)?, 1);
    let projects = http_json(server.port, "/api/v1/projects", None)?.1;
    assert_eq!(projects["projects"].as_array().unwrap().len(), 1);
    assert!(!projects.to_string().contains("must-not-follow-lost-ack"));
    assert_eq!(
        http_json(server.port, "/api/v1/jobs", None)?.1["jobs"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    Ok(())
}

#[test]
fn real_stored_incomplete_jobs_remain_inspectable_but_not_accepted_results() -> Result<()> {
    let owner = LiveAgent::start()?;
    let peer = LiveAgent::start()?;
    let data = Scratch::new()?;
    let server = start_live_server_with_result_fixtures(
        Some(owner.port()),
        Some(peer.port()),
        Some(&data.0),
        true,
    )?;
    let mut direct = ServiceHeadlessExecutor::new(&format!("http://127.0.0.1:{}", server.port));
    let before_projects = http_json(server.port, "/api/v1/projects", None)?.1;
    let before_jobs = http_json(server.port, "/api/v1/jobs", None)?.1;
    let before_results = http_json(server.port, "/api/v1/results", None)?.1;
    for status in ["queued", "solving", "failed", "cancelled"] {
        let id = format!("owned-result-gate-{status}");
        let path = format!("/api/v1/jobs/{id}");
        let original = http_json(server.port, &path, None)?.1;
        let payload = json!({"job_id":id,"project_id":"owned-result-fixture",
            "model_version_id":null,"simulation_case_id":"case"});
        let observed = direct
            .execute_step("job_fetch", 1, &payload)
            .map_err(|e| e.message)?
            .result;
        assert_eq!(observed["status"], status);
        assert_eq!(observed["raw"], original);
        assert_eq!(observed["job"]["has_result"], true);
        assert_eq!(observed["result"]["artifacts"]["partial"]["fixture"], true);
        let mut read_only = document("job_fetch", payload.clone());
        read_only.steps.truncate(1);
        let report = cli_report(&read_only, server.port, None)?;
        assert_eq!(report["steps"][0]["result_preview"]["status"], status);
        // Inspection of retained diagnostic data is not successful result admission.
        let error = direct
            .execute_step("result_fetch", 1, &payload)
            .unwrap_err();
        assert!(error.message.starts_with("job_result_unavailable:"));
        assert_eq!(http_json(server.port, &path, None)?.1, original);
    }
    assert_eq!(
        http_json(server.port, "/api/v1/projects", None)?.1,
        before_projects
    );
    assert_eq!(http_json(server.port, "/api/v1/jobs", None)?.1, before_jobs);
    assert_eq!(
        http_json(server.port, "/api/v1/results", None)?.1,
        before_results
    );
    assert_eq!(count(&owner)? + count(&peer)?, 0);
    assert!(!data.0.join("operator-task-dispatches").exists());
    Ok(())
}
