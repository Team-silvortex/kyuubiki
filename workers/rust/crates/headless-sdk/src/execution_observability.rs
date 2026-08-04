use crate::HeadlessExecutionStepReport;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::HashMap;

pub const HEADLESS_EXECUTION_SUMMARY_SCHEMA_VERSION: &str =
    "kyuubiki.headless-execution-summary/v1";
pub const HEADLESS_FAILURE_RECEIPT_SCHEMA_VERSION: &str = "kyuubiki.headless-failure-receipt/v1";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HeadlessExecutionSummary {
    pub schema_version: String,
    pub job_count: usize,
    pub job_ids: Vec<String>,
    pub jobs: Vec<HeadlessJobTimeline>,
    pub failure: Option<HeadlessFailureReceipt>,
}

impl Default for HeadlessExecutionSummary {
    fn default() -> Self {
        Self {
            schema_version: HEADLESS_EXECUTION_SUMMARY_SCHEMA_VERSION.to_string(),
            job_count: 0,
            job_ids: Vec::new(),
            jobs: Vec::new(),
            failure: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HeadlessJobTimeline {
    pub job_id: String,
    pub status: Option<String>,
    pub worker_id: Option<String>,
    pub phase: Option<String>,
    pub created_at: Option<String>,
    pub execution_started_at: Option<String>,
    pub updated_at: Option<String>,
    pub queue_wait_ms: Option<u64>,
    pub execution_elapsed_ms: Option<u64>,
    pub total_elapsed_ms: Option<u64>,
    pub effective_timeout_ms: Option<u64>,
    pub effective_deadline: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HeadlessFailureReceipt {
    pub schema_version: String,
    pub error_code: String,
    pub category: String,
    pub stage: String,
    pub step_index: usize,
    pub action: String,
    pub message: String,
    pub retryable: bool,
    pub retry_strategy: String,
    pub recommended_action: String,
}

pub(crate) fn summarize_execution(
    steps: &[HeadlessExecutionStepReport],
) -> HeadlessExecutionSummary {
    let mut jobs = Vec::<HeadlessJobTimeline>::new();
    let mut job_indices = HashMap::<String, usize>::new();

    for step in steps {
        if let Some(timeline) = job_timeline(&step.result_preview) {
            if let Some(index) = job_indices.get(&timeline.job_id).copied() {
                jobs[index] = timeline;
            } else {
                job_indices.insert(timeline.job_id.clone(), jobs.len());
                jobs.push(timeline);
            }
        }
    }

    let failure = steps.iter().find_map(|step| {
        step.result_preview
            .get("failure_receipt")
            .cloned()
            .and_then(|value| serde_json::from_value(value).ok())
    });
    let job_ids = jobs.iter().map(|job| job.job_id.clone()).collect();

    HeadlessExecutionSummary {
        schema_version: HEADLESS_EXECUTION_SUMMARY_SCHEMA_VERSION.to_string(),
        job_count: jobs.len(),
        job_ids,
        jobs,
        failure,
    }
}

pub(crate) fn failure_preview(step_index: usize, action: &str, message: String) -> Value {
    let receipt = classify_failure(step_index, action, message);
    json!({
        "error": receipt.message,
        "error_code": receipt.error_code,
        "failure_receipt": receipt,
    })
}

fn classify_failure(step_index: usize, action: &str, message: String) -> HeadlessFailureReceipt {
    let normalized = message.to_ascii_lowercase();
    let (category, stage, retryable, retry_strategy, recommended_action) = if action == "job_wait"
        && normalized.contains("timed out waiting")
    {
        (
            "job_wait_timeout",
            "job_wait",
            true,
            "bounded_exponential_backoff",
            "Fetch the job once, inspect its timing phase, then resume job_wait with a larger client timeout if the server deadline is still active.",
        )
    } else if normalized.contains("agent_queue_timeout")
        || normalized.contains("waiting for agent capacity")
    {
        (
            "agent_queue_timeout",
            "queue",
            true,
            "bounded_exponential_backoff",
            "Inspect agent capacity and queue health, then retry without resubmitting an already accepted job.",
        )
    } else if normalized.contains("401")
        || normalized.contains("403")
        || normalized.contains("unauthorized")
        || normalized.contains("forbidden")
    {
        (
            "authorization_failure",
            "authorization",
            false,
            "none",
            "Repair credentials or policy before retrying.",
        )
    } else if normalized.contains("endpoint not deployed") || normalized.contains("(404)") {
        (
            "endpoint_not_deployed",
            "routing",
            false,
            "none",
            "Connect to a compatible control-plane release or choose a supported action.",
        )
    } else if normalized.contains("connection refused")
        || normalized.contains("failed to connect")
        || normalized.contains("failed to read service response")
        || normalized.contains("failed to write service request")
        || normalized.contains("invalid http response")
    {
        (
            "transport_failure",
            "transport",
            true,
            "bounded_exponential_backoff",
            "Check control-plane health and network reachability before retrying.",
        )
    } else if action == "result_fetch" {
        (
            "result_fetch_failure",
            "result_fetch",
            true,
            "bounded_exponential_backoff",
            "Keep the job_id, verify terminal job state, then retry result_fetch.",
        )
    } else if normalized.contains("cancelled") {
        (
            "job_cancelled",
            "execution",
            false,
            "none",
            "Inspect the cancellation actor and reason before creating a replacement job.",
        )
    } else if normalized.contains("missing required")
        || normalized.contains("not compatible")
        || normalized.contains("unsupported")
        || normalized.contains("validation")
    {
        (
            "contract_failure",
            "validation",
            false,
            "none",
            "Repair the execution batch or executor selection before retrying.",
        )
    } else {
        (
            "runtime_failure",
            "execution",
            false,
            "none",
            "Inspect the failed step and service job status before deciding whether to retry.",
        )
    };

    HeadlessFailureReceipt {
        schema_version: HEADLESS_FAILURE_RECEIPT_SCHEMA_VERSION.to_string(),
        error_code: format!("kyuubiki.headless.{category}"),
        category: category.to_string(),
        stage: stage.to_string(),
        step_index,
        action: action.to_string(),
        message,
        retryable,
        retry_strategy: retry_strategy.to_string(),
        recommended_action: recommended_action.to_string(),
    }
}

fn job_timeline(preview: &Value) -> Option<HeadlessJobTimeline> {
    let job = preview.get("job").unwrap_or(preview);
    let job_id = string_field(preview, "job_id").or_else(|| string_field(job, "job_id"))?;
    let timing = job
        .get("status_detail")
        .and_then(|value| value.get("timing"));

    Some(HeadlessJobTimeline {
        job_id,
        status: string_field(preview, "status").or_else(|| string_field(job, "status")),
        worker_id: string_field(job, "worker_id"),
        phase: timing.and_then(|value| string_field(value, "phase")),
        created_at: string_field(job, "created_at"),
        execution_started_at: string_field(job, "execution_started_at"),
        updated_at: string_field(job, "updated_at"),
        queue_wait_ms: timing.and_then(|value| u64_field(value, "queue_wait_ms")),
        execution_elapsed_ms: timing.and_then(|value| u64_field(value, "execution_elapsed_ms")),
        total_elapsed_ms: timing.and_then(|value| u64_field(value, "total_elapsed_ms")),
        effective_timeout_ms: timing.and_then(|value| u64_field(value, "effective_timeout_ms")),
        effective_deadline: timing.and_then(|value| string_field(value, "effective_deadline")),
    })
}

fn string_field(value: &Value, key: &str) -> Option<String> {
    value.get(key).and_then(Value::as_str).map(str::to_string)
}

fn u64_field(value: &Value, key: &str) -> Option<u64> {
    value.get(key).and_then(Value::as_u64)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::HeadlessRisk;

    fn step(
        index: usize,
        action: &str,
        status: &str,
        result_preview: Value,
    ) -> HeadlessExecutionStepReport {
        HeadlessExecutionStepReport {
            index,
            action: action.to_string(),
            risk: HeadlessRisk::Normal,
            status: status.to_string(),
            payload: json!({}),
            result_preview,
            requires_confirmation: false,
        }
    }

    #[test]
    fn summary_keeps_latest_timeline_for_each_job() {
        let steps = vec![
            step(
                1,
                "solve_heat_plane_quad_2d",
                "executed",
                json!({
                    "job_id": "job-a",
                    "status": "queued",
                    "job": {"job_id": "job-a", "status": "queued"}
                }),
            ),
            step(
                2,
                "job_wait",
                "executed",
                json!({
                    "job_id": "job-a",
                    "status": "completed",
                    "job": {
                        "job_id": "job-a",
                        "status": "completed",
                        "worker_id": "agent-a",
                        "status_detail": {"timing": {
                            "phase": "execution",
                            "queue_wait_ms": 25,
                            "execution_elapsed_ms": 80,
                            "total_elapsed_ms": 105,
                            "effective_timeout_ms": 1800000
                        }}
                    }
                }),
            ),
        ];

        let summary = summarize_execution(&steps);

        assert_eq!(summary.job_ids, ["job-a"]);
        assert_eq!(summary.jobs[0].status.as_deref(), Some("completed"));
        assert_eq!(summary.jobs[0].queue_wait_ms, Some(25));
        assert_eq!(summary.jobs[0].execution_elapsed_ms, Some(80));

        let schema: Value = serde_json::from_str(include_str!(
            "../../../../../schemas/headless-execution-summary.schema.json"
        ))
        .expect("execution summary schema");
        assert_eq!(
            schema["properties"]["schema_version"]["const"],
            HEADLESS_EXECUTION_SUMMARY_SCHEMA_VERSION
        );
    }

    #[test]
    fn timeout_failure_is_machine_actionable() {
        let preview = failure_preview(2, "job_wait", "timed out waiting for job job-a".to_string());

        assert_eq!(preview["error_code"], "kyuubiki.headless.job_wait_timeout");
        assert_eq!(preview["failure_receipt"]["retryable"], true);
        assert_eq!(
            preview["failure_receipt"]["retry_strategy"],
            "bounded_exponential_backoff"
        );

        let schema: Value = serde_json::from_str(include_str!(
            "../../../../../schemas/headless-failure-receipt.schema.json"
        ))
        .expect("failure receipt schema");
        assert_eq!(
            schema["properties"]["schema_version"]["const"],
            HEADLESS_FAILURE_RECEIPT_SCHEMA_VERSION
        );
        assert!(
            schema["properties"]["category"]["enum"]
                .as_array()
                .is_some_and(
                    |categories| categories.contains(&preview["failure_receipt"]["category"])
                )
        );
    }
}
