use serde_json::Value;

pub(crate) const REJECTION_SCHEMA: &str = "kyuubiki.agent-durable-admission-rejection/v1";

pub(crate) fn is_rejection(report: &Value) -> bool {
    report["schema_version"] == REJECTION_SCHEMA || report["status"] == "not_admitted"
}

// A rejection is a decision for this live submit exchange, never a lookup result or replay ticket.
pub(crate) fn validate_rejection(
    report: &Value,
    rpc_method: &str,
    params: &Value,
    request_id: &str,
) -> Result<(), String> {
    let text = |key: &str, max: usize| {
        report[key]
            .as_str()
            .is_some_and(|v| !v.is_empty() && v.len() <= max)
    };
    let hex = |key: &str, length: usize| {
        report[key].as_str().is_some_and(|v| {
            v.len() == length
                && v.bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        })
    };
    if rpc_method != "submit_durable_agent_task"
        || report
            .as_object()
            .is_none_or(|value| value.len() != 11 + usize::from(value.contains_key("store_id")))
        || (report.get("store_id").is_some() && !hex("store_id", 32))
        || (params.get("expected_store_id").is_some()
            && report["store_id"] != params["expected_store_id"])
        || report["schema_version"] != REJECTION_SCHEMA
        || report["status"] != "not_admitted"
        || report["execution_started"] != false
        || report["automatic_replay_authorized"] != false
        || report["execution_id"] != params["execution_id"]
        || !hex("execution_id", 32)
        || report["method"] != params["method"]
        || !text("method", 256)
        || report["request_id"] != request_id
        || !hex("request_digest", 64)
        || !text("process_instance_id", 256)
        || !text("message", 2048)
        || !matches!(
            report["reason"].as_str(),
            Some("store_capacity" | "execution_unavailable")
        )
    {
        return Err(
            "invalid or unbound durable admission rejection; inspect the original execution".into(),
        );
    }
    Ok(())
}
