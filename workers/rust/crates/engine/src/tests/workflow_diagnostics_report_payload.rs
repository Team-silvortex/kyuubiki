use crate::{
    workflow_bundle_transforms::{
        compose_diagnostics_bundle, compose_diagnostics_report_payload,
        evaluate_diagnostics_bundle_guard,
    },
    workflow_executor::run_transform_operator,
};

fn sample_bundle_and_guard() -> (serde_json::Value, serde_json::Value) {
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
                "diagnostic_metric_groups": ["temperature_delta", "stress", "thermal_strain"],
                "thermo_temperature_delta_max": 125.0,
                "thermo_peak_stress": 220.0,
                "thermo_peak_thermal_strain": 0.00048
            }
        }),
        serde_json::json!({}),
    )
    .expect("bundle should compose");
    let guard = evaluate_diagnostics_bundle_guard(
        bundle.clone(),
        serde_json::json!({
            "rules": [
                { "source": "thermal", "field": "thermal_temperature_max", "threshold": 120.0, "severity": "warn", "label": "thermal temperature" }
            ]
        }),
    )
    .expect("guard should evaluate");
    (bundle, guard)
}

#[test]
fn composes_diagnostics_report_payload_with_guard() {
    let (bundle, guard) = sample_bundle_and_guard();
    let report = compose_diagnostics_report_payload(
        serde_json::json!({
            "bundle": bundle,
            "guard": guard
        }),
        serde_json::json!({}),
    )
    .expect("report payload should compose");

    assert_eq!(
        report["report_contract"].as_str(),
        Some("kyuubiki.workflow_report_payload/v1")
    );
    assert_eq!(
        report["report_kind"].as_str(),
        Some("diagnostics_bundle_report_payload")
    );
    assert_eq!(report["report_guard_status"].as_str(), Some("warn"));
    assert_eq!(
        report["report_guard_recommendation"].as_str(),
        Some("review_before_continue")
    );
    assert_eq!(
        report["report_focus_metrics"]["thermal.temperature_max"].as_f64(),
        Some(125.0)
    );
    assert_eq!(
        report["report_focus_metrics"]["thermo.stress_peak"].as_f64(),
        Some(220.0)
    );
    let highlights = report["report_highlights"]
        .as_array()
        .expect("report highlights should be an array");
    assert!(highlights.iter().any(|item| {
        item["id"].as_str() == Some("thermal.temperature_max")
            && item["attention"].as_bool() == Some(true)
    }));
    assert!(highlights.iter().any(|item| {
        item["id"].as_str() == Some("thermo.stress_peak")
            && item["attention"].as_bool() == Some(false)
    }));
    assert!(report.get("guard_payload").is_some());
    assert!(report.get("bundle_items").is_some());
}

#[test]
fn composes_diagnostics_report_payload_with_pruned_fields() {
    let (bundle, guard) = sample_bundle_and_guard();
    let report = compose_diagnostics_report_payload(
        serde_json::json!({
            "bundle": bundle,
            "guard": guard
        }),
        serde_json::json!({
            "include_guard": false,
            "include_bundle_items": false
        }),
    )
    .expect("report payload should compose");

    assert!(report.get("guard_payload").is_none());
    assert!(report.get("report_guard_status").is_none());
    assert!(report.get("bundle_items").is_none());
    assert_eq!(
        report["report_focus_metrics"]["thermo.thermal_strain_peak"].as_f64(),
        Some(0.00048)
    );
    let sources = report["report_sources"]
        .as_array()
        .expect("report sources should be an array");
    assert_eq!(sources.len(), 2);
}

#[test]
fn runs_diagnostics_report_payload_through_transform_executor() {
    let (bundle, guard) = sample_bundle_and_guard();
    let report = run_transform_operator(
        "transform.compose_diagnostics_report_payload",
        serde_json::json!({
            "bundle": bundle,
            "guard": guard
        }),
        serde_json::json!({
            "include_bundle_items": false
        }),
    )
    .expect("transform report payload should succeed");

    assert_eq!(report["report_guard_status"].as_str(), Some("warn"));
    assert!(report.get("bundle_items").is_none());
}
