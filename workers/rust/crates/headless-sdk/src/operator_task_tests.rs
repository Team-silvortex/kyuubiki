use crate::{
    HeadlessExecutionBatch, HeadlessExecutionBatchStep, operator_task_error_preview,
    prepare_operator_task_payload, preview_operator_task_execute_payload, run_batch_dry,
};
use kyuubiki_protocol::compute_operator_task_digest;
use serde_json::{Value, json};

#[test]
fn prepare_operator_task_payload_returns_execution_summary() {
    let preview = prepare_operator_task_payload(&json!({
        "task": golden_task_fixture(false)
    }))
    .expect("task should verify");

    assert_eq!(preview["status"], "verified");
    assert_eq!(preview["operator_id"], "transform.fixture");
    assert_eq!(preview["program_id"], "transform.fixture");
    assert_eq!(
        preview["task_execution_preview"]["dispatch_route"],
        "fetch_package_then_operator_task"
    );
    assert_eq!(
        preview["task_execution_preview"]["package_readiness_gate"],
        "central_package_readiness"
    );
    assert_eq!(
        preview["task_execution_preview"]["result_serialization"],
        "json"
    );
    assert_eq!(
        preview["task_digest"],
        "86c14d1f22af9d14ab35669a2fcb869afab097a9883e6deabf92a362d8f4469f"
    );
    assert_eq!(
        preview["security_profile"]["schema_version"],
        crate::HEADLESS_OPERATOR_TASK_SECURITY_SCHEMA_VERSION
    );
    assert_eq!(
        preview["security_profile"]["security_owner"],
        "headless_sdk"
    );
    assert_eq!(preview["security_profile"]["package_fetch_required"], true);
    assert_eq!(preview["security_profile"]["offline_runnable"], false);
    assert_eq!(
        preview["security_profile"]["requires_runtime_attachment"],
        true
    );
    assert_eq!(
        preview["security_profile"]["trust_boundaries"][0],
        "central_operator_library"
    );
    assert_eq!(
        preview["provenance_profile"]["schema_version"],
        crate::HEADLESS_OPERATOR_TASK_PROVENANCE_SCHEMA_VERSION
    );
    assert_eq!(
        preview["provenance_profile"]["provenance_owner"],
        "headless_sdk"
    );
    assert_eq!(preview["provenance_profile"]["retention_scope"], "job");
    assert_eq!(
        preview["provenance_profile"]["lineage"]["digest_verified"],
        true
    );
    assert_eq!(
        preview["provenance_profile"]["lineage"]["preview_digest"],
        preview["task_digest"]
    );
}

#[test]
fn prepare_operator_task_payload_rejects_digest_mismatch() {
    let error = prepare_operator_task_payload(&json!({
        "task": golden_task_fixture(true)
    }))
    .expect_err("tampered task should fail");

    assert!(error.contains("operator task digest mismatch"));
}

#[test]
fn prepare_operator_task_payload_rejects_digest_valid_mirror_mismatch() {
    let mut task = golden_task_fixture(false);
    task["runtime_hints"]["package_ref"] = json!("orchestra://operator-package/wrong");
    task["integrity"]["task_digest"] =
        json!(compute_operator_task_digest(&task).expect("changed task should digest"));

    let error = prepare_operator_task_payload(&json!({
        "task": task
    }))
    .expect_err("mirror mismatch should fail");

    assert!(error.contains("runtime_hints.package_ref must match execution_program.package_ref"));
    assert_eq!(
        operator_task_error_preview(error)["error_code"],
        "operator_task_mirror_mismatch"
    );
}

#[test]
fn operator_task_prepare_runs_as_headless_dry_step() {
    let batch = HeadlessExecutionBatch {
        schema_version: "kyuubiki.headless-execution-batch/v1".to_string(),
        exported_at: "1970-01-01T00:00:00.000Z".to_string(),
        language: "en".to_string(),
        workflow_id: "operator-task-fixture".to_string(),
        steps: vec![HeadlessExecutionBatchStep {
            index: 1,
            action: "operator_task_prepare".to_string(),
            risk: crate::HeadlessRisk::Normal,
            payload: json!({ "task": golden_task_fixture(false) }),
        }],
        warnings: vec![],
    };

    let report = run_batch_dry(&batch, false, false);

    assert_eq!(report.status, "ok");
    assert_eq!(report.executed_step_count, 1);
    assert_eq!(report.steps[0].result_preview["status"], "verified");
}

#[test]
fn operator_task_prepare_dry_run_reports_structured_mirror_error() {
    let mut task = golden_task_fixture(false);
    task["runtime_hints"]["operator_kind"] = json!("solver");
    task["integrity"]["task_digest"] =
        json!(compute_operator_task_digest(&task).expect("changed task should digest"));

    let report = run_batch_dry(&operator_task_batch(task), false, false);

    assert_eq!(report.status, "failed");
    assert_eq!(report.steps[0].status, "failed");
    assert_eq!(
        report.steps[0].result_preview["error_code"],
        "operator_task_mirror_mismatch"
    );
}

#[test]
fn operator_task_prepare_dry_run_reports_missing_digest() {
    let mut task = golden_task_fixture(false);
    task["integrity"] = json!({});

    let report = run_batch_dry(&operator_task_batch(task), false, false);

    assert_eq!(report.status, "failed");
    assert_eq!(
        report.steps[0].result_preview["error_code"],
        "operator_task_digest_missing"
    );
}

#[test]
fn operator_task_prepare_dry_run_reports_execution_abi_mismatch() {
    let mut task = golden_task_fixture(false);
    task["execution_program"]["abi"]["kind"] = json!("solver_rpc");
    task["integrity"]["task_digest"] =
        json!(compute_operator_task_digest(&task).expect("changed task should digest"));

    let report = run_batch_dry(&operator_task_batch(task), false, false);

    assert_eq!(report.status, "failed");
    assert_eq!(
        report.steps[0].result_preview["error_code"],
        "operator_task_execution_abi_mismatch"
    );
}

#[test]
fn operator_task_execute_preview_verifies_before_runtime_dispatch() {
    let preview = preview_operator_task_execute_payload(&json!({
        "task": golden_task_fixture(false)
    }))
    .expect("task should verify");

    assert_eq!(preview["status"], "verified_pending_execution");
    assert_eq!(preview["operator_id"], "transform.fixture");
    assert_eq!(preview["execution_readiness"]["status"], "blocked");
    assert_eq!(
        preview["execution_readiness"]["required_action"],
        "attach_operator_package_runtime"
    );
    assert_eq!(
        preview["package_fetch_request"]["request_status"],
        "blocked_runtime_not_attached"
    );
    assert_eq!(
        preview["task_execution_preview"]["package_fetch_required"],
        true
    );
    assert_eq!(preview["task_execution_preview"]["offline_runnable"], false);
    assert_eq!(preview["execution_plan"][2]["stage"], "fetch_package");
    assert_eq!(preview["execution_plan"][2]["gate"], "blocked");
    assert_eq!(
        preview["security_profile"]["allowed_execution_mode"],
        "orchestra_fetch"
    );
    assert_eq!(
        preview["security_profile"]["trust_boundaries"][1],
        "operator_package_runtime"
    );
    assert_eq!(
        preview["provenance_profile"]["dispatch_route"],
        "fetch_package_then_operator_task"
    );
    assert_eq!(
        preview["provenance_profile"]["package_ref"],
        "orchestra://operator-package/transform.fixture"
    );
}

#[test]
fn operator_task_execute_runs_as_headless_dry_step_with_readiness() {
    let batch = HeadlessExecutionBatch {
        schema_version: "kyuubiki.headless-execution-batch/v1".to_string(),
        exported_at: "1970-01-01T00:00:00.000Z".to_string(),
        language: "en".to_string(),
        workflow_id: "operator-task-fixture".to_string(),
        steps: vec![HeadlessExecutionBatchStep {
            index: 1,
            action: "operator_task_execute".to_string(),
            risk: crate::HeadlessRisk::Normal,
            payload: json!({ "task": golden_task_fixture(false) }),
        }],
        warnings: vec![],
    };

    let report = run_batch_dry(&batch, false, false);

    assert_eq!(report.status, "ok");
    assert_eq!(
        report.steps[0].result_preview["execution_readiness"]["status"],
        "blocked"
    );
    assert_eq!(
        report.steps[0].result_preview["next_stage"],
        "fetch_package"
    );
}

fn operator_task_batch(task: Value) -> HeadlessExecutionBatch {
    HeadlessExecutionBatch {
        schema_version: "kyuubiki.headless-execution-batch/v1".to_string(),
        exported_at: "1970-01-01T00:00:00.000Z".to_string(),
        language: "en".to_string(),
        workflow_id: "operator-task-fixture".to_string(),
        steps: vec![HeadlessExecutionBatchStep {
            index: 1,
            action: "operator_task_prepare".to_string(),
            risk: crate::HeadlessRisk::Normal,
            payload: json!({ "task": task }),
        }],
        warnings: vec![],
    }
}

fn golden_task_fixture(tampered: bool) -> Value {
    let alpha = !tampered;
    json!({
        "schema_version": "kyuubiki.operator-task-ir/v1",
        "task_id": "fixture-task",
        "operator": {
            "id": "transform.fixture",
            "family": "fixture",
            "kind": "transform",
            "execution": {
                "package_ref": "orchestra://operator-package/transform.fixture"
            }
        },
        "descriptor_authoring": {
            "schema_version": "kyuubiki.operator-descriptor-authoring/v1",
            "mode": "rust_native",
            "runtime": "rust",
            "source": "fixture",
            "hot_reloadable": false,
            "execution_language": "language_neutral"
        },
        "node": {},
        "input_artifact": {
            "x": 1
        },
        "config": {
            "alpha": alpha
        },
        "execution_program": {
            "schema_version": "kyuubiki.operator-execution-program/v1",
            "program_id": "transform.fixture",
            "program_family": "fixture",
            "program_kind": "transform",
            "operator_category_id": null,
            "package_ref": "orchestra://operator-package/transform.fixture",
            "package_version": "library-managed",
            "package_integrity": null,
            "runtime_protocol": "kyuubiki.operator-execution/v1",
            "abi": {
                "kind": "operator_task",
                "input_encoding": "json",
                "output_encoding": "json"
            },
            "entrypoint": {
                "kind": "operator_id",
                "name": "transform.fixture",
                "operator_kind": "transform"
            },
            "bindings": {
                "input_artifact": "task.input_artifact",
                "config": "task.config",
                "output_artifact": "task.output_artifact"
            },
            "node_binding": {
                "node_id": null,
                "input_ports": [],
                "output_ports": []
            }
        },
        "dataset_contract": {},
        "orchestration_context": {},
        "runtime_hints": {
            "authority_mode": "central_operator_library",
            "execution_mode": "orchestra_fetch",
            "source_ref": null,
            "package_ref": "orchestra://operator-package/transform.fixture",
            "package_version": "library-managed",
            "placement_tags": [],
            "required_capabilities": [],
            "cache_scope": "job",
            "agent_fetchable": true,
            "operator_kind": "transform"
        },
        "integrity": {
            "task_digest": "86c14d1f22af9d14ab35669a2fcb869afab097a9883e6deabf92a362d8f4469f"
        }
    })
}
