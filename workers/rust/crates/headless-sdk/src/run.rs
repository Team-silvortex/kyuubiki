use crate::execution_observability::summarize_execution;
use crate::operator_task::operator_task_prepare_preview_or_error;
use crate::preflight_report::build_batch_validation_failure_report;
use crate::{
    HeadlessExecutionBatch, HeadlessExecutionSummary, HeadlessRisk, HeadlessValidationReport,
    is_operator_task_execute_action, is_operator_task_prepare_action, operator_task_error_preview,
    prepare_operator_task_payload, preview_operator_task_execute_payload, validate_batch,
};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::borrow::Cow;
use std::collections::HashMap;

const MAX_REPORT_ARRAY_ITEMS: usize = 128;
const REPORT_ARRAY_SAMPLE_ITEMS: usize = 3;
const MAX_REPORT_STRING_BYTES: usize = 4_096;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HeadlessBlockedConfirmation {
    pub index: usize,
    pub risk: HeadlessRisk,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HeadlessExecutionStepReport {
    pub index: usize,
    pub action: String,
    pub risk: HeadlessRisk,
    pub status: String,
    pub payload: Value,
    pub result_preview: Value,
    pub requires_confirmation: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HeadlessRunReport {
    pub schema_version: String,
    pub workflow_id: String,
    pub mode: String,
    pub status: String,
    pub executed_step_count: usize,
    pub warning_count: usize,
    pub blocked_by_confirmation: Option<HeadlessBlockedConfirmation>,
    pub validation: HeadlessValidationReport,
    pub execution_summary: HeadlessExecutionSummary,
    pub steps: Vec<HeadlessExecutionStepReport>,
}

pub fn run_batch_dry(
    batch: &HeadlessExecutionBatch,
    allow_sensitive: bool,
    allow_destructive: bool,
) -> HeadlessRunReport {
    let validation = validate_batch(batch);
    if !validation.ok {
        return build_batch_validation_failure_report(batch, "dry_run", validation);
    }
    let mut results = HashMap::<usize, Value>::new();
    let mut steps = Vec::with_capacity(batch.steps.len());
    let mut executed_step_count = 0;
    let mut blocked_by_confirmation = None;
    let mut status = if validation.ok { "ok" } else { "invalid" }.to_string();

    for step in &batch.steps {
        let requires_confirmation = matches!(
            step.risk,
            HeadlessRisk::Sensitive | HeadlessRisk::Destructive
        );
        let blocked = (step.risk == HeadlessRisk::Sensitive && !allow_sensitive)
            || (step.risk == HeadlessRisk::Destructive && !allow_destructive);
        let payload = resolve_step_payload(&step.payload, &results);
        if !blocked
            && (is_operator_task_prepare_action(&step.action)
                || is_operator_task_execute_action(&step.action))
        {
            let prepared = if is_operator_task_execute_action(&step.action) {
                preview_operator_task_execute_payload(&payload)
            } else {
                prepare_operator_task_payload(&payload)
            };

            match prepared {
                Ok(preview) => {
                    executed_step_count += 1;
                    results.insert(step.index, preview.clone());
                    steps.push(HeadlessExecutionStepReport {
                        index: step.index,
                        action: step.action.clone(),
                        risk: step.risk,
                        status: "dry_run".to_string(),
                        payload: compact_report_value(&payload),
                        result_preview: preview,
                        requires_confirmation,
                    });
                }
                Err(_) => {
                    status = "failed".to_string();
                    let result_preview = operator_task_prepare_preview_or_error(&payload);
                    steps.push(HeadlessExecutionStepReport {
                        index: step.index,
                        action: step.action.clone(),
                        risk: step.risk,
                        status: "failed".to_string(),
                        payload: compact_report_value(&payload),
                        result_preview,
                        requires_confirmation,
                    });
                    break;
                }
            }
            continue;
        }

        let result_preview = build_result_preview(&step.action, step.index, &payload);
        let step_status = if blocked { "blocked" } else { "dry_run" }.to_string();
        if blocked && blocked_by_confirmation.is_none() {
            blocked_by_confirmation = Some(HeadlessBlockedConfirmation {
                index: step.index,
                risk: step.risk,
            });
            status = "blocked".to_string();
        }
        if !blocked {
            executed_step_count += 1;
            results.insert(step.index, result_preview.clone());
        }
        steps.push(HeadlessExecutionStepReport {
            index: step.index,
            action: step.action.clone(),
            risk: step.risk,
            status: step_status,
            payload: compact_report_value(&payload),
            result_preview,
            requires_confirmation,
        });
    }

    let execution_summary = summarize_execution(&steps);

    HeadlessRunReport {
        schema_version: "kyuubiki.headless-execution-run/v1".to_string(),
        workflow_id: batch.workflow_id.clone(),
        mode: "dry_run".to_string(),
        status,
        executed_step_count,
        warning_count: batch.warnings.len(),
        blocked_by_confirmation,
        validation,
        execution_summary,
        steps,
    }
}

pub(crate) fn resolve_step_payload<'a>(
    value: &'a Value,
    results: &HashMap<usize, Value>,
) -> Cow<'a, Value> {
    if results.is_empty() {
        Cow::Borrowed(value)
    } else {
        Cow::Owned(resolve_value(value, results))
    }
}

fn resolve_value(value: &Value, results: &HashMap<usize, Value>) -> Value {
    match value {
        Value::String(text) => parse_binding(text)
            .and_then(|(step, output)| {
                results
                    .get(&step)
                    .and_then(|result| result.get(&output))
                    .cloned()
            })
            .unwrap_or_else(|| value.clone()),
        Value::Array(items) => Value::Array(
            items
                .iter()
                .map(|item| resolve_value(item, results))
                .collect(),
        ),
        Value::Object(fields) => Value::Object(
            fields
                .iter()
                .map(|(key, value)| (key.clone(), resolve_value(value, results)))
                .collect::<Map<String, Value>>(),
        ),
        _ => value.clone(),
    }
}

pub(crate) fn compact_report_value(value: &Value) -> Value {
    match value {
        Value::Array(items) if items.len() > MAX_REPORT_ARRAY_ITEMS => {
            let sample = items
                .iter()
                .take(REPORT_ARRAY_SAMPLE_ITEMS)
                .map(compact_report_value)
                .collect::<Vec<_>>();
            Value::Object(Map::from_iter([
                (
                    "$kyuubiki_report_summary".to_string(),
                    Value::String("array".to_string()),
                ),
                ("item_count".to_string(), Value::from(items.len() as u64)),
                ("sample".to_string(), Value::Array(sample)),
                (
                    "omitted_item_count".to_string(),
                    Value::from((items.len() - REPORT_ARRAY_SAMPLE_ITEMS) as u64),
                ),
            ]))
        }
        Value::Array(items) => Value::Array(items.iter().map(compact_report_value).collect()),
        Value::Object(fields) => Value::Object(
            fields
                .iter()
                .map(|(key, value)| (key.clone(), compact_report_value(value)))
                .collect(),
        ),
        Value::String(text) if text.len() > MAX_REPORT_STRING_BYTES => {
            Value::Object(Map::from_iter([
                (
                    "$kyuubiki_report_summary".to_string(),
                    Value::String("string".to_string()),
                ),
                ("byte_count".to_string(), Value::from(text.len() as u64)),
                (
                    "prefix".to_string(),
                    Value::String(text.chars().take(256).collect()),
                ),
            ]))
        }
        _ => value.clone(),
    }
}

fn parse_binding(text: &str) -> Option<(usize, String)> {
    let trimmed = text.trim();
    let inner = trimmed.strip_prefix("{{")?.strip_suffix("}}")?.trim();
    let rest = inner.strip_prefix("steps.")?;
    let (step_text, output_path) = rest.split_once(".result.")?;
    Some((step_text.parse().ok()?, output_path.trim().to_string()))
}

fn build_result_preview(action: &str, step_index: usize, payload: &Value) -> Value {
    let mut map = Map::new();
    map.insert("step_index".to_string(), Value::from(step_index as u64));
    map.insert("action".to_string(), Value::from(action.to_string()));
    match action {
        "service_health" => {
            map.insert("status".to_string(), Value::from("ok"));
            map.insert(
                "solver_endpoints".to_string(),
                Value::Array(vec![Value::from("127.0.0.1:5001")]),
            );
        }
        "project_create" => {
            map.insert(
                "project_id".to_string(),
                Value::from(format!("project_{step_index:03}")),
            );
        }
        "model_create" => {
            map.insert(
                "model_id".to_string(),
                Value::from(format!("model_{step_index:03}")),
            );
            map.insert(
                "latest_version_id".to_string(),
                Value::from(format!("version_{step_index:03}")),
            );
        }
        "model_version_create" => {
            map.insert(
                "model_version_id".to_string(),
                Value::from(format!("version_{step_index:03}")),
            );
        }
        "workflow_submit_catalog"
        | "workflow_submit_graph"
        | "direct_mesh_solve"
        | "solve_from_model_version" => {
            map.insert(
                "job_id".to_string(),
                Value::from(format!("job_{step_index:03}")),
            );
            map.insert("status".to_string(), Value::from("submitted"));
        }
        "solve_and_wait_from_model_version" => {
            map.insert(
                "job_id".to_string(),
                Value::from(format!("job_{step_index:03}")),
            );
            map.insert("status".to_string(), Value::from("completed"));
            map.insert(
                "result".to_string(),
                Value::Object(Map::from_iter([(
                    "kind".to_string(),
                    Value::from("simulated_result"),
                )])),
            );
        }
        "operator_task_prepare" => {
            return operator_task_prepare_preview_or_error(payload);
        }
        "operator_task_execute" => {
            return preview_operator_task_execute_payload(payload)
                .unwrap_or_else(operator_task_error_preview);
        }
        "job_wait" | "job_fetch" => {
            map.insert(
                "job_id".to_string(),
                payload
                    .get("job_id")
                    .cloned()
                    .unwrap_or_else(|| Value::from(format!("job_{step_index:03}"))),
            );
            map.insert("status".to_string(), Value::from("completed"));
            map.insert("progress".to_string(), Value::from(1.0));
        }
        "result_fetch" => {
            map.insert(
                "job_id".to_string(),
                payload
                    .get("job_id")
                    .cloned()
                    .unwrap_or_else(|| Value::from(format!("job_{step_index:03}"))),
            );
            map.insert(
                "result".to_string(),
                Value::Object(Map::from_iter([(
                    "kind".to_string(),
                    Value::from("simulated_result"),
                )])),
            );
        }
        "open_page" => {
            map.insert(
                "url".to_string(),
                payload
                    .get("url")
                    .cloned()
                    .unwrap_or_else(|| Value::from("about:blank")),
            );
            map.insert("status".to_string(), Value::from("opened"));
            map.insert("ok".to_string(), Value::Bool(true));
        }
        "click" => {
            map.insert(
                "selector".to_string(),
                payload
                    .get("selector")
                    .cloned()
                    .unwrap_or_else(|| Value::from("")),
            );
        }
        "type" => {
            map.insert(
                "selector".to_string(),
                payload
                    .get("selector")
                    .cloned()
                    .unwrap_or_else(|| Value::from("")),
            );
            map.insert(
                "value".to_string(),
                payload
                    .get("value")
                    .cloned()
                    .unwrap_or_else(|| Value::from("")),
            );
        }
        "press" => {
            map.insert(
                "key".to_string(),
                payload
                    .get("key")
                    .cloned()
                    .unwrap_or_else(|| Value::from("")),
            );
        }
        "select" => {
            map.insert(
                "selector".to_string(),
                payload
                    .get("selector")
                    .cloned()
                    .unwrap_or_else(|| Value::from("")),
            );
            map.insert(
                "values".to_string(),
                payload
                    .get("value")
                    .cloned()
                    .unwrap_or_else(|| Value::Array(vec![])),
            );
        }
        "wait" => {
            map.insert(
                "timeout_ms".to_string(),
                payload
                    .get("timeout")
                    .cloned()
                    .unwrap_or_else(|| Value::from(0)),
            );
        }
        "assert_text" => {
            map.insert(
                "selector".to_string(),
                payload
                    .get("selector")
                    .cloned()
                    .unwrap_or_else(|| Value::from("")),
            );
            map.insert(
                "text".to_string(),
                payload
                    .get("text")
                    .cloned()
                    .unwrap_or_else(|| Value::from("")),
            );
        }
        "snapshot" => {
            map.insert(
                "path".to_string(),
                payload
                    .get("file")
                    .cloned()
                    .unwrap_or_else(|| Value::from(format!("snapshot-{step_index:03}.png"))),
            );
        }
        _ => {}
    }
    Value::Object(map)
}

#[cfg(test)]
mod tests {
    use super::run_batch_dry;
    use crate::{HeadlessExecutionBatch, HeadlessExecutionBatchStep, HeadlessRisk};
    use serde_json::{Value, json};

    #[test]
    fn dry_run_summarizes_large_mesh_arrays_instead_of_echoing_them() {
        let nodes = (0..1_000)
            .map(|index| json!({ "x": index, "temperature": 20.0 }))
            .collect::<Vec<Value>>();
        let batch = HeadlessExecutionBatch {
            schema_version: "kyuubiki.headless-execution-batch/v1".to_string(),
            exported_at: "1970-01-01T00:00:00.000Z".to_string(),
            language: "en".to_string(),
            workflow_id: "large-report-fixture".to_string(),
            template_id: None,
            steps: vec![HeadlessExecutionBatchStep {
                index: 1,
                action: "solve_heat_plane_quad_2d".to_string(),
                risk: HeadlessRisk::Normal,
                payload: json!({ "model": { "nodes": nodes, "elements": [] } }),
            }],
            warnings: vec![],
        };

        let report = run_batch_dry(&batch, false, false);
        let summary = &report.steps[0].payload["model"]["nodes"];
        assert_eq!(summary["$kyuubiki_report_summary"], "array");
        assert_eq!(summary["item_count"], 1_000);
        assert_eq!(summary["sample"].as_array().map(Vec::len), Some(3));
        assert!(serde_json::to_vec(&report).unwrap().len() < 20_000);
    }

    #[test]
    fn invalid_dry_run_fails_before_previewing_any_step() {
        let batch = HeadlessExecutionBatch {
            schema_version: "kyuubiki.headless-execution-batch/v1".to_string(),
            exported_at: "1970-01-01T00:00:00.000Z".to_string(),
            language: "en".to_string(),
            workflow_id: "invalid-dry-run".to_string(),
            template_id: None,
            steps: vec![HeadlessExecutionBatchStep {
                index: 1,
                action: "project_create".to_string(),
                risk: HeadlessRisk::Normal,
                payload: json!({}),
            }],
            warnings: vec![],
        };

        let report = run_batch_dry(&batch, false, false);

        assert_eq!(report.status, "invalid");
        assert_eq!(report.executed_step_count, 0);
        assert!(report.steps.is_empty());
        let failure = report.execution_summary.failure.expect("failure receipt");
        assert_eq!(failure.error_code, "kyuubiki.headless.document_validation");
        assert_eq!(failure.stage, "batch_validation");
    }
}
