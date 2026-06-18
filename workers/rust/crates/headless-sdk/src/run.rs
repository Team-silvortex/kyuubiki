use crate::{HeadlessExecutionBatch, HeadlessRisk, HeadlessValidationReport, validate_batch};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::HashMap;

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
    pub steps: Vec<HeadlessExecutionStepReport>,
}

pub fn run_batch_dry(
    batch: &HeadlessExecutionBatch,
    allow_sensitive: bool,
    allow_destructive: bool,
) -> HeadlessRunReport {
    let validation = validate_batch(batch);
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
        let payload = resolve_value(&step.payload, &results);
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
            payload,
            result_preview,
            requires_confirmation,
        });
    }

    HeadlessRunReport {
        schema_version: "kyuubiki.headless-execution-run/v1".to_string(),
        workflow_id: batch.workflow_id.clone(),
        mode: "dry_run".to_string(),
        status,
        executed_step_count,
        warning_count: batch.warnings.len(),
        blocked_by_confirmation,
        validation,
        steps,
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
