use crate::{
    HeadlessExecutionBatch, HeadlessExecutionBatchStep, HeadlessExecutor, HeadlessExecutorError,
    HeadlessExecutorOutcome, HeadlessRisk, MockHeadlessExecutor, execute_batch_with_executor,
};
use serde_json::{Value, json};

struct OutcomeExecutor {
    status: String,
    calls: usize,
}

impl HeadlessExecutor for OutcomeExecutor {
    fn name(&self) -> &'static str {
        "service"
    }

    fn execute_step(
        &mut self,
        _action: &str,
        _index: usize,
        _payload: &Value,
    ) -> Result<HeadlessExecutorOutcome, HeadlessExecutorError> {
        self.calls += 1;
        Ok(HeadlessExecutorOutcome {
            status: self.status.clone(),
            result: json!({"execution_readiness": {
                "status": self.status, "required_action": "repair_runtime"
            }}),
        })
    }
}

fn batch(action: &str, payload: Value) -> HeadlessExecutionBatch {
    HeadlessExecutionBatch {
        schema_version: "kyuubiki.headless-execution-batch/v1".into(),
        exported_at: "1970-01-01T00:00:00.000Z".into(),
        language: "en".into(),
        workflow_id: "execution-gate-regression".into(),
        template_id: None,
        steps: vec![
            HeadlessExecutionBatchStep {
                index: 1,
                action: action.into(),
                risk: HeadlessRisk::Normal,
                payload,
            },
            HeadlessExecutionBatchStep {
                index: 2,
                action: "project_create".into(),
                risk: HeadlessRisk::Normal,
                payload: json!({"name":"must-not-run-after-noncompletion"}),
            },
        ],
        warnings: vec![],
    }
}

#[test]
fn noncompleted_executor_outcomes_halt_without_counting_success_or_later_side_effects() {
    let batch = batch("service_health", json!({}));
    for status in [
        "blocked",
        "failed",
        "cancelled",
        "pending",
        "dry_run",
        "",
        "EXECUTED",
    ] {
        let mut executor = OutcomeExecutor {
            status: status.into(),
            calls: 0,
        };
        let report = execute_batch_with_executor(&batch, &mut executor, false, false);
        assert_eq!(
            report.status,
            if status == "blocked" {
                "blocked"
            } else {
                "failed"
            }
        );
        assert_eq!(report.executed_step_count, 0, "{status}");
        assert_eq!(report.steps.len(), 1, "{status}");
        assert_eq!(executor.calls, 1, "{status}");
        assert!(report.blocked_by_confirmation.is_none());
        if status == "blocked" {
            assert_eq!(report.steps[0].status, "blocked");
            assert_eq!(
                report.steps[0].result_preview["execution_readiness"]["required_action"],
                "repair_runtime"
            );
        }
        executor.status = "executed".into();
        let replay = execute_batch_with_executor(&batch, &mut executor, false, false);
        assert_eq!(replay.status, "ok");
        assert_eq!(replay.executed_step_count, 2);
        assert_eq!(executor.calls, 3);
    }
}

#[test]
fn mock_task_execution_is_a_blocked_preview_not_completed_computation() {
    let task = golden_task();
    let batch = batch("operator_task_execute", json!({"task": task}));
    let report = execute_batch_with_executor(&batch, &mut MockHeadlessExecutor, false, false);
    assert_eq!(report.status, "blocked");
    assert_eq!(report.executed_step_count, 0);
    assert_eq!(report.steps.len(), 1);
    assert_eq!(report.steps[0].status, "blocked");
    assert_eq!(
        report.steps[0].result_preview["status"],
        "verified_pending_execution"
    );
}

#[test]
fn invalid_task_execution_never_reaches_a_custom_executor() {
    let mut task = golden_task();
    task["input_artifact"]["x"] = json!(2);
    let mut executor = OutcomeExecutor {
        status: "executed".into(),
        calls: 0,
    };
    let report = execute_batch_with_executor(
        &batch("operator_task_execute", json!({"task": task})),
        &mut executor,
        false,
        false,
    );
    assert_eq!(report.status, "failed");
    assert_eq!(report.executed_step_count, 0);
    assert_eq!(executor.calls, 0);
    assert_eq!(
        report.steps[0].result_preview["error_code"],
        "operator_task_digest_mismatch"
    );
}

pub(crate) fn golden_task() -> Value {
    let batch: Value = serde_json::from_str(include_str!(
        "../../../../../schemas/examples.operator-task-batch.json"
    ))
    .unwrap();
    batch["tasks"][0]["task_ir"].clone()
}
