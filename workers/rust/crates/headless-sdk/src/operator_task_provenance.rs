use kyuubiki_protocol::{OperatorTaskExecutionPreview, OperatorTaskExecutionSummary};
use serde_json::{Map, Value};

pub const HEADLESS_OPERATOR_TASK_PROVENANCE_SCHEMA_VERSION: &str =
    "kyuubiki.headless-operator-task-provenance/v1";

pub fn operator_task_provenance_profile(
    summary: &OperatorTaskExecutionSummary,
    preview: &OperatorTaskExecutionPreview,
) -> Value {
    Value::Object(Map::from_iter([
        (
            "schema_version".to_string(),
            Value::from(HEADLESS_OPERATOR_TASK_PROVENANCE_SCHEMA_VERSION),
        ),
        ("provenance_owner".to_string(), Value::from("headless_sdk")),
        (
            "retention_scope".to_string(),
            optional_string(summary.cache_scope.clone()),
        ),
        (
            "task_digest".to_string(),
            Value::from(summary.task_digest.clone()),
        ),
        ("task_id".to_string(), Value::from(summary.task_id.clone())),
        (
            "operator_id".to_string(),
            Value::from(summary.operator_id.clone()),
        ),
        (
            "program_id".to_string(),
            Value::from(summary.program_id.clone()),
        ),
        (
            "package_ref".to_string(),
            optional_string(summary.package_ref.clone()),
        ),
        (
            "package_version".to_string(),
            optional_string(summary.package_version.clone()),
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
            "agent_fetchable".to_string(),
            summary
                .agent_fetchable
                .map(Value::from)
                .unwrap_or(Value::Null),
        ),
        (
            "lineage".to_string(),
            Value::Object(Map::from_iter([
                ("digest_verified".to_string(), Value::from(true)),
                ("execution_program_verified".to_string(), Value::from(true)),
                (
                    "preview_digest".to_string(),
                    Value::from(preview.task_digest.clone()),
                ),
            ])),
        ),
    ]))
}

fn optional_string(value: Option<String>) -> Value {
    value.map(Value::from).unwrap_or(Value::Null)
}
