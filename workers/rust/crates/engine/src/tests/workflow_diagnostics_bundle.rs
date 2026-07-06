use crate::{
    workflow_bundle_transforms::compose_diagnostics_bundle,
    workflow_executor::run_transform_operator,
};

#[test]
fn composes_diagnostics_bundle_from_multiple_domains() {
    let bundle = compose_diagnostics_bundle(
        serde_json::json!({
            "electrostatic": {
                "diagnostic_contract": "kyuubiki.workflow_diagnostics/v1",
                "diagnostic_domain": "electrostatic",
                "diagnostic_subject": "electrostatic_result",
                "diagnostic_prefix": "electrostatic",
                "diagnostic_node_count": 4,
                "diagnostic_element_count": 1,
                "diagnostic_metric_groups": ["potential", "field"],
                "electrostatic_potential_max": 10.0
            },
            "thermal": {
                "diagnostic_contract": "kyuubiki.workflow_diagnostics/v1",
                "diagnostic_domain": "thermal",
                "diagnostic_subject": "thermal_result",
                "diagnostic_prefix": "thermal",
                "diagnostic_node_count": 4,
                "diagnostic_element_count": 1,
                "diagnostic_metric_groups": ["temperature", "flux"],
                "thermal_temperature_max": 80.0
            },
            "ignored": {
                "foo": 1.0
            }
        }),
        serde_json::json!({}),
    )
    .expect("diagnostics bundle should compose");

    assert_eq!(
        bundle["bundle_contract"].as_str(),
        Some("kyuubiki.workflow_diagnostics_bundle/v1")
    );
    assert_eq!(bundle["bundle_source_count"].as_u64(), Some(2));
    assert_eq!(bundle["bundle_total_node_count"].as_u64(), Some(8));
    assert_eq!(bundle["bundle_total_element_count"].as_u64(), Some(2));
    assert_eq!(bundle["bundle_numeric_field_count"].as_u64(), Some(4));
    let domains = bundle["bundle_domains"]
        .as_array()
        .expect("bundle domains should be an array");
    assert_eq!(domains.len(), 2);
    assert_eq!(domains[0].as_str(), Some("electrostatic"));
    assert_eq!(domains[1].as_str(), Some("thermal"));
    let items = bundle["bundle_items"]
        .as_array()
        .expect("bundle items should be an array");
    assert_eq!(items.len(), 2);
    assert_eq!(items[0]["source"].as_str(), Some("electrostatic"));
    assert!(bundle["bundle_payloads"].get("thermal").is_some());
}

#[test]
fn composes_diagnostics_bundle_through_transform_executor() {
    let bundle = run_transform_operator(
        "transform.compose_diagnostics_bundle",
        serde_json::json!({
            "thermal": {
                "diagnostic_contract": "kyuubiki.workflow_diagnostics/v1",
                "diagnostic_domain": "thermal",
                "diagnostic_subject": "thermal_result",
                "diagnostic_prefix": "thermal",
                "diagnostic_node_count": 3,
                "diagnostic_element_count": 2,
                "diagnostic_metric_groups": ["temperature", "flux"],
                "thermal_temperature_max": 75.0
            },
            "thermo": {
                "diagnostic_contract": "kyuubiki.workflow_diagnostics/v1",
                "diagnostic_domain": "thermo_mechanical",
                "diagnostic_subject": "thermo_result",
                "diagnostic_prefix": "thermo",
                "diagnostic_node_count": 3,
                "diagnostic_element_count": 2,
                "diagnostic_metric_groups": ["temperature_delta", "stress"],
                "thermo_stress_peak": 180.0
            }
        }),
        serde_json::json!({
            "include_numeric_fields": true,
            "include_payloads": false
        }),
    )
    .expect("transform bundle should succeed");

    assert_eq!(bundle["bundle_source_count"].as_u64(), Some(2));
    assert!(bundle.get("bundle_payloads").is_none());
    let fields = bundle["bundle_numeric_fields"]
        .as_array()
        .expect("bundle numeric fields should be an array");
    assert!(
        fields
            .iter()
            .any(|field| field.as_str() == Some("thermal_temperature_max"))
    );
    assert!(
        fields
            .iter()
            .any(|field| field.as_str() == Some("thermo_stress_peak"))
    );
}
