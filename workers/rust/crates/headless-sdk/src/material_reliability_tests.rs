use crate::{material_validation_quality_gate, material_validation_repair_hint};
use serde_json::json;

#[test]
fn validation_payload_becomes_material_quality_gate_and_repair_hint() {
    let validation = json!({
        "validation_contract": "kyuubiki.summary_tolerance_validation/v1",
        "validation_passed": false,
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
