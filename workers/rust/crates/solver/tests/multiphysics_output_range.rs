use kyuubiki_solver::solver_control::{SolverControl, SolverStage, with_solver_observer};
use kyuubiki_solver::*;
use serde_json::{Value, json};

fn square_nodes() -> Vec<Value> {
    [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]]
        .into_iter()
        .enumerate()
        .map(|(i, [x, y])| json!({"id":format!("n{i}"), "x":x, "y":y}))
        .collect()
}

fn plane_model(quad: bool, modulus: f64) -> Value {
    let nodes: Vec<_> = square_nodes()
        .into_iter()
        .map(|mut node| {
            let right = node["x"] == 1.0;
            node["fix_x"] = json!(!right);
            node["fix_y"] = json!(true);
            node["load_x"] = json!(if right { 0.0005 * modulus } else { 0.0 });
            node["load_y"] = json!(0.0);
            node
        })
        .collect();
    let mut element = json!({"id":"bulk", "node_i":0, "node_j":1, "node_k":2,
        "thickness":1.0, "youngs_modulus":modulus, "poisson_ratio":0.0});
    let elements = if quad {
        element["node_l"] = json!(3);
        vec![element]
    } else {
        let mut second = element.clone();
        second["id"] = json!("upper");
        second["node_j"] = json!(2);
        second["node_k"] = json!(3);
        vec![element, second]
    };
    json!({"nodes":nodes, "elements":elements})
}

fn plane_solve(quad: bool, input: Value) -> Result<Value, String> {
    if quad {
        solve_plane_quad_2d(&serde_json::from_value(input).unwrap())
            .map(|r| serde_json::to_value(r).unwrap())
    } else {
        solve_plane_triangle_2d(&serde_json::from_value(input).unwrap())
            .map(|r| serde_json::to_value(r).unwrap())
    }
}

fn close(actual: &Value, expected: f64) {
    let actual = actual
        .as_f64()
        .expect("successful physical output must remain numeric");
    assert!(
        actual.is_finite() && (actual / expected - 1.0).abs() < 1e-10,
        "{actual:e} != {expected:e}"
    );
}

fn numeric_output(value: &Value) {
    match value {
        Value::Null => panic!("a physical result must not hide a non-finite float as null"),
        Value::Array(values) => values.iter().for_each(numeric_output),
        Value::Object(fields) => {
            for (key, field) in fields {
                if key != "input" {
                    numeric_output(field);
                }
            }
        }
        _ => {}
    }
}

#[test]
fn mechanical_plane_uniaxial_patch_preserves_stress_and_energy_across_scales() {
    for quad in [false, true] {
        for modulus in [1e-200, 1.0, 1e200] {
            let result = plane_solve(quad, plane_model(quad, modulus)).unwrap();
            numeric_output(&result);
            close(&result["max_displacement"], 1e-3);
            close(&result["max_stress"], 1e-3 * modulus);
            close(&result["total_strain_energy"], 5e-7 * modulus);
            for element in result["elements"].as_array().unwrap() {
                close(&element["stress_x"], 1e-3 * modulus);
                close(&element["von_mises"], 1e-3 * modulus);
                close(&element["principal_stress_1"], 1e-3 * modulus);
                close(&element["max_in_plane_shear"], 5e-4 * modulus);
            }
        }
    }
}

#[test]
fn mechanical_plane_unrepresentable_energy_is_rejected_before_success() {
    for quad in [false, true] {
        let mut input = plane_model(quad, 1.0);
        for node in input["nodes"].as_array_mut().unwrap() {
            node["load_x"] = json!(node["load_x"].as_f64().unwrap() * 1e163);
        }
        let error = plane_solve(quad, input).unwrap_err();
        assert!(
            error.contains("plane") && error.contains("representable"),
            "{error}"
        );
        assert!(plane_solve(quad, plane_model(quad, 1.0)).is_ok());
    }
}

fn solid_model(modulus: f64) -> Value {
    let nodes: Vec<_> = [
        [0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.0, 0.0, 1.0],
    ]
    .into_iter()
    .enumerate()
    .map(|(i, [x, y, z])| {
        json!({
            "id":format!("n{i}"), "x":x, "y":y, "z":z,
            "fix_x":true, "fix_y":true, "fix_z":i != 3,
            "load_x":0.0, "load_y":0.0,
            "load_z":if i == 3 { modulus * (1e-3 / 6.0) } else { 0.0 }
        })
    })
    .collect();
    json!({"nodes":nodes, "elements":[{
        "id":"bulk", "node_a":0, "node_b":1, "node_c":2, "node_d":3,
        "youngs_modulus":modulus, "poisson_ratio":0.0
    }]})
}

fn solid_solve(input: Value) -> Result<Value, String> {
    solve_solid_tetra_3d(&serde_json::from_value(input).unwrap())
        .map(|r| serde_json::to_value(r).unwrap())
}

#[test]
fn mechanical_solid_tetra_uniaxial_patch_preserves_equivalent_stress_across_scales() {
    for modulus in [1e-200, 1.0, 1e200] {
        let result = solid_solve(solid_model(modulus)).unwrap();
        numeric_output(&result);
        close(
            &result["equilibrium"]["applied_force_scale"],
            modulus * (1e-3 / 6.0),
        );
        close(&result["max_displacement"], 1e-3);
        close(&result["max_von_mises_stress"], 1e-3 * modulus);
        close(&result["total_strain_energy"], (5e-7 / 6.0) * modulus);
    }
}

#[test]
fn mechanical_solid_unrepresentable_energy_is_rejected_and_replay_is_clean() {
    let mut input = solid_model(1.0);
    input["nodes"][3]["load_z"] = json!(1e160 / 6.0);
    let error = solid_solve(input).unwrap_err();
    assert!(
        error.contains("bulk") && error.contains("representable"),
        "{error}"
    );
    numeric_output(&solid_solve(solid_model(1.0)).unwrap());
}

#[test]
fn mechanical_result_stages_cancel_without_partial_success_and_replay_cleanly() {
    for kind in 0..3 {
        let solve = || match kind {
            0 => plane_solve(false, plane_model(false, 1.0)),
            1 => plane_solve(true, plane_model(true, 1.0)),
            _ => solid_solve(solid_model(1.0)),
        };
        let baseline = solve().unwrap();
        for stage in [
            SolverStage::ResultNodes,
            SolverStage::ResultElements,
            SolverStage::ResultTotals,
        ] {
            for terminal in [false, true] {
                let control = SolverControl::default();
                let cancel = control.clone();
                let result = with_solver_observer(
                    &control,
                    move |point| {
                        if point.stage == stage && (point.completed_steps > 0) == terminal {
                            cancel.request_cancel();
                        }
                    },
                    || {
                        let result = solve();
                        assert!(
                            result.is_err(),
                            "raw solver published partial results at {stage:?}"
                        );
                        result
                    },
                );
                assert!(result.unwrap_err().starts_with("solver cancelled"));
                assert!(control.was_interrupted());
                assert_eq!(solve().unwrap(), baseline);
            }
        }
    }
}

fn scalar_model(quad: bool, magnetic: bool, scale: f64) -> Value {
    let mut nodes = square_nodes();
    if !quad {
        nodes.pop();
    }
    for node in &mut nodes {
        node[if magnetic {
            "fix_vector_potential"
        } else {
            "fix_potential"
        }] = json!(true);
        node[if magnetic {
            "vector_potential"
        } else {
            "potential"
        }] = json!(2.0 * node["x"].as_f64().unwrap() - node["y"].as_f64().unwrap());
    }
    let mut element = json!({"id":"bulk", "node_i":0, "node_j":1, "node_k":2, "thickness":1.0});
    element[if magnetic {
        "permeability"
    } else {
        "permittivity"
    }] = json!(if magnetic { 1.0 / scale } else { scale });
    if quad {
        element["node_l"] = json!(3);
    }
    json!({"nodes":nodes, "elements":[element]})
}

#[test]
fn electromagnetic_constitutive_vector_magnitudes_preserve_representable_scales() {
    for quad in [false, true] {
        for magnetic in [false, true] {
            for scale in [1e-200, 1.0, 1e200] {
                let input = scalar_model(quad, magnetic, scale);
                macro_rules! run {
                    ($solver:path) => {
                        serde_json::to_value(
                            $solver(&serde_json::from_value(input).unwrap()).unwrap(),
                        )
                        .unwrap()
                    };
                }
                let result = match (quad, magnetic) {
                    (false, false) => run!(solve_electrostatic_plane_triangle_2d),
                    (true, false) => run!(solve_electrostatic_plane_quad_2d),
                    (false, true) => run!(solve_magnetostatic_plane_triangle_2d),
                    (true, true) => run!(solve_magnetostatic_plane_quad_2d),
                };
                let key = if magnetic {
                    "magnetic_field_strength_magnitude"
                } else {
                    "electric_flux_density_magnitude"
                };
                numeric_output(&result);
                close(&result["elements"][0][key], 5.0_f64.sqrt() * scale);
                close(
                    &result["total_stored_energy"],
                    2.5 * scale * if quad { 1.0 } else { 0.5 },
                );
            }
        }
    }
}

#[test]
fn stokes_uniform_velocity_magnitude_and_reynolds_number_survive_scale_changes() {
    for quad in [false, true] {
        for speed in [1e-200, 1.0, 1e200] {
            let mut nodes = square_nodes();
            if !quad {
                nodes.pop();
            }
            for node in &mut nodes {
                node["fix_velocity_x"] = json!(true);
                node["fix_velocity_y"] = json!(true);
                node["fix_pressure"] = json!(true);
                node["velocity_x"] = json!(speed);
                node["velocity_y"] = json!(2.0 * speed);
                node["pressure"] = json!(0.0);
            }
            let density = if speed > 1.0 { 1e-201 } else { 0.1 };
            let mut element = json!({"id":"bulk", "node_i":0, "node_j":1, "node_k":2,
                "thickness":1.0, "viscosity":1.0, "density":density});
            if quad {
                element["node_l"] = json!(3);
            }
            let input = json!({"nodes":nodes, "elements":[element]});
            let result = if quad {
                serde_json::to_value(
                    solve_stokes_flow_plane_quad_2d(&serde_json::from_value(input).unwrap())
                        .unwrap(),
                )
                .unwrap()
            } else {
                serde_json::to_value(
                    solve_stokes_flow_plane_triangle_2d(&serde_json::from_value(input).unwrap())
                        .unwrap(),
                )
                .unwrap()
            };
            let magnitude = 5.0_f64.sqrt() * speed;
            numeric_output(&result);
            for key in ["shear_rate", "viscous_dissipation", "divergence_error"] {
                assert_eq!(
                    result["elements"][0][key].as_f64(),
                    Some(0.0),
                    "{key}: {result}"
                );
            }
            close(&result["max_velocity"], magnitude);
            close(
                &result["elements"][0]["average_velocity_magnitude"],
                magnitude,
            );
            close(
                &result["max_reynolds_number"],
                density * magnitude * if quad { 1.0 } else { 0.5_f64.sqrt() },
            );
        }
    }
}
