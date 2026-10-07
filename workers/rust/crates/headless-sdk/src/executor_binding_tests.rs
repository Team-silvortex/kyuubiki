use crate::{
    HeadlessExecutionBatch, HeadlessExecutionBatchStep, HeadlessExecutor, HeadlessExecutorError,
    HeadlessExecutorOutcome, HeadlessRisk, execute_batch_with_executor, run_batch_dry,
    validate_batch,
};
use serde_json::{Value, json};

#[derive(Default)]
struct RecordingExecutor {
    first_result: Value,
    calls: Vec<Value>,
}

impl HeadlessExecutor for RecordingExecutor {
    fn name(&self) -> &'static str {
        "service"
    }

    fn execute_step(
        &mut self,
        _action: &str,
        _index: usize,
        payload: &Value,
    ) -> Result<HeadlessExecutorOutcome, HeadlessExecutorError> {
        self.calls.push(payload.clone());
        Ok(HeadlessExecutorOutcome {
            status: "executed".into(),
            result: self.first_result.clone(),
        })
    }
}

fn batch(first_action: &str, first_payload: Value, next_payload: Value) -> HeadlessExecutionBatch {
    HeadlessExecutionBatch {
        schema_version: "kyuubiki.headless-execution-batch/v1".into(),
        exported_at: "2026-10-07T00:00:00Z".into(),
        language: "en".into(),
        workflow_id: "runtime-binding-regression".into(),
        template_id: None,
        steps: vec![
            HeadlessExecutionBatchStep {
                index: 1,
                action: first_action.into(),
                risk: HeadlessRisk::Normal,
                payload: first_payload,
            },
            HeadlessExecutionBatchStep {
                index: 2,
                action: "project_create".into(),
                risk: HeadlessRisk::Normal,
                payload: next_payload,
            },
            HeadlessExecutionBatchStep {
                index: 3,
                action: "project_create".into(),
                risk: HeadlessRisk::Normal,
                payload: json!({"name":"later-side-effect-must-not-run"}),
            },
        ],
        warnings: vec![],
    }
}

#[test]
fn missing_declared_output_halts_before_any_dependent_executor_call() {
    let batch = batch(
        "service_health",
        json!({}),
        json!({"name":"{{steps.1.result.service}}"}),
    );
    assert!(
        validate_batch(&batch).ok,
        "declared output passes preflight"
    );
    for result in [json!({"status":"ok"}), json!(null), json!([]), json!("ok")] {
        let mut executor = RecordingExecutor {
            first_result: result,
            ..Default::default()
        };
        let report = execute_batch_with_executor(&batch, &mut executor, false, false);
        assert_eq!(report.status, "failed");
        assert_eq!(report.executed_step_count, 1);
        assert_eq!(report.steps.len(), 2);
        assert_eq!(
            executor.calls.len(),
            1,
            "dependent side effect was dispatched"
        );
        assert_eq!(report.steps[0].status, "executed");
        assert_eq!(report.steps[1].status, "failed");
        let failure = report.execution_summary.failure.unwrap();
        assert_eq!(failure.error_code, "kyuubiki.headless.binding_resolution");
        assert_eq!(failure.stage, "payload_resolution");
        assert_eq!(failure.step_index, 2);
        assert!(!failure.retryable);
        assert_eq!(failure.retry_strategy, "none");
        assert!(failure.message.contains("step 1"));
        assert!(failure.message.contains("service"));
    }
}

#[test]
fn resolved_null_or_empty_required_value_halts_before_dispatch() {
    let batch = batch(
        "project_create",
        json!({"name":"source-project"}),
        json!({"name":"{{steps.1.result.name}}"}),
    );
    for name in [json!(null), json!(""), json!("   ")] {
        let mut executor = RecordingExecutor {
            first_result: json!({"project_id":"existing-project", "name":name}),
            ..Default::default()
        };
        let report = execute_batch_with_executor(&batch, &mut executor, false, false);
        assert_eq!(report.status, "failed");
        assert_eq!(report.executed_step_count, 1);
        assert_eq!(executor.calls.len(), 1);
        assert_eq!(report.steps.len(), 2);
        assert!(
            report
                .execution_summary
                .failure
                .unwrap()
                .message
                .contains("name")
        );
    }
}

#[test]
fn missing_nested_binding_halts_without_exposing_unrelated_source_values() {
    let batch = batch(
        "service_health",
        json!({}),
        json!({"name":"dependent", "metadata":[{"forwarded":"{{steps.1.result.service}}"}]}),
    );
    let mut executor = RecordingExecutor {
        first_result: json!({"status":"ok", "private_debug":"secret-source-value"}),
        ..Default::default()
    };
    let report = execute_batch_with_executor(&batch, &mut executor, false, false);
    assert_eq!(report.status, "failed");
    assert_eq!(executor.calls.len(), 1);
    assert!(
        !report
            .execution_summary
            .failure
            .unwrap()
            .message
            .contains("secret-source-value")
    );
}

#[test]
fn valid_bindings_preserve_full_values_and_optional_null_without_recursive_expansion() {
    let batch = batch(
        "service_health",
        json!({}),
        json!({
            "name":"dependent", "description":"{{steps.1.result.service}}",
            "metadata":{"forwarded":["{{steps.1.result.solver_endpoints}}"]},
            "literal":"prefix {{steps.1.result.status}} suffix"
        }),
    );
    let endpoints = json!({"nodes":(0..256).collect::<Vec<_>>(),
        "literal":"{{steps.1.result.status}}"});
    let mut executor = RecordingExecutor {
        first_result: json!({"service":null,"status":"ok","solver_endpoints":endpoints}),
        ..Default::default()
    };
    let report = execute_batch_with_executor(&batch, &mut executor, false, false);
    assert_eq!(report.status, "ok");
    assert_eq!(executor.calls.len(), 3);
    assert_eq!(executor.calls[1]["description"], Value::Null);
    assert_eq!(executor.calls[1]["metadata"]["forwarded"][0], endpoints);
    assert_eq!(
        executor.calls[1]["literal"],
        batch.steps[1].payload["literal"]
    );
    assert_eq!(
        report.steps[0].result_preview["solver_endpoints"]["nodes"]["item_count"],
        256
    );
}

#[test]
fn confirmation_gate_precedes_binding_resolution_and_remains_explicit() {
    let mut batch = batch("service_health", json!({}), json!({"name":"dependent"}));
    batch.steps[1].action = "project_delete".into();
    batch.steps[1].risk = HeadlessRisk::Destructive;
    batch.steps[1].payload = json!({"project_id":"{{steps.1.result.service}}"});
    let mut executor = RecordingExecutor {
        first_result: json!({"status":"ok"}),
        ..Default::default()
    };
    let report = execute_batch_with_executor(&batch, &mut executor, false, false);
    assert_eq!(report.status, "blocked");
    assert_eq!(report.blocked_by_confirmation.unwrap().index, 2);
    assert!(report.execution_summary.failure.is_none());
    assert_eq!(executor.calls.len(), 1);
    let approved = execute_batch_with_executor(&batch, &mut executor, false, true);
    assert_eq!(approved.status, "failed");
    assert_eq!(
        executor.calls.len(),
        2,
        "approval cannot fabricate a missing output"
    );
}

#[test]
fn dry_run_missing_preview_output_cannot_claim_a_complete_chain() {
    let batch = batch(
        "service_health",
        json!({}),
        json!({"name":"{{steps.1.result.service}}"}),
    );
    let report = run_batch_dry(&batch, false, false);
    assert_eq!(report.status, "failed");
    assert_eq!(report.executed_step_count, 1);
    assert_eq!(report.steps.len(), 2);
    assert_eq!(
        report.execution_summary.failure.unwrap().error_code,
        "kyuubiki.headless.binding_resolution"
    );
}

#[test]
fn dry_run_confirmation_stops_before_dependent_binding_or_later_previews() {
    let mut batch = batch(
        "workflow_submit_catalog",
        json!({"workflow_id":"explicit-workflow"}),
        json!({"name":"{{steps.1.result.job_id}}"}),
    );
    batch.steps[0].risk = HeadlessRisk::Sensitive;
    let report = run_batch_dry(&batch, false, false);
    assert_eq!(report.status, "blocked");
    assert_eq!(report.executed_step_count, 0);
    assert_eq!(report.steps.len(), 1);
    assert_eq!(report.blocked_by_confirmation.unwrap().index, 1);
    let approved = run_batch_dry(&batch, true, false);
    assert_eq!(approved.status, "ok");
    assert_eq!(approved.steps.len(), 3);
}
