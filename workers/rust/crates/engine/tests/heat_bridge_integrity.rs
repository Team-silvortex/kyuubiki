#[path = "support/heat_bridge.rs"]
mod bridge;
use serde_json::{Value, json};

#[test]
fn reductions_cover_unequal_areas_negative_scale_and_temperature_origins() {
    for shape in ["triangle", "quad"] {
        let (heat, seed) = bridge::fixture(shape);
        for reference in [20.0, 293.15] {
            for scale in [-2.0, 0.0, 2.0] {
                for reduction in ["copy", "mean", "sum", "area_weighted_mean", "min", "max"] {
                    let mut source = heat.clone();
                    source["elements"][0]["average_temperature"] = json!(reference + 10.0);
                    source["elements"][1]["average_temperature"] = json!(reference + 30.0);
                    let mut config = bridge::config(shape, &seed, reduction);
                    config["contract"]["transform"]["reference_temperature"] = json!(reference);
                    config["contract"]["transform"]["scale"] = json!(scale);
                    let result = bridge::run(shape, source, config).unwrap();
                    let expected = match reduction {
                        "copy" | "mean" => 20.0 * scale,
                        "sum" => 40.0 * scale,
                        "area_weighted_mean" => 25.0 * scale,
                        "min" => (10.0_f64 * scale).min(30.0 * scale),
                        "max" => (10.0_f64 * scale).max(30.0 * scale),
                        _ => unreachable!(),
                    };
                    let actual = result["nodes"][1]["temperature_delta"].as_f64().unwrap();
                    assert!(
                        (actual - expected).abs() < 1e-10,
                        "{shape} {reduction}: {actual} != {expected}"
                    );
                }
            }
        }
    }
}

#[test]
fn default_element_mapping_uses_the_actual_element_shape() {
    for shape in ["triangle", "quad"] {
        let (heat, seed) = bridge::fixture(shape);
        let mut config = bridge::config(shape, &seed, "mean");
        config["contract"]["source"]
            .as_object_mut()
            .unwrap()
            .remove("node_index_fields");
        let result = bridge::run(shape, heat, config).unwrap();
        assert_eq!(result["nodes"][1]["temperature_delta"], 40.0);
    }
}

#[test]
fn malformed_contract_values_do_not_silently_change_the_physics() {
    let (heat, seed) = bridge::fixture("quad");
    for (path, bad) in [
        ("/contract/transform/scale", json!("2")),
        ("/contract/transform/scale", Value::Null),
        ("/contract/transform/default_value", json!(false)),
        ("/contract/transform/reduction", json!(42)),
        ("/contract/source/node_index_fields", json!(["node_i", 3])),
        (
            "/contract/source/node_index_fields",
            json!(["node_i", "node_i"]),
        ),
        ("/contract/source/node_index_fields", json!([])),
    ] {
        let mut config = bridge::config("quad", &seed, "sum");
        *config.pointer_mut(path).unwrap() = bad;
        assert!(
            bridge::run("quad", heat.clone(), config).is_err(),
            "accepted {path}"
        );
    }
}

#[test]
fn incomplete_result_elements_and_invalid_indexes_are_rejected() {
    for shape in ["triangle", "quad"] {
        let (heat, seed) = bridge::fixture(shape);
        for bad in [json!(-1), json!(1000), json!("1"), Value::Null] {
            let mut source = heat.clone();
            source["elements"][0]["node_j"] = bad;
            assert!(bridge::run(shape, source, bridge::config(shape, &seed, "mean")).is_err());
        }
        let mut truncated = heat.clone();
        truncated["elements"].as_array_mut().unwrap().pop();
        assert!(
            bridge::run(shape, truncated, bridge::config(shape, &seed, "mean")).is_err(),
            "truncated {shape}"
        );
        let mut mismatched = heat;
        mismatched["elements"][0]["node_j"] = json!(0);
        assert!(
            bridge::run(shape, mismatched, bridge::config(shape, &seed, "mean")).is_err(),
            "connectivity drift {shape}"
        );
    }
}

#[test]
fn overflowing_reductions_never_produce_a_successful_null_temperature() {
    for reduction in ["sum", "mean", "area_weighted_mean"] {
        let (mut heat, seed) = bridge::fixture("quad");
        for element in heat["elements"].as_array_mut().unwrap() {
            element["average_temperature"] = json!(1e308);
        }
        let mut config = bridge::config("quad", &seed, reduction);
        config["contract"]["transform"]["scale"] = json!(1.0);
        assert!(
            bridge::run("quad", heat, config).is_err(),
            "overflow accepted: {reduction}"
        );
    }
    let (heat, seed) = bridge::fixture("quad");
    assert!(bridge::run("quad", heat, bridge::config("quad", &seed, "mean")).is_ok());
}

#[test]
fn native_typed_nonfinite_values_are_rejected_not_defaulted() {
    let (heat, seed) = bridge::fixture("quad");
    let mut heat: kyuubiki_protocol::SolveHeatPlaneQuad2dResult =
        serde_json::from_value(heat).unwrap();
    let seed = serde_json::from_value(seed).unwrap();
    heat.nodes[0].temperature = f64::NAN;
    assert!(kyuubiki_engine::bridge_heat_result_to_thermal_plane_quad_model(&heat, &seed).is_err());
    heat.nodes[0].temperature = 30.0;
    heat.nodes[0].x = f64::NAN;
    assert!(kyuubiki_engine::bridge_heat_result_to_thermal_plane_quad_model(&heat, &seed).is_err());
}
