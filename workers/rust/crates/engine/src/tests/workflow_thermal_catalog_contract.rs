use crate::workflow_executor::run_extract_operator;
use serde_json::{Value, json};

#[test]
fn thermal_catalog_metrics_preserve_vector_aliases_and_signed_nonzero_load_counts() {
    let result = run_extract_operator(
        "extract.thermal_result_diagnostics",
        json!({
            "nodes":[{"heat_load":0.0},{"heat_load":-0.0},{"heat_load":-4.0},{"heat_load":4.0}],
            "elements":[{"id":"e0","heat_flux_x":-1800.0,"heat_flux_y":2400.0}]
        }),
        json!({}),
    )
    .unwrap();
    assert_eq!(result["thermal_loaded_node_count"], 2);
    assert_eq!(result["thermal_heat_load_count"], 4);
    assert_eq!(result["thermal_total_heat_load"], 0.0);
    assert_eq!(result["thermal_peak_flux_magnitude"], 3000.0);
    assert_eq!(
        result["thermal_peak_flux_magnitude"],
        result["thermal_flux_peak_magnitude"]
    );
    assert_eq!(result["thermal_peak_flux_x"], -1800.0);
    assert_eq!(result["thermal_peak_flux_element_id"], "e0");
}

#[test]
fn thermal_catalog_aliases_follow_custom_sources_fields_and_prefix() {
    let result = run_extract_operator(
        "extract.thermal_result_diagnostics",
        json!({
            "points":[{"load":-2.0},{"load":0.0}],
            "cells":[{"id":"peak","magnitude":7.0}]
        }),
        json!({"node_source":"points","element_source":"cells","output_prefix":"sample",
        "heat_load_field":"load","flux_magnitude_field":"magnitude"}),
    )
    .unwrap();
    assert_eq!(result["sample_loaded_node_count"], 1);
    assert_eq!(result["sample_total_heat_load"], -2.0);
    assert_eq!(result["sample_peak_flux_magnitude"], 7.0);
    assert!(result.get("thermal_loaded_node_count").is_none());
    assert!(result.get("sample_peak_flux_x").is_none());
}

#[test]
fn thermo_catalog_counts_temperature_changes_and_accepts_solver_displacement_names() {
    let result = run_extract_operator(
        "extract.thermo_result_diagnostics",
        json!({
            "nodes":[
                {"id":"a","temperature_delta":0.0,"displacement_x":0.0,"displacement_y":0.0},
                {"id":"b","temperature_delta":-2.0,"displacement_x":3.0,"displacement_y":4.0},
                {"id":"c","temperature_delta":2.0,"ux":0.0,"uy":0.0}],
            "elements":[]
        }),
        json!({"output_prefix":"sample"}),
    )
    .unwrap();
    assert_eq!(result["sample_heated_node_count"], 2);
    assert_eq!(result["sample_peak_displacement"], 5.0);
    assert_eq!(result["sample_displacement_peak_magnitude"], 5.0);
    assert_eq!(result["sample_peak_displacement_id"], "b");
}

#[test]
fn absent_or_incomplete_evidence_does_not_create_zero_aliases() {
    for (id, field, alias) in [
        (
            "extract.thermal_result_diagnostics",
            "heat_load",
            "thermal_loaded_node_count",
        ),
        (
            "extract.thermo_result_diagnostics",
            "temperature_delta",
            "thermo_heated_node_count",
        ),
    ] {
        let payload = json!({"nodes":[],"elements":[]});
        assert!(run_extract_operator(id, payload, json!({})).is_err());
        for invalid in [json!({}), json!({field:Value::Null}), json!({field:"2"})] {
            let result = run_extract_operator(
                id,
                json!({"nodes":[{field:1.0},invalid],"elements":[]}),
                json!({}),
            );
            assert!(result.is_err(), "{alias} must not claim partial evidence");
        }
    }
    let result = run_extract_operator(
        "extract.thermal_result_diagnostics",
        json!({"nodes":[{"temperature":20.0}],"elements":[]}),
        json!({}),
    )
    .unwrap();
    assert!(result.get("thermal_loaded_node_count").is_none());
    assert!(result.get("thermal_peak_flux_magnitude").is_none());
}

#[test]
fn corrupt_selected_displacement_does_not_fall_back_to_an_alias() {
    for config in [json!({}), json!({"displacement_x_field":"ux"})] {
        let result = run_extract_operator(
            "extract.thermo_result_diagnostics",
            json!({
                "nodes":[{"ux":null,"uy":4.0,"displacement_x":3.0,"temperature_delta":1.0}],
                "elements":[]
            }),
            config,
        );
        assert!(result.unwrap_err().contains("payload.nodes[0].ux"));
    }
}
