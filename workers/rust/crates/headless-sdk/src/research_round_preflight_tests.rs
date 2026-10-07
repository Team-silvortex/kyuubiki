use super::tests::{batch, patch, report, spec};
use super::*;
use serde_json::json;

fn first_evidence(batch: &HeadlessExecutionBatch) -> HeadlessResearchRoundEvidence {
    build_headless_research_round_evidence(
        batch,
        &report(batch, json!(48.0)),
        &spec("round-1", 1),
        None,
        None,
    )
    .unwrap()
}

#[test]
fn static_plan_validation_accepts_contiguous_lineage_without_mutating_inputs() {
    let first_batch = batch();
    validate_headless_research_round_plan(&first_batch, &spec("round-1", 1), None, None).unwrap();
    let previous = first_evidence(&first_batch);
    let mut second_batch = first_batch.clone();
    let receipt = patch(&mut second_batch);
    let current = spec("round-2", 2);
    let snapshots = serde_json::to_value((&second_batch, &current, &receipt, &previous)).unwrap();
    validate_headless_research_round_plan(&second_batch, &current, Some(&receipt), Some(&previous))
        .unwrap();
    assert_eq!(
        serde_json::to_value((&second_batch, &current, &receipt, &previous)).unwrap(),
        snapshots
    );
}

#[test]
fn static_metric_step_references_use_zero_based_positions_not_one_based_action_indices() {
    let first = batch();
    assert_eq!(first.steps[0].index, 1);
    validate_headless_research_round_plan(&first, &spec("round-1", 1), None, None).unwrap();
    let mut current = spec("round-1", 1);
    for index in [1, 1_000_000, usize::MAX] {
        current.metrics[0].pointer = format!("/steps/{index}/result_preview/result/temperature");
        let error =
            validate_headless_research_round_plan(&first, &current, None, None).unwrap_err();
        assert!(
            error.contains(&format!("missing batch step {index}")),
            "{error}"
        );
        assert_eq!(
            build_headless_research_round_evidence(
                &first,
                &report(&first, json!(48.0)),
                &current,
                None,
                None
            )
            .unwrap_err(),
            error
        );
    }
    let mut two_steps = first.clone();
    two_steps.steps.push(first.steps[0].clone());
    two_steps.steps[1].index = 2;
    current.metrics[0].pointer = "/steps/1/result_preview/result/temperature".into();
    validate_headless_research_round_plan(&two_steps, &current, None, None).unwrap();
}

#[test]
fn static_plan_cannot_predict_missing_or_non_numeric_runtime_results() {
    let batch = batch();
    let mut current = spec("round-1", 1);
    current.metrics[0].pointer = "/steps/0/result_preview/result/unknown_field".into();
    validate_headless_research_round_plan(&batch, &current, None, None).unwrap();
    assert!(
        build_headless_research_round_evidence(
            &batch,
            &report(&batch, json!(48.0)),
            &current,
            None,
            None
        )
        .unwrap_err()
        .contains("missing or non-numeric")
    );
    for value in [Value::Null, json!("48"), json!(true), json!([]), json!({})] {
        let current = spec("round-1", 1);
        validate_headless_research_round_plan(&batch, &current, None, None).unwrap();
        assert!(
            build_headless_research_round_evidence(
                &batch,
                &report(&batch, value),
                &current,
                None,
                None
            )
            .unwrap_err()
            .contains("missing or non-numeric")
        );
    }
}

#[test]
fn static_plan_rejects_invalid_batch_and_workflow_before_execution() {
    let mut invalid = batch();
    invalid.schema_version = "wrong-batch/v1".into();
    assert!(
        validate_headless_research_round_plan(&invalid, &spec("round-1", 1), None, None)
            .unwrap_err()
            .contains("batch is invalid")
    );
    let mut wrong_spec = spec("round-1", 1);
    wrong_spec.workflow_id = "different.workflow".into();
    assert!(
        validate_headless_research_round_plan(&batch(), &wrong_spec, None, None)
            .unwrap_err()
            .contains("workflow mismatch")
    );
}

#[test]
fn static_lineage_rejects_baseline_patch_and_missing_previous_or_receipt() {
    let first = batch();
    let previous = first_evidence(&first);
    let mut changed = first.clone();
    let receipt = patch(&mut changed);
    for (current, receipt, previous, expected) in [
        (
            spec("round-1", 1),
            Some(&receipt),
            None,
            "effective baseline",
        ),
        (
            spec("round-1", 1),
            None,
            Some(&previous),
            "cannot declare a previous round",
        ),
        (
            spec("round-2", 2),
            Some(&receipt),
            None,
            "requires previous-round evidence",
        ),
        (
            spec("round-2", 2),
            None,
            Some(&previous),
            "requires a parameter patch receipt",
        ),
        (
            spec("round-3", 3),
            Some(&receipt),
            Some(&previous),
            "iteration is not contiguous",
        ),
        (
            spec("round-1", 2),
            Some(&receipt),
            Some(&previous),
            "round_id must change",
        ),
    ] {
        assert!(
            validate_headless_research_round_plan(&changed, &current, receipt, previous)
                .unwrap_err()
                .contains(expected)
        );
    }
}

#[test]
fn static_lineage_reuses_previous_evidence_and_patch_target_validation() {
    let first = batch();
    let previous = first_evidence(&first);
    let mut changed = first.clone();
    let receipt = patch(&mut changed);
    let current = spec("round-2", 2);
    for (field, value, expected) in [
        (
            "qualified",
            json!(false),
            "previous evidence is not qualified",
        ),
        (
            "run_mode",
            json!("dry_run"),
            "previous evidence is not qualified",
        ),
        (
            "workflow_id",
            json!("different.workflow"),
            "cross workflow boundaries",
        ),
        (
            "batch_content_sha256",
            json!("a".repeat(64)),
            "does not start from the previous batch",
        ),
    ] {
        let mut serialized = serde_json::to_value(&previous).unwrap();
        serialized[field] = value;
        let wrong = serde_json::from_value(serialized).unwrap();
        assert!(
            validate_headless_research_round_plan(&changed, &current, Some(&receipt), Some(&wrong))
                .unwrap_err()
                .contains(expected)
        );
    }
    for field in [
        "after_sha256",
        "before_sha256",
        "workflow_id",
        "schema_version",
    ] {
        let mut serialized = serde_json::to_value(&receipt).unwrap();
        serialized[field] = if field.contains("sha256") {
            json!("b".repeat(64))
        } else {
            json!("different")
        };
        let wrong = serde_json::from_value(serialized).unwrap();
        assert!(
            validate_headless_research_round_plan(
                &changed,
                &current,
                Some(&wrong),
                Some(&previous)
            )
            .is_err(),
            "accepted {field}"
        );
    }
    changed.steps[0].payload["research_input"] = json!(13.0);
    assert!(
        validate_headless_research_round_plan(&changed, &current, Some(&receipt), Some(&previous))
            .unwrap_err()
            .contains("does not match the current batch")
    );
}
