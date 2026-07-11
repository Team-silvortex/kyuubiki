use kyuubiki_protocol::{
    OperatorTaskDigestError, OperatorTaskSummaryError, OperatorTaskSummaryErrorCode,
    summarize_operator_task_execution_checked, verify_operator_task_digest,
};
use serde_json::{Map, Value};

use crate::operator_task_readiness::{
    OPERATOR_PACKAGE_RUNTIME_NOT_ATTACHED, OPERATOR_TASK_FETCH_STAGE, detached_execution_plan,
    detached_execution_readiness, package_fetch_request_preview,
};

pub const OPERATOR_TASK_PREPARE_ACTION: &str = "operator_task_prepare";
pub const OPERATOR_TASK_EXECUTE_ACTION: &str = "operator_task_execute";

pub fn is_operator_task_prepare_action(action: &str) -> bool {
    action == OPERATOR_TASK_PREPARE_ACTION
}

pub fn is_operator_task_execute_action(action: &str) -> bool {
    action == OPERATOR_TASK_EXECUTE_ACTION
}

pub fn prepare_operator_task_payload(payload: &Value) -> Result<Value, String> {
    prepare_operator_task_payload_checked(payload).map_err(|error| error.message)
}

fn prepare_operator_task_payload_checked(
    payload: &Value,
) -> Result<Value, OperatorTaskPreviewError> {
    let task = payload.get("task").ok_or_else(|| {
        OperatorTaskPreviewError::new(
            "operator_task_invalid_params",
            "operator_task_prepare requires task",
        )
    })?;

    verify_operator_task_digest(task).map_err(classify_digest_error)?;
    let summary =
        summarize_operator_task_execution_checked(task).map_err(classify_summary_error)?;

    Ok(Value::Object(Map::from_iter([
        ("status".to_string(), Value::from("verified")),
        ("task_digest".to_string(), Value::from(summary.task_digest)),
        ("task_id".to_string(), Value::from(summary.task_id)),
        ("operator_id".to_string(), Value::from(summary.operator_id)),
        (
            "operator_kind".to_string(),
            Value::from(summary.operator_kind),
        ),
        ("program_id".to_string(), Value::from(summary.program_id)),
        (
            "program_kind".to_string(),
            Value::from(summary.program_kind),
        ),
        (
            "runtime_protocol".to_string(),
            Value::from(summary.runtime_protocol),
        ),
        ("abi_kind".to_string(), Value::from(summary.abi_kind)),
        (
            "entrypoint_kind".to_string(),
            Value::from(summary.entrypoint_kind),
        ),
        (
            "entrypoint_name".to_string(),
            Value::from(summary.entrypoint_name),
        ),
        ("package_ref".to_string(), json_string(summary.package_ref)),
        (
            "package_version".to_string(),
            json_string(summary.package_version),
        ),
        (
            "authority_mode".to_string(),
            json_string(summary.authority_mode),
        ),
        (
            "execution_mode".to_string(),
            json_string(summary.execution_mode),
        ),
        ("cache_scope".to_string(), json_string(summary.cache_scope)),
        (
            "agent_fetchable".to_string(),
            summary
                .agent_fetchable
                .map(Value::from)
                .unwrap_or(Value::Null),
        ),
    ])))
}

pub fn preview_operator_task_execute_payload(payload: &Value) -> Result<Value, String> {
    prepare_operator_task_payload(payload).map(|mut preview| {
        preview["status"] = Value::from("verified_pending_execution");
        preview["execution_runtime_status"] = Value::from(OPERATOR_PACKAGE_RUNTIME_NOT_ATTACHED);
        preview["operator_package_runtime_ready"] = Value::from(false);
        preview["blocked_stage"] = Value::from(OPERATOR_TASK_FETCH_STAGE);
        preview["next_stage"] = Value::from(OPERATOR_TASK_FETCH_STAGE);
        preview["execution_readiness"] = detached_execution_readiness();
        preview["package_fetch_request"] = package_fetch_request_preview(&preview);
        preview["execution_plan"] = detached_execution_plan();
        preview
    })
}

pub fn operator_task_error_preview(message: impl AsRef<str>) -> Value {
    let message = message.as_ref();
    Value::Object(Map::from_iter([
        ("error".to_string(), Value::from(message.to_string())),
        (
            "error_code".to_string(),
            Value::from(operator_task_error_code(message)),
        ),
    ]))
}

fn json_string(value: Option<String>) -> Value {
    value.map(Value::from).unwrap_or(Value::Null)
}

fn operator_task_error_code(message: &str) -> &'static str {
    if message.contains("digest mismatch") {
        return "operator_task_digest_mismatch";
    }
    if message.contains("digest is missing") {
        return "operator_task_digest_missing";
    }
    if message.contains("requires task") {
        return "operator_task_invalid_params";
    }
    if message.contains("must match") {
        return "operator_task_mirror_mismatch";
    }
    if message.contains("inconsistent runtime protocol, abi, or entrypoint") {
        return "operator_task_execution_abi_mismatch";
    }
    if message.contains("execution program does not match operator") {
        return "operator_task_program_mismatch";
    }
    if message.contains("entrypoint does not match operator id") {
        return "operator_task_entrypoint_mismatch";
    }
    "operator_task_invalid"
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct OperatorTaskPreviewError {
    code: &'static str,
    message: String,
}

impl OperatorTaskPreviewError {
    fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

fn classify_digest_error(error: OperatorTaskDigestError) -> OperatorTaskPreviewError {
    match error {
        OperatorTaskDigestError::Missing => OperatorTaskPreviewError::new(
            "operator_task_digest_missing",
            "operator task digest is missing",
        ),
        OperatorTaskDigestError::InvalidTask(message) => OperatorTaskPreviewError::new(
            "operator_task_digest_invalid",
            format!("operator task is invalid: {message}"),
        ),
        OperatorTaskDigestError::Mismatch { expected, actual } => OperatorTaskPreviewError::new(
            "operator_task_digest_mismatch",
            format!("operator task digest mismatch: expected {expected}, actual {actual}"),
        ),
    }
}

fn classify_summary_error(error: OperatorTaskSummaryError) -> OperatorTaskPreviewError {
    let code = match error.code {
        OperatorTaskSummaryErrorCode::MirrorMismatch => "operator_task_mirror_mismatch",
        OperatorTaskSummaryErrorCode::ExecutionAbiMismatch => {
            "operator_task_execution_abi_mismatch"
        }
        OperatorTaskSummaryErrorCode::ProgramMismatch => "operator_task_program_mismatch",
        OperatorTaskSummaryErrorCode::EntrypointMismatch => "operator_task_entrypoint_mismatch",
        OperatorTaskSummaryErrorCode::MissingField | OperatorTaskSummaryErrorCode::Invalid => {
            "operator_task_invalid"
        }
    };
    OperatorTaskPreviewError::new(code, error.message)
}

fn operator_task_error_preview_checked(error: OperatorTaskPreviewError) -> Value {
    Value::Object(Map::from_iter([
        ("error".to_string(), Value::from(error.message)),
        ("error_code".to_string(), Value::from(error.code)),
    ]))
}

pub(crate) fn prepare_operator_task_error_preview(payload: &Value) -> Option<Value> {
    prepare_operator_task_payload_checked(payload)
        .err()
        .map(operator_task_error_preview_checked)
}

pub(crate) fn operator_task_prepare_preview_or_error(payload: &Value) -> Value {
    prepare_operator_task_payload(payload).unwrap_or_else(|message| {
        prepare_operator_task_error_preview(payload)
            .unwrap_or_else(|| operator_task_error_preview(message))
    })
}

#[cfg(test)]
mod tests {
    use super::{
        operator_task_error_preview, prepare_operator_task_payload,
        preview_operator_task_execute_payload,
    };
    use crate::{HeadlessExecutionBatch, HeadlessExecutionBatchStep, run_batch_dry};
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
            preview["task_digest"],
            "86c14d1f22af9d14ab35669a2fcb869afab097a9883e6deabf92a362d8f4469f"
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

        assert!(
            error.contains("runtime_hints.package_ref must match execution_program.package_ref")
        );
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
        assert_eq!(preview["execution_plan"][2]["stage"], "fetch_package");
        assert_eq!(preview["execution_plan"][2]["gate"], "blocked");
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
}
