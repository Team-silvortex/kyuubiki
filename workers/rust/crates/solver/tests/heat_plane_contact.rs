use kyuubiki_protocol::{SolveHeatPlaneQuad2dRequest, SolveHeatPlaneTriangle2dRequest};
use kyuubiki_solver::{
    solve_heat_plane_quad_2d, solve_heat_plane_quad_2d_owned, solve_heat_plane_triangle_2d,
    solve_heat_plane_triangle_2d_owned,
};
use serde_json::{Value, json};

fn model() -> Value {
    serde_json::from_str(include_str!(
        "../../protocol/fixtures/heat-plane-contact-quad.json"
    ))
    .unwrap()
}

fn triangles(mut input: Value) -> Value {
    let mut elements = Vec::new();
    for quad in input["elements"].as_array().unwrap() {
        for (index, indices) in [
            [
                quad["node_i"].clone(),
                quad["node_j"].clone(),
                quad["node_k"].clone(),
            ],
            [
                quad["node_i"].clone(),
                quad["node_k"].clone(),
                quad["node_l"].clone(),
            ],
        ]
        .into_iter()
        .enumerate()
        {
            elements.push(
                json!({"id":format!("{}-{index}", quad["id"].as_str().unwrap()),
                "node_i":indices[0], "node_j":indices[1], "node_k":indices[2],
                "conductivity":quad["conductivity"], "thickness":quad["thickness"]}),
            );
        }
    }
    input["elements"] = json!(elements);
    input
}

fn solve(input: Value, triangle: bool, owned: bool) -> Result<Value, String> {
    if triangle {
        let request: SolveHeatPlaneTriangle2dRequest =
            serde_json::from_value(triangles(input)).unwrap();
        let result = if owned {
            solve_heat_plane_triangle_2d_owned(request)
        } else {
            solve_heat_plane_triangle_2d(&request)
        }?;
        Ok(serde_json::to_value(result).unwrap())
    } else {
        let request: SolveHeatPlaneQuad2dRequest = serde_json::from_value(input).unwrap();
        let result = if owned {
            solve_heat_plane_quad_2d_owned(request)
        } else {
            solve_heat_plane_quad_2d(&request)
        }?;
        Ok(serde_json::to_value(result).unwrap())
    }
}

fn number(value: &Value) -> f64 {
    value.as_f64().expect("finite numerical field")
}

fn close(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() < 1.0e-10 * expected.abs().max(1.0),
        "actual={actual}, expected={expected}"
    );
}

#[test]
fn concave_quad_contact_uses_the_actual_adjacent_subtriangle() {
    let mut input = model();
    let coordinates = [
        [0.0, 0.0],
        [1.0, 0.0],
        [0.25, 0.25],
        [0.0, 1.0],
        [1.0, 0.0],
        [1.5, 0.5],
        [0.75, 0.75],
        [0.25, 0.25],
    ];
    for (index, [x, y]) in coordinates.into_iter().enumerate() {
        let node = &mut input["nodes"][index];
        node["x"] = json!(x);
        node["y"] = json!(y);
        node["fix_temperature"] = json!(true);
        node["temperature"] = json!(if index < 4 { 20.0 } else { 0.0 });
    }
    for triangle in [true, false] {
        let result = solve(input.clone(), triangle, false).unwrap();
        close(
            number(&result["contact_interfaces"][0]["heat_flow_a_to_b_w"]),
            0.75_f64.hypot(0.25) * 0.5 * 20.0,
        );
    }
    // The nonadjacent corner is across the edge, but the actual subtriangle
    // now overlaps side A. Checking that corner would incorrectly admit it.
    for (index, [x, y]) in [(5, [0.75, -0.25]), (6, [0.0, 0.0])] {
        input["nodes"][index]["x"] = json!(x);
        input["nodes"][index]["y"] = json!(y);
    }
    for triangle in [true, false] {
        assert!(
            solve(input.clone(), triangle, false)
                .unwrap_err()
                .contains("opposite sides")
        );
    }
}

#[test]
fn series_bulk_and_contact_resistances_resolve_a_real_temperature_jump() {
    for triangle in [false, true] {
        for owned in [false, true] {
            for resistance in [0.01, 1.0, 100.0] {
                let mut input = model();
                input["contact_interfaces"][0]["thermal_resistance_m2_k_w"] = json!(resistance);
                let result = solve(input, triangle, owned).unwrap();
                let power = 20.0 / (1.0 + 2.0 * resistance + 0.5);
                for index in [1, 2] {
                    close(number(&result["nodes"][index]["temperature"]), 20.0 - power);
                }
                for index in [4, 7] {
                    close(number(&result["nodes"][index]["temperature"]), 0.5 * power);
                }
                let contact = &result["contact_interfaces"][0];
                close(number(&contact["area_m2"]), 0.5);
                close(
                    number(&contact["average_temperature_jump_k"]),
                    2.0 * resistance * power,
                );
                close(number(&contact["heat_flow_a_to_b_w"]), power);
                close(number(&contact["heat_flux_a_to_b_w_m2"]), power / 0.5);
                for nodal_power in contact["nodal_heat_flow_a_to_b_w"].as_array().unwrap() {
                    close(number(nodal_power), power / 2.0);
                }
                for element in result["elements"].as_array().unwrap() {
                    close(number(&element["heat_flux_x"]) * 0.5, power);
                    close(number(&element["heat_flux_y"]), 0.0);
                }
            }
        }
    }
}

#[test]
fn source_driven_body_can_be_anchored_through_contact_without_losing_heat() {
    for triangle in [false, true] {
        for offset in [0.0, -2.0_f64.powi(40), 2.0_f64.powi(40)] {
            let mut input = model();
            for node in input["nodes"].as_array_mut().unwrap() {
                node["temperature"] = json!(offset + 35.0);
            }
            for index in [0, 3] {
                input["nodes"][index]["fix_temperature"] = json!(false);
                input["nodes"][index]["heat_load"] = json!(2.0);
            }
            let result = solve(input, triangle, false).unwrap();
            for (indices, rise) in [([0, 3], 14.0), ([1, 2], 10.0), ([4, 7], 2.0)] {
                for index in indices {
                    close(
                        number(&result["nodes"][index]["temperature"]) - offset,
                        35.0 + rise,
                    );
                }
            }
            close(
                number(&result["contact_interfaces"][0]["heat_flow_a_to_b_w"]),
                4.0,
            );
            close(
                number(&result["contact_interfaces"][0]["average_temperature_jump_k"]),
                8.0,
            );
            for element in result["elements"].as_array().unwrap() {
                close(number(&element["heat_flux_x"]) * 0.5, 4.0);
            }
        }
    }
}

#[test]
fn consistent_edge_integration_preserves_nonuniform_jump_not_point_lumping() {
    for triangle in [false, true] {
        let mut input = model();
        for node in input["nodes"].as_array_mut().unwrap() {
            node["fix_temperature"] = json!(true);
            node["temperature"] = json!(0.0);
        }
        input["nodes"][1]["temperature"] = json!(10.0);
        input["nodes"][2]["temperature"] = json!(20.0);
        let result = solve(input, triangle, false).unwrap();
        let contact = &result["contact_interfaces"][0];
        close(
            number(&contact["nodal_heat_flow_a_to_b_w"][0]),
            0.5 * 40.0 / 6.0,
        );
        close(
            number(&contact["nodal_heat_flow_a_to_b_w"][1]),
            0.5 * 50.0 / 6.0,
        );
        close(number(&contact["heat_flow_a_to_b_w"]), 7.5);
        close(number(&contact["max_abs_temperature_jump_k"]), 20.0);
    }
}

#[test]
fn nonuniform_free_interface_solves_the_independent_four_unknown_system() {
    // Eliminating the fixed exterior nodes gives 12*K =
    // [[14,-5,-2,-1],[-5,14,-1,-2],[-2,-1,26,-11],[-1,-2,-11,26]]
    // and 12*f = [60,120,0,0]. These rational values exercise assembly,
    // not only contact postprocessing on an all-prescribed field.
    for triangle in [false, true] {
        let mut input = model();
        input["nodes"][0]["temperature"] = json!(10.0);
        let result = solve(input, triangle, false).unwrap();
        for (node, numerator) in [(1, 7480.0), (2, 10070.0), (4, 1720.0), (7, 1790.0)] {
            close(
                number(&result["nodes"][node]["temperature"]),
                numerator / 819.0,
            );
        }
        close(
            number(&result["contact_interfaces"][0]["nodal_heat_flow_a_to_b_w"][0]),
            1650.0 / 819.0,
        );
        close(
            number(&result["contact_interfaces"][0]["nodal_heat_flow_a_to_b_w"][1]),
            1860.0 / 819.0,
        );
    }
}

#[test]
fn side_reversal_changes_only_signed_contact_flow() {
    let baseline = solve(model(), false, false).unwrap();
    for triangle in [false, true] {
        let mut input = model();
        input["contact_interfaces"][0]["side_a"] = json!([7, 4]);
        input["contact_interfaces"][0]["side_b"] = json!([1, 2]);
        let reversed = solve(input, triangle, false).unwrap();
        for index in 0..8 {
            close(
                number(&reversed["nodes"][index]["temperature"]),
                number(&baseline["nodes"][index]["temperature"]),
            );
        }
        close(
            number(&reversed["contact_interfaces"][0]["heat_flow_a_to_b_w"]),
            -number(&baseline["contact_interfaces"][0]["heat_flow_a_to_b_w"]),
        );
        assert_eq!(reversed["contact_interfaces"][0]["side_b"], json!([2, 1]));
    }
}

#[test]
fn invalid_contact_geometry_and_topology_fail_before_the_solve() {
    let mut invalid = Vec::new();
    for field in [json!([1, 1]), json!([1, 99]), json!([0, 2]), json!([1, 2])] {
        let mut input = model();
        input["contact_interfaces"][0]["side_b"] = field;
        invalid.push(input);
    }
    let mut mismatch = model();
    mismatch["nodes"][4]["x"] = json!(1.01);
    invalid.push(mismatch);
    let mut thickness = model();
    thickness["elements"][1]["thickness"] = json!(0.6);
    invalid.push(thickness);
    let mut duplicate = model();
    let contact = duplicate["contact_interfaces"][0].clone();
    duplicate["contact_interfaces"]
        .as_array_mut()
        .unwrap()
        .push(contact);
    invalid.push(duplicate.clone());
    duplicate["contact_interfaces"][1]["id"] = json!("another-id-same-edge");
    invalid.push(duplicate);
    for triangle in [false, true] {
        for input in &invalid {
            let error =
                solve(input.clone(), triangle, false).expect_err("invalid interface must fail");
            assert!(error.contains("thermal contact"), "{error}");
        }
    }
}

#[test]
fn invalid_or_unrepresentable_resistance_is_rejected_instead_of_clamped() {
    for resistance in [0.0, -1.0, f64::from_bits(1)] {
        let mut input = model();
        input["contact_interfaces"][0]["thermal_resistance_m2_k_w"] = json!(resistance);
        for triangle in [false, true] {
            assert!(
                solve(input.clone(), triangle, false)
                    .expect_err("invalid R")
                    .contains("thermal contact")
            );
        }
    }
}

#[test]
fn legacy_no_contact_requests_keep_their_serialized_shape_and_solution() {
    let mut input = model();
    input.as_object_mut().unwrap().remove("contact_interfaces");
    for triangle in [false, true] {
        let result = solve(input.clone(), triangle, false).unwrap();
        assert!(result.get("contact_interfaces").is_none());
        assert!(result["input"].get("contact_interfaces").is_none());
        for index in [0, 1, 2, 3] {
            close(number(&result["nodes"][index]["temperature"]), 20.0);
        }
        for index in [4, 5, 6, 7] {
            close(number(&result["nodes"][index]["temperature"]), 0.0);
        }
    }
}

#[test]
fn split_contact_edges_preserve_total_power_under_mesh_refinement_and_rotation() {
    for divisions in [1, 2, 4, 8, 16] {
        let side_count = 2 * (divisions + 1);
        let mut nodes = Vec::new();
        let mut elements = Vec::new();
        let mut contacts = Vec::new();
        for body in 0..2 {
            for row in 0..=divisions {
                for column in 0..2 {
                    let x = (body + column) as f64;
                    nodes.push(json!({"id":format!("{body}-{row}-{column}"), "x":x,
                        "y":row as f64 / divisions as f64, "fix_temperature":x == 0.0 || x == 2.0,
                        "temperature":if x == 0.0 {20.0} else {0.0}}));
                }
            }
            for row in 0..divisions {
                let start = body * side_count + 2 * row;
                elements.push(json!({"id":format!("bulk-{body}-{row}"), "node_i":start,
                    "node_j":start+1, "node_k":start+3, "node_l":start+2,
                    "thickness":0.5, "conductivity":if body == 0 {2.0} else {4.0}}));
            }
        }
        for row in 0..divisions {
            contacts.push(
                json!({"id":format!("bond-{row}"), "side_a":[2*row+1,2*row+3],
                "side_b":[side_count+2*row,side_count+2*row+2], "thermal_resistance_m2_k_w":1.0}),
            );
        }
        let input = json!({"nodes":nodes,"elements":elements,"contact_interfaces":contacts});
        for triangle in [false, true] {
            for rotated in [false, true] {
                let mut input = input.clone();
                if rotated {
                    for node in input["nodes"].as_array_mut().unwrap() {
                        let x = number(&node["x"]);
                        let y = number(&node["y"]);
                        node["x"] = json!(2.0_f64.powi(40) - y);
                        node["y"] = json!(2.0_f64.powi(40) + x);
                    }
                }
                let result = solve(input, triangle, false).unwrap();
                let power = 20.0 / 3.5;
                let mut total = 0.0;
                for contact in result["contact_interfaces"].as_array().unwrap() {
                    let flow = number(&contact["heat_flow_a_to_b_w"]);
                    close(flow, power / divisions as f64);
                    close(number(&contact["average_temperature_jump_k"]), 2.0 * power);
                    total += flow;
                }
                close(total, power);
                for element in result["elements"].as_array().unwrap() {
                    close(
                        number(
                            &element[if rotated {
                                "heat_flux_y"
                            } else {
                                "heat_flux_x"
                            }],
                        ) * 0.5,
                        power,
                    );
                }
            }
        }
    }
}

#[test]
fn nonfinite_resistance_in_typed_requests_is_rejected() {
    for resistance in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        let mut quad: SolveHeatPlaneQuad2dRequest = serde_json::from_value(model()).unwrap();
        quad.contact_interfaces[0].thermal_resistance_m2_k_w = resistance;
        assert!(
            solve_heat_plane_quad_2d(&quad)
                .unwrap_err()
                .contains("thermal contact")
        );
        let mut triangle: SolveHeatPlaneTriangle2dRequest =
            serde_json::from_value(triangles(model())).unwrap();
        triangle.contact_interfaces[0].thermal_resistance_m2_k_w = resistance;
        assert!(
            solve_heat_plane_triangle_2d(&triangle)
                .unwrap_err()
                .contains("thermal contact")
        );
    }
}

#[test]
fn subnormal_interface_weights_cannot_silently_change_the_contact_law() {
    let mut input = model();
    for element in input["elements"].as_array_mut().unwrap() {
        element["thickness"] = json!(8e-15);
    }
    input["contact_interfaces"][0]["thermal_resistance_m2_k_w"] = json!(1e308);
    for triangle in [false, true] {
        let error = solve(input.clone(), triangle, false).unwrap_err();
        assert!(
            error.contains("integrated conductance is not representable"),
            "{error}"
        );
    }
}

#[test]
fn thermal_contact_rejects_overlapping_bodies_internal_edges_and_blank_ids() {
    let mut cases = Vec::new();
    let mut overlap = model();
    for index in [5, 6] {
        overlap["nodes"][index]["x"] = json!(0.0);
    }
    cases.push(overlap);
    let mut internal = model();
    let element = internal["elements"][0].clone();
    internal["elements"].as_array_mut().unwrap().push(element);
    cases.push(internal);
    let mut blank = model();
    blank["contact_interfaces"][0]["id"] = json!("  ");
    cases.push(blank);
    for triangle in [false, true] {
        for input in &cases {
            assert!(
                solve(input.clone(), triangle, false)
                    .unwrap_err()
                    .contains("thermal contact")
            );
        }
    }
}

#[test]
fn a_disconnected_unanchored_island_is_not_hidden_by_other_contacted_bodies() {
    let mut input = model();
    let mut island = input["nodes"].as_array().unwrap()[..4].to_vec();
    for (index, node) in island.iter_mut().enumerate() {
        node["id"] = json!(format!("island-{index}"));
        node["x"] = json!(number(&node["x"]) + 4.0);
        node["fix_temperature"] = json!(false);
    }
    input["nodes"].as_array_mut().unwrap().extend(island);
    input["elements"]
        .as_array_mut()
        .unwrap()
        .push(json!({"id":"island",
        "node_i":8,"node_j":9,"node_k":10,"node_l":11,"thickness":0.5,"conductivity":2.0}));
    for triangle in [false, true] {
        let error = solve(input.clone(), triangle, false).unwrap_err();
        assert!(error.contains("has no temperature support"), "{error}");
    }
}

#[test]
fn equal_temperature_and_opposing_endpoint_flows_do_not_generate_heat() {
    for triangle in [false, true] {
        for jump in [[0.0, 0.0], [10.0, -10.0]] {
            let mut input = model();
            for node in input["nodes"].as_array_mut().unwrap() {
                node["fix_temperature"] = json!(true);
                node["temperature"] = json!(35.0);
            }
            input["nodes"][1]["temperature"] = json!(35.0 + jump[0]);
            input["nodes"][2]["temperature"] = json!(35.0 + jump[1]);
            let result = solve(input, triangle, false).unwrap();
            let contact = &result["contact_interfaces"][0];
            close(number(&contact["heat_flow_a_to_b_w"]), 0.0);
            close(
                number(&contact["nodal_heat_flow_a_to_b_w"][0]),
                jump[0] / 12.0,
            );
            close(
                number(&contact["nodal_heat_flow_a_to_b_w"][1]),
                jump[1] / 12.0,
            );
        }
    }
}
