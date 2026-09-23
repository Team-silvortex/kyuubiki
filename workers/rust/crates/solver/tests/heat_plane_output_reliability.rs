use kyuubiki_solver::{solve_heat_plane_quad_2d, solve_heat_plane_triangle_2d};
use serde_json::{Value, json};

fn model(quad: bool, conductivity: f64) -> Value {
    let points = if quad {
        vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]]
    } else {
        vec![[0.0, 0.0], [1.0, 0.0], [0.0, 1.0]]
    };
    let nodes: Vec<_> = points
        .iter()
        .enumerate()
        .map(|(index, [x, y])| {
            json!({
                "id": format!("n{index}"), "x": x, "y": y,
                "fix_temperature": true, "temperature": 20.0 * x - 10.0 * y
            })
        })
        .collect();
    let mut element = json!({
        "id": "bulk", "node_i": 0, "node_j": 1, "node_k": 2,
        "conductivity": conductivity, "thickness": 1.0
    });
    if quad {
        element["node_l"] = json!(3);
    }
    json!({"nodes": nodes, "elements": [element]})
}

fn solve(quad: bool, input: Value) -> Result<Value, String> {
    if quad {
        solve_heat_plane_quad_2d(&serde_json::from_value(input).unwrap())
            .map(|result| serde_json::to_value(result).unwrap())
    } else {
        solve_heat_plane_triangle_2d(&serde_json::from_value(input).unwrap())
            .map(|result| serde_json::to_value(result).unwrap())
    }
}

fn close(value: &Value, expected: f64) {
    let actual = value.as_f64().expect("a successful result must be numeric");
    if expected == 0.0 {
        assert_eq!(actual, 0.0);
    } else {
        assert!(
            (actual / expected - 1.0).abs() < 1e-12,
            "{actual:e} != {expected:e}"
        );
    }
}

#[test]
fn representable_heat_flux_survives_square_overflow_and_underflow() {
    for quad in [false, true] {
        for conductivity in [1e-200, 1.0, 1e200] {
            let result = solve(quad, model(quad, conductivity)).unwrap();
            let element = &result["elements"][0];
            let magnitude = 20.0_f64.hypot(10.0) * conductivity;
            close(&element["heat_flux_x"], -20.0 * conductivity);
            close(&element["heat_flux_y"], 10.0 * conductivity);
            close(&element["heat_flux_magnitude"], magnitude);
            close(
                &element["heat_flow_rate"],
                magnitude * if quad { 1.0 } else { 0.5 },
            );
            close(&result["max_heat_flux"], magnitude);
        }
    }
}

#[test]
fn representable_temperature_mean_does_not_overflow_its_intermediate_sum() {
    for quad in [false, true] {
        let mut input = model(quad, 1e-308);
        for (index, node) in input["nodes"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .enumerate()
        {
            node["temperature"] = json!(if index == 1 || index == 2 {
                1.2e308
            } else {
                0.0
            });
        }
        let result = solve(quad, input).unwrap();
        close(
            &result["elements"][0]["average_temperature"],
            if quad { 6e307 } else { 8e307 },
        );
    }
}

#[test]
fn large_area_and_small_thickness_do_not_overflow_a_representable_flow_rate() {
    for quad in [false, true] {
        let mut input = model(quad, 1e200);
        for node in input["nodes"].as_array_mut().unwrap() {
            for key in ["x", "y", "temperature"] {
                node[key] = json!(node[key].as_f64().unwrap() * 1e100);
            }
        }
        input["elements"][0]["thickness"] = json!(1e-200);
        let result = solve(quad, input).unwrap();
        close(
            &result["elements"][0]["heat_flow_rate"],
            20.0_f64.hypot(10.0) * 1e200 * if quad { 1.0 } else { 0.5 },
        );
    }
}

#[test]
fn quad_gradient_weighting_does_not_overflow_before_dividing_by_area() {
    let mut input = model(true, 1e-308);
    for (node, temperature) in input["nodes"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .zip([0.0, 1e308, 0.0, -1e308])
    {
        node["x"] = json!(4.0 * node["x"].as_f64().unwrap());
        node["y"] = json!(4.0 * node["y"].as_f64().unwrap());
        node["temperature"] = json!(temperature);
    }
    let result = solve(true, input).unwrap();
    let element = &result["elements"][0];
    close(&element["temperature_gradient_x"], 2.5e307);
    close(&element["temperature_gradient_y"], -2.5e307);
    close(&element["heat_flux_magnitude"], 0.25_f64.hypot(0.25));
    close(&element["average_temperature"], 0.0);
}

#[test]
fn unrepresentable_flux_is_an_error_not_successful_null_or_zero_output() {
    for quad in [false, true] {
        for overflow in [false, true] {
            let mut input = model(quad, if overflow { 1e308 } else { 1e-200 });
            if overflow {
                input["elements"][0]["thickness"] = json!(0.01);
            } else {
                for node in input["nodes"].as_array_mut().unwrap() {
                    node["temperature"] = json!(node["temperature"].as_f64().unwrap() * 1e-200);
                }
            }
            let error = solve(quad, input).unwrap_err();
            assert!(
                error.contains("heat plane") && error.contains("heat flux"),
                "{error}"
            );
            assert!(
                error.contains("bulk"),
                "identify the failed element: {error}"
            );
            assert!(solve(quad, model(quad, 1.0)).is_ok());
        }
    }
}

#[test]
fn finite_element_flows_cannot_overflow_the_total_silently() {
    for quad in [false, true] {
        let single = model(quad, 5e306);
        let count = single["nodes"].as_array().unwrap().len();
        let mut input = json!({"nodes": [], "elements": []});
        for body in 0..4 {
            for node in single["nodes"].as_array().unwrap() {
                let mut node = node.clone();
                node["id"] = json!(format!("body-{body}-{}", node["id"].as_str().unwrap()));
                node["y"] = json!(node["y"].as_f64().unwrap() + 2.0 * body as f64);
                input["nodes"].as_array_mut().unwrap().push(node);
            }
            let mut element = single["elements"][0].clone();
            element["id"] = json!(format!("body-{body}"));
            for key in ["node_i", "node_j", "node_k", "node_l"] {
                if let Some(index) = element[key].as_u64() {
                    element[key] = json!(index as usize + body * count);
                }
            }
            input["elements"].as_array_mut().unwrap().push(element);
        }
        let error = solve(quad, input).unwrap_err();
        assert!(error.contains("total heat flow"), "{error}");
    }
}

#[test]
fn exact_constant_temperature_remains_a_valid_zero_flow_case() {
    for quad in [false, true] {
        for conductivity in [1e-200, 1.0, 1e200] {
            let mut input = model(quad, conductivity);
            for node in input["nodes"].as_array_mut().unwrap() {
                node["temperature"] = json!(1e300);
            }
            let result = solve(quad, input).unwrap();
            close(&result["elements"][0]["average_temperature"], 1e300);
            close(&result["elements"][0]["heat_flux_magnitude"], 0.0);
            close(&result["total_abs_heat_flow_rate"], 0.0);
        }
    }
}
