use super::*;
use crate::{
    HEADLESS_PARAMETER_PATCH_SCHEMA_VERSION, HeadlessExecutionBatchStep, HeadlessParameterChange,
    HeadlessParameterPatch, HeadlessRisk, apply_parameter_patch, run_batch_dry,
};
use serde_json::json;

pub(super) fn batch() -> HeadlessExecutionBatch {
    HeadlessExecutionBatch {
        schema_version: "kyuubiki.headless-execution-batch/v1".to_string(),
        exported_at: "1970-01-01T00:00:00.000Z".to_string(),
        language: "en".to_string(),
        workflow_id: "research.thermal-rounds".to_string(),
        template_id: None,
        steps: vec![HeadlessExecutionBatchStep {
            index: 1,
            action: "service_health".to_string(),
            risk: HeadlessRisk::Normal,
            payload: json!({"research_input": 10.0}),
        }],
        warnings: vec![],
    }
}

pub(super) fn report(batch: &HeadlessExecutionBatch, value: Value) -> HeadlessRunReport {
    let mut report = run_batch_dry(batch, false, false);
    report.mode = "execute:service".to_string();
    report.steps[0].status = "executed".to_string();
    report.steps[0].result_preview = json!({"result": {"max_temperature_c": value}});
    report
}

pub(super) fn spec(round_id: &str, iteration: u64) -> HeadlessResearchRoundSpec {
    HeadlessResearchRoundSpec {
        schema_version: HEADLESS_RESEARCH_ROUND_SPEC_SCHEMA_VERSION.to_string(),
        round_id: round_id.to_string(),
        workflow_id: "research.thermal-rounds".to_string(),
        iteration,
        primary_metric_ids: vec!["max_temperature_c".to_string()],
        metrics: vec![HeadlessResearchMetricSpec {
            metric_id: "max_temperature_c".to_string(),
            pointer: "/steps/0/result_preview/result/max_temperature_c".to_string(),
            unit: "degC".to_string(),
            objective: HeadlessResearchMetricObjective::Minimize,
        }],
    }
}

pub(super) fn patch(batch: &mut HeadlessExecutionBatch) -> HeadlessParameterPatchReceipt {
    apply_parameter_patch(
        batch,
        &HeadlessParameterPatch {
            schema_version: HEADLESS_PARAMETER_PATCH_SCHEMA_VERSION.to_string(),
            patch_id: "thermal-input-round-2".to_string(),
            workflow_id: batch.workflow_id.clone(),
            template_id: None,
            changes: vec![HeadlessParameterChange {
                path: "/steps/0/payload/research_input".to_string(),
                expected: json!(10.0),
                value: json!(12.0),
            }],
        },
    )
    .expect("patch")
}

fn saved_source_round() -> (HeadlessExecutionBatch, HeadlessRunReport) {
    let source = crate::headless_saved_model_source(
        &json!({
            "version_id":"v", "project_id":"p", "model_id":"m", "kind":"heat_bar_1d",
            "payload":{"nodes":[],"elements":[]}
        }),
        crate::HeadlessModelSourceKind::ModelVersion,
    )
    .unwrap();
    let source = serde_json::to_value(source).unwrap();
    let mut batch = batch();
    batch.steps[0].action = "solve_and_wait_from_model_version".into();
    batch.steps[0].payload = json!({"model_version_id":"v","expected_model_source":source,
        "endpoints":["127.0.0.1:7001"]});
    let mut report = report(&batch, json!(48.0));
    report.steps[0].result_preview["model_source"] = source.clone();
    report.steps[0].result_preview["solve"] = json!({"model_source":source});
    (batch, report)
}

#[test]
fn saved_research_evidence_requires_matching_source_identity_pin_and_combined_receipt() {
    let (batch, report) = saved_source_round();
    build_headless_research_round_evidence(&batch, &report, &spec("saved-source", 1), None, None)
        .unwrap();
    for pointer in [
        "/steps/0/result_preview/model_source/project_id",
        "/steps/0/result_preview/model_source/model_version_id",
        "/steps/0/result_preview/model_source/sha256",
        "/steps/0/result_preview/solve/model_source/sha256",
    ] {
        let mut invalid = report.clone();
        let mut encoded = serde_json::to_value(&invalid).unwrap();
        *encoded.pointer_mut(pointer).unwrap() = if pointer.ends_with("sha256") {
            json!("0".repeat(64))
        } else {
            json!("other")
        };
        invalid = serde_json::from_value(encoded).unwrap();
        assert!(
            build_headless_research_round_evidence(
                &batch,
                &invalid,
                &spec("saved-source", 1),
                None,
                None
            )
            .unwrap_err()
            .contains("saved model source"),
            "{pointer}"
        );
    }
    for missing in ["model_source", "solve"] {
        let mut invalid = report.clone();
        invalid.steps[0]
            .result_preview
            .as_object_mut()
            .unwrap()
            .remove(missing);
        assert!(
            build_headless_research_round_evidence(
                &batch,
                &invalid,
                &spec("saved-source", 1),
                None,
                None
            )
            .unwrap_err()
            .contains("saved model source")
        );
    }
}

#[test]
fn saved_research_source_bindings_use_prior_outputs_not_untrusted_resolved_payload_labels() {
    let (mut batch, mut report) = saved_source_round();
    let source = report.steps[0].result_preview["model_source"].clone();
    batch.steps[0].action = "solve_from_model_version".into();
    let mut second = batch.steps[0].clone();
    second.index = 2;
    second.payload = json!({"model_version_id":"{{steps.1.result.model_version_id}}","endpoints":["127.0.0.1:7001"],
        "expected_model_source":"{{steps.1.result.model_source}}"});
    batch.steps.push(second);
    report = report_for_saved_bindings(&batch, source);
    build_headless_research_round_evidence(&batch, &report, &spec("saved-source", 1), None, None)
        .unwrap();
    // An unchanged report payload label cannot conceal a changed actual source output.
    report.steps[1].payload = json!({"model_version_id":"v","expected_model_source":report.steps[0].result_preview["model_source"]});
    report.steps[0].result_preview["model_version_id"] = json!("other");
    assert!(
        build_headless_research_round_evidence(
            &batch,
            &report,
            &spec("saved-source", 1),
            None,
            None
        )
        .unwrap_err()
        .contains("saved model source")
    );
}

fn report_for_saved_bindings(batch: &HeadlessExecutionBatch, source: Value) -> HeadlessRunReport {
    struct SourceExecutor(Value);
    impl crate::HeadlessExecutor for SourceExecutor {
        fn name(&self) -> &'static str {
            "service"
        }
        fn execute_step(
            &mut self,
            _: &str,
            _: usize,
            _: &Value,
        ) -> Result<crate::HeadlessExecutorOutcome, crate::HeadlessExecutorError> {
            Ok(crate::HeadlessExecutorOutcome {
                status: "executed".into(),
                result: json!({"model_version_id":"v","model_source":self.0,
                    "result":{"max_temperature_c":48.0}}),
            })
        }
    }
    crate::execute_batch_with_executor(batch, &mut SourceExecutor(source), false, false)
}

#[test]
fn qualifies_contiguous_rounds_with_changed_input_and_numeric_metrics() {
    let first_batch = batch();
    let first = build_headless_research_round_evidence(
        &first_batch,
        &report(&first_batch, json!(48.0)),
        &spec("thermal-round-1", 1),
        None,
        None,
    )
    .expect("first round");

    let mut second_batch = first_batch.clone();
    let receipt = patch(&mut second_batch);
    let second = build_headless_research_round_evidence(
        &second_batch,
        &report(&second_batch, json!(44.0)),
        &spec("thermal-round-2", 2),
        Some(&receipt),
        Some(&first),
    )
    .expect("second round");

    assert!(second.qualified);
    assert_eq!(second.metrics[0].value, 44.0);
    assert_eq!(
        second
            .previous_round
            .as_ref()
            .map(|link| link.round_id.as_str()),
        Some("thermal-round-1")
    );
    assert_eq!(receipt.after_sha256, second.batch_content_sha256);

    let mut tampered_previous = first.clone();
    tampered_previous.run_mode = "dry_run".to_string();
    assert!(
        build_headless_research_round_evidence(
            &second_batch,
            &report(&second_batch, json!(44.0)),
            &spec("thermal-round-2", 2),
            Some(&receipt),
            Some(&tampered_previous),
        )
        .expect_err("tampered previous evidence")
        .contains("previous evidence is not qualified")
    );
}

#[test]
fn rejects_a_stale_report_for_changed_inputs_with_identical_validation() {
    let original = batch();
    let old_report = report(&original, json!(48.0));
    let mut changed = original.clone();
    changed.steps[0].payload["research_input"] = json!(12.0);
    assert_eq!(validate_batch(&changed), old_report.validation);

    assert!(
        build_headless_research_round_evidence(
            &changed,
            &old_report,
            &spec("thermal-round-1", 1),
            None,
            None,
        )
        .expect_err("old results must not qualify changed inputs")
        .contains("execution input fingerprint")
    );
}

#[test]
fn old_or_invalid_input_receipts_are_readable_but_cannot_qualify_research() {
    let source = batch();
    let correct = report(&source, json!(48.0));
    let encoded = serde_json::to_value(&correct).unwrap();
    for receipt in [
        None,
        Some(json!(null)),
        Some(
            json!({"schema_version":"kyuubiki.headless-execution-input/v2","sha256":"0".repeat(64)}),
        ),
        Some(json!({"schema_version":"kyuubiki.headless-execution-input/v1","sha256":"invalid"})),
    ] {
        let mut value = encoded.clone();
        value.as_object_mut().unwrap().remove("execution_input");
        if let Some(receipt) = receipt {
            value["execution_input"] = receipt;
        }
        let reloaded: HeadlessRunReport = serde_json::from_value(value).unwrap();
        assert!(
            build_headless_research_round_evidence(
                &source,
                &reloaded,
                &spec("thermal-round-1", 1),
                None,
                None,
            )
            .unwrap_err()
            .contains("execution input fingerprint")
        );
    }
    let reloaded = serde_json::from_value(encoded).unwrap();
    let evidence = build_headless_research_round_evidence(
        &source,
        &reloaded,
        &spec("thermal-round-1", 1),
        None,
        None,
    )
    .unwrap();
    verify_headless_research_round_evidence(&source, &reloaded, &evidence, None).unwrap();
}

#[test]
fn exact_input_gate_rejects_changes_hidden_by_legacy_digest_rounding() {
    let mut source = batch();
    source.steps[0].payload["research_input"] = json!(1e-30);
    let old_report = report(&source, json!(48.0));
    let evidence = build_headless_research_round_evidence(
        &source,
        &old_report,
        &spec("thermal-round-1", 1),
        None,
        None,
    )
    .unwrap();
    let mut changed = source.clone();
    changed.steps[0].payload["research_input"] = json!(2e-30);
    assert_eq!(
        headless_batch_content_sha256(&source).unwrap(),
        headless_batch_content_sha256(&changed).unwrap()
    );
    assert!(
        verify_headless_research_round_evidence(&changed, &old_report, &evidence, None)
            .unwrap_err()
            .contains("execution input fingerprint")
    );
}

#[test]
fn rejects_success_status_with_a_retained_execution_failure() {
    let source = batch();
    let mut failed = report(&source, json!(48.0));
    failed.execution_summary.failure = Some(crate::HeadlessFailureReceipt {
        schema_version: crate::HEADLESS_FAILURE_RECEIPT_SCHEMA_VERSION.into(),
        error_code: "kyuubiki.headless.execution_failure".into(),
        category: "execution_failure".into(),
        stage: "execution".into(),
        step_index: 1,
        action: "service_health".into(),
        message: "retained failure".into(),
        retryable: false,
        retry_strategy: "none".into(),
        recommended_action: "inspect".into(),
    });
    assert!(
        build_headless_research_round_evidence(
            &source,
            &failed,
            &spec("thermal-round-1", 1),
            None,
            None,
        )
        .unwrap_err()
        .contains("successful validated run")
    );
}

#[test]
fn rejects_repeat_rounds_dry_runs_and_non_numeric_metrics() {
    let first_batch = batch();
    let first = build_headless_research_round_evidence(
        &first_batch,
        &report(&first_batch, json!(48.0)),
        &spec("thermal-round-1", 1),
        None,
        None,
    )
    .expect("first round");
    let missing_patch = build_headless_research_round_evidence(
        &first_batch,
        &report(&first_batch, json!(48.0)),
        &spec("thermal-round-2", 2),
        None,
        Some(&first),
    )
    .expect_err("repeat round");
    assert!(missing_patch.contains("requires a parameter patch receipt"));

    let mut patched_first = first_batch.clone();
    let receipt = patch(&mut patched_first);
    assert!(
        build_headless_research_round_evidence(
            &patched_first,
            &report(&patched_first, json!(47.0)),
            &spec("thermal-round-1", 1),
            Some(&receipt),
            None,
        )
        .expect_err("first round patch")
        .contains("effective baseline")
    );

    let mut wrong_report = report(&first_batch, json!(48.0));
    wrong_report.schema_version = "kyuubiki.headless-execution-run/v2".to_string();
    assert!(
        build_headless_research_round_evidence(
            &first_batch,
            &wrong_report,
            &spec("thermal-round-1", 1),
            None,
            None,
        )
        .expect_err("wrong report schema")
        .contains(HEADLESS_EXECUTION_RUN_SCHEMA_VERSION)
    );

    let dry = run_batch_dry(&first_batch, false, false);
    assert!(
        build_headless_research_round_evidence(
            &first_batch,
            &dry,
            &spec("thermal-round-1", 1),
            None,
            None,
        )
        .expect_err("dry run")
        .contains("execute:service")
    );
    assert!(
        build_headless_research_round_evidence(
            &first_batch,
            &report(&first_batch, json!("n/a")),
            &spec("thermal-round-1", 1),
            None,
            None,
        )
        .expect_err("non numeric")
        .contains("missing or non-numeric")
    );

    let mut progress_spec = spec("thermal-round-1", 1);
    progress_spec.metrics[0].pointer = "/steps/0/result_preview/progress".to_string();
    assert!(
        validate_headless_research_round_spec(&progress_spec)
            .expect_err("progress is not a domain metric")
            .contains("/result/")
    );
}

#[test]
fn schemas_and_example_share_the_runtime_contract() {
    let spec_schema: Value = serde_json::from_str(include_str!(
        "../../../../../schemas/headless-research-round-spec.schema.json"
    ))
    .expect("spec schema");
    let evidence_schema: Value = serde_json::from_str(include_str!(
        "../../../../../schemas/headless-research-round-evidence.schema.json"
    ))
    .expect("evidence schema");
    let example: HeadlessResearchRoundSpec = serde_json::from_str(include_str!(
        "../../../../../schemas/examples.headless-research-round-spec.json"
    ))
    .expect("spec example");

    assert_eq!(
        spec_schema["properties"]["schema_version"]["const"],
        HEADLESS_RESEARCH_ROUND_SPEC_SCHEMA_VERSION
    );
    assert_eq!(
        evidence_schema["properties"]["schema_version"]["const"],
        HEADLESS_RESEARCH_ROUND_EVIDENCE_SCHEMA_VERSION
    );
    validate_headless_research_round_spec(&example).expect("example validates");
}
