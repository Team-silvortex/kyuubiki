use kyuubiki_protocol::{OperatorTaskExecutionPreview, OperatorTaskExecutionSummary};
use serde_json::{Map, Value};

pub const HEADLESS_OPERATOR_TASK_SECURITY_SCHEMA_VERSION: &str =
    "kyuubiki.headless-operator-task-security/v1";

pub fn operator_task_security_profile(
    summary: &OperatorTaskExecutionSummary,
    preview: &OperatorTaskExecutionPreview,
) -> Value {
    Value::Object(Map::from_iter([
        (
            "schema_version".to_string(),
            Value::from(HEADLESS_OPERATOR_TASK_SECURITY_SCHEMA_VERSION),
        ),
        ("security_owner".to_string(), Value::from("headless_sdk")),
        (
            "task_digest".to_string(),
            Value::from(summary.task_digest.clone()),
        ),
        (
            "operator_id".to_string(),
            Value::from(summary.operator_id.clone()),
        ),
        (
            "runtime_protocol".to_string(),
            Value::from(summary.runtime_protocol.clone()),
        ),
        (
            "abi_kind".to_string(),
            Value::from(summary.abi_kind.clone()),
        ),
        (
            "dispatch_route".to_string(),
            Value::from(preview.dispatch_route.clone()),
        ),
        (
            "package_fetch_required".to_string(),
            Value::from(preview.package_fetch_required),
        ),
        (
            "offline_runnable".to_string(),
            Value::from(preview.offline_runnable),
        ),
        (
            "agent_fetchable".to_string(),
            summary
                .agent_fetchable
                .map(Value::from)
                .unwrap_or(Value::Null),
        ),
        (
            "requires_runtime_attachment".to_string(),
            Value::from(preview.package_fetch_required),
        ),
        (
            "allowed_authority".to_string(),
            optional_string(summary.authority_mode.clone()),
        ),
        (
            "allowed_execution_mode".to_string(),
            optional_string(summary.execution_mode.clone()),
        ),
        (
            "trust_boundaries".to_string(),
            Value::Array(
                trust_boundaries(preview)
                    .into_iter()
                    .map(Value::from)
                    .collect(),
            ),
        ),
    ]))
}

fn optional_string(value: Option<String>) -> Value {
    value.map(Value::from).unwrap_or(Value::Null)
}

fn trust_boundaries(preview: &OperatorTaskExecutionPreview) -> Vec<&'static str> {
    if preview.package_fetch_required {
        vec![
            "central_operator_library",
            "operator_package_runtime",
            "agent_dispatch",
        ]
    } else {
        vec!["agent_native_builtin", "agent_dispatch"]
    }
}
