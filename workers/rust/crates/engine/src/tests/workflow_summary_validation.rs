use crate::workflow_executor::run_transform_operator;
use serde_json::{Value, json};

#[test]
fn runs_summary_tolerance_validation_through_sdk_registry() {
    let summary = run_transform_operator(
        "transform.validate_summary_tolerance",
        serde_json::json!({
            "left": { "max_stress": 10.0, "max_displacement": 1.5 },
            "right": { "max_stress": 10.01, "max_displacement": 1.5005 }
        }),
        serde_json::json!({
            "fields": ["max_stress", "max_displacement"],
            "absolute_tolerance": 0.02,
            "relative_tolerance": 0.001
        }),
    )
    .expect("transform.validate_summary_tolerance should succeed");

    assert_eq!(
        summary["validation_contract"].as_str(),
        Some("kyuubiki.summary_tolerance_validation/v1")
    );
    assert_eq!(summary["validation_passed"].as_bool(), Some(true));
    assert_eq!(summary["validation_checked_field_count"].as_u64(), Some(2));
}

#[test]
fn zero_comparisons_block_even_when_missing_fields_are_optional() {
    let report = validate(
        json!({"left": {}, "right": {"stress": 12.0}}),
        json!({"fields": ["stress"], "fail_on_missing": false}),
    )
    .expect("missing evidence should produce a blocking report");
    assert_eq!(report["validation_checked_field_count"], 0);
    assert_eq!(report["validation_missing_fields"], json!(["stress"]));
    assert_eq!(report["validation_passed"], false);
    assert_eq!(report["validation_grade"], "block");
    let objective = run_transform_operator(
        "transform.compose_quality_objective",
        json!({"cross_check": report}),
        json!({}),
    )
    .expect("blocking report should remain available for repair planning");
    assert_eq!(objective["composite_quality_ready"], false);
    assert_eq!(objective["composite_quality_blocked_term_count"], 1);
}

#[test]
fn automatic_fields_retain_one_sided_numeric_evidence() {
    for payload in [
        json!({"left": {"stress": 1.0, "temperature": 2.0, "label": "a"},
               "right": {"stress": 1.0, "label": "b"}}),
        json!({"left": {"stress": 1.0, "label": "a"},
               "right": {"stress": 1.0, "temperature": 2.0, "label": "b"}}),
    ] {
        let report = validate(payload, Value::Null).unwrap();
        assert_eq!(report["validation_checked_field_count"], 1);
        assert_eq!(report["validation_missing_fields"], json!(["temperature"]));
        assert_eq!(report["validation_passed"], false);
    }
}

#[test]
fn optional_missing_fields_require_at_least_one_real_comparison() {
    for (fail_on_missing, passed) in [(false, true), (true, false)] {
        let report = validate(
            pair(),
            json!({
                "fields": ["stress", "temperature"], "fail_on_missing": fail_on_missing,
            }),
        )
        .unwrap();
        assert_eq!(report["validation_checked_field_count"], 1);
        assert_eq!(report["validation_passed"], passed);
        let objective = run_transform_operator(
            "transform.compose_quality_objective",
            json!({"cross_check": report}),
            json!({}),
        )
        .unwrap();
        assert_eq!(objective["composite_quality_ready"], passed);
        assert_eq!(
            objective["composite_quality_blocking_terms"]
                .as_array()
                .unwrap()
                .is_empty(),
            passed
        );
    }
}

#[test]
fn malformed_tolerances_and_missing_policy_do_not_use_defaults() {
    for field in ["absolute_tolerance", "relative_tolerance"] {
        for value in [
            json!(-1.0),
            json!("0.1"),
            Value::Null,
            json!(true),
            json!([]),
        ] {
            let error = validate(pair(), json!({field: value})).expect_err("invalid tolerance");
            assert!(error.contains(field), "{error}");
        }
    }
    for value in [json!("false"), json!(0), Value::Null] {
        let error =
            validate(pair(), json!({"fail_on_missing": value})).expect_err("invalid policy");
        assert!(error.contains("fail_on_missing"), "{error}");
    }
    for config in [json!([]), json!(false), json!("defaults")] {
        assert!(validate(pair(), config).is_err());
    }
}

#[test]
fn malformed_or_duplicate_field_selections_cannot_shrink_or_inflate_coverage() {
    for fields in [
        json!("stress"),
        Value::Null,
        json!([]),
        json!(["stress", null]),
        json!(["stress", 1]),
        json!(["stress", ""]),
        json!([" "]),
        json!(["stress", "stress"]),
    ] {
        let error = validate(pair(), json!({"fields": fields})).expect_err("invalid fields");
        assert!(error.contains("fields"), "{error}");
    }
}

#[test]
fn unrepresentable_error_fails_before_a_null_metric_can_escape() {
    let error = validate(
        json!({"left": {"stress": -1.0e308}, "right": {"stress": 1.0e308}}),
        json!({}),
    )
    .expect_err("overflowed error must not serialize as null");
    assert!(
        error.contains("stress") && error.contains("non-finite"),
        "{error}"
    );
    let report = validate(
        json!({"left": {"stress": 1.0e308}, "right": {"stress": 1.0e308}}),
        json!({"absolute_tolerance": 0.0, "relative_tolerance": 0.0}),
    )
    .unwrap();
    assert_eq!(report["validation_passed"], true);
    assert_eq!(report["validation_max_absolute_error"], 0.0);
}

#[test]
fn zero_tolerance_preserves_exact_comparison() {
    let config = json!({"absolute_tolerance": 0.0, "relative_tolerance": 0.0});
    assert_eq!(
        validate(pair(), config.clone()).unwrap()["validation_passed"],
        true
    );
    let report = validate(
        json!({"left": {"stress": 1.0}, "right": {"stress": 1.0001}}),
        config,
    )
    .unwrap();
    assert_eq!(report["validation_passed"], false);
}

#[test]
fn quality_objective_rejects_empty_or_contradictory_pass_claims() {
    for (checked, failed, missing) in [(0, 0, 0), (1, 1, 0), (1, 0, 1)] {
        let objective = run_transform_operator(
            "transform.compose_quality_objective",
            json!({"cross_check": {
                "validation_contract": "kyuubiki.summary_tolerance_validation/v1",
                "validation_passed": true,
                "validation_checked_field_count": checked,
                "validation_failed_field_count": failed,
                "validation_missing_field_count": missing,
                "validation_fail_on_missing": true,
                "validation_max_absolute_error": 0.0,
                "validation_max_relative_error": 0.0,
            }}),
            json!({}),
        )
        .unwrap();
        assert_eq!(objective["composite_quality_ready"], false);
        assert!(
            !objective["composite_quality_blocking_terms"]
                .as_array()
                .unwrap()
                .is_empty()
        );
    }
}

#[test]
fn quality_objective_rejects_malformed_success_metadata() {
    let report = validate(pair(), Value::Null).unwrap();
    for field in [
        "validation_passed",
        "validation_checked_field_count",
        "validation_failed_field_count",
        "validation_missing_field_count",
    ] {
        let mut missing = report.clone();
        missing.as_object_mut().unwrap().remove(field);
        assert_blocked_objective(missing);
        for value in [Value::Null, json!("0"), json!(-1)] {
            let mut malformed = report.clone();
            malformed[field] = value;
            assert_blocked_objective(malformed);
        }
    }
    for value in [Value::Null, json!("false"), json!(0)] {
        let mut malformed = report.clone();
        malformed["validation_fail_on_missing"] = value;
        assert_blocked_objective(malformed);
    }
}

#[test]
fn quality_objective_does_not_hide_contradictory_failure_details() {
    let mut report = validate(pair(), Value::Null).unwrap();
    report["validation_failures"] = json!([{"field": "stress"}]);
    assert_blocked_objective(report);
}

fn assert_blocked_objective(report: Value) {
    let objective = run_transform_operator(
        "transform.compose_quality_objective",
        json!({"cross_check": report}),
        json!({}),
    )
    .unwrap();
    assert_eq!(objective["composite_quality_ready"], false);
    assert!(
        !objective["composite_quality_blocking_terms"]
            .as_array()
            .unwrap()
            .is_empty()
    );
}

fn validate(payload: Value, config: Value) -> Result<Value, String> {
    run_transform_operator("transform.validate_summary_tolerance", payload, config)
}

fn pair() -> Value {
    json!({"left": {"stress": 1.0}, "right": {"stress": 1.0}})
}
