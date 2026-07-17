use kyuubiki_protocol::{
    OperatorTaskDigestError, OperatorTaskExecutionPreview, OperatorTaskSummaryError,
    OperatorTaskSummaryErrorCode, preview_operator_task_execution,
    summarize_operator_task_execution_checked, verify_operator_task_digest,
};
use serde_json::{Map, Value};

use crate::operator_task_provenance::operator_task_provenance_profile;
use crate::operator_task_readiness::{
    OPERATOR_PACKAGE_RUNTIME_NOT_ATTACHED, OPERATOR_TASK_FETCH_STAGE, detached_execution_plan,
    detached_execution_readiness, package_fetch_request_preview,
};
use crate::operator_task_security::operator_task_security_profile;

pub const OPERATOR_TASK_PREPARE_ACTION: &str = "operator_task_prepare";
pub const OPERATOR_TASK_EXECUTE_ACTION: &str = "operator_task_execute";
const HEADLESS_OPERATOR_TASK_FAILURE_SCHEMA_VERSION: &str =
    "kyuubiki.headless-operator-task-failure/v1";

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

    verify_operator_task_digest(task).map_err(|error| classify_digest_error(error, task))?;
    let summary = summarize_operator_task_execution_checked(task)
        .map_err(|error| classify_summary_error(error, task))?;
    let execution_preview = preview_operator_task_execution(task)
        .map_err(|error| classify_summary_error(error, task))?;

    Ok(Value::Object(Map::from_iter([
        ("status".to_string(), Value::from("verified")),
        (
            "task_execution_preview".to_string(),
            task_execution_preview_payload(&execution_preview),
        ),
        (
            "security_profile".to_string(),
            operator_task_security_profile(&summary, &execution_preview),
        ),
        (
            "provenance_profile".to_string(),
            operator_task_provenance_profile(&summary, &execution_preview),
        ),
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

fn task_execution_preview_payload(preview: &OperatorTaskExecutionPreview) -> Value {
    Value::Object(Map::from_iter([
        (
            "task_digest".to_string(),
            Value::from(preview.task_digest.clone()),
        ),
        ("task_id".to_string(), Value::from(preview.task_id.clone())),
        (
            "operator_id".to_string(),
            Value::from(preview.operator_id.clone()),
        ),
        (
            "operator_kind".to_string(),
            Value::from(preview.operator_kind.clone()),
        ),
        (
            "dispatch_route".to_string(),
            Value::from(preview.dispatch_route.clone()),
        ),
        (
            "package_ref".to_string(),
            json_string(preview.package_ref.clone()),
        ),
        (
            "package_version".to_string(),
            json_string(preview.package_version.clone()),
        ),
        (
            "package_fetch_required".to_string(),
            Value::from(preview.package_fetch_required),
        ),
        (
            "package_readiness_gate".to_string(),
            Value::from(preview.package_readiness_gate.clone()),
        ),
        (
            "result_serialization".to_string(),
            Value::from(preview.result_serialization.clone()),
        ),
        (
            "authority_mode".to_string(),
            json_string(preview.authority_mode.clone()),
        ),
        (
            "execution_mode".to_string(),
            json_string(preview.execution_mode.clone()),
        ),
        (
            "cache_scope".to_string(),
            json_string(preview.cache_scope.clone()),
        ),
        (
            "agent_fetchable".to_string(),
            preview
                .agent_fetchable
                .map(Value::from)
                .unwrap_or(Value::Null),
        ),
        (
            "offline_runnable".to_string(),
            Value::from(preview.offline_runnable),
        ),
        (
            "dispatch_warnings".to_string(),
            Value::Array(
                preview
                    .dispatch_warnings
                    .iter()
                    .cloned()
                    .map(Value::from)
                    .collect(),
            ),
        ),
    ]))
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
    let code = operator_task_error_code(message);
    Value::Object(Map::from_iter([
        ("error".to_string(), Value::from(message.to_string())),
        ("error_code".to_string(), Value::from(code)),
        (
            "failure_receipt".to_string(),
            operator_task_failure_receipt(code, message, "preview_request", None),
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
    details: Value,
}

impl OperatorTaskPreviewError {
    fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self::with_task(code, message, "preview_request", None)
    }

    fn with_task(
        code: &'static str,
        message: impl Into<String>,
        stage: &'static str,
        task: Option<&Value>,
    ) -> Self {
        let message = message.into();
        Self {
            code,
            details: operator_task_failure_receipt(code, &message, stage, task),
            message,
        }
    }
}

fn classify_digest_error(error: OperatorTaskDigestError, task: &Value) -> OperatorTaskPreviewError {
    match error {
        OperatorTaskDigestError::Missing => OperatorTaskPreviewError::with_task(
            "operator_task_digest_missing",
            "operator task digest is missing",
            "verify_digest",
            Some(task),
        ),
        OperatorTaskDigestError::InvalidTask(message) => OperatorTaskPreviewError::with_task(
            "operator_task_digest_invalid",
            format!("operator task is invalid: {message}"),
            "verify_digest",
            Some(task),
        ),
        OperatorTaskDigestError::Mismatch { expected, actual } => {
            OperatorTaskPreviewError::with_task(
                "operator_task_digest_mismatch",
                format!("operator task digest mismatch: expected {expected}, actual {actual}"),
                "verify_digest",
                Some(task),
            )
        }
    }
}

fn classify_summary_error(
    error: OperatorTaskSummaryError,
    task: &Value,
) -> OperatorTaskPreviewError {
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
    OperatorTaskPreviewError::with_task(
        code,
        error.message,
        "summarize_execution_program",
        Some(task),
    )
}

fn operator_task_error_preview_checked(error: OperatorTaskPreviewError) -> Value {
    Value::Object(Map::from_iter([
        ("error".to_string(), Value::from(error.message)),
        ("error_code".to_string(), Value::from(error.code)),
        ("failure_receipt".to_string(), error.details),
    ]))
}

fn operator_task_failure_receipt(
    code: &str,
    message: &str,
    stage: &str,
    task: Option<&Value>,
) -> Value {
    Value::Object(Map::from_iter([
        (
            "schema_version".to_string(),
            Value::from(HEADLESS_OPERATOR_TASK_FAILURE_SCHEMA_VERSION),
        ),
        ("failure_owner".to_string(), Value::from("headless_sdk")),
        ("failure_stage".to_string(), Value::from(stage.to_string())),
        ("reason_code".to_string(), Value::from(code.to_string())),
        ("message".to_string(), Value::from(message.to_string())),
        (
            "task_id".to_string(),
            task.and_then(|entry| entry.get("task_id"))
                .cloned()
                .unwrap_or(Value::Null),
        ),
        (
            "operator_id".to_string(),
            task.and_then(|entry| entry.pointer("/operator/id"))
                .cloned()
                .unwrap_or(Value::Null),
        ),
        (
            "task_digest".to_string(),
            task.and_then(|entry| entry.pointer("/integrity/task_digest"))
                .cloned()
                .unwrap_or(Value::Null),
        ),
        (
            "recovery".to_string(),
            Value::Object(Map::from_iter([
                ("retryable".to_string(), Value::from(false)),
                (
                    "required_action".to_string(),
                    Value::from(required_failure_action(code)),
                ),
                (
                    "safe_to_continue_other_tasks".to_string(),
                    Value::from(true),
                ),
            ])),
        ),
    ]))
}

fn required_failure_action(code: &str) -> &'static str {
    match code {
        "operator_task_digest_missing"
        | "operator_task_digest_mismatch"
        | "operator_task_digest_invalid" => "rebuild_task_ir_and_recompute_digest",
        "operator_task_mirror_mismatch"
        | "operator_task_execution_abi_mismatch"
        | "operator_task_program_mismatch"
        | "operator_task_entrypoint_mismatch" => "fix_task_ir_contract_mirror_fields",
        "operator_task_invalid_params" => "fix_headless_step_payload",
        _ => "inspect_task_ir",
    }
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
