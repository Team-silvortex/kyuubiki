use super::ack_loss_proxy::AckLossProxy;
use super::agent_support::{LiveAgent, wait_for_lifecycle};
use super::orchestra_completion::http_json;
use super::write_ack_loss::{assert_failure, count, document, run_cli};
use super::{Scratch, start_live_server_with_state};
use kyuubiki_headless_sdk::{
    HeadlessExecutionBatchStep, HeadlessExecutor, HeadlessRisk, ServiceHeadlessExecutor,
    execute_batch_with_executor, find_action_contract,
};
use serde_json::{Value, json};
use std::error::Error;

type Result<T> = std::result::Result<T, Box<dyn Error>>;

fn model(force: f64) -> Value {
    json!({"model_schema_version":"kyuubiki.model/v1","kind":"axial_bar_1d",
        "name":"receipt-bar","material":"steel","length":1.0,"area":0.01,
        "youngs_modulus_gpa":210.0,"elements":4,"tip_force":force})
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
fn real_library_commits_with_corrupted_identities_stop_without_replay() -> Result<()> {
    let data = Scratch::new()?;
    let server = start_live_server_with_state(None, None, Some(&data.0))?;
    let mut direct = ServiceHeadlessExecutor::new(&format!("http://127.0.0.1:{}", server.port));
    let project = direct
        .execute_step("project_create", 1, &json!({"name":"parent"}))
        .map_err(|error| error.message)?
        .result;
    let project_id = project["project_id"].as_str().unwrap();
    let model = direct.execute_step("model_create", 1,
        &json!({"project_id":project_id,"name":"parent-bar","kind":"axial_bar_1d","payload":model(1000.0)}))
        .map_err(|error| error.message)?.result;
    let model_id = model["model_id"].as_str().unwrap();
    for (action, method, path, record, identity, payload) in [
        (
            "project_create",
            "POST",
            "/api/v1/projects".into(),
            "project",
            "project_id",
            json!({"name":"corrupted-create"}),
        ),
        (
            "project_update",
            "PATCH",
            format!("/api/v1/projects/{project_id}"),
            "project",
            "project_id",
            json!({"project_id":project_id,"name":"corrupted-update"}),
        ),
        (
            "model_create",
            "POST",
            format!("/api/v1/projects/{project_id}/models"),
            "model",
            "model_id",
            json!({"project_id":project_id,"name":"corrupted-model","kind":"axial_bar_1d","payload":self::model(2000.0)}),
        ),
        (
            "model_version_create",
            "POST",
            format!("/api/v1/models/{model_id}/versions"),
            "version",
            "version_id",
            json!({"model_id":model_id,"payload":self::model(3000.0)}),
        ),
    ] {
        let proxy = AckLossProxy::http_without_record_identity(
            server.port,
            method,
            &path,
            record,
            identity,
        )?;
        let mut batch = document(action, payload.clone());
        batch.steps[0].risk = find_action_contract(action).unwrap().risk;
        let report = execute_batch_with_executor(
            &batch,
            &mut ServiceHeadlessExecutor::new(&format!("http://127.0.0.1:{}", proxy.port)),
            false,
            false,
        );
        let report = serde_json::to_value(report)?;
        assert_failure(&report);
        assert!(
            report["execution_summary"]["failure"]["message"]
                .as_str()
                .unwrap()
                .contains("library_receipt_invalid:")
        );
        // A distinct CLI experiment creates/updates a new named record, not a recovery replay.
        if action == "project_create" || action == "model_create" {
            let mut cli = document(action, payload);
            cli.steps[0].payload["name"] = json!("corrupted-cli-create");
            run_cli(&cli, &proxy)?;
        }
        let captured = proxy.capture();
        assert!(captured.errors.is_empty(), "{captured:?}");
        let expected = if action == "project_create" || action == "model_create" {
            2
        } else {
            1
        };
        assert_eq!(captured.requests.len(), expected);
        assert_eq!(captured.discarded.len(), expected);
        assert_eq!(captured.replies.len(), expected);
        for (original, corrupted) in captured.discarded.iter().zip(&captured.replies) {
            let mut only_identity_removed = original.clone();
            only_identity_removed[record]
                .as_object_mut()
                .unwrap()
                .remove(identity);
            assert_eq!(*corrupted, only_identity_removed);
            let id = original[record][identity].as_str().unwrap();
            let read_path = match action {
                "model_create" => format!("/api/v1/models/{id}"),
                "model_version_create" => format!("/api/v1/model-versions/{id}"),
                _ => format!("/api/v1/projects/{id}"),
            };
            // Independent test observation proves the write happened despite SDK failure.
            assert_eq!(
                http_json(server.port, &read_path, None)?.1[record][identity],
                id
            );
        }
    }
    let doomed = direct
        .execute_step("project_create", 1, &json!({"name":"delete-owned"}))
        .map_err(|error| error.message)?
        .result;
    let doomed_id = doomed["project_id"].as_str().unwrap();
    let path = format!("/api/v1/projects/{doomed_id}");
    let proxy = AckLossProxy::http_without_record_identity(
        server.port,
        "DELETE",
        &path,
        "project",
        "project_id",
    )?;
    let mut batch = document("project_delete", json!({"project_id":doomed_id}));
    batch.steps[0].risk = HeadlessRisk::Destructive;
    let report = execute_batch_with_executor(
        &batch,
        &mut ServiceHeadlessExecutor::new(&format!("http://127.0.0.1:{}", proxy.port)),
        true,
        true,
    );
    assert_failure(&serde_json::to_value(report)?);
    let capture = proxy.capture();
    assert!(capture.errors.is_empty(), "{capture:?}");
    assert_eq!(capture.requests.len(), 1);
    assert_eq!(capture.discarded.len(), 1);
    assert_eq!(capture.replies.len(), 1);
    assert_eq!(capture.discarded[0]["project"]["project_id"], doomed_id);
    assert_eq!(http_json(server.port, &path, None)?.0, 404);
    let projects = http_json(server.port, "/api/v1/projects", None)?.1;
    assert_eq!(projects["projects"].as_array().unwrap().len(), 3);
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
        3
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
        2
    );
    assert_eq!(
        http_json(server.port, "/api/v1/jobs", None)?.1["jobs"],
        json!([])
    );
    Ok(())
}

#[test]
fn real_library_bindings_drive_model_version_solve_and_cleanup() -> Result<()> {
    let owner = LiveAgent::start()?;
    let peer = LiveAgent::start()?;
    let data = Scratch::new()?;
    let server =
        start_live_server_with_state(Some(owner.port()), Some(peer.port()), Some(&data.0))?;
    let mut batch = document("project_create", json!({"name":"complete-library-chain"}));
    batch.steps = vec![
        step(
            "project_create",
            1,
            json!({"name":"complete-library-chain"}),
        ),
        step(
            "project_update",
            2,
            json!({"project_id":"{{steps.1.result.project_id}}","name":"updated-chain"}),
        ),
        step(
            "model_create",
            3,
            json!({"project_id":"{{steps.2.result.project_id}}","name":"initial-bar","kind":"axial_bar_1d","payload":model(1000.0)}),
        ),
        step(
            "model_version_create",
            4,
            json!({"model_id":"{{steps.3.result.model_id}}","payload":model(2000.0)}),
        ),
        step(
            "solve_and_wait_from_model_version",
            5,
            json!({"model_version_id":"{{steps.4.result.model_version_id}}",
                "endpoints":[format!("127.0.0.1:{}", owner.port()),format!("127.0.0.1:{}", peer.port())],
                "timeout_ms":5000,"interval_ms":10}),
        ),
        step(
            "result_fetch",
            6,
            json!({"job_id":"{{steps.5.result.job_id}}"}),
        ),
        step(
            "project_delete",
            7,
            json!({"project_id":"{{steps.1.result.project_id}}"}),
        ),
    ];
    let report = execute_batch_with_executor(
        &batch,
        &mut ServiceHeadlessExecutor::new(&format!("http://127.0.0.1:{}", server.port)),
        true,
        true,
    );
    assert_eq!(report.status, "ok", "{report:?}");
    assert_eq!(report.executed_step_count, 7);
    assert_eq!(
        report.steps[5].result_preview["job"]["project_id"],
        report.steps[0].result_preview["project_id"]
    );
    assert_eq!(
        report.steps[5].result_preview["job"]["model_version_id"],
        report.steps[3].result_preview["model_version_id"]
    );
    let displacement = report.steps[5].result_preview["result"]["tip_displacement"]
        .as_f64()
        .unwrap();
    assert!((displacement / (2000.0 / (210e9 * 0.01)) - 1.0).abs() < 1e-12);
    assert_eq!(
        http_json(server.port, "/api/v1/projects", None)?.1["projects"],
        json!([])
    );
    wait_for_lifecycle(&owner, "accepting", 0)?;
    wait_for_lifecycle(&peer, "accepting", 0)?;
    assert_eq!(count(&owner)? + count(&peer)?, 1);
    Ok(())
}
