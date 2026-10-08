use super::ack_loss_proxy::AckLossProxy;
use super::agent_support::{LiveAgent, wait_for_lifecycle};
use super::orchestra_completion::http_json;
use super::write_ack_loss::{count, document};
use super::{
    Scratch, parse_json_output, run_headless_command, start_live_server_with_state, write_temp_json,
};
use kyuubiki_headless_sdk::{
    HeadlessExecutionBatchStep, HeadlessExecutor, ServiceHeadlessExecutor,
    execute_batch_with_executor, find_action_contract,
};
use serde_json::{Value, json};
use std::{error::Error, fs};

type Result<T> = std::result::Result<T, Box<dyn Error>>;

pub(super) fn model(force: f64) -> Value {
    json!({"model_schema_version":"kyuubiki.model/v1","kind":"axial_bar_1d",
        "name":"reference-bar","material":"steel","length":1.0,"area":0.01,
        "youngs_modulus_gpa":210.0,"elements":4,"tip_force":force})
}

pub(super) fn library(executor: &mut ServiceHeadlessExecutor) -> Result<(Value, Value)> {
    let project = executor
        .execute_step("project_create", 1, &json!({"name":"reference-parent"}))
        .map_err(|error| error.message)?
        .result;
    let model = executor
        .execute_step(
            "model_create",
            1,
            &json!({"project_id":project["project_id"],
            "name":"reference-bar","kind":"axial_bar_1d","payload":model(1000.0)}),
        )
        .map_err(|error| error.message)?
        .result;
    Ok((project, model))
}

fn assert_read_failure(report: &Value) {
    assert_eq!(report["status"], "failed");
    assert_eq!(report["executed_step_count"], 0);
    assert_eq!(report["steps"].as_array().unwrap().len(), 1);
    let failure = &report["execution_summary"]["failure"];
    assert_eq!(failure["category"], "contract_failure");
    assert_eq!(failure["stage"], "validation");
    assert_eq!(failure["retryable"], false);
    assert_eq!(failure["retry_strategy"], "none");
    assert!(
        failure["message"]
            .as_str()
            .unwrap()
            .starts_with("model_reference_invalid:")
    );
    for identity in ["job_id", "project_id", "model_id", "model_version_id"] {
        assert!(report["steps"][0]["result_preview"].get(identity).is_none());
    }
}

#[test]
fn real_corrupted_model_reads_never_submit_or_start_calculation() -> Result<()> {
    let owner = LiveAgent::start()?;
    let peer = LiveAgent::start()?;
    let data = Scratch::new()?;
    let server =
        start_live_server_with_state(Some(owner.port()), Some(peer.port()), Some(&data.0))?;
    let mut direct = ServiceHeadlessExecutor::new(&format!("http://127.0.0.1:{}", server.port));
    let (project, model) = library(&mut direct)?;
    let project_id = project["project_id"].as_str().unwrap();
    let model_id = model["model_id"].as_str().unwrap();
    let version_id = model["latest_version_id"].as_str().unwrap();
    for (action, route, record, identity, id, payload) in [
        (
            "solve_from_model_version",
            "model-versions",
            "version",
            "version_id",
            version_id,
            json!({"model_version_id":version_id,"endpoints":[format!("127.0.0.1:{}",owner.port())]}),
        ),
        (
            "solve_and_wait_from_model_version",
            "model-versions",
            "version",
            "version_id",
            version_id,
            json!({"model_version_id":version_id,"endpoints":[format!("127.0.0.1:{}",owner.port())],
                "timeout_ms":5000,"interval_ms":10}),
        ),
        (
            "direct_mesh_solve",
            "model-versions",
            "version",
            "version_id",
            version_id,
            json!({"model_version_id":version_id}),
        ),
        (
            "direct_mesh_solve",
            "models",
            "model",
            "model_id",
            model_id,
            json!({"model_id":model_id}),
        ),
    ] {
        let path = format!("/api/v1/{route}/{id}");
        let proxy = AckLossProxy::http_without_record_identity(
            server.port,
            "GET",
            &path,
            record,
            identity,
        )?;
        let mut batch = document(action, payload);
        batch.steps[0].risk = find_action_contract(action).unwrap().risk;
        let report = execute_batch_with_executor(
            &batch,
            &mut ServiceHeadlessExecutor::new(&format!("http://127.0.0.1:{}", proxy.port)),
            false,
            false,
        );
        assert_read_failure(&serde_json::to_value(report)?);
        if action == "solve_from_model_version" {
            let source = write_temp_json("invalid-real-reference", &serde_json::to_value(&batch)?);
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
            assert_read_failure(&report);
            assert_eq!(
                report,
                serde_json::from_slice::<Value>(&fs::read(report_path)?)?
            );
            let error: Value = serde_json::from_slice(&output.stderr)?;
            assert_eq!(error["error"]["code"], "headless_execution_failed");
            assert_eq!(error["error"]["retryable"], false);
        }
        let captured = proxy.capture();
        assert!(captured.errors.is_empty(), "{captured:?}");
        let expected = if action == "solve_from_model_version" {
            2
        } else {
            1
        };
        assert_eq!(captured.requests.len(), expected);
        assert_eq!(captured.discarded.len(), expected);
        assert_eq!(captured.replies.len(), expected);
        for (original, corrupted) in captured.discarded.iter().zip(&captured.replies) {
            assert_eq!(original[record][identity], id);
            let mut expected = original.clone();
            expected[record].as_object_mut().unwrap().remove(identity);
            assert_eq!(*corrupted, expected);
            assert_eq!(http_json(server.port, &path, None)?.1, *original);
        }
    }
    assert_eq!(
        http_json(server.port, "/api/v1/jobs", None)?.1["jobs"],
        json!([])
    );
    let projects = http_json(server.port, "/api/v1/projects", None)?.1;
    assert_eq!(projects["projects"].as_array().unwrap().len(), 1);
    assert!(!projects.to_string().contains("must-not-follow-lost-ack"));
    assert_eq!(
        http_json(
            server.port,
            &format!("/api/v1/projects/{project_id}/models"),
            None
        )?
        .1["models"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        http_json(
            server.port,
            &format!("/api/v1/models/{model_id}/versions"),
            None
        )?
        .1["versions"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert_eq!(count(&owner)? + count(&peer)?, 0);
    assert!(!data.0.join("operator-task-dispatches").exists());
    Ok(())
}

fn step(action: &str, index: usize, payload: Value) -> HeadlessExecutionBatchStep {
    HeadlessExecutionBatchStep {
        action: action.into(),
        index,
        payload,
        risk: find_action_contract(action).unwrap().risk,
    }
}

#[test]
fn real_native_mesh_fallback_preserves_verified_reference_context() -> Result<()> {
    let owner = LiveAgent::start()?;
    let peer = LiveAgent::start()?;
    let data = Scratch::new()?;
    let server =
        start_live_server_with_state(Some(owner.port()), Some(peer.port()), Some(&data.0))?;
    let mut direct = ServiceHeadlessExecutor::new(&format!("http://127.0.0.1:{}", server.port));
    let (project, model) = library(&mut direct)?;
    let project_id = &project["project_id"];
    let model_id = model["model_id"].as_str().unwrap();
    let version = direct
        .execute_step(
            "model_version_create",
            1,
            &json!({"model_id":model_id,"payload":self::model(2000.0)}),
        )
        .map_err(|error| error.message)?
        .result;
    let current = http_json(server.port, &format!("/api/v1/models/{model_id}"), None)?.1;
    let current_force = current["model"]["payload"]["tip_force"].as_f64().unwrap();
    let mut job_ids = Vec::new();
    for (reference, expected_version, force) in [
        (
            json!({"model_id":model_id,"projectId":project_id}),
            Value::Null,
            current_force,
        ),
        (
            json!({"model_version_id":version["model_version_id"],"modelId":model_id,
            "projectId":project_id}),
            version["model_version_id"].clone(),
            2000.0,
        ),
    ] {
        let mut batch = document("direct_mesh_solve", reference.clone());
        batch.steps = vec![
            step("direct_mesh_solve", 1, reference),
            step(
                "job_wait",
                2,
                json!({"job_id":"{{steps.1.result.job_id}}",
                "timeout_ms":5000,"interval_ms":10}),
            ),
            step(
                "result_fetch",
                3,
                json!({"job_id":"{{steps.2.result.job_id}}"}),
            ),
        ];
        let report = execute_batch_with_executor(&batch, &mut direct, false, false);
        assert_eq!(report.status, "ok", "{report:?}");
        assert_eq!(report.executed_step_count, 3);
        let result = &report.steps[2].result_preview;
        assert_eq!(&result["job"]["project_id"], project_id);
        assert_eq!(result["job"]["model_version_id"], expected_version);
        let displacement = result["result"]["tip_displacement"].as_f64().unwrap();
        assert!((displacement / (force / (210e9 * 0.01)) - 1.0).abs() < 1e-12);
        job_ids.push(result["job"]["job_id"].as_str().unwrap().to_owned());
    }
    assert_ne!(job_ids[0], job_ids[1]);
    wait_for_lifecycle(&owner, "accepting", 0)?;
    wait_for_lifecycle(&peer, "accepting", 0)?;
    assert_eq!(count(&owner)? + count(&peer)?, 2);
    assert_eq!(
        http_json(server.port, "/api/v1/jobs", None)?.1["jobs"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    assert_eq!(
        http_json(server.port, "/api/v1/projects", None)?.1["projects"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    Ok(())
}
