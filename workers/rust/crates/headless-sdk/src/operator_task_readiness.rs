use serde_json::{Map, Value};

pub(crate) const OPERATOR_TASK_FETCH_STAGE: &str = "fetch_package";
pub(crate) const OPERATOR_PACKAGE_RUNTIME_NOT_ATTACHED: &str =
    "operator_package_runtime_not_yet_attached";
const OPERATOR_PACKAGE_FETCH_REQUEST_SCHEMA: &str = "kyuubiki.operator-package-fetch-request/v1";

pub(crate) fn detached_execution_readiness() -> Value {
    Value::Object(Map::from_iter([
        ("status".to_string(), Value::from("blocked")),
        ("requested_mode".to_string(), Value::from("execute")),
        ("ready_to_dispatch".to_string(), Value::from(false)),
        (
            "current_stage".to_string(),
            Value::from(OPERATOR_TASK_FETCH_STAGE),
        ),
        (
            "blocking_stage".to_string(),
            Value::from(OPERATOR_TASK_FETCH_STAGE),
        ),
        (
            "blocking_reason".to_string(),
            Value::from(OPERATOR_PACKAGE_RUNTIME_NOT_ATTACHED),
        ),
        (
            "blocking_owner".to_string(),
            Value::from("operator_package_runtime"),
        ),
        (
            "required_action".to_string(),
            Value::from("attach_operator_package_runtime"),
        ),
    ]))
}

pub(crate) fn package_fetch_request_preview(preview: &Value) -> Value {
    Value::Object(Map::from_iter([
        (
            "schema_version".to_string(),
            Value::from(OPERATOR_PACKAGE_FETCH_REQUEST_SCHEMA),
        ),
        (
            "request_status".to_string(),
            Value::from("blocked_runtime_not_attached"),
        ),
        (
            "package_ref".to_string(),
            preview.get("package_ref").cloned().unwrap_or(Value::Null),
        ),
        (
            "package_version".to_string(),
            preview
                .get("package_version")
                .cloned()
                .unwrap_or(Value::Null),
        ),
        (
            "task_digest".to_string(),
            preview.get("task_digest").cloned().unwrap_or(Value::Null),
        ),
        (
            "operator_id".to_string(),
            preview.get("operator_id").cloned().unwrap_or(Value::Null),
        ),
        (
            "program_id".to_string(),
            preview.get("program_id").cloned().unwrap_or(Value::Null),
        ),
        (
            "runtime_protocol".to_string(),
            preview
                .get("runtime_protocol")
                .cloned()
                .unwrap_or(Value::Null),
        ),
        (
            "abi_kind".to_string(),
            preview.get("abi_kind").cloned().unwrap_or(Value::Null),
        ),
        ("agent_fetchable".to_string(), Value::from(true)),
        (
            "target".to_string(),
            Value::Object(Map::from_iter([
                ("runtime_attached".to_string(), Value::from(false)),
                ("host_id".to_string(), Value::Null),
                ("packages_root".to_string(), Value::Null),
            ])),
        ),
    ]))
}

pub(crate) fn detached_execution_plan() -> Value {
    Value::Array(vec![
        execution_plan_stage("verify_digest", "complete", "agent_runtime", "passed"),
        execution_plan_stage(
            "summarize_execution_program",
            "complete",
            "agent_runtime",
            "passed",
        ),
        Value::Object(Map::from_iter([
            ("stage".to_string(), Value::from(OPERATOR_TASK_FETCH_STAGE)),
            ("status".to_string(), Value::from("blocked")),
            ("owner".to_string(), Value::from("operator_package_runtime")),
            ("gate".to_string(), Value::from("blocked")),
            ("requested_mode".to_string(), Value::from("execute")),
            (
                "reason".to_string(),
                Value::from(OPERATOR_PACKAGE_RUNTIME_NOT_ATTACHED),
            ),
        ])),
        execution_plan_stage(
            "verify_package_integrity",
            "pending",
            "operator_package_runtime",
            "waiting_for_fetch",
        ),
        execution_plan_stage(
            "dispatch_entrypoint",
            "pending",
            "operator_package_runtime",
            "waiting_for_integrity",
        ),
        execution_plan_stage(
            "serialize_result",
            "pending",
            "operator_package_runtime",
            "waiting_for_dispatch",
        ),
    ])
}

fn execution_plan_stage(stage: &str, status: &str, owner: &str, gate: &str) -> Value {
    Value::Object(Map::from_iter([
        ("stage".to_string(), Value::from(stage)),
        ("status".to_string(), Value::from(status)),
        ("owner".to_string(), Value::from(owner)),
        ("gate".to_string(), Value::from(gate)),
    ]))
}
