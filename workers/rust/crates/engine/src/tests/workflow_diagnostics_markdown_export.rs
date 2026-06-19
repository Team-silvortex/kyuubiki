use crate::{
    workflow_bundle_exports::export_diagnostics_bundle_markdown,
    workflow_bundle_transforms::{
        compose_diagnostics_bundle, compose_diagnostics_report_payload,
        evaluate_diagnostics_bundle_guard,
    },
    workflow_executor::run_export_operator,
};

fn sample_report_payload() -> serde_json::Value {
    let bundle = compose_diagnostics_bundle(
        serde_json::json!({
            "electrostatic": {
                "diagnostic_contract": "kyuubiki.workflow_diagnostics/v1",
                "diagnostic_domain": "electrostatic",
                "diagnostic_subject": "electrostatic_result",
                "diagnostic_prefix": "electrostatic",
                "diagnostic_node_count": 4,
                "diagnostic_element_count": 1,
                "diagnostic_metric_groups": ["field"],
                "electrostatic_field_peak_magnitude": 10.0
            },
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
                "thermo_stress_peak": 220.0,
                "peak_element_id": "te1",
                "peak_stress_x": 14.0,
                "peak_stress_y": 9.0,
                "peak_tau_xy": 3.0,
                "peak_element_temperature_delta": 35.0
            }
        }),
        serde_json::json!({}),
    )
    .expect("bundle should compose");
    let guard = evaluate_diagnostics_bundle_guard(
        bundle.clone(),
        serde_json::json!({
            "rules": [
                { "source": "thermal", "field": "thermal_temperature_max", "threshold": 120.0, "severity": "warn", "label": "thermal temperature" },
                { "source": "thermo", "field": "thermo_stress_peak", "comparison": "gt", "threshold": 180.0, "severity": "block", "label": "stress ceiling" }
            ]
        }),
    )
    .expect("guard should evaluate");
    compose_diagnostics_report_payload(
        serde_json::json!({
            "bundle": bundle,
            "guard": guard
        }),
        serde_json::json!({}),
    )
    .expect("report payload should compose")
}

#[test]
fn exports_diagnostics_bundle_markdown() {
    let export = export_diagnostics_bundle_markdown(
        sample_report_payload(),
        serde_json::json!({
            "title": "Diagnostics Bundle Report"
        }),
    )
    .expect("markdown export should succeed");

    assert_eq!(export["format"].as_str(), Some("markdown"));
    let content = export["content"]
        .as_str()
        .expect("markdown content should be a string");
    assert!(content.contains("# Diagnostics Bundle Report"));
    assert!(content.contains("## Key Highlights"));
    assert!(content.contains("[info] Electrostatic field peak: 10.0"));
    assert!(content.contains("[attention] Thermal temperature peak: 125.0"));
    assert!(content.contains("[attention] Thermo stress peak: 220.0"));
    assert!(content.contains("  - source: thermo"));
    assert!(content.contains("  - value_field: thermo_stress_peak"));
    assert!(content.contains("  - peak_stress_x: 14.0"));
    assert!(content.contains("  - peak_element_id: te1"));
    assert!(content.contains("## Diagnostics Sources"));
    assert!(content.contains("## Guard Decision"));
    assert!(content.contains("### Guard Triggers"));
    assert!(content.contains("thermo.stress ceiling"));
}

#[test]
fn runs_diagnostics_bundle_markdown_export_through_executor() {
    let export = run_export_operator(
        "export.diagnostics_bundle_markdown",
        sample_report_payload(),
        serde_json::json!({
            "title": "Diagnostics Bundle Report",
            "item_count": 1
        }),
    )
    .expect("export operator should succeed");

    assert_eq!(export["content_type"].as_str(), Some("text/markdown"));
    let content = export["content"]
        .as_str()
        .expect("markdown content should be a string");
    assert!(content.contains("Contract: kyuubiki.workflow_diagnostics_bundle/v1"));
    assert!(content.contains("## Key Highlights"));
    assert!(content.contains("Status: block"));
}
