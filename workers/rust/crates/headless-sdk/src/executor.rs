use crate::{
    HeadlessBlockedConfirmation, HeadlessEngine, HeadlessExecutionBatch,
    HeadlessExecutionStepReport, HeadlessRisk, HeadlessRunReport, find_action_contract,
    validate_batch,
};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HeadlessExecutorOutcome {
    pub status: String,
    pub result: Value,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeadlessExecutorError {
    pub message: String,
}

pub trait HeadlessExecutor {
    fn name(&self) -> &'static str;
    fn execute_step(
        &mut self,
        action: &str,
        step_index: usize,
        payload: &Value,
    ) -> Result<HeadlessExecutorOutcome, HeadlessExecutorError>;
}

#[derive(Debug, Default)]
pub struct MockHeadlessExecutor;

impl HeadlessExecutor for MockHeadlessExecutor {
    fn name(&self) -> &'static str {
        "mock"
    }

    fn execute_step(
        &mut self,
        action: &str,
        step_index: usize,
        payload: &Value,
    ) -> Result<HeadlessExecutorOutcome, HeadlessExecutorError> {
        Ok(HeadlessExecutorOutcome {
            status: "executed".to_string(),
            result: build_result_preview(action, step_index, payload),
        })
    }
}

pub fn collect_executor_compatibility_issues(
    batch: &HeadlessExecutionBatch,
    executor_name: &str,
) -> Vec<String> {
    batch
        .steps
        .iter()
        .filter_map(|step| {
            if executor_supports_action(executor_name, &step.action) {
                None
            } else {
                Some(format!(
                    "step {} ({}) is not compatible with executor {}",
                    step.index, step.action, executor_name
                ))
            }
        })
        .collect()
}

pub fn executor_supports_action(executor_name: &str, action: &str) -> bool {
    match executor_name {
        "mock" => true,
        "hybrid" => find_action_contract(action).is_some(),
        "service" => find_action_contract(action)
            .is_some_and(|contract| contract.engine == HeadlessEngine::Service),
        _ => false,
    }
}

pub fn execute_batch_with_executor<E: HeadlessExecutor>(
    batch: &HeadlessExecutionBatch,
    executor: &mut E,
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
        if blocked {
            if blocked_by_confirmation.is_none() {
                blocked_by_confirmation = Some(HeadlessBlockedConfirmation {
                    index: step.index,
                    risk: step.risk,
                });
                status = "blocked".to_string();
            }
            steps.push(HeadlessExecutionStepReport {
                index: step.index,
                action: step.action.clone(),
                risk: step.risk,
                status: "blocked".to_string(),
                payload,
                result_preview: build_result_preview(&step.action, step.index, &step.payload),
                requires_confirmation,
            });
            continue;
        }

        match executor.execute_step(&step.action, step.index, &payload) {
            Ok(outcome) => {
                executed_step_count += 1;
                results.insert(step.index, outcome.result.clone());
                steps.push(HeadlessExecutionStepReport {
                    index: step.index,
                    action: step.action.clone(),
                    risk: step.risk,
                    status: outcome.status,
                    payload,
                    result_preview: outcome.result,
                    requires_confirmation,
                });
            }
            Err(error) => {
                status = "failed".to_string();
                steps.push(HeadlessExecutionStepReport {
                    index: step.index,
                    action: step.action.clone(),
                    risk: step.risk,
                    status: "failed".to_string(),
                    payload,
                    result_preview: Value::Object(Map::from_iter([(
                        "error".to_string(),
                        Value::from(error.message),
                    )])),
                    requires_confirmation,
                });
                break;
            }
        }
    }

    HeadlessRunReport {
        schema_version: "kyuubiki.headless-execution-run/v1".to_string(),
        workflow_id: batch.workflow_id.clone(),
        mode: format!("execute:{}", executor.name()),
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
