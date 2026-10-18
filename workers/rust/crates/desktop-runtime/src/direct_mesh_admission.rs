use kyuubiki_protocol::{DurableAgentTask, RpcMethod, validate_durable_execution_id};
use serde_json::Value;

pub(crate) const REJECTION_SCHEMA: &str = "kyuubiki.agent-durable-admission-rejection/v1";

pub(crate) fn is_rejection(report: &Value) -> bool {
    report["schema_version"] == REJECTION_SCHEMA || report["status"] == "not_admitted"
}

pub(crate) struct SubmissionIdentity<'a> {
    execution_id: &'a str,
    method: &'a str,
    digest: String,
}

impl<'a> SubmissionIdentity<'a> {
    pub(crate) fn from_params(params: &'a Value) -> Result<Self, String> {
        let execution_id = params["execution_id"]
            .as_str()
            .ok_or("missing execution identity")?;
        validate_durable_execution_id(execution_id)?;
        let method = params["method"].as_str().ok_or("missing durable method")?;
        let typed_method: RpcMethod = serde_json::from_value(params["method"].clone())
            .map_err(|error| format!("invalid durable method: {error}"))?;
        let digest = DurableAgentTask::payload_digest(&typed_method, &params["params"])?;
        Ok(Self {
            execution_id,
            method,
            digest,
        })
    }

    pub(crate) fn validate(&self, report: &Value) -> Result<(), String> {
        // Repeated submissions can return the original process/request IDs, but never
        // a receipt for another payload or a lookup-only absence decision.
        if report["execution_id"] != self.execution_id
            || report["method"] != self.method
            || report["request_digest"] != self.digest
            || report["status"] == "not_found"
        {
            return Err("durable submission reply does not match the submitted task; inspect the original execution".into());
        }
        Ok(())
    }
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
