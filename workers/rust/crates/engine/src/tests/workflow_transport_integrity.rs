use crate::workflow_executor::{run_extract_operator, run_transform_operator};
use serde_json::{Value, json};

fn payload() -> Value {
    json!({"nodes":[{"id":"n0","concentration":0.0,"source":0.5},
        {"id":"n1","concentration":1.0,"source":-0.25}],"elements":[
        {"id":"e0","total_flux":-1.0,"diffusive_flux":-0.25,"advective_flux":-0.75,"peclet_number":1.0},
        {"id":"e1","total_flux":2.0,"diffusive_flux":0.5,"advective_flux":1.5,"peclet_number":2.0}]})
}

fn extract(payload: Value, config: Value) -> Result<Value, String> {
    run_extract_operator("extract.transport_result_diagnostics", payload, config)
}

fn rejects(payload: Value, config: Value, path: &str) {
    let error = extract(payload, config).unwrap_err();
    assert!(error.contains(path), "expected {path}, got {error}");
}

#[test]
fn transport_metadata_only_or_failed_results_cannot_be_assessed() {
    for input in [
        json!({"nodes":[],"elements":[]}),
        json!({"nodes":[{}],"elements":[{}]}),
    ] {
        rejects(input, json!({}), "did not find any diagnostic fields");
    }
    for marker in [Value::Null, json!(false), json!("true")] {
        let mut input = payload();
        input["converged"] = marker;
        rejects(input, json!({}), "payload.converged");
    }
}

#[test]
fn missing_sources_remain_unavailable_instead_of_becoming_zero() {
    let mut input = payload();
    for node in input["nodes"].as_array_mut().unwrap() {
        node.as_object_mut().unwrap().remove("source");
    }
    let diagnostic = extract(input, json!({})).unwrap();
    for field in [
        "transport_source_sum",
        "transport_source_mean",
        "transport_source_count",
    ] {
        assert!(diagnostic.get(field).is_none(), "{field}");
    }
    let quality =
        run_transform_operator("transform.score_transport_quality", diagnostic, json!({})).unwrap();
    assert_eq!(quality["transport_quality_ready"], false);
    assert_eq!(quality["transport_quality_missing_metric_count"], 1);
}

#[test]
fn absent_concentration_is_not_an_invented_zero_mean() {
    let result = extract(
        json!({"nodes":[],"elements":[{"total_flux":-1.0}]}),
        json!({}),
    )
    .unwrap();
    assert!(result.get("transport_concentration_mean").is_none());
    assert!(result.get("transport_source_sum").is_none());
    assert_eq!(result["transport_total_flux_peak"], -1.0);
}

#[test]
fn transport_rejects_nonobjects_and_partial_node_metrics() {
    for source in ["nodes", "elements"] {
        let mut input = payload();
        input[source][1] = Value::Null;
        rejects(input, json!({}), &format!("payload.{source}[1]"));
    }
    for field in ["concentration", "source"] {
        let mut input = payload();
        input["nodes"][1].as_object_mut().unwrap().remove(field);
        rejects(input, json!({}), &format!("payload.nodes[1].{field}"));
    }
}

#[test]
fn corrupt_node_aliases_cannot_fall_through_to_healthy_data() {
    for (field, alias) in [("concentration", "c"), ("source", "source_density")] {
        for bad in [Value::Null, json!("1"), json!(false)] {
            let mut input = payload();
            input["nodes"][1][field] = bad;
            input["nodes"][1][alias] = json!(0.1);
            rejects(input, json!({}), &format!("payload.nodes[1].{field}"));
        }
    }
}

#[test]
fn every_flux_and_peclet_group_requires_complete_valid_samples() {
    for (field, alias) in [
        ("total_flux", "flux"),
        ("diffusive_flux", "diffusion_flux"),
        ("advective_flux", "advection_flux"),
        ("peclet_number", "pe"),
    ] {
        let mut input = payload();
        input["elements"][1][field] = Value::Null;
        input["elements"][1][alias] = json!(1.0);
        rejects(input, json!({}), &format!("payload.elements[1].{field}"));
        let mut input = payload();
        input["elements"][1].as_object_mut().unwrap().remove(field);
        rejects(input, json!({}), &format!("payload.elements[1].{field}"));
    }
}

#[test]
fn vector_flux_fallback_requires_both_finite_components() {
    for (field, x, y) in [
        ("total_flux", "flux_x", "flux_y"),
        ("diffusive_flux", "diffusive_flux_x", "diffusive_flux_y"),
        ("advective_flux", "advective_flux_x", "advective_flux_y"),
    ] {
        let mut input = payload();
        input["elements"][1].as_object_mut().unwrap().remove(field);
        input["elements"][1][x] = json!(3.0);
        rejects(
            input.clone(),
            json!({}),
            &format!("payload.elements[1].{y}"),
        );
        input["elements"][1][y] = Value::Null;
        rejects(input, json!({}), &format!("payload.elements[1].{y}"));
    }
}

#[test]
fn flux_hypot_preserves_representable_large_and_small_values() {
    for scale in [1e200, 1e-200] {
        let result = extract(
            json!({"nodes":[],"elements":[{"flux_x":3.0*scale,"flux_y":4.0*scale}]}),
            json!({}),
        )
        .unwrap();
        let norm = result["transport_total_flux_peak_magnitude"]
            .as_f64()
            .unwrap();
        assert!((norm / (5.0 * scale) - 1.0).abs() < 1e-14);
    }
    rejects(
        json!({"nodes":[],"elements":[{"flux_x":f64::MAX,"flux_y":f64::MAX}]}),
        json!({}),
        "payload.elements[0]",
    );
}

#[test]
fn required_transport_totals_and_spans_must_remain_finite() {
    let mut input = payload();
    input["nodes"][0]["source"] = json!(1e308);
    input["nodes"][1]["source"] = json!(1e308);
    rejects(input, json!({}), "transport_source_sum");
    let mut input = payload();
    input["nodes"][0]["concentration"] = json!(-1e308);
    input["nodes"][1]["concentration"] = json!(1e308);
    rejects(input, json!({}), "transport_concentration_span");
}

#[test]
fn concentration_mean_does_not_require_an_unused_representable_sum() {
    let mut input = payload();
    input["nodes"][0]["concentration"] = json!(1e308);
    input["nodes"][1]["concentration"] = json!(1e308);
    let result = extract(input, json!({})).unwrap();
    assert_eq!(result["transport_concentration_mean"], 1e308);
    assert_eq!(result["transport_concentration_span"], 0.0);
}

#[test]
fn transport_invalid_prefix_or_configuration_is_not_silently_defaulted() {
    for config in [
        json!([]),
        json!(false),
        json!({"output_prefix":false}),
        json!({"output_prefix":" "}),
    ] {
        rejects(payload(), config, "config");
    }
}

#[test]
fn signed_flux_peaks_and_last_record_ties_are_preserved() {
    let mut input = payload();
    input["elements"][0]["total_flux"] = json!(5.0);
    input["elements"][1]["total_flux"] = json!(-5.0);
    let result = extract(input, json!({"output_prefix":"species-A"})).unwrap();
    assert_eq!(result["diagnostic_prefix"], "species-A");
    assert_eq!(result["diagnostic_subject"], "advection_diffusion_result");
    assert_eq!(result["species-A_total_flux_peak"], -5.0);
    assert_eq!(result["species-A_total_flux_peak_magnitude"], 5.0);
    assert_eq!(result["species-A_total_flux_peak_id"], "e1");
}

#[test]
fn valid_signed_scalar_flux_keeps_precedence_over_unused_vectors() {
    let mut input = payload();
    input["elements"][1]["flux_x"] = Value::Null;
    input["elements"][1]["flux_y"] = Value::Null;
    let result = extract(input, Value::Null).unwrap();
    assert_eq!(result["transport_total_flux_peak"], 2.0);
}

#[test]
fn explicit_zero_source_is_valid_evidence_not_absence() {
    let mut input = payload();
    for row in input["nodes"].as_array_mut().unwrap() {
        row["source"] = json!(0.0);
    }
    let diagnostic = extract(input, json!({})).unwrap();
    assert_eq!(diagnostic["transport_source_sum"], 0.0);
    assert_eq!(diagnostic["transport_source_count"], 2);
    let quality =
        run_transform_operator("transform.score_transport_quality", diagnostic, json!({})).unwrap();
    assert_eq!(quality["transport_quality_missing_metric_count"], 0);
    assert_eq!(quality["transport_quality_ready"], true);
}

#[test]
fn late_corruption_is_not_hidden_by_healthy_transport_samples() {
    let mut input = payload();
    input["elements"] = json!(vec![json!({"total_flux":1.0}); 4096]);
    input["elements"][4095]["total_flux"] = Value::Null;
    rejects(input, json!({}), "payload.elements[4095].total_flux");
}

#[test]
fn mixed_transport_aliases_match_independent_reductions_across_sample_counts() {
    for count in [1, 2, 3, 64, 257] {
        let values = (0..count)
            .map(|i| i as f64 / count as f64 - 0.5)
            .collect::<Vec<_>>();
        let nodes = values
            .iter()
            .enumerate()
            .map(|(i, value)| {
                let c = ["concentration", "c", "species", "scalar"][i % 4];
                let s = ["source", "source_density", "net_source", "source_term"][i % 4];
                json!({c:value,s:0.25})
            })
            .collect::<Vec<_>>();
        let elements = (0..count)
            .map(|i| {
                let flux = ["total_flux", "flux", "flux_total"][i % 3];
                let pe = ["peclet_number", "peclet", "pe"][i % 3];
                json!({"id":i,flux:-(i as f64+1.0),pe:i as f64})
            })
            .collect::<Vec<_>>();
        let result = extract(json!({"nodes":nodes,"elements":elements}), json!({})).unwrap();
        assert_eq!(result["transport_concentration_min"], values[0]);
        assert_eq!(result["transport_concentration_max"], values[count - 1]);
        let mean = values.iter().sum::<f64>() / count as f64;
        assert!((result["transport_concentration_mean"].as_f64().unwrap() - mean).abs() < 1e-14);
        assert_eq!(result["transport_source_sum"], 0.25 * count as f64);
        assert_eq!(result["transport_source_mean"], 0.25);
        assert_eq!(result["transport_total_flux_peak"], -(count as f64));
        assert_eq!(result["transport_total_flux_peak_id"], count - 1);
        assert_eq!(result["transport_peclet_peak"], (count - 1) as f64);
    }
}
