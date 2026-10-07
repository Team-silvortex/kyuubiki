use crate::service_executor::request_json_with_timeout;
use crate::service_executor_operator_task::operator_task_execution_outcome;
use crate::{HeadlessExecutorError, HeadlessExecutorOutcome};
use serde::Deserialize;
use serde_json::{Value, json};
use std::time::{Duration, Instant};

const SCHEMA: &str = "kyuubiki.operator-task-request-budget/v1";
const MAX_PHASE_MS: u64 = 600_000;
const RESPONSE_GRACE_MS: u64 = 10_000;
pub(crate) const TRANSPORT_FAILURE_PREFIX: &str = "operator task transport failed (execution outcome unknown; inspect the task before explicit rerun): ";

/// Per-request waiting policy, outside the signed TaskIR and solver configuration.
/// A client timeout is an unknown execution outcome, not proof of cancellation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OperatorTaskRequestBudget {
    queue_timeout_ms: u64,
    request_timeout_ms: u64,
}

impl Default for OperatorTaskRequestBudget {
    fn default() -> Self {
        Self {
            queue_timeout_ms: 120_000,
            request_timeout_ms: 120_000,
        }
    }
}

impl OperatorTaskRequestBudget {
    /// Each phase must be in 1..=600_000 ms. HTTP gets an additional 10 s for
    /// connection/framing overhead; received bytes never renew this total budget.
    pub fn new(
        queue_timeout_ms: u64,
        request_timeout_ms: u64,
    ) -> Result<Self, HeadlessExecutorError> {
        if !(1..=MAX_PHASE_MS).contains(&queue_timeout_ms)
            || !(1..=MAX_PHASE_MS).contains(&request_timeout_ms)
        {
            return Err(invalid_budget());
        }
        Ok(Self {
            queue_timeout_ms,
            request_timeout_ms,
        })
    }

    pub fn queue_timeout(self) -> Duration {
        Duration::from_millis(self.queue_timeout_ms)
    }

    pub fn request_timeout(self) -> Duration {
        Duration::from_millis(self.request_timeout_ms)
    }

    pub fn http_timeout(self) -> Duration {
        Duration::from_millis(self.queue_timeout_ms + self.request_timeout_ms + RESPONSE_GRACE_MS)
    }

    pub fn to_value(self) -> Value {
        json!({"schema_version":SCHEMA, "queue_timeout_ms":self.queue_timeout_ms,
            "request_timeout_ms":self.request_timeout_ms})
    }

    fn from_value(value: &Value) -> Result<Self, HeadlessExecutorError> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct WireBudget {
            schema_version: String,
            queue_timeout_ms: u64,
            request_timeout_ms: u64,
        }
        let wire: WireBudget =
            serde_json::from_value(value.clone()).map_err(|_| invalid_budget())?;
        if wire.schema_version != SCHEMA {
            return Err(invalid_budget());
        }
        Self::new(wire.queue_timeout_ms, wire.request_timeout_ms)
    }
}

pub(crate) fn execute_operator_task(
    base_url: &str,
    api_token: Option<&str>,
    payload: &Value,
    default_budget: OperatorTaskRequestBudget,
) -> Result<HeadlessExecutorOutcome, HeadlessExecutorError> {
    let started = Instant::now();
    let budget = payload
        .get("execution_budget")
        .map(OperatorTaskRequestBudget::from_value)
        .transpose()?
        .unwrap_or(default_budget);
    let deadline = started + budget.http_timeout();
    let prepared = crate::prepare_operator_task_payload(payload)
        .map_err(|message| HeadlessExecutorError { message })?;
    let mut body = payload.clone();
    body["execution_budget"] = budget.to_value();
    let result = request_json_with_timeout(
        base_url,
        api_token,
        "POST",
        "/api/v1/operator-tasks/execute",
        Some(body),
        Some(deadline),
        budget.http_timeout(),
    )
    .map_err(|error| HeadlessExecutorError {
        message: format!("{TRANSPORT_FAILURE_PREFIX}{}", error.message),
    })?;
    // Receipt verification remains mandatory even when transport completes successfully.
    operator_task_execution_outcome(&prepared, result)
}

fn invalid_budget() -> HeadlessExecutorError {
    HeadlessExecutorError {
        message: "invalid operator task execution_budget: require the v1 schema and integer queue_timeout_ms/request_timeout_ms in 1..=600000, with no unknown fields".into(),
    }
}

#[cfg(test)]
#[path = "service_executor_task_budget_tests.rs"]
mod tests;
