use crate::execution_observability::{failure_preview, summarize_execution};
use crate::operator_task::operator_task_prepare_preview_or_error;
use crate::preflight_report::build_batch_validation_failure_report;
use crate::run::{compact_report_value, resolve_step_payload};
use crate::{
    HeadlessBlockedConfirmation, HeadlessEngine, HeadlessExecutionBatch,
    HeadlessExecutionStepReport, HeadlessRisk, HeadlessRunReport, find_action_contract,
    is_operator_task_execute_action, is_operator_task_prepare_action, operator_task_error_preview,
    prepare_operator_task_payload, preview_operator_task_execute_payload,
    service_executor_supports_action, validate_batch,
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
        if is_operator_task_prepare_action(action) {
            return prepare_operator_task_payload(payload)
                .map(|result| HeadlessExecutorOutcome {
                    status: "executed".to_string(),
                    result,
                })
                .map_err(|message| HeadlessExecutorError { message });
        }
        if is_operator_task_execute_action(action) {
            return preview_operator_task_execute_payload(payload)
                .map(|result| HeadlessExecutorOutcome {
                    status: "executed".to_string(),
                    result,
                })
                .map_err(|message| HeadlessExecutorError { message });
        }

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
        "hybrid" => find_action_contract(action).is_some_and(|contract| {
            contract.engine == HeadlessEngine::Browser || service_executor_supports_action(action)
        }),
        "service" => service_executor_supports_action(action),
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
    if !validation.ok {
        return build_batch_validation_failure_report(
            batch,
            &format!("execute:{}", executor.name()),
            validation,
        );
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
                payload: compact_report_value(&payload),
                result_preview: build_result_preview(&step.action, step.index, &step.payload),
                requires_confirmation,
            });
            break;
        }

        if is_operator_task_prepare_action(&step.action) {
            match prepare_operator_task_payload(&payload) {
                Ok(preview) => {
                    executed_step_count += 1;
                    results.insert(step.index, preview.clone());
                    steps.push(HeadlessExecutionStepReport {
                        index: step.index,
                        action: step.action.clone(),
                        risk: step.risk,
                        status: "executed".to_string(),
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

        match executor.execute_step(&step.action, step.index, &payload) {
            Ok(outcome) => {
                executed_step_count += 1;
                let result_preview = compact_report_value(&outcome.result);
                results.insert(step.index, outcome.result);
                steps.push(HeadlessExecutionStepReport {
                    index: step.index,
                    action: step.action.clone(),
                    risk: step.risk,
                    status: outcome.status,
                    payload: compact_report_value(&payload),
                    result_preview,
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
                    payload: compact_report_value(&payload),
                    result_preview: failure_preview(step.index, &step.action, error.message),
                    requires_confirmation,
                });
                break;
            }
        }
    }

    let execution_summary = summarize_execution(&steps);

    HeadlessRunReport {
        schema_version: "kyuubiki.headless-execution-run/v1".to_string(),
        workflow_id: batch.workflow_id.clone(),
        mode: format!("execute:{}", executor.name()),
        status,
        executed_step_count,
        warning_count: batch.warnings.len(),
        blocked_by_confirmation,
        validation,
        execution_summary,
        steps,
    }
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
    use super::{
        HeadlessExecutor, HeadlessExecutorError, HeadlessExecutorOutcome, MockHeadlessExecutor,
        execute_batch_with_executor,
    };
    use crate::{HeadlessExecutionBatch, HeadlessExecutionBatchStep, HeadlessRisk};
    use kyuubiki_protocol::compute_operator_task_digest;
    use serde_json::{Value, json};

    struct TimeoutExecutor;

    impl HeadlessExecutor for TimeoutExecutor {
        fn name(&self) -> &'static str {
            "service"
        }

        fn execute_step(
            &mut self,
            _action: &str,
            _step_index: usize,
            _payload: &Value,
        ) -> Result<HeadlessExecutorOutcome, HeadlessExecutorError> {
            Err(HeadlessExecutorError {
                message: "timed out waiting for job job-timeout".to_string(),
            })
        }
    }

    #[derive(Default)]
    struct CountingExecutor {
        calls: usize,
    }

    impl HeadlessExecutor for CountingExecutor {
        fn name(&self) -> &'static str {
            "service"
        }

        fn execute_step(
            &mut self,
            _action: &str,
            _step_index: usize,
            _payload: &Value,
        ) -> Result<HeadlessExecutorOutcome, HeadlessExecutorError> {
            self.calls += 1;
            Ok(HeadlessExecutorOutcome {
                status: "executed".to_string(),
                result: json!({"unexpected": true}),
            })
        }
    }

    #[derive(Default)]
    struct LargeResultExecutor {
        calls: usize,
    }

    impl HeadlessExecutor for LargeResultExecutor {
        fn name(&self) -> &'static str {
            "service"
        }

        fn execute_step(
            &mut self,
            _action: &str,
            _step_index: usize,
            payload: &Value,
        ) -> Result<HeadlessExecutorOutcome, HeadlessExecutorError> {
            self.calls += 1;
            if self.calls == 1 {
                return Ok(HeadlessExecutorOutcome {
                    status: "executed".to_string(),
                    result: json!({"solver_endpoints": (0..256).collect::<Vec<_>>() }),
                });
            }
            assert_eq!(payload["forwarded"].as_array().map(Vec::len), Some(256));
            Ok(HeadlessExecutorOutcome {
                status: "executed".to_string(),
                result: json!({"status": "bound"}),
            })
        }
    }

    #[test]
    fn execution_reports_compact_results_without_breaking_bindings() {
        let batch = HeadlessExecutionBatch {
            schema_version: "kyuubiki.headless-execution-batch/v1".to_string(),
            exported_at: "1970-01-01T00:00:00.000Z".to_string(),
            language: "en".to_string(),
            workflow_id: "large-result-fixture".to_string(),
            template_id: None,
            steps: vec![
                HeadlessExecutionBatchStep {
                    index: 1,
                    action: "service_health".to_string(),
                    risk: HeadlessRisk::Normal,
                    payload: json!({}),
                },
                HeadlessExecutionBatchStep {
                    index: 2,
                    action: "service_health".to_string(),
                    risk: HeadlessRisk::Normal,
                    payload: json!({
                        "forwarded": "{{steps.1.result.solver_endpoints}}"
                    }),
                },
            ],
            warnings: vec![],
        };

        let report =
            execute_batch_with_executor(&batch, &mut LargeResultExecutor::default(), false, false);
        assert_eq!(report.status, "ok");
        assert_eq!(
            report.steps[0].result_preview["solver_endpoints"]["item_count"],
            256
        );
    }

    #[test]
    fn failed_execution_promotes_failure_receipt_to_run_summary() {
        let batch = HeadlessExecutionBatch {
            schema_version: "kyuubiki.headless-execution-batch/v1".to_string(),
            exported_at: "1970-01-01T00:00:00.000Z".to_string(),
            language: "en".to_string(),
            workflow_id: "timeout-fixture".to_string(),
            template_id: None,
            steps: vec![HeadlessExecutionBatchStep {
                index: 1,
                action: "job_wait".to_string(),
                risk: HeadlessRisk::Normal,
                payload: json!({ "job_id": "job-timeout" }),
            }],
            warnings: vec![],
        };

        let report = execute_batch_with_executor(&batch, &mut TimeoutExecutor, false, false);

        let failure = report.execution_summary.failure.expect("failure receipt");
        assert_eq!(failure.error_code, "kyuubiki.headless.job_wait_timeout");
        assert!(failure.retryable);
        assert_eq!(
            report.steps[0].result_preview["error_code"],
            failure.error_code
        );
    }

    #[test]
    fn destructive_block_halts_before_later_side_effects() {
        let batch = HeadlessExecutionBatch {
            schema_version: "kyuubiki.headless-execution-batch/v1".to_string(),
            exported_at: "1970-01-01T00:00:00.000Z".to_string(),
            language: "en".to_string(),
            workflow_id: "risk-stop-fixture".to_string(),
            template_id: None,
            steps: vec![
                HeadlessExecutionBatchStep {
                    index: 1,
                    action: "project_delete".to_string(),
                    risk: HeadlessRisk::Destructive,
                    payload: json!({ "project_id": "project-1" }),
                },
                HeadlessExecutionBatchStep {
                    index: 2,
                    action: "project_create".to_string(),
                    risk: HeadlessRisk::Normal,
                    payload: json!({ "name": "must-not-run" }),
                },
            ],
            warnings: vec![],
        };
        let mut executor = MockHeadlessExecutor;

        let report = execute_batch_with_executor(&batch, &mut executor, false, false);

        assert_eq!(report.status, "blocked");
        assert_eq!(report.executed_step_count, 0);
        assert_eq!(report.steps.len(), 1);
        assert_eq!(report.steps[0].action, "project_delete");
    }

    #[test]
    fn invalid_batch_never_reaches_executor() {
        let batch = HeadlessExecutionBatch {
            schema_version: "kyuubiki.headless-execution-batch/v1".to_string(),
            exported_at: "1970-01-01T00:00:00.000Z".to_string(),
            language: "en".to_string(),
            workflow_id: "invalid-execution".to_string(),
            template_id: None,
            steps: vec![HeadlessExecutionBatchStep {
                index: 1,
                action: "project_create".to_string(),
                risk: HeadlessRisk::Normal,
                payload: json!({}),
            }],
            warnings: vec![],
        };
        let mut executor = CountingExecutor::default();

        let report = execute_batch_with_executor(&batch, &mut executor, false, false);

        assert_eq!(report.status, "invalid");
        assert_eq!(report.executed_step_count, 0);
        assert!(report.steps.is_empty());
        assert_eq!(executor.calls, 0);
    }

    #[test]
    fn mock_executor_reports_structured_operator_task_mirror_error() {
        let mut task = golden_task_fixture();
        task["runtime_hints"]["operator_kind"] = json!("solver");
        task["integrity"]["task_digest"] =
            json!(compute_operator_task_digest(&task).expect("changed task should digest"));
        let batch = HeadlessExecutionBatch {
            schema_version: "kyuubiki.headless-execution-batch/v1".to_string(),
            exported_at: "1970-01-01T00:00:00.000Z".to_string(),
            language: "en".to_string(),
            workflow_id: "operator-task-fixture".to_string(),
            template_id: None,
            steps: vec![HeadlessExecutionBatchStep {
                index: 1,
                action: "operator_task_prepare".to_string(),
                risk: HeadlessRisk::Normal,
                payload: json!({ "task": task }),
            }],
            warnings: vec![],
        };
        let mut executor = MockHeadlessExecutor;

        let report = execute_batch_with_executor(&batch, &mut executor, false, false);

        assert_eq!(report.status, "failed");
        assert_eq!(report.steps[0].status, "failed");
        assert_eq!(
            report.steps[0].result_preview["error_code"],
            "operator_task_mirror_mismatch"
        );
    }

    fn golden_task_fixture() -> Value {
        json!({
            "schema_version": "kyuubiki.operator-task-ir/v1",
            "task_id": "fixture-task",
            "operator": {
                "id": "transform.fixture",
                "family": "fixture",
                "kind": "transform",
                "execution": {
                    "package_ref": "orchestra://operator-package/transform.fixture"
                }
            },
            "descriptor_authoring": {
                "schema_version": "kyuubiki.operator-descriptor-authoring/v1",
                "mode": "rust_native",
                "runtime": "rust",
                "source": "fixture",
                "hot_reloadable": false,
                "execution_language": "language_neutral"
            },
            "node": {},
            "input_artifact": { "x": 1 },
            "config": { "alpha": true },
            "execution_program": {
                "schema_version": "kyuubiki.operator-execution-program/v1",
                "program_id": "transform.fixture",
                "program_family": "fixture",
                "program_kind": "transform",
                "operator_category_id": null,
                "package_ref": "orchestra://operator-package/transform.fixture",
                "package_version": "library-managed",
                "package_integrity": null,
                "runtime_protocol": "kyuubiki.operator-execution/v1",
                "abi": {
                    "kind": "operator_task",
                    "input_encoding": "json",
                    "output_encoding": "json"
                },
                "entrypoint": {
                    "kind": "operator_id",
                    "name": "transform.fixture",
                    "operator_kind": "transform"
                },
                "bindings": {
                    "input_artifact": "task.input_artifact",
                    "config": "task.config",
                    "output_artifact": "task.output_artifact"
                },
                "node_binding": {
                    "node_id": null,
                    "input_ports": [],
                    "output_ports": []
                }
            },
            "dataset_contract": {},
            "orchestration_context": {},
            "runtime_hints": {
                "authority_mode": "central_operator_library",
                "execution_mode": "orchestra_fetch",
                "source_ref": null,
                "package_ref": "orchestra://operator-package/transform.fixture",
                "package_version": "library-managed",
                "placement_tags": [],
                "required_capabilities": [],
                "cache_scope": "job",
                "agent_fetchable": true,
                "operator_kind": "transform"
            },
            "integrity": {
                "task_digest": "86c14d1f22af9d14ab35669a2fcb869afab097a9883e6deabf92a362d8f4469f"
            }
        })
    }
}
