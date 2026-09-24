use kyuubiki_protocol::SolveFrame2dMaterialPDeltaRequest;
use kyuubiki_solver::solve_frame_2d_material_p_delta;
use serde_json::{Value, json};

fn model() -> Value {
    json!({
        "stability": {"buckling": {"frame": {
            "nodes": (0..3).map(|i| json!({
                "id": format!("n{i}"), "x": 0.0, "y": 2.0 * i as f64,
                "fix_x": i == 0, "fix_y": i == 0, "fix_rz": i == 0,
                "load_x": 0.0, "load_y": if i == 2 { -2.5e6 } else { 0.0 }, "moment_z": 0.0
            })).collect::<Vec<_>>(),
            "elements": (0..2).map(|i| json!({
                "id": format!("e{i}"), "node_i": i, "node_j": i + 1,
                "area": 0.01, "youngs_modulus": 2e11,
                "moment_of_inertia": 5e-4, "section_modulus": 5e-4 / 0.3
            })).collect::<Vec<_>>()
        }, "mode_count": 1}, "imperfection_amplitude": 1e-12,
        "kinematics": "corotational", "max_iterations": 64,
        "tolerance": 1e-9, "max_step_cutbacks": 12},
        "materials": (0..2).map(|i| json!({
            "element_id": format!("e{i}"), "yield_strength": 2.5e8, "hardening_ratio": 0.05
        })).collect::<Vec<_>>(), "load_factor_schedule": [0.0, 0.1]
    })
}

fn fiber_model(stresses: [f64; 4]) -> Value {
    let mut input = model();
    for material in input["materials"].as_array_mut().unwrap() {
        material["section_fibers"] = json!(
            [-0.3_f64, -0.1, 0.1, 0.3]
                .into_iter()
                .zip(stresses)
                .map(|(y, stress)| json!({
                    "y": y, "area": 0.0025, "initial_axial_stress": stress,
                    "material_id": if y.abs() > 0.2 { "soft" } else { "stiff" }
                }))
                .collect::<Vec<_>>()
        );
        material["fiber_materials"] = json!([
            {"id": "soft", "youngs_modulus": 1.5e11, "yield_strength": 2e8, "hardening_ratio": 0.05},
            {"id": "stiff", "youngs_modulus": 2.5e11, "yield_strength": 3e8, "hardening_ratio": 0.05}
        ]);
    }
    input
}

fn request(input: Value) -> SolveFrame2dMaterialPDeltaRequest {
    serde_json::from_value(input).unwrap()
}

fn assert_unbalanced(input: Value) {
    let error = solve_frame_2d_material_p_delta(&request(input))
        .expect_err("a nonequilibrated initial stress must fail before load stepping");
    assert!(
        error.contains("not self-equilibrated on free DOFs"),
        "{error}"
    );
}

#[test]
fn tiny_uniform_prestress_is_not_hidden_by_yield_strength_or_an_absolute_floor() {
    for stress in [1e-6, 1e-12, 1e-200] {
        let mut input = model();
        for material in input["materials"].as_array_mut().unwrap() {
            material["initial_axial_stress"] = json!(stress);
        }
        assert_unbalanced(input);
    }
}

#[test]
fn unused_parent_strength_cannot_mask_overridden_fiber_prestress() {
    for strength in [2.5e8, 1e30, 1e300] {
        let mut input = fiber_model([1e6; 4]);
        for material in input["materials"].as_array_mut().unwrap() {
            material["yield_strength"] = json!(strength);
        }
        assert_unbalanced(input);
    }
}

#[test]
fn unrelated_supported_member_cannot_relax_free_node_initial_equilibrium() {
    let mut input = model();
    input["materials"][0]["initial_axial_stress"] = json!(1e6);
    input["stability"]["buckling"]["frame"]["nodes"]
        .as_array_mut()
        .unwrap()
        .push(json!({
            "id": "guard", "x": 1.0, "y": 0.0,
            "fix_x": true, "fix_y": true, "fix_rz": true,
            "load_x": 0.0, "load_y": 0.0, "moment_z": 0.0
        }));
    input["stability"]["buckling"]["frame"]["elements"]
        .as_array_mut()
        .unwrap()
        .push(json!({
            "id": "supported", "node_i": 0, "node_j": 3,
            "area": 0.01, "youngs_modulus": 2e11,
            "moment_of_inertia": 5e-4, "section_modulus": 5e-4 / 0.3
        }));
    input["materials"].as_array_mut().unwrap().push(json!({
        "element_id": "supported", "yield_strength": 1e30,
        "hardening_ratio": 0.05, "initial_axial_stress": 1e6
    }));
    assert_unbalanced(input);
}

#[test]
fn unbalanced_initial_moment_is_rejected_in_every_tested_length_unit() {
    for length_scale in [1e-3, 1.0, 1e3] {
        let mut input = request(fiber_model([-1e-4, -1e-4, 1e-4, 1e-4]));
        input.stability.imperfection_amplitude *= length_scale;
        for node in &mut input.stability.buckling.frame.nodes {
            node.x *= length_scale;
            node.y *= length_scale;
            node.moment_z *= length_scale;
        }
        for element in &mut input.stability.buckling.frame.elements {
            element.area *= length_scale.powi(2);
            element.moment_of_inertia *= length_scale.powi(4);
            element.section_modulus *= length_scale.powi(3);
            element.youngs_modulus /= length_scale.powi(2);
        }
        for material in &mut input.materials {
            material.yield_strength /= length_scale.powi(2);
            for fiber in &mut material.section_fibers {
                fiber.y *= length_scale;
                fiber.area *= length_scale.powi(2);
                fiber.initial_axial_stress /= length_scale.powi(2);
            }
            for phase in &mut material.fiber_materials {
                phase.youngs_modulus /= length_scale.powi(2);
                phase.yield_strength /= length_scale.powi(2);
            }
        }
        let error = solve_frame_2d_material_p_delta(&input).expect_err("unbalanced initial moment");
        assert!(
            error.contains("not self-equilibrated on free DOFs"),
            "{error}"
        );
    }
}

#[test]
fn mixed_self_equilibrated_fibers_preserve_zero_load_and_ignore_parent_strength() {
    let mut input = fiber_model([-1e6, 1e6, 1e6, -1e6]);
    let reference = solve_frame_2d_material_p_delta(&request(input.clone())).unwrap();
    assert!(reference.stability_result.converged);
    for material in input["materials"].as_array_mut().unwrap() {
        material["yield_strength"] = json!(1e30);
    }
    let actual = solve_frame_2d_material_p_delta(&request(input)).unwrap();
    assert!(actual.stability_result.converged);
    assert_eq!(
        serde_json::to_value(&actual.material_history).unwrap(),
        serde_json::to_value(&reference.material_history).unwrap()
    );
    for state in &actual.material_history[0].material_states {
        assert!(state.section_axial_force.unwrap().abs() < 1e-8);
        assert!(state.section_end_moment_i.unwrap().abs() < 1e-8);
        assert!(state.section_end_moment_j.unwrap().abs() < 1e-8);
        assert_eq!(state.equivalent_plastic_strain, 0.0);
    }
}

#[test]
fn rejected_initial_stress_does_not_poison_a_clean_replay() {
    let input = request(model());
    let reference = solve_frame_2d_material_p_delta(&input).unwrap();
    let mut invalid = model();
    invalid["materials"][0]["initial_axial_stress"] = json!(1e-6);
    assert_unbalanced(invalid);
    let replay = solve_frame_2d_material_p_delta(&input).unwrap();
    assert_eq!(
        serde_json::to_value(replay).unwrap(),
        serde_json::to_value(reference).unwrap()
    );
}

#[test]
fn support_balanced_prestress_keeps_nonzero_member_forces() {
    let mut input = model();
    let nodes = &mut input["stability"]["buckling"]["frame"]["nodes"];
    nodes[2]["fix_x"] = json!(true);
    nodes[2]["fix_y"] = json!(true);
    nodes[2]["fix_rz"] = json!(true);
    nodes[2]["load_y"] = json!(0.0);
    nodes[1]["load_y"] = json!(-2.5e6);
    for material in input["materials"].as_array_mut().unwrap() {
        material["initial_axial_stress"] = json!(5e6);
    }
    let result = solve_frame_2d_material_p_delta(&request(input)).unwrap();
    assert!(result.stability_result.converged);
    for state in &result.material_history[0].material_states {
        assert!((state.axial_stress - 5e6).abs() < 1e-2);
        assert_eq!(state.equivalent_plastic_strain, 0.0);
    }
}
