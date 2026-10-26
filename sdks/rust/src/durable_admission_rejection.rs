//! A transient admission decision is not a durable execution receipt.
use crate::{DurableExecutionIntent, SdkError, SdkResult, durable_execution::invalid};
use serde::Deserialize;
use serde_json::Value;

const SCHEMA: &str = "kyuubiki.agent-durable-admission-rejection/v1";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Rejection<'a> {
    schema_version: &'a str,
    store_id: &'a str,
    execution_id: &'a str,
    request_digest: &'a str,
    process_instance_id: &'a str,
    request_id: &'a str,
    method: &'a str,
    status: &'a str,
    reason: &'a str,
    message: &'a str,
    execution_started: bool,
    automatic_replay_authorized: bool,
}

pub(crate) fn check(
    intent: &DurableExecutionIntent,
    request_id: &str,
    report: &Value,
) -> SdkResult<()> {
    if report["schema_version"] != SCHEMA {
        return Ok(());
    }
    let rejection = Rejection::deserialize(report).map_err(|_| {
        invalid("invalid durable admission rejection shape; outcome not established")
    })?;
    // Bind a refusal to this attempt. Existing execution receipts may instead
    // legitimately carry the request ID of their original admission.
    if rejection.schema_version != SCHEMA
        || rejection.store_id != intent.store_id
        || rejection.execution_id != intent.execution_id
        || rejection.request_digest != intent.request_digest
        || rejection.method != intent.method
        || rejection.request_id != request_id
        || !bounded_text(rejection.request_id, 256)
        || !bounded_text(rejection.process_instance_id, 256)
        || rejection.status != "not_admitted"
        || rejection.reason != "store_capacity"
        || !bounded_text(rejection.message, 1024)
        || rejection.execution_started
        || rejection.automatic_replay_authorized
    {
        return Err(invalid(
            "durable admission rejection identity or policy mismatch; outcome not established",
        ));
    }
    Err(SdkError::DurableAdmissionRejected {
        report: report.clone(),
    })
}

fn bounded_text(value: &str, maximum: usize) -> bool {
    !value.trim().is_empty() && value.len() <= maximum && !value.chars().any(char::is_control)
}
