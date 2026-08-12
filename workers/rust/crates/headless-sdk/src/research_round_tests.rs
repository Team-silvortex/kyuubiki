use super::*;
use crate::{
    HEADLESS_PARAMETER_PATCH_SCHEMA_VERSION, HeadlessExecutionBatchStep, HeadlessParameterChange,
    HeadlessParameterPatch, HeadlessRisk, apply_parameter_patch, run_batch_dry,
};
use serde_json::json;

fn batch() -> HeadlessExecutionBatch {
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

fn report(batch: &HeadlessExecutionBatch, value: Value) -> HeadlessRunReport {
    let mut report = run_batch_dry(batch, false, false);
    report.mode = "execute:service".to_string();
    report.steps[0].status = "executed".to_string();
    report.steps[0].result_preview = json!({"result": {"max_temperature_c": value}});
    report
}

fn spec(round_id: &str, iteration: u64) -> HeadlessResearchRoundSpec {
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

fn patch(batch: &mut HeadlessExecutionBatch) -> HeadlessParameterPatchReceipt {
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
