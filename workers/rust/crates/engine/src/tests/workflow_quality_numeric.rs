use crate::workflow_quality_objective::compose_quality_objective;
use serde_json::{Value, json};

#[test]
fn rejects_overflow_in_weighted_score_and_penalty_components() {
    for (summary, config) in [
        (quality(1.0e308), json!({"weights": {"thermal": 2.0}})),
        (
            json!({"thermal_quality_score": 1.0, "thermal_quality_ready": false,
                "thermal_quality_missing_metric_count": u64::MAX}),
            json!({"missing_metric_penalty": 1.0e308}),
        ),
        (
            json!({"thermal_quality_score": 1.0e308, "thermal_quality_ready": false}),
            json!({"not_ready_penalty": 1.0e308}),
        ),
    ] {
        let error = compose_quality_objective(json!({"thermal": summary}), config)
            .expect_err("overflow must not serialize as a null score");
        assert!(
            error.contains("thermal") && error.contains("non-finite"),
            "{error}"
        );
    }
}

#[test]
fn finite_terms_cannot_overflow_the_composite_score() {
    let error = compose_quality_objective(
        json!({"first": quality(1.0e308), "second": quality(1.0e308)}),
        Value::Null,
    )
    .expect_err("aggregate overflow must be explicit");
    assert!(
        error.contains("composite_quality_score") && error.contains("second"),
        "{error}"
    );
}

#[test]
fn quality_counter_aggregation_never_wraps() {
    for field in [
        "thermal_quality_missing_metric_count",
        "thermal_quality_watch_count",
    ] {
        let mut first = quality(0.0);
        first[field] = json!(u64::MAX);
        let mut second = quality(0.0);
        second[field] = json!(1);
        let error = compose_quality_objective(
            json!({"first": first, "second": second}),
            json!({"missing_metric_penalty": 0.0}),
        )
        .expect_err("counter overflow must not panic or wrap");
        assert!(
            error.contains("overflow") && error.contains("second"),
            "{error}"
        );
    }
}

#[test]
fn validation_error_metrics_cannot_be_malformed_or_overflow_the_score() {
    for field in [
        "validation_max_absolute_error",
        "validation_max_relative_error",
    ] {
        for value in [Value::Null, json!("0"), json!(-1.0)] {
            let mut report = validation();
            report[field] = value;
            let error = compose_quality_objective(json!({"cross_check": report}), Value::Null)
                .expect_err("invalid metrics cannot become a zero score");
            assert!(
                error.contains(field) && error.contains("cross_check"),
                "{error}"
            );
        }
    }
    let mut report = validation();
    report["validation_max_relative_error"] = json!(1.0e308);
    let error = compose_quality_objective(json!({"cross_check": report}), Value::Null)
        .expect_err("scaled validation error must remain finite");
    assert!(
        error.contains("non-finite") && error.contains("cross_check"),
        "{error}"
    );
}

#[test]
fn negative_quality_scores_cannot_cancel_other_penalties() {
    let error = compose_quality_objective(
        json!({"thermal": quality(-100.0), "other": quality(20.0)}),
        Value::Null,
    )
    .expect_err("quality penalties cannot be negative");
    assert!(
        error.contains("thermal_quality_score") && error.contains("nonnegative"),
        "{error}"
    );
}

#[test]
fn zero_weight_and_large_finite_values_remain_supported() {
    let objective = compose_quality_objective(
        json!({"thermal": quality(1.0e308)}),
        json!({"weights": {"thermal": 0.0}, "max_ready_score": 0.0}),
    )
    .unwrap();
    assert_eq!(objective["composite_quality_score"], 0.0);
    assert_eq!(objective["composite_quality_ready"], true);
    let objective = compose_quality_objective(
        json!({"thermal": quality(1.0e308)}),
        json!({"max_ready_score": 1.0e308}),
    )
    .unwrap();
    assert_eq!(objective["composite_quality_score"], 1.0e308);
}

#[test]
fn malformed_quality_config_does_not_silently_use_defaults() {
    for field in [
        "missing_metric_penalty",
        "not_ready_penalty",
        "max_ready_score",
    ] {
        for value in [Value::Null, json!("0"), json!(-1.0), json!(false)] {
            let error =
                compose_quality_objective(json!({"thermal": quality(1.0)}), json!({field: value}))
                    .expect_err("invalid configured numbers must be rejected");
            assert!(error.contains(field), "{error}");
        }
    }
    for config in [
        json!([]),
        json!(false),
        json!({"weights": null}),
        json!({"weights": []}),
        json!({"weights": {"thermal": -1.0}}),
        json!({"weights": {"thermal": "1"}}),
    ] {
        assert!(compose_quality_objective(json!({"thermal": quality(1.0)}), config).is_err());
    }
}

#[test]
fn nonfinite_grade_is_always_blocking() {
    for score in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert_eq!(
            crate::workflow_quality_terms::composite_grade(score, 0, 1.0),
            "block"
        );
    }
}

fn quality(score: f64) -> Value {
    json!({"thermal_quality_score": score, "thermal_quality_ready": true})
}

fn validation() -> Value {
    json!({
        "validation_contract": "kyuubiki.summary_tolerance_validation/v1",
        "validation_passed": true, "validation_checked_field_count": 1,
        "validation_failed_field_count": 0, "validation_missing_field_count": 0,
        "validation_max_absolute_error": 0.0, "validation_max_relative_error": 0.0
    })
}
