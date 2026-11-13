#[path = "support/bilayer.rs"]
mod bilayer;
use bilayer::Case;
use kyuubiki_protocol::{OperatorRunContext, OperatorRunRequest, SolveThermalQ9RectRequest};
use serde_json::{Value, json};

fn execute(id: &str, payload: Value) -> Result<Value, String> {
    kyuubiki_operator_thermal_q9::run(OperatorRunRequest {
        operator_id: id.into(),
        context: OperatorRunContext::default(),
        input: json!({"payload":payload,"config":{}}),
    })
    .map(|r| r.summary)
    .map_err(|e| e.to_string())
}

fn payload(case: &Case) -> Value {
    let heat = kyuubiki_solver::solve_heat_plane_quad_2d(&case.heat).unwrap();
    let k = case.heat.elements[0].conductivity;
    let expected_flux_y = -k * case.temperature[0][1];
    for element in &heat.elements {
        assert!(element.heat_flux_x.abs() < 1e-8);
        assert!((element.heat_flux_y - expected_flux_y).abs() < 1e-8);
    }
    json!({"schema_version":"kyuubiki.heat-to-thermal-q9-nodal/v1", "temperature_unit":"kelvin",
        "reference_temperature":case.reference_temperature, "heat_result":heat, "seed_model":case.seed})
}

#[test]
fn bridge_refuses_contacts_unknown_controls_and_missing_temperature() {
    let case = Case::new(2, 2, [10.0, 40.0], [10e-6, 30e-6], [0.0, 20.0]);
    let original = payload(&case);
    let contact =
        json!({"id":"contact", "side_a":[0,1], "side_b":[2,3], "thermal_resistance_m2_k_w":1.0});
    let mut bad = original.clone();
    bad["heat_result"]["input"]["contact_interfaces"] = json!([contact]);
    assert!(execute("workflow_bridge.heat_to_thermal_q9_rect_2d", bad).is_err());
    let mut bad = original.clone();
    bad["heat_result"]["contact_interfaces"] = json!([{
        "index":0, "id":"contact", "side_a":[0,1], "side_b":[2,3], "thermal_resistance_m2_k_w":1.0,
        "area_m2":1.0, "average_temperature_jump_k":1.0, "max_abs_temperature_jump_k":1.0,
        "heat_flow_a_to_b_w":1.0, "heat_flux_a_to_b_w_m2":1.0, "nodal_heat_flow_a_to_b_w":[0.5,0.5]
    }]);
    assert!(execute("workflow_bridge.heat_to_thermal_q9_rect_2d", bad).is_err());
    let mut bad = original.clone();
    bad["temperature_scale"] = json!(2.0);
    assert!(execute("workflow_bridge.heat_to_thermal_q9_rect_2d", bad).is_err());
    let mut bad = original.clone();
    bad.as_object_mut().unwrap().remove("reference_temperature");
    assert!(execute("workflow_bridge.heat_to_thermal_q9_rect_2d", bad).is_err());
    let mut bad = original.clone();
    bad["heat_result"]["nodes"][3]
        .as_object_mut()
        .unwrap()
        .remove("temperature");
    assert!(execute("workflow_bridge.heat_to_thermal_q9_rect_2d", bad).is_err());
    assert!(
        kyuubiki_operator_thermal_q9::run(OperatorRunRequest {
            operator_id: "workflow_bridge.heat_to_thermal_q9_rect_2d".into(),
            context: OperatorRunContext::default(),
            input: json!({"payload":original, "config":{"temperature_scale":2.0}}),
        })
        .is_err()
    );
}

#[test]
fn actual_heat_solve_bridges_all_nodes_before_q9_mechanics() {
    for (nx, ny) in [(2, 2), (8, 4), (16, 8)] {
        for rise in [[0.0, 20.0], [10.0, 10.0]] {
            let case = Case::new(nx, ny, [10.0, 40.0], [10e-6, 30e-6], rise);
            let input = payload(&case);
            let bridged =
                execute("workflow_bridge.heat_to_thermal_q9_rect_2d", input.clone()).unwrap();
            let model: SolveThermalQ9RectRequest = serde_json::from_value(bridged.clone()).unwrap();
            for (n, original) in model.nodes.iter().zip(&case.seed.nodes) {
                assert!((n.temperature_delta - case.expected_delta(n.y)).abs() < 1e-8);
                assert_eq!(
                    (n.load_x, n.load_y, n.fix_x, n.fix_y),
                    (
                        original.load_x,
                        original.load_y,
                        original.fix_x,
                        original.fix_y
                    )
                );
            }
            assert_eq!(
                serde_json::to_value(&model.elements).unwrap(),
                input["seed_model"]["elements"]
            );
            let result = execute("solve.thermal_plane_q9_rect_2d", bridged).unwrap();
            let energy_error =
                (result["total_strain_energy_j"].as_f64().unwrap() - case.energy).abs();
            assert!(energy_error < 1e-8, "{nx}x{ny}: {energy_error}");
            for n in result["nodes"].as_array().unwrap() {
                let [ux, uy] =
                    case.expected_displacement(n["x"].as_f64().unwrap(), n["y"].as_f64().unwrap());
                assert!((n["ux"].as_f64().unwrap() - ux).abs() < 1e-10);
                assert!((n["uy"].as_f64().unwrap() - uy).abs() < 1e-10);
            }
        }
    }
}

#[test]
fn bridge_rejects_corruption_without_mutating_seed_and_recovers() {
    let case = Case::new(2, 2, [10.0, 40.0], [10e-6, 30e-6], [0.0, 20.0]);
    let original = payload(&case);
    for (pointer, value) in [
        ("/schema_version", json!("unknown")),
        ("/temperature_unit", json!("celsius")),
        ("/reference_temperature", json!(-1.0)),
        ("/heat_result/nodes/3/id", json!("wrong")),
        ("/heat_result/nodes/3/x", json!(10.0)),
        ("/heat_result/nodes/3/index", json!(0)),
        ("/heat_result/nodes/3/temperature", json!(-1.0)),
        ("/heat_result/elements/0/node_i", json!(999)),
        ("/heat_result/input/elements/0/conductivity", json!(0.0)),
        ("/heat_result/input/elements/0/thickness", json!(0.02)),
        ("/heat_result/input/elements/0/node_l", json!(20)),
        ("/heat_result/input/nodes/3/id", json!("n0_0")),
        ("/heat_result/input/nodes/0/temperature", json!(300.0)),
        ("/heat_result/nodes/3/heat_load", json!(1.0)),
        ("/heat_result/nodes/3/temperature", Value::Null),
        ("/heat_result/nodes", json!([])),
        ("/heat_result/elements", json!([])),
        ("/heat_result/elements/1/index", json!(0)),
        ("/heat_result/elements/0/area", json!(0.0)),
        ("/heat_result/max_temperature", json!(-1.0)),
        ("/seed_model/nodes/3/id", json!("missing")),
        ("/seed_model/nodes/3/x", json!(0.333)),
        ("/seed_model/elements/0/nodes/8", json!(999)),
    ] {
        let mut bad = original.clone();
        *bad.pointer_mut(pointer).unwrap() = value;
        assert!(
            execute("workflow_bridge.heat_to_thermal_q9_rect_2d", bad).is_err(),
            "{pointer}"
        );
        assert!(
            execute(
                "workflow_bridge.heat_to_thermal_q9_rect_2d",
                original.clone()
            )
            .is_ok()
        );
    }
    let mut reordered = original.clone();
    reordered["heat_result"]["nodes"]
        .as_array_mut()
        .unwrap()
        .reverse();
    assert_eq!(
        execute("workflow_bridge.heat_to_thermal_q9_rect_2d", reordered).unwrap(),
        execute(
            "workflow_bridge.heat_to_thermal_q9_rect_2d",
            original.clone()
        )
        .unwrap()
    );
    assert!(case.seed.nodes.iter().all(|n| n.temperature_delta == 0.0));
}

#[test]
fn bridge_maps_reindexed_heat_input_and_result_by_identity_through_c_abi() {
    use kyuubiki_operator_sdk::{OperatorJsonAbiBuffer, decode_operator_json_abi_response};
    use kyuubiki_operator_thermal_q9::{kyuubiki_operator_json_free, run_thermal_q9_operator_json};
    let case = Case::new(2, 2, [10.0, 40.0], [10e-6, 30e-6], [0.0, 20.0]);
    let original = payload(&case);
    let expected = execute(
        "workflow_bridge.heat_to_thermal_q9_rect_2d",
        original.clone(),
    )
    .unwrap();
    let mut reordered = original;
    let count = case.heat.nodes.len();
    reordered["heat_result"]["input"]["nodes"]
        .as_array_mut()
        .unwrap()
        .reverse();
    for node in reordered["heat_result"]["nodes"].as_array_mut().unwrap() {
        node["index"] = json!(count - 1 - node["index"].as_u64().unwrap() as usize);
    }
    for pointer in ["/heat_result/input/elements", "/heat_result/elements"] {
        for element in reordered
            .pointer_mut(pointer)
            .unwrap()
            .as_array_mut()
            .unwrap()
        {
            for field in ["node_i", "node_j", "node_k", "node_l"] {
                element[field] = json!(count - 1 - element[field].as_u64().unwrap() as usize);
            }
            // A cyclic corner shift is still the same oriented Q4 cell.
            let corners = ["node_j", "node_k", "node_l", "node_i"].map(|f| element[f].clone());
            for (field, value) in ["node_i", "node_j", "node_k", "node_l"]
                .into_iter()
                .zip(corners)
            {
                element[field] = value;
            }
        }
    }
    reordered["heat_result"]["elements"]
        .as_array_mut()
        .unwrap()
        .reverse();
    let request = OperatorRunRequest {
        operator_id: "workflow_bridge.heat_to_thermal_q9_rect_2d".into(),
        context: OperatorRunContext::default(),
        input: json!({"payload":reordered,"config":{}}),
    };
    let bytes = serde_json::to_vec(&request).unwrap();
    let mut buffer = OperatorJsonAbiBuffer::empty();
    let status = unsafe { run_thermal_q9_operator_json(bytes.as_ptr(), bytes.len(), &mut buffer) };
    let result = unsafe {
        decode_operator_json_abi_response(
            status,
            std::slice::from_raw_parts(buffer.ptr, buffer.len),
        )
    };
    unsafe { kyuubiki_operator_json_free(buffer) };
    assert_eq!(result.unwrap().summary, expected);
}
