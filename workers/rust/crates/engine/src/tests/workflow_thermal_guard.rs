use crate::{
    thermal_quality::score_thermal_quality, workflow_executor::run_transform_operator,
    workflow_guard_transforms::evaluate_thermal_guard,
};

fn approx_eq(left: Option<f64>, right: f64) {
    let value = left.expect("expected numeric quality value");
    assert!(
        (value - right).abs() < 1.0e-9,
        "left={value}, right={right}"
    );
}

#[test]
fn evaluates_thermal_guard_as_pass() {
    let guard = evaluate_thermal_guard(
        serde_json::json!({
            "thermal_temperature_max": 80.0,
            "thermal_flux_peak_magnitude": 12.0
        }),
        serde_json::json!({
            "rules": [
                { "field": "thermal_temperature_max", "comparison": "gt", "threshold": 100.0, "severity": "warn", "label": "peak_temp" },
                { "field": "thermal_flux_peak_magnitude", "comparison": "gt", "threshold": 20.0, "severity": "block", "label": "peak_flux" }
            ]
        }),
    )
    .expect("guard should evaluate");

    assert_eq!(guard["guard_status"].as_str(), Some("pass"));
    assert_eq!(guard["guard_passed"].as_bool(), Some(true));
    assert_eq!(guard["guard_trigger_count"].as_u64(), Some(0));
    assert_eq!(
        guard["guard_summary"].as_str(),
        Some("All thermal guard rules passed.")
    );
}

#[test]
fn evaluates_thermal_guard_as_warn_and_block() {
    let guard = evaluate_thermal_guard(
        serde_json::json!({
            "thermo_temperature_delta_max": 135.0,
            "thermo_stress_peak": 260.0
        }),
        serde_json::json!({
            "rules": [
                { "field": "thermo_temperature_delta_max", "comparison": "gte", "threshold": 120.0, "severity": "warn", "label": "delta_max" },
                { "field": "thermo_stress_peak", "comparison": "gt", "value": 250.0, "severity": "block", "label": "stress_peak" }
            ]
        }),
    )
    .expect("guard should evaluate");

    assert_eq!(guard["guard_status"].as_str(), Some("block"));
    assert_eq!(guard["guard_passed"].as_bool(), Some(false));
    assert_eq!(guard["guard_warn_count"].as_u64(), Some(1));
    assert_eq!(guard["guard_block_count"].as_u64(), Some(1));
    assert_eq!(guard["guard_trigger_count"].as_u64(), Some(2));
    assert_eq!(
        guard["guard_recommendation"].as_str(),
        Some("hold_and_review")
    );
    let triggers = guard["guard_triggers"]
        .as_array()
        .expect("guard triggers should be an array");
    assert_eq!(triggers.len(), 2);
    assert_eq!(triggers[0]["label"].as_str(), Some("delta_max"));
    assert_eq!(triggers[1]["severity"].as_str(), Some("block"));
}

#[test]
fn runs_thermal_guard_through_transform_executor() {
    let guard = run_transform_operator(
        "transform.evaluate_thermal_guard",
        serde_json::json!({
            "thermal_temperature_max": 140.0,
            "thermal_flux_peak_magnitude": 9.0
        }),
        serde_json::json!({
            "rules": [
                { "field": "thermal_temperature_max", "comparison": "gte", "threshold": 120.0, "severity": "warn", "label": "temperature_limit" },
                { "field": "thermal_flux_peak_magnitude", "comparison": "gt", "threshold": 20.0, "severity": "block", "label": "flux_limit" }
            ]
        }),
    )
    .expect("workflow transform should evaluate");

    assert_eq!(guard["guard_status"].as_str(), Some("warn"));
    assert_eq!(guard["guard_warn_count"].as_u64(), Some(1));
    assert_eq!(guard["guard_block_count"].as_u64(), Some(0));
    assert_eq!(
        guard["guard_recommendation"].as_str(),
        Some("review_before_continue")
    );
}

#[test]
fn scores_thermal_quality_with_temperature_flux_and_stress_terms() {
    let quality = score_thermal_quality(
        serde_json::json!({
            "thermal_temperature_max": 80.0,
            "thermo_temperature_delta_max": 60.0,
            "thermal_flux_peak_magnitude": 10.0,
            "thermo_stress_peak": 150.0
        }),
        serde_json::json!({
            "targets": {
                "thermal_temperature_max": 120.0,
                "thermo_temperature_delta_max": 80.0,
                "thermal_flux_peak_magnitude": 20.0,
                "thermo_stress_peak": 250.0
            },
            "max_ready_score": 8.0
        }),
    )
    .expect("thermal quality should score");

    assert_eq!(
        quality["thermal_quality_contract"].as_str(),
        Some("kyuubiki.thermal_quality_score/v1")
    );
    assert_eq!(quality["thermal_quality_ready"].as_bool(), Some(true));
    assert_eq!(quality["thermal_quality_grade"].as_str(), Some("good"));
    assert_eq!(
        quality["thermal_quality_missing_metric_count"].as_u64(),
        Some(0)
    );
    approx_eq(quality["thermal_quality_score"].as_f64(), 5.1);
}

#[test]
fn blocks_thermal_quality_when_required_metrics_are_missing() {
    let quality = score_thermal_quality(
        serde_json::json!({
            "thermal_temperature_max": 80.0
        }),
        serde_json::json!({}),
    )
    .expect("thermal quality should report missing metrics");

    assert_eq!(quality["thermal_quality_ready"].as_bool(), Some(false));
    assert_eq!(quality["thermal_quality_grade"].as_str(), Some("block"));
    assert_eq!(
        quality["thermal_quality_missing_metric_count"].as_u64(),
        Some(3)
    );
}

#[test]
fn runs_thermal_quality_through_transform_executor() {
    let quality = run_transform_operator(
        "transform.score_thermal_quality",
        serde_json::json!({
            "thermal_temperature_max": 30.0,
            "thermo_temperature_delta_max": 10.0,
            "thermal_flux_peak_magnitude": 2.0,
            "thermo_stress_peak": 25.0
        }),
        serde_json::json!({
            "max_ready_score": 8.0
        }),
    )
    .expect("thermal quality should run through executor");

    assert_eq!(quality["thermal_quality_ready"].as_bool(), Some(true));
    assert_eq!(quality["thermal_quality_grade"].as_str(), Some("excellent"));
    assert_eq!(quality["thermal_quality_term_count"].as_u64(), Some(4));
}
