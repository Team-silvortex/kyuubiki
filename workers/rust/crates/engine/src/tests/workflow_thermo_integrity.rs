use crate::workflow_executor::run_extract_operator;
use serde_json::{Value, json};

fn payload() -> Value {
    json!({"nodes":[
        {"id":"n0","temperature_delta":10.0,"ux":3.0,"uy":4.0},
        {"id":"n1","temperature_delta":20.0,"ux":5.0,"uy":12.0}],
        "elements":[{"id":"e0","von_mises":30.0},{"id":"e1","von_mises":50.0}]})
}

fn extract(payload: Value, config: Value) -> Result<Value, String> {
    run_extract_operator("extract.thermo_result_diagnostics", payload, config)
}

fn rejects(payload: Value, config: Value, path: &str) {
    let error = extract(payload, config).unwrap_err();
    assert!(error.contains(path), "expected {path}, got {error}");
}

#[test]
fn thermo_rejects_metadata_only_and_explicit_nonconvergence() {
    rejects(
        json!({"nodes":[],"elements":[]}),
        json!({}),
        "did not find any diagnostic fields",
    );
    for marker in [json!(false), Value::Null, json!("true")] {
        let mut input = payload();
        input["converged"] = marker;
        rejects(input, json!({}), "payload.converged");
    }
    let mut input = payload();
    input["stability_result"] = json!({"converged":false});
    rejects(input, json!({}), "payload.stability_result.converged");
}

#[test]
fn thermo_rejects_malformed_source_collections_and_records() {
    for source in ["nodes", "elements"] {
        for value in [Value::Null, json!({}), json!(false)] {
            let mut input = payload();
            input[source] = value.clone();
            rejects(input, json!({}), &format!("payload.{source}"));
            if !value.is_object() {
                let mut input = payload();
                input[source][1] = value;
                rejects(input, json!({}), &format!("payload.{source}[1]"));
            }
        }
    }
}

#[test]
fn temperature_statistics_require_every_selected_sample() {
    for index in [0, 1] {
        let mut input = payload();
        input["nodes"][index]
            .as_object_mut()
            .unwrap()
            .remove("temperature_delta");
        rejects(
            input,
            json!({}),
            &format!("payload.nodes[{index}].temperature_delta"),
        );
    }
    let mut input = payload();
    input["nodes"][1]["temperature_delta"] = json!("20");
    rejects(input, json!({}), "payload.nodes[1].temperature_delta");
}

#[test]
fn mixed_von_mises_aliases_participate_in_the_same_peak_reduction() {
    let mut input = payload();
    input["elements"][0] = json!({"id":"e0","von_mises_stress":30.0});
    let result = extract(input, json!({})).unwrap();
    assert_eq!(result["thermo_stress_peak"], 50.0);
    assert_eq!(result["thermo_peak_stress_id"], "e1");
}

#[test]
fn corrupt_stress_cannot_fall_back_to_alias_summary_or_components() {
    let mut input = payload();
    input["max_stress"] = json!(1.0);
    input["elements"][1]["von_mises_stress"] = Value::Null;
    input["elements"][1]["stress_x"] = json!(1.0);
    rejects(input, json!({}), "payload.elements[1].von_mises_stress");
}

#[test]
fn partial_stress_group_cannot_fall_back_to_a_healthy_global_summary() {
    let mut input = payload();
    input["max_stress"] = json!(1.0);
    input["elements"][1] = json!({"id":"e1","stress_x":1.0});
    rejects(input, json!({}), "payload.elements[1]");
}

#[test]
fn selected_global_stress_summary_must_be_valid() {
    for value in [Value::Null, json!("30"), json!(false)] {
        let mut input = payload();
        input["elements"] = json!([{"stress_x":1.0},{"stress_x":2.0}]);
        input["max_stress"] = value;
        rejects(input, json!({}), "payload.max_stress");
    }
}

#[test]
fn stress_component_reduction_requires_complete_selected_axes() {
    for field in ["stress_x", "stress_y", "stress_z", "stress_xy"] {
        let mut input = payload();
        input["elements"] = json!([{field:-20.0},{field:-30.0}]);
        input["elements"][1][field] = Value::Null;
        rejects(input, json!({}), &format!("payload.elements[1].{field}"));
        let mut input = payload();
        input["elements"] = json!([{field:-20.0},{}]);
        rejects(input, json!({}), "payload.elements[1]");
    }
}

#[test]
fn every_strain_group_rejects_invalid_or_partial_scalar_evidence() {
    for field in ["thermal_strain", "mechanical_strain", "total_strain"] {
        for value in [Value::Null, json!("0.1")] {
            let mut input = payload();
            input["elements"][0][field] = json!(0.1);
            input["elements"][1][field] = value;
            input["elements"][1][format!("{field}_x")] = json!(0.1);
            rejects(input, json!({}), &format!("payload.elements[1].{field}"));
        }
        let mut input = payload();
        input["elements"][0][field] = json!(0.1);
        input["elements"][1][format!("{field}_x")] = json!(0.1);
        rejects(input, json!({}), &format!("payload.elements[1].{field}"));
    }
}

#[test]
fn every_strain_component_axis_is_validated_without_skipping_rows() {
    for group in ["thermal_strain", "mechanical_strain", "total_strain"] {
        for axis in ["x", "y", "z", "xy"] {
            let field = format!("{group}_{axis}");
            let mut input = payload();
            input["elements"][0][&field] = json!(-0.1);
            input["elements"][1][&field] = Value::Null;
            rejects(input, json!({}), &format!("payload.elements[1].{field}"));
        }
    }
}

#[test]
fn displacement_diagnostics_reject_partial_vectors_and_bad_magnitudes() {
    let mut input = payload();
    input["nodes"][1].as_object_mut().unwrap().remove("uy");
    rejects(input, json!({}), "payload.nodes[1].uy");
    for value in [Value::Null, json!(-1.0)] {
        let mut input = payload();
        input["nodes"][1]["displacement_magnitude"] = value;
        rejects(input, json!({}), "payload.nodes[1].displacement_magnitude");
    }
}

#[test]
fn thermo_invalid_configuration_cannot_silently_select_defaults() {
    for config in [json!([]), json!(false)] {
        rejects(payload(), config, "config");
    }
    for key in [
        "temperature_delta_field",
        "temperature_field",
        "stress_field",
        "thermal_strain_field",
        "mechanical_strain_field",
        "total_strain_field",
        "displacement_z_field",
        "output_prefix",
    ] {
        for invalid in [Value::Null, json!(false), json!(" ")] {
            rejects(payload(), json!({key:invalid}), &format!("config.{key}"));
        }
    }
}

#[test]
fn explicit_missing_stress_and_strain_fields_do_not_use_fallbacks() {
    for key in [
        "stress_field",
        "thermal_strain_field",
        "mechanical_strain_field",
        "total_strain_field",
    ] {
        let mut input = payload();
        input["max_stress"] = json!(1.0);
        rejects(
            input,
            json!({key:"selected"}),
            "payload.elements[0].selected",
        );
    }
}

#[test]
fn thermo_rejects_unrepresentable_statistics_and_displacements() {
    let mut input = payload();
    input["nodes"][0]["temperature_delta"] = json!(1e308);
    input["nodes"][1]["temperature_delta"] = json!(1e308);
    rejects(input, json!({}), "thermo_temperature_delta_sum");
    let mut input = payload();
    input["nodes"][0]["temperature_delta"] = json!(-1e308);
    input["nodes"][1]["temperature_delta"] = json!(1e308);
    rejects(input, json!({}), "thermo_temperature_delta_span");
    let mut input = payload();
    input["nodes"][1]["ux"] = json!(f64::MAX);
    input["nodes"][1]["uy"] = json!(f64::MAX);
    rejects(input, json!({}), "payload.nodes[1]");
}

#[test]
fn thermo_preserves_representable_displacement_norms_and_explicit_z() {
    for scale in [1e200, 1e-200] {
        let mut input = payload();
        for index in [0, 1] {
            input["nodes"][index]["ux"] = json!(3.0 * scale);
            input["nodes"][index]["uy"] = json!(4.0 * scale);
        }
        let result = extract(input, json!({})).unwrap();
        let norm = result["thermo_displacement_peak_magnitude"]
            .as_f64()
            .unwrap();
        assert!((norm / (5.0 * scale) - 1.0).abs() < 1e-14);
    }
    rejects(
        payload(),
        json!({"displacement_z_field":"uz"}),
        "payload.nodes[0].uz",
    );
    let mut input = payload();
    for index in [0, 1] {
        input["nodes"][index]["uz"] = json!(84.0);
    }
    assert_eq!(
        extract(input, json!({"displacement_z_field":"uz"})).unwrap()["thermo_displacement_peak_magnitude"],
        85.0
    );
}

#[test]
fn signed_component_peaks_keep_axis_order_and_last_record_ties() {
    let mut input = payload();
    input["elements"] = json!([
        {"id":"first","stress_x":-10.0,"stress_y":10.0,"mechanical_strain_x":-0.2},
        {"id":"last","stress_x":10.0,"stress_y":-10.0,"mechanical_strain_x":-0.2}]);
    let result = extract(input, json!({})).unwrap();
    assert_eq!(result["thermo_stress_peak"], -10.0);
    assert_eq!(result["thermo_peak_stress_id"], "last");
    assert_eq!(result["thermo_peak_mechanical_strain"], -0.2);
    assert_eq!(result["thermo_peak_mechanical_strain_id"], "last");
}

#[test]
fn authoritative_scalar_evidence_does_not_reinterpret_unused_fallback_data() {
    let mut input = payload();
    input["max_stress"] = Value::Null;
    input["elements"][0]["stress_x"] = Value::Null;
    let result = extract(input, Value::Null).unwrap();
    assert_eq!(result["thermo_stress_peak"], 50.0);
    let result = extract(
        json!({"nodes":[],"elements":[],"max_stress":12.0}),
        json!({}),
    )
    .unwrap();
    assert_eq!(result["thermo_stress_peak"], 12.0);
    assert_eq!(result["thermo_peak_stress_id"], "max_stress");
    assert!(result.get("thermo_thermal_strain_peak").is_none());
}

#[test]
fn thermo_custom_mapping_prefix_and_legacy_temperature_key_work() {
    let result = extract(json!({"points":[{"dt":-10.0},{"dt":20.0}],"cells":[{"s":-2.0},{"s":-1.0}]}),
        json!({"node_source":"points","element_source":"cells","temperature_field":"dt","stress_field":"s","output_prefix":"test result"})).unwrap();
    assert_eq!(result["diagnostic_domain"], "thermo_mechanical");
    assert_eq!(result["diagnostic_subject"], "thermo_result");
    assert_eq!(result["test_result_temperature_delta_sum"], 10.0);
    assert_eq!(result["test_result_stress_peak"], -1.0);
    assert_eq!(result["test_result_peak_stress_id"], "unknown");
}

#[test]
fn thermo_selected_temperature_mapping_and_explicit_null_identity_keep_precedence() {
    let mut input = payload();
    input["elements"][1]["id"] = Value::Null;
    let result = extract(
        input.clone(),
        json!({"temperature_delta_field":"temperature_delta","temperature_field":"unused"}),
    )
    .unwrap();
    assert_eq!(result["thermo_temperature_delta_max"], 20.0);
    assert_eq!(result["thermo_peak_stress_id"], Value::Null);
    assert_eq!(result["thermo_stress_peak_element_id"], Value::Null);
    rejects(
        input,
        json!({"temperature_delta_field":false,"temperature_field":"temperature_delta"}),
        "config.temperature_delta_field",
    );
}
