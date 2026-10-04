use super::*;
use kyuubiki_solver::solver_control::{SolverControl, SolverStage, with_solver_observer};
use std::{cell::Cell, rc::Rc};

const SEGMENTS: usize = 129;

#[path = "final_validation.rs"]
mod final_validation;

fn properties(index: usize, uniform: bool) -> (f64, f64) {
    if uniform {
        (1.0, 1.0)
    } else {
        (
            1.0 + 0.25 * (index % 3) as f64,
            1.0 + 0.125 * (index % 4) as f64,
        )
    }
}

fn model(space: bool, uniform: bool, scale: f64) -> Value {
    let mut input = axial_chain(space, [scale; 2], scale, 1);
    let nodes: Vec<_> = (0..=SEGMENTS)
        .map(|index| {
            let mut node = input["nodes"][usize::from(index > 0)].clone();
            node["id"] = json!(format!("node-{index}"));
            node["x"] = json!(index as f64);
            node
        })
        .collect();
    let elements: Vec<_> = (0..SEGMENTS)
        .map(|index| {
            let (modulus, density) = properties(index, uniform);
            let mut element = input["elements"][0].clone();
            element["id"] = json!(format!("beam-{index}"));
            element["node_i"] = json!(index);
            element["node_j"] = json!(index + 1);
            element["youngs_modulus"] = json!(modulus * scale);
            element["density"] = json!(density * scale);
            if space {
                element["shear_modulus"] = json!(0.4 * modulus * scale);
            }
            element
        })
        .collect();
    input["nodes"] = json!(nodes);
    input["elements"] = json!(elements);
    input
}

fn check_physical_mode(space: bool, uniform: bool, result: &Value) {
    let dofs = if space { 6 } else { 3 };
    assert_eq!(result["free_dofs"].as_array().unwrap().len(), SEGMENTS);
    assert_eq!(result["modes"].as_array().unwrap().len(), 1);
    let mode = &result["modes"][0];
    let shape: Vec<_> = mode["shape"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_f64().unwrap())
        .collect();
    assert_eq!(shape.len(), (SEGMENTS + 1) * dofs);
    let mut norm = 0.0_f64;
    for (index, &value) in shape.iter().enumerate() {
        assert!(value.is_finite());
        norm = norm.hypot(value);
        if index < dofs || index % dofs != 0 {
            assert_eq!(value, 0.0);
        }
    }
    assert!((norm - 1.0).abs() < 1e-10);
    assert!((mode["participation_norm"].as_f64().unwrap() - 1.0).abs() < 1e-10);
    let lambda = eigenvalue(result);
    assert!(lambda.is_finite() && lambda > 0.0);
    let frequency = mode["natural_frequency_hz"].as_f64().unwrap();
    let period = mode["period_s"].as_f64().unwrap();
    assert!((frequency * period - 1.0).abs() < 1e-12);
    assert!((frequency * std::f64::consts::TAU / lambda.sqrt() - 1.0).abs() < 1e-12);
    let uniform_lambda = 4.0
        * (std::f64::consts::PI / (4.0 * SEGMENTS as f64))
            .sin()
            .powi(2);
    if uniform {
        assert!((lambda / uniform_lambda - 1.0).abs() < 1e-12);
    } else {
        // Rayleigh bounds use the independent segment properties, not the solved matrix.
        assert!(lambda >= uniform_lambda / 1.375 * (1.0 - 1e-12));
        assert!(lambda <= 1.5 * uniform_lambda * (1.0 + 1e-12));
    }
    let mut residual = 0.0_f64;
    let mut stiffness_norm = 0.0_f64;
    let mut mass_norm = 0.0_f64;
    for node in 1..=SEGMENTS {
        let current = shape[node * dofs];
        let (left_stiffness, left_mass) = properties(node - 1, uniform);
        let mut force = left_stiffness * (current - shape[(node - 1) * dofs]);
        let mut mass = 0.5 * left_mass;
        if node < SEGMENTS {
            let (right_stiffness, right_mass) = properties(node, uniform);
            force += right_stiffness * (current - shape[(node + 1) * dofs]);
            mass += 0.5 * right_mass;
        }
        let inertia = lambda * mass * current;
        residual = residual.hypot(force - inertia);
        stiffness_norm = stiffness_norm.hypot(force);
        mass_norm = mass_norm.hypot(inertia);
    }
    // Remove only the common physical scale; preserve the actual heterogeneous field.
    let relative = residual / stiffness_norm.max(mass_norm);
    assert!(
        relative.is_finite() && relative < 1e-9,
        "physical residual={relative:e}"
    );
}

fn cancel_and_replay(space: bool) {
    for uniform in [false, true] {
        for scale in [2.0_f64.powi(-600), 1.0, 2.0_f64.powi(600)] {
            let input = model(space, uniform, scale);
            let stage = if uniform {
                SolverStage::SparseDiagonalMagnitude
            } else {
                SolverStage::SparseMatrixScale
            };
            let target = if uniform { 2 } else { 1 };
            let control = SolverControl::default();
            let cancel = control.clone();
            let seen = Rc::new(Cell::new(0));
            let observed = seen.clone();
            let error = with_solver_observer(
                &control,
                move |point| {
                    assert_ne!(
                        point.stage,
                        SolverStage::SparseMatvec,
                        "cancel before residual products"
                    );
                    assert_ne!(
                        point.stage,
                        SolverStage::ModalIteration,
                        "cancel before spectral search"
                    );
                    if point.stage == stage && point.completed_steps == 64 {
                        observed.set(observed.get() + 1);
                        if observed.get() == target {
                            cancel.request_cancel();
                        }
                    }
                },
                || {
                    let result = solve(space, input.clone());
                    assert!(result.is_err(), "cancel must propagate before scope exit");
                    result.map(|_| ())
                },
            )
            .unwrap_err();
            assert!(error.contains("cancel"), "{error}");
            assert_eq!(seen.get(), target);
            assert_eq!(control.last_checkpoint().unwrap().stage, stage);
            assert_eq!(control.last_checkpoint().unwrap().completed_steps, 64);
            let replay = solve(space, input.clone()).unwrap();
            check_physical_mode(space, uniform, &replay);
            let owned = if space {
                serde_json::to_value(
                    solve_modal_frame_3d_owned(serde_json::from_value(input).unwrap()).unwrap(),
                )
                .unwrap()
            } else {
                serde_json::to_value(
                    solve_modal_frame_2d_owned(serde_json::from_value(input).unwrap()).unwrap(),
                )
                .unwrap()
            };
            assert_eq!(owned, replay);
        }
    }
}

#[test]
fn planar_tridiagonal_preparation_cancels_then_replays_checked_physical_modes() {
    cancel_and_replay(false);
}

#[test]
fn spatial_tridiagonal_preparation_cancels_then_replays_checked_physical_modes() {
    cancel_and_replay(true);
}
