use crate::{
    workflow_executor::run_transform_operator, workflow_guard_transforms::evaluate_thermal_guard,
};

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
