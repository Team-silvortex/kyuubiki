use crate::{
    HeadlessExecutionBatch, HeadlessExecutionBatchStep, HeadlessExecutor, HeadlessExecutorError,
    HeadlessExecutorOutcome, HeadlessRisk, HeadlessRunReport, all_action_contracts,
    build_execution_plan, execute_batch_with_executor, run_batch_dry, validate_batch,
};
use serde_json::{Value, json};

const RISKS: [HeadlessRisk; 3] = [
    HeadlessRisk::Normal,
    HeadlessRisk::Sensitive,
    HeadlessRisk::Destructive,
];

#[derive(Default)]
struct RecordingExecutor(Vec<String>);

impl HeadlessExecutor for RecordingExecutor {
    fn name(&self) -> &'static str {
        "mock"
    }

    fn execute_step(
        &mut self,
        action: &str,
        _index: usize,
        _payload: &Value,
    ) -> Result<HeadlessExecutorOutcome, HeadlessExecutorError> {
        self.0.push(action.into());
        Ok(HeadlessExecutorOutcome {
            status: "executed".into(),
            result: json!({"project_id":"owned-project","job_id":"owned-job","path":"preview"}),
        })
    }
}

fn batch(action: &str, risk: HeadlessRisk, payload: Value) -> HeadlessExecutionBatch {
    HeadlessExecutionBatch {
        schema_version: "kyuubiki.headless-execution-batch/v1".into(),
        exported_at: "2026-10-07T00:00:00Z".into(),
        language: "en".into(),
        workflow_id: "contract-owned-risk".into(),
        template_id: None,
        steps: vec![HeadlessExecutionBatchStep {
            index: 1,
            action: action.into(),
            risk,
            payload,
        }],
        warnings: vec![],
    }
}

fn assert_invalid(report: &HeadlessRunReport) {
    assert_eq!(report.status, "invalid", "{report:?}");
    assert_eq!(report.executed_step_count, 0);
    assert!(report.steps.is_empty());
    assert!(report.blocked_by_confirmation.is_none());
    let failure = report.execution_summary.failure.as_ref().unwrap();
    assert_eq!(failure.error_code, "kyuubiki.headless.document_validation");
    assert_eq!(failure.stage, "batch_validation");
    assert!(!failure.retryable);
    assert_eq!(failure.retry_strategy, "none");
}

#[test]
fn every_action_contract_rejects_both_alternative_risk_labels() {
    for contract in all_action_contracts() {
        let payload = contract
            .required_payload_keys
            .iter()
            .map(|key| ((*key).to_owned(), json!("fixture")))
            .collect::<serde_json::Map<_, _>>();
        for risk in RISKS {
            let document = batch(contract.id, risk, Value::Object(payload.clone()));
            let validation = validate_batch(&document);
            // Payload-specific validation is independent of this metadata invariant.
            let mismatch = validation
                .issues
                .iter()
                .any(|issue| issue.contains("risk must match action contract"));
            assert_eq!(
                mismatch,
                risk != contract.risk,
                "{}: {validation:?}",
                contract.id
            );
        }
    }
}

#[test]
fn imported_risk_mismatch_stops_entire_batch_before_earlier_side_effects() {
    for (action, expected, payload) in [
        (
            "project_create",
            HeadlessRisk::Normal,
            json!({"name":"target"}),
        ),
        (
            "project_delete",
            HeadlessRisk::Destructive,
            json!({"project_id":"owned"}),
        ),
        (
            "workflow_submit_catalog",
            HeadlessRisk::Sensitive,
            json!({"workflow_id":"owned"}),
        ),
        ("snapshot", HeadlessRisk::Sensitive, json!({})),
    ] {
        let mut correct = batch(action, expected, payload);
        correct.steps[0].index = 2;
        correct.steps.insert(
            0,
            HeadlessExecutionBatchStep {
                index: 1,
                action: "project_create".into(),
                risk: HeadlessRisk::Normal,
                payload: json!({"name":"earlier-write-must-not-run"}),
            },
        );
        assert!(validate_batch(&correct).ok);
        for risk in RISKS.into_iter().filter(|risk| *risk != expected) {
            let mut imported = serde_json::to_value(&correct).unwrap();
            imported["steps"][1]["risk"] = serde_json::to_value(risk).unwrap();
            let imported: HeadlessExecutionBatch = serde_json::from_value(imported).unwrap();
            for (sensitive, destructive) in
                [(false, false), (true, false), (false, true), (true, true)]
            {
                let mut executor = RecordingExecutor::default();
                assert_invalid(&execute_batch_with_executor(
                    &imported,
                    &mut executor,
                    sensitive,
                    destructive,
                ));
                assert!(
                    executor.0.is_empty(),
                    "earlier write executed before validation"
                );
                assert_invalid(&run_batch_dry(&imported, sensitive, destructive));
            }
        }
    }
}

#[test]
fn invalid_plan_still_exposes_contract_owned_confirmation_and_policy() {
    for (action, expected, flag, payload) in [
        (
            "project_delete",
            HeadlessRisk::Destructive,
            "--allow-destructive",
            json!({"project_id":"owned"}),
        ),
        (
            "workflow_submit_catalog",
            HeadlessRisk::Sensitive,
            "--allow-sensitive",
            json!({"workflow_id":"owned"}),
        ),
        (
            "snapshot",
            HeadlessRisk::Sensitive,
            "--allow-sensitive",
            json!({}),
        ),
    ] {
        for risk in RISKS.into_iter().filter(|risk| *risk != expected) {
            let document = batch(action, risk, payload.clone());
            let plan = build_execution_plan(&document);
            assert!(!plan.ok, "mismatched risk produced an executable plan");
            assert_eq!(plan.confirmation_count, 1);
            assert_eq!(plan.steps[0].risk, expected);
            assert!(plan.steps[0].requires_confirmation);
            assert_eq!(plan.steps[0].confirmation_flag.as_deref(), Some(flag));
            assert_eq!(plan.confirmations[0].risk, expected);
            assert_eq!(plan.confirmations[0].flag, flag);
            let key = match expected {
                HeadlessRisk::Sensitive => "sensitive",
                HeadlessRisk::Destructive => "destructive",
                HeadlessRisk::Normal => unreachable!(),
            };
            assert_eq!(plan.policy.unwrap().risk_counts[key], 1);
        }
    }
    let plan = build_execution_plan(&batch(
        "project_create",
        HeadlessRisk::Destructive,
        json!({"name":"owned"}),
    ));
    assert!(!plan.ok);
    assert_eq!(plan.steps[0].risk, HeadlessRisk::Normal);
    assert_eq!(plan.confirmation_count, 0);
}

#[test]
fn matching_sensitive_and_destructive_risks_require_their_own_explicit_flags() {
    for (action, risk, payload) in [
        (
            "project_delete",
            HeadlessRisk::Destructive,
            json!({"project_id":"owned"}),
        ),
        (
            "workflow_submit_catalog",
            HeadlessRisk::Sensitive,
            json!({"workflow_id":"owned"}),
        ),
        ("snapshot", HeadlessRisk::Sensitive, json!({})),
    ] {
        let document = batch(action, risk, payload);
        assert!(validate_batch(&document).ok);
        for (sensitive, destructive) in [(false, false), (true, false), (false, true), (true, true)]
        {
            let allowed = match risk {
                HeadlessRisk::Sensitive => sensitive,
                HeadlessRisk::Destructive => destructive,
                HeadlessRisk::Normal => unreachable!(),
            };
            let mut executor = RecordingExecutor::default();
            let report =
                execute_batch_with_executor(&document, &mut executor, sensitive, destructive);
            let preview = run_batch_dry(&document, sensitive, destructive);
            for result in [&report, &preview] {
                assert_eq!(result.status, if allowed { "ok" } else { "blocked" });
                assert_eq!(result.executed_step_count, usize::from(allowed));
                assert_eq!(result.steps[0].risk, risk);
                assert_eq!(
                    result
                        .blocked_by_confirmation
                        .as_ref()
                        .map(|gate| gate.risk),
                    (!allowed).then_some(risk)
                );
            }
            assert_eq!(executor.0.len(), usize::from(allowed));
        }
    }
}
