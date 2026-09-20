use kyuubiki_protocol::{
    SolveElectricConductionPlaneQuad2dRequest, SolveElectricConductionPlaneQuad2dResult,
};
use kyuubiki_solver::{
    SpdPreconditioner, SpdSolveOptions, solve_electric_conduction_plane_quad_2d,
    solve_electric_conduction_plane_quad_2d_owned,
    solve_electric_conduction_plane_quad_2d_with_options,
};
use serde_json::{Value, json};

fn square() -> SolveElectricConductionPlaneQuad2dRequest {
    serde_json::from_value(json!({
        "nodes": [
            {"id": "a", "x": 0.0, "y": 0.0, "fix_electric_potential": true},
            {"id": "b", "x": 1.0, "y": 0.0, "fix_electric_potential": false, "current_source_a": 0.5},
            {"id": "c", "x": 1.0, "y": 1.0, "fix_electric_potential": false, "current_source_a": 0.5},
            {"id": "d", "x": 0.0, "y": 1.0, "fix_electric_potential": true}
        ],
        "elements": [{"id": "bulk", "node_i": 0, "node_j": 1, "node_k": 2, "node_l": 3,
            "thickness": 1.0, "electrical_conductivity_s_m": 3.0}]
    })).unwrap()
}

fn shifted(
    mut request: SolveElectricConductionPlaneQuad2dRequest,
    offset: f64,
) -> SolveElectricConductionPlaneQuad2dRequest {
    for node in &mut request.nodes {
        node.electric_potential_v += offset;
    }
    for terminal in &mut request.terminals {
        terminal.external_potential_v += offset;
    }
    request
}

fn solve(
    request: &SolveElectricConductionPlaneQuad2dRequest,
) -> SolveElectricConductionPlaneQuad2dResult {
    let result = solve_electric_conduction_plane_quad_2d(request).unwrap();
    assert_eq!(
        serde_json::to_value(&result.input).unwrap(),
        serde_json::to_value(request).unwrap()
    );
    result
}

fn close(actual: f64, expected: f64, label: &str) {
    assert!(
        (actual - expected).abs() <= 1.0e-10 + 1.0e-8 * expected.abs(),
        "{label}: actual={actual}, expected={expected}"
    );
}

fn check_shift(request: &SolveElectricConductionPlaneQuad2dRequest) {
    let baseline = solve(request);
    for offset in [-2.0_f64.powi(40), 2.0_f64.powi(40)] {
        let moved = shifted(request.clone(), offset);
        let actual = solve(&moved);
        let actual_json = serde_json::to_value(&actual).unwrap();
        let expected_json = serde_json::to_value(&baseline).unwrap();
        for (key, expected) in expected_json.as_object().unwrap() {
            if key != "max_electric_potential_v" {
                if let Some(expected) = expected.as_f64() {
                    close(actual_json[key].as_f64().unwrap(), expected, key);
                }
            }
        }
        for collection in ["nodes", "elements", "contact_interfaces", "terminals"] {
            for (actual, expected) in actual_json[collection]
                .as_array()
                .unwrap()
                .iter()
                .zip(expected_json[collection].as_array().unwrap())
            {
                compare_record(collection, actual, expected, offset);
            }
        }
        for (input, node) in moved.nodes.iter().zip(&actual.nodes) {
            if input.fix_electric_potential {
                assert_eq!(node.electric_potential_v, input.electric_potential_v);
            }
        }
        close(
            actual.max_electric_potential_v,
            actual
                .nodes
                .iter()
                .map(|node| node.electric_potential_v.abs())
                .fold(0.0, f64::max),
            "absolute peak",
        );
    }
}

fn compare_record(collection: &str, actual: &Value, expected: &Value, offset: f64) {
    for (key, expected_value) in expected.as_object().unwrap() {
        let Some(expected_value) = expected_value.as_f64() else {
            assert_eq!(&actual[key], expected_value);
            continue;
        };
        let actual = actual[key].as_f64().unwrap();
        if [
            "electric_potential_v",
            "average_electric_potential_v",
            "external_potential_v",
            "node_potential_v",
        ]
        .contains(&key.as_str())
        {
            assert!(
                (actual - (expected_value + offset)).abs() <= 0.0005,
                "{collection}.{key} was not restored"
            );
        } else if collection == "terminals"
            && ["source_power_w", "power_delivered_to_domain_w"].contains(&key.as_str())
        {
            let expected =
                expected_value + offset * expected["current_into_domain_a"].as_f64().unwrap();
            assert!(
                (actual - expected).abs() <= 0.001,
                "individual terminal power lost its absolute reference"
            );
        } else {
            close(actual, expected_value, &format!("{collection}.{key}"));
        }
    }
}

#[test]
fn constant_skew_conductor_has_no_spurious_current_or_power() {
    let mut request = square();
    for (node, [x, y]) in
        request
            .nodes
            .iter_mut()
            .zip([[0.0, 0.0], [1.5, 0.5], [1.75, 1.75], [0.25, 1.25]])
    {
        node.x = x;
        node.y = y;
        node.fix_electric_potential = true;
        node.current_source_a = 0.0;
    }
    check_shift(&request);
}

#[test]
fn current_driven_field_and_joule_power_are_reference_invariant() {
    let request = square();
    let result = solve(&request);
    close(result.total_joule_power_w, 1.0 / 3.0, "I squared R");
    check_shift(&request);
}

#[test]
fn impedance_only_anchoring_preserves_source_and_loss_balance_under_reference_shift() {
    let mut request = square();
    request.elements[0].electrical_conductivity_s_m = 1.0;
    for node in &mut request.nodes {
        node.fix_electric_potential = false;
        node.current_source_a = 0.0;
    }
    request.terminals = serde_json::from_value(json!([
        {"id": "left-a", "node": 0, "external_potential_v": 0.0, "impedance_ohm": 2.0},
        {"id": "right-b", "node": 1, "external_potential_v": 1.0, "impedance_ohm": 2.0},
        {"id": "right-c", "node": 2, "external_potential_v": 1.0, "impedance_ohm": 2.0},
        {"id": "left-d", "node": 3, "external_potential_v": 0.0, "impedance_ohm": 2.0}
    ]))
    .unwrap();
    let result = solve(&request);
    close(result.total_bulk_joule_power_w, 1.0 / 9.0, "bulk loss");
    close(
        result.total_terminal_impedance_power_w,
        2.0 / 9.0,
        "terminal loss",
    );
    close(result.total_source_power_w, 1.0 / 3.0, "source power");
    check_shift(&request);
}

#[test]
fn contact_voltage_drop_and_partitioned_power_are_reference_invariant() {
    let mut request = square();
    request.elements[0].electrical_conductivity_s_m = 1.0;
    for node in &mut request.nodes {
        node.current_source_a = 0.0;
    }
    let mut right = request.nodes.clone();
    for node in &mut right {
        node.id = format!("right-{}", node.id);
        node.x += 1.0;
        node.fix_electric_potential = node.x == 2.0;
        node.electric_potential_v = if node.fix_electric_potential {
            1.0
        } else {
            0.0
        };
    }
    request.nodes.extend(right);
    let mut element = request.elements[0].clone();
    element.id = "right-bulk".into();
    element.node_i += 4;
    element.node_j += 4;
    element.node_k += 4;
    element.node_l += 4;
    request.elements.push(element);
    request.contact_interfaces = serde_json::from_value(json!([
        {"id": "bottom", "node_i": 1, "node_j": 4, "contact_resistance_ohm": 2.0},
        {"id": "top", "node_i": 2, "node_j": 7, "contact_resistance_ohm": 2.0}
    ]))
    .unwrap();
    let result = solve(&request);
    close(result.total_bulk_joule_power_w, 2.0 / 9.0, "bulk loss");
    close(
        result.total_contact_joule_power_w,
        1.0 / 9.0,
        "contact loss",
    );
    close(result.total_source_power_w, 1.0 / 3.0, "source power");
    check_shift(&request);
}

#[test]
fn fixed_and_impedance_sources_share_a_reference_without_double_counting_power() {
    let mut request = square();
    request.terminals = serde_json::from_value(json!([
        {"id": "fixed-electrode", "node": 0, "external_potential_v": 1.0, "impedance_ohm": 2.0},
        {"id": "free-electrode", "node": 1, "external_potential_v": 0.5, "impedance_ohm": 3.0},
        {"id": "free-electrode-extra", "node": 1, "external_potential_v": -0.5, "impedance_ohm": 4.0}
    ])).unwrap();
    let baseline = solve(&request);
    assert!(baseline.power_balance_relative_error < 1.0e-12);
    assert!(baseline.source_power_balance_relative_error < 1.0e-12);
    check_shift(&request);
}

#[test]
fn ownership_path_preserves_relative_fields_and_original_input() {
    let request = shifted(square(), 2.0_f64.powi(40));
    let borrowed = solve(&request);
    let owned = solve_electric_conduction_plane_quad_2d_owned(request).unwrap();
    assert_eq!(
        serde_json::to_value(borrowed).unwrap(),
        serde_json::to_value(owned).unwrap()
    );
}

#[test]
fn reversing_current_reverses_fields_but_preserves_joule_power_at_large_reference() {
    let positive = solve(&shifted(square(), 2.0_f64.powi(40)));
    let mut request = square();
    for node in &mut request.nodes {
        node.current_source_a = -node.current_source_a;
    }
    let negative = solve(&shifted(request, 2.0_f64.powi(40)));
    close(
        negative.elements[0].electric_field_x_v_m,
        -positive.elements[0].electric_field_x_v_m,
        "reversed field",
    );
    close(
        negative.total_joule_power_w,
        positive.total_joule_power_w,
        "reversed Joule power",
    );
    close(
        negative.total_source_power_w,
        positive.total_source_power_w,
        "reversed source power",
    );
    assert!(negative.source_power_balance_relative_error < 1.0e-12);
}

#[test]
fn a_large_source_cancelled_by_a_fixed_electrode_does_not_erase_domain_current() {
    let mut request = square();
    for node in &mut request.nodes {
        node.fix_electric_potential = true;
        node.electric_potential_v = node.x;
        node.current_source_a = 0.0;
    }
    request.nodes[1].current_source_a = 2.0_f64.powi(60);
    let result = solve(&request);
    close(
        result.nodes[1].net_injected_current_a,
        1.5,
        "electrode net current",
    );
    close(result.total_injected_current_a, 3.0, "domain total current");
    close(result.total_source_power_w, 3.0, "net source power");
    assert!(result.current_balance_relative_error < 1.0e-12);
    assert!(result.source_power_balance_relative_error < 1.0e-12);
}

#[test]
fn nonrepresentable_terminal_and_prescribed_ranges_fail_without_poisoning_next_solve() {
    let mut request = square();
    request.nodes[0].electric_potential_v = -f64::MAX;
    request.nodes[3].electric_potential_v = f64::MAX;
    let error = solve_electric_conduction_plane_quad_2d(&request).unwrap_err();
    assert!(error.contains("relative potential range must be finite"));
    request.nodes[3].electric_potential_v = -f64::MAX;
    request.terminals = serde_json::from_value(json!([
        {"id": "outside-range", "node": 1, "external_potential_v": f64::MAX, "impedance_ohm": 2.0}
    ]))
    .unwrap();
    let error = solve_electric_conduction_plane_quad_2d(&request).unwrap_err();
    assert!(error.contains("relative potential range must be finite"));
    close(
        solve(&square()).total_joule_power_w,
        1.0 / 3.0,
        "fresh solve",
    );
}

#[test]
fn nonrepresentable_interface_conductance_is_rejected_even_when_all_nodes_are_fixed() {
    let mut request = square();
    for node in &mut request.nodes {
        node.fix_electric_potential = true;
    }
    request.terminals = serde_json::from_value(json!([
        {"id": "tiny-impedance", "node": 0, "external_potential_v": 0.0, "impedance_ohm": 1.0e-320}
    ]))
    .unwrap();
    let error = solve_electric_conduction_plane_quad_2d(&request).unwrap_err();
    assert!(error.contains("terminal parameters are invalid"));
    request.terminals.clear();
    request.contact_interfaces = serde_json::from_value(json!([
        {"id": "tiny-resistance", "node_i": 0, "node_j": 1, "contact_resistance_ohm": 1.0e-320}
    ]))
    .unwrap();
    let error = solve_electric_conduction_plane_quad_2d(&request).unwrap_err();
    assert!(error.contains("contact interface parameters are invalid"));
}

fn current_driven_grid(divisions: usize) -> SolveElectricConductionPlaneQuad2dRequest {
    let mut request = square();
    let node_template = request.nodes[0].clone();
    let element_template = request.elements[0].clone();
    request.nodes.clear();
    request.elements.clear();
    for j in 0..=divisions {
        for i in 0..=divisions {
            let mut node = node_template.clone();
            node.id = format!("n{i}-{j}");
            node.x = i as f64 / divisions as f64;
            node.y = j as f64 / divisions as f64;
            node.fix_electric_potential = i == 0;
            if i == divisions {
                let weight = if j == 0 || j == divisions { 0.5 } else { 1.0 };
                node.current_source_a = weight / divisions as f64;
            }
            request.nodes.push(node);
        }
    }
    for j in 0..divisions {
        for i in 0..divisions {
            let mut element = element_template.clone();
            element.id = format!("e{i}-{j}");
            element.node_i = j * (divisions + 1) + i;
            element.node_j = element.node_i + 1;
            element.node_k = element.node_j + divisions + 1;
            element.node_l = element.node_i + divisions + 1;
            request.elements.push(element);
        }
    }
    request
}

#[test]
fn sparse_current_driven_patch_preserves_ohms_law_with_all_preconditioners() {
    // 1225 nodes, 1190 free DOFs: above the dense LU threshold.
    let request = current_driven_grid(34);
    for preconditioner in [
        SpdPreconditioner::Jacobi,
        SpdPreconditioner::SymmetricGaussSeidel,
        SpdPreconditioner::IncompleteCholesky,
    ] {
        for offset in [0.0, -2.0_f64.powi(40), 2.0_f64.powi(40)] {
            let result = solve_electric_conduction_plane_quad_2d_with_options(
                &shifted(request.clone(), offset),
                SpdSolveOptions {
                    preconditioner,
                    progress_interval: None,
                },
            )
            .unwrap();
            close(result.total_injected_current_a, 1.0, "total current");
            close(result.total_joule_power_w, 1.0 / 3.0, "sparse I squared R");
            assert!(result.source_power_balance_relative_error < 1.0e-8);
            assert!(result.free_current_residual_relative_error < 1.0e-8);
            for element in &result.elements {
                close(element.electric_field_x_v_m, -1.0 / 3.0, "sparse field x");
                assert!(element.electric_field_y_v_m.abs() < 1.0e-8);
            }
        }
    }
}
