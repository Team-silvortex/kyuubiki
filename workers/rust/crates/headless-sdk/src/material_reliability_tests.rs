use crate::{material_validation_quality_gate, material_validation_repair_hint};
use serde_json::json;

#[test]
fn validation_payload_becomes_material_quality_gate_and_repair_hint() {
    let validation = json!({
        "validation_contract": "kyuubiki.summary_tolerance_validation/v1",
        "validation_passed": false,
        "validation_checked_field_count": 1,
        "validation_failed_field_count": 1,
        "validation_missing_field_count": 0,
        "validation_fail_on_missing": true,
        "validation_failures": [{
            "field": "peak_temperature_c",
            "absolute_error": 4.2,
            "relative_error": 0.06
        }],
        "validation_missing_fields": []
    });

    let gate = material_validation_quality_gate(&validation).expect("validation gate");
    assert_eq!(gate.id, "gate.summary_tolerance_validation");
    assert_eq!(gate.status, "violate");
    assert_eq!(gate.actual_value, Some(1.0));

    let hint = material_validation_repair_hint(&validation).expect("repair hint");
    assert_eq!(hint.action, "fix_validation_failure");
    assert_eq!(hint.strategy, "rerun_validation_focused_sweep");
    assert_eq!(hint.domain, "validation");
    assert_eq!(hint.focus_field.as_deref(), Some("peak_temperature_c"));
    assert_eq!(hint.blocking_gate_id, gate.id);
}

#[test]
fn passing_validation_payload_exposes_gate_without_repair_hint() {
    let validation = json!({
        "validation_contract": "kyuubiki.summary_tolerance_validation/v1",
        "validation_passed": true,
        "validation_checked_field_count": 2,
        "validation_failed_field_count": 0,
        "validation_missing_field_count": 0,
        "validation_fail_on_missing": true,
        "validation_failures": [],
        "validation_missing_fields": []
    });

    let gate = material_validation_quality_gate(&validation).expect("validation gate");
    assert_eq!(gate.status, "pass");
    assert_eq!(gate.actual_value, Some(0.0));
    assert!(material_validation_repair_hint(&validation).is_none());
}

#[test]
fn missing_validation_field_uses_fill_missing_repair_strategy() {
    let validation = json!({
        "validation_contract": "kyuubiki.summary_tolerance_validation/v1",
        "validation_passed": false,
        "validation_checked_field_count": 0,
        "validation_failed_field_count": 0,
        "validation_missing_field_count": 1,
        "validation_fail_on_missing": true,
        "validation_failures": [],
        "validation_missing_fields": ["max_stress_mpa"]
    });

    let gate = material_validation_quality_gate(&validation).expect("validation gate");
    assert_eq!(gate.status, "violate");

    let hint = material_validation_repair_hint(&validation).expect("repair hint");
    assert_eq!(hint.strategy, "fill_missing_summary_field");
    assert_eq!(hint.focus_field.as_deref(), Some("max_stress_mpa"));
}

#[test]
fn empty_or_contradictory_validation_claims_block_and_request_repair() {
    for (checked, failed, missing) in [(0, 0, 0), (1, 1, 0), (1, 0, 1)] {
        let report = json!({
            "validation_contract": "kyuubiki.summary_tolerance_validation/v1",
            "validation_passed": true,
            "validation_checked_field_count": checked,
            "validation_failed_field_count": failed,
            "validation_missing_field_count": missing,
            "validation_fail_on_missing": true,
        });
        let gate = material_validation_quality_gate(&report).unwrap();
        assert_eq!(gate.status, "violate");
        assert!(material_validation_repair_hint(&report).is_some());
    }
}

#[test]
fn missing_or_malformed_validation_metadata_cannot_be_inferred_as_success() {
    let report = json!({
        "validation_contract": "kyuubiki.summary_tolerance_validation/v1",
        "validation_passed": true,
        "validation_checked_field_count": 1,
        "validation_failed_field_count": 0,
        "validation_missing_field_count": 0,
        "validation_fail_on_missing": true,
    });
    for field in [
        "validation_passed",
        "validation_checked_field_count",
        "validation_failed_field_count",
        "validation_missing_field_count",
    ] {
        let mut missing = report.clone();
        missing.as_object_mut().unwrap().remove(field);
        assert_eq!(
            material_validation_quality_gate(&missing).unwrap().status,
            "violate"
        );
        assert!(material_validation_repair_hint(&missing).is_some());
        let mut malformed = report.clone();
        malformed[field] = json!("0");
        assert_eq!(
            material_validation_quality_gate(&malformed).unwrap().status,
            "violate"
        );
    }
}

#[test]
fn oversized_validation_counters_do_not_panic_or_wrap_to_success() {
    let report = json!({
        "validation_contract": "kyuubiki.summary_tolerance_validation/v1",
        "validation_passed": false,
        "validation_checked_field_count": 1,
        "validation_failed_field_count": u64::MAX,
        "validation_missing_field_count": 1,
        "validation_fail_on_missing": true,
    });
    let gate = material_validation_quality_gate(&report).unwrap();
    assert_eq!(gate.status, "violate");
    assert!(gate.actual_value.unwrap().is_finite());
    assert!(gate.actual_value.unwrap() > 0.0);
}

#[test]
fn optional_missing_validation_still_needs_a_checked_field() {
    let mut report = json!({
        "validation_contract": "kyuubiki.summary_tolerance_validation/v1",
        "validation_passed": true,
        "validation_checked_field_count": 1,
        "validation_failed_field_count": 0,
        "validation_missing_field_count": 1,
        "validation_fail_on_missing": false,
    });
    assert_eq!(
        material_validation_quality_gate(&report).unwrap().status,
        "pass"
    );
    report["validation_checked_field_count"] = json!(0);
    assert_eq!(
        material_validation_quality_gate(&report).unwrap().status,
        "violate"
    );
    assert!(material_validation_repair_hint(&report).is_some());
}

#[test]
fn validation_cannot_ignore_malformed_policy_or_failure_details() {
    let report = json!({
        "validation_contract": "kyuubiki.summary_tolerance_validation/v1",
        "validation_passed": true,
        "validation_checked_field_count": 1,
        "validation_failed_field_count": 0,
        "validation_missing_field_count": 0,
    });
    assert_eq!(
        material_validation_quality_gate(&report).unwrap().status,
        "pass"
    );
    for patch in [
        json!({"validation_fail_on_missing": "false"}),
        json!({"validation_fail_on_missing": null}),
        json!({"validation_failures": [{"field": "stress"}]}),
        json!({"validation_failures": null}),
    ] {
        let mut invalid = report.clone();
        invalid
            .as_object_mut()
            .unwrap()
            .extend(patch.as_object().unwrap().clone());
        assert_eq!(
            material_validation_quality_gate(&invalid).unwrap().status,
            "violate"
        );
        assert!(material_validation_repair_hint(&invalid).is_some());
    }
}
