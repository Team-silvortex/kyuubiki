use crate::workflow_executor::{run_extract_operator, run_transform_operator};
use serde_json::{Value, json};

fn payload() -> Value {
    json!({"input":{"scheme":"upwind"},
    "nodes":[{"concentration":0.0,"source":0.0},{"concentration":0.25,"source":4.0},
             {"concentration":1.0,"source":0.0}],
    "elements":[
        {"id":"e0","total_flux":2.0,"diffusive_flux":-0.5,"advective_flux":2.5,"peclet_number":5.0,
         "stabilization":{"artificial_diffusivity":5.0,"stabilization_flux":-2.5,"numerical_flux":-0.5}},
        {"id":"e1","total_flux":11.0,"diffusive_flux":-1.5,"advective_flux":12.5,"peclet_number":5.0,
         "stabilization":{"artificial_diffusivity":5.0,"stabilization_flux":-7.5,"numerical_flux":3.5}}
    ]})
}

fn extract(input: Value, config: Value) -> Result<Value, String> {
    run_extract_operator("extract.transport_result_diagnostics", input, config)
}

fn quality(input: Value, config: Value) -> Result<Value, String> {
    run_transform_operator("transform.score_transport_quality", input, config)
}

fn rejects(input: Value, path: &str) {
    let error = extract(input, Value::Null).unwrap_err();
    assert!(error.contains(path), "{path}: {error}");
}

#[test]
fn upwind_diagnostics_preserve_distinct_peaks_and_custom_prefixes() {
    let output = extract(payload(), json!({"output_prefix":"species-A"})).unwrap();
    assert_eq!(output["diagnostic_scheme"], "upwind");
    assert_eq!(output["species-A_total_flux_peak"], 11.0);
    assert_eq!(output["species-A_artificial_diffusivity_peak"], 5.0);
    assert_eq!(output["species-A_artificial_diffusivity_peak_id"], "e1");
    assert_eq!(output["species-A_stabilization_flux_peak"], -7.5);
    assert_eq!(output["species-A_stabilization_flux_peak_magnitude"], 7.5);
    assert_eq!(output["species-A_numerical_flux_peak"], 3.5);
    assert_eq!(output["species-A_numerical_flux_peak_magnitude"], 3.5);
    assert!(
        output["diagnostic_metric_groups"]
            .as_array()
            .unwrap()
            .contains(&json!("stabilization"))
    );
    let mut tied = payload();
    tied["elements"][0]["total_flux"] = json!(4.0);
    tied["elements"][0]["stabilization"]["numerical_flux"] = json!(1.5);
    tied["elements"][1]["total_flux"] = json!(-0.5);
    tied["elements"][1]["stabilization"]["stabilization_flux"] = json!(-1.0);
    tied["elements"][1]["stabilization"]["numerical_flux"] = json!(-1.5);
    let output = extract(tied, Value::Null).unwrap();
    assert_eq!(output["transport_numerical_flux_peak"], -1.5);
    assert_eq!(output["transport_numerical_flux_peak_id"], "e1");
}

#[test]
fn upwind_diagnostics_reject_missing_partial_and_wrong_typed_stabilization() {
    for field in [
        "artificial_diffusivity",
        "stabilization_flux",
        "numerical_flux",
    ] {
        let mut input = payload();
        input["elements"][1]["stabilization"]
            .as_object_mut()
            .unwrap()
            .remove(field);
        rejects(input, &format!("payload.elements[1].stabilization.{field}"));
        for bad in [Value::Null, json!(true), json!("1"), json!({}), json!([])] {
            let mut input = payload();
            input["elements"][1]["stabilization"][field] = bad;
            rejects(input, &format!("payload.elements[1].stabilization.{field}"));
        }
    }
    let mut input = payload();
    input["elements"][1]
        .as_object_mut()
        .unwrap()
        .remove("stabilization");
    rejects(input, "payload.elements[1].stabilization");
    for bad in [Value::Null, json!(false), json!([])] {
        let mut input = payload();
        input["elements"][1]["stabilization"] = bad;
        rejects(input, "payload.elements[1].stabilization");
    }
    let mut input = payload();
    input["elements"][1]["stabilization"]["artificial_diffusivity"] = json!(-1.0);
    rejects(
        input,
        "payload.elements[1].stabilization.artificial_diffusivity",
    );
}

#[test]
fn upwind_scheme_requires_diagnostics_and_cannot_be_inferred_or_contradicted() {
    let mut input = payload();
    for e in input["elements"].as_array_mut().unwrap() {
        e.as_object_mut().unwrap().remove("stabilization");
    }
    rejects(input, "payload.elements[0].stabilization");
    let mut input = payload();
    input["elements"] = json!([]);
    rejects(input, "stabilization");
    for scheme in [Value::Null, json!("auto"), json!(true), json!("galerkin")] {
        let mut input = payload();
        input["input"]["scheme"] = scheme;
        rejects(input, "payload.input.scheme");
    }
    let mut input = payload();
    input.as_object_mut().unwrap().remove("input");
    rejects(input, "payload.input.scheme");
}

#[test]
fn stabilized_numerical_flux_must_agree_with_physical_plus_artificial_flux() {
    let mut input = payload();
    input["elements"][1]["stabilization"]["numerical_flux"] = json!(11.0);
    rejects(input, "payload.elements[1].stabilization.numerical_flux");
    for bad in [Value::Null, json!("11")] {
        let mut input = payload();
        input["elements"][1]["total_flux"] = bad;
        rejects(input, "payload.elements[1].total_flux");
    }
    let mut input = payload();
    input["elements"][1]
        .as_object_mut()
        .unwrap()
        .remove("total_flux");
    rejects(input, "payload.elements[1].total_flux");
}

#[test]
fn stabilization_identity_is_scaled_without_unused_overflow_or_absolute_floor() {
    for scale in [1e308, 1e-300] {
        let mut input = payload();
        for e in input["elements"].as_array_mut().unwrap() {
            e["total_flux"] = json!(scale);
            e["stabilization"]["stabilization_flux"] = json!(-scale);
            e["stabilization"]["numerical_flux"] = json!(0.0);
        }
        let output = extract(input.clone(), Value::Null).unwrap();
        assert_eq!(output["transport_numerical_flux_peak"], 0.0);
        input["elements"][1]["stabilization"]["numerical_flux"] = json!(scale * 0.1);
        rejects(input, "payload.elements[1].stabilization.numerical_flux");
    }
    let mut input = payload();
    input["elements"][1]["total_flux"] = json!(f64::MAX);
    input["elements"][1]["stabilization"]["stabilization_flux"] = json!(f64::MAX);
    input["elements"][1]["stabilization"]["numerical_flux"] = json!(f64::MAX);
    rejects(input, "payload.elements[1].stabilization.numerical_flux");
}

#[test]
fn stabilized_metrics_are_opt_in_without_changing_default_quality() {
    let diagnostic = extract(payload(), Value::Null).unwrap();
    let defaults = quality(diagnostic.clone(), Value::Null).unwrap();
    assert_eq!(defaults["transport_quality_term_count"], 4);
    assert!((defaults["transport_quality_score"].as_f64().unwrap() - 26.05).abs() < 1e-12);
    let scored = quality(diagnostic, json!({
        "enabled_terms":["transport_artificial_diffusivity_peak", "transport_stabilization_flux_peak_magnitude", "transport_numerical_flux_peak_magnitude"],
        "targets":{"transport_artificial_diffusivity_peak":5.0,"transport_stabilization_flux_peak_magnitude":7.5,"transport_numerical_flux_peak_magnitude":3.5},
        "weights":{"transport_artificial_diffusivity_peak":1.0,"transport_stabilization_flux_peak_magnitude":1.0,"transport_numerical_flux_peak_magnitude":1.0}
    })).unwrap();
    assert_eq!(scored["transport_quality_term_count"], 3);
    assert_eq!(scored["transport_quality_missing_metric_count"], 0);
    assert_eq!(scored["transport_quality_score"], 3.0);
    assert_eq!(scored["transport_quality_ready"], true);
}

#[test]
fn selected_stabilization_metrics_never_default_missing_or_corrupt_data_to_zero() {
    for field in [
        "transport_artificial_diffusivity_peak",
        "transport_stabilization_flux_peak_magnitude",
        "transport_numerical_flux_peak_magnitude",
    ] {
        let config = json!({"enabled_terms":[field]});
        let missing = quality(json!({}), config.clone()).unwrap();
        assert_eq!(missing["transport_quality_ready"], false);
        assert_eq!(missing["transport_quality_missing_metric_count"], 1);
        for bad in [Value::Null, json!(false), json!("0")] {
            assert!(
                quality(json!({field:bad}), config.clone())
                    .unwrap_err()
                    .contains(field)
            );
        }
        let zero = quality(json!({field:0.0}), config).unwrap();
        assert_eq!(zero["transport_quality_ready"], true);
        assert_eq!(zero["transport_quality_missing_metric_count"], 0);
    }
}

#[test]
fn late_stabilization_corruption_cannot_hide_behind_healthy_peaks() {
    let mut input = payload();
    input["elements"] = json!(vec![input["elements"][0].clone(); 4096]);
    input["elements"][4095]["stabilization"]["numerical_flux"] = Value::Null;
    rejects(input, "payload.elements[4095].stabilization.numerical_flux");
}
