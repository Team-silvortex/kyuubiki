use crate::{
    workflow_bundle_transforms::{compose_diagnostics_bundle, evaluate_diagnostics_bundle_guard},
    workflow_executor::run_transform_operator,
};

#[test]
fn evaluates_diagnostics_bundle_guard_with_source_rules() {
    let bundle = compose_diagnostics_bundle(
        serde_json::json!({
            "thermal": {
                "diagnostic_contract": "kyuubiki.workflow_diagnostics/v1",
                "diagnostic_domain": "thermal",
                "diagnostic_subject": "thermal_result",
                "diagnostic_prefix": "thermal",
                "diagnostic_node_count": 4,
                "diagnostic_element_count": 1,
                "diagnostic_metric_groups": ["temperature", "flux"],
                "thermal_temperature_max": 125.0
            },
            "thermo": {
                "diagnostic_contract": "kyuubiki.workflow_diagnostics/v1",
                "diagnostic_domain": "thermo_mechanical",
                "diagnostic_subject": "thermo_result",
                "diagnostic_prefix": "thermo",
                "diagnostic_node_count": 4,
                "diagnostic_element_count": 1,
                "diagnostic_metric_groups": ["temperature_delta", "stress"],
                "thermo_stress_peak": 220.0
            }
        }),
        serde_json::json!({}),
    )
    .expect("bundle should compose");

    let guard = evaluate_diagnostics_bundle_guard(
        bundle,
        serde_json::json!({
            "rules": [
                { "source": "thermal", "field": "thermal_temperature_max", "threshold": 120.0, "severity": "warn", "label": "thermal temperature" },
                { "source": "thermo", "field": "thermo_stress_peak", "comparison": "gt", "threshold": 180.0, "severity": "block", "label": "stress ceiling" }
            ]
        }),
    )
    .expect("bundle guard should evaluate");

    assert_eq!(
        guard["guard_contract"].as_str(),
        Some("kyuubiki.workflow_guard_result/v1")
    );
    assert_eq!(
        guard["guard_scope"].as_str(),
        Some("workflow_diagnostics_bundle")
    );
    assert_eq!(guard["guard_status"].as_str(), Some("block"));
    assert_eq!(guard["guard_warn_count"].as_u64(), Some(1));
    assert_eq!(guard["guard_block_count"].as_u64(), Some(1));
    let triggers = guard["guard_triggers"]
        .as_array()
        .expect("guard triggers should be an array");
    assert_eq!(triggers.len(), 2);
    assert_eq!(triggers[0]["source"].as_str(), Some("thermal"));
    assert_eq!(triggers[1]["severity"].as_str(), Some("block"));
}

#[test]
fn runs_diagnostics_bundle_guard_through_transform_executor() {
    let bundle = compose_diagnostics_bundle(
        serde_json::json!({
            "electrostatic": {
                "diagnostic_contract": "kyuubiki.workflow_diagnostics/v1",
                "diagnostic_domain": "electrostatic",
                "diagnostic_subject": "electrostatic_result",
                "diagnostic_prefix": "electrostatic",
                "diagnostic_node_count": 3,
                "diagnostic_element_count": 1,
                "diagnostic_metric_groups": ["field"],
                "electrostatic_field_peak_magnitude": 8.5
            }
        }),
        serde_json::json!({}),
    )
    .expect("bundle should compose");

    let guard = run_transform_operator(
        "transform.evaluate_diagnostics_bundle_guard",
        bundle,
        serde_json::json!({
            "rules": [
                { "source": "electrostatic", "field": "electrostatic_field_peak_magnitude", "comparison": "gt", "threshold": 9.0, "severity": "warn", "label": "field ceiling" }
            ]
        }),
    )
    .expect("transform bundle guard should succeed");

    assert_eq!(guard["guard_status"].as_str(), Some("pass"));
    assert_eq!(guard["guard_passed"].as_bool(), Some(true));
    assert_eq!(
        guard["guard_summary"].as_str(),
        Some("All diagnostics bundle guard rules passed.")
    );
}
