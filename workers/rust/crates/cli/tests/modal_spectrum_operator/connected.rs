use super::planned_solve;
use kyuubiki_solver::solver_control::{SolverControl, SolverStage, with_solver_observer};
use serde_json::{Value, json};
use std::{cell::Cell, rc::Rc};

const ROW_LENGTH: usize = 129;

fn model(count: usize) -> Value {
    model_with_masses(count, ROW_LENGTH, [1.0, 1.0])
}

fn model_with_masses(count: usize, row_length: usize, masses: [f64; 2]) -> Value {
    let mut nodes = Vec::new();
    let mut elements = Vec::new();
    let member = |i: usize, j: usize, stiffness: f64, density: f64| {
        json!({
        "id":format!("member-{i}-{j}"), "node_i":i, "node_j":j, "area":1.0,
        "youngs_modulus":stiffness, "shear_modulus":0.4*stiffness,
        "torsion_constant":1.0/12.0, "moment_of_inertia_y":1.0/12.0,
        "moment_of_inertia_z":1.0/12.0, "density":density})
    };
    for (row, mass) in masses.iter().enumerate() {
        for position in 0..row_length {
            let root = nodes.len();
            for tip in [false, true] {
                nodes.push(json!({"id":format!("node-{}", nodes.len()),
                    "x":f64::from(tip), "y":position as f64, "z":row as f64,
                    "fix_x":!tip, "fix_y":true, "fix_z":true,
                    "fix_rx":true, "fix_ry":true, "fix_rz":true,
                    "load_x":0.0, "load_y":0.0, "load_z":0.0,
                    "moment_x":0.0, "moment_y":0.0, "moment_z":0.0}));
            }
            let degree = usize::from(position > 0) + usize::from(position + 1 < row_length);
            elements.push(member(
                root,
                root + 1,
                *mass,
                2.0 * mass - 0.1 * (degree + 1) as f64,
            ));
            if position > 0 {
                elements.push(member(root - 1, root + 1, 10_000.0 * mass, 0.1));
            }
            if row == 1 {
                elements.push(member(root + 1 - 2 * row_length, root + 1, 1e-4, 0.1));
            }
        }
    }
    json!({"nodes":nodes, "elements":elements, "mode_count":count})
}

fn check_result(result: &Value, count: usize) {
    check_rows(result, ROW_LENGTH, &[1.0, 1.0002][..count]);
}

fn check_rows(result: &Value, row_length: usize, expected: &[f64]) {
    let free = result["free_dofs"].as_array().unwrap();
    assert_eq!(free.len(), 2 * row_length);
    let modes = result["modes"].as_array().unwrap();
    assert_eq!(modes.len(), expected.len());
    for (index, (mode, reference)) in modes.iter().zip(expected).enumerate() {
        assert_eq!(mode["index"], json!(index));
        assert!(
            (mode["eigenvalue_rad_s_squared"].as_f64().unwrap() / reference - 1.0).abs() < 1e-8
        );
        let shape = mode["shape"].as_array().unwrap();
        assert_eq!(shape.len(), 4 * row_length * 6);
        let mut norm = 0.0;
        for (dof, value) in shape.iter().enumerate() {
            let value = value.as_f64().unwrap();
            assert!(value.is_finite());
            if !free.contains(&json!(dof)) {
                assert_eq!(value, 0.0);
            }
            norm += value * value;
        }
        assert!((norm - 1.0).abs() < 1e-10);
        assert!(
            (mode["natural_frequency_hz"].as_f64().unwrap() * mode["period_s"].as_f64().unwrap()
                - 1.0)
                .abs()
                < 1e-12
        );
    }
}

#[test]
fn headless_connected_sparse_mode_matches_the_independent_row_pair_reference() {
    let visited = Rc::new(Cell::new(false));
    let observed = visited.clone();
    let result = with_solver_observer(
        &SolverControl::default(),
        move |point| {
            if point.stage == SolverStage::ModalIteration {
                observed.set(true);
            }
        },
        || planned_solve(true, model(1)),
    )
    .unwrap();
    assert!(visited.get());
    check_result(&result, 1);
    check_result(&planned_solve(true, model(2)).unwrap(), 2);
}

#[test]
fn headless_connected_projection_cancellation_returns_no_partial_mode_and_replays() {
    let input = model(1);
    let control = SolverControl::default();
    let cancel = control.clone();
    let error = with_solver_observer(
        &control,
        move |point| {
            if point.stage == SolverStage::ModalSweep {
                cancel.request_cancel();
            }
        },
        || {
            let result = planned_solve(true, input.clone());
            assert!(
                result.is_err(),
                "Engine must not publish an unvalidated sparse mode"
            );
            result
        },
    )
    .unwrap_err();
    assert!(error.contains("cancel"), "{error}");
    check_result(&planned_solve(true, input).unwrap(), 1);
}

#[test]
fn headless_connected_unequal_mass_mode_uses_one_ic0_factor_without_dense_fallback() {
    let counts = Rc::new(Cell::new([0; 3]));
    let observed = counts.clone();
    let result = with_solver_observer(
        &SolverControl::default(),
        move |point| {
            if point.completed_steps == 0 {
                let mut values = observed.get();
                match point.stage {
                    SolverStage::IncompleteCholeskyFactor => values[0] += 1,
                    SolverStage::SparseIteration => values[1] += 1,
                    SolverStage::DenseFactor => values[2] += 1,
                    _ => (),
                }
                observed.set(values);
            }
        },
        || planned_solve(true, model_with_masses(1, 513, [1.0, 4.0])),
    )
    .unwrap();
    assert_eq!(counts.get()[0], 1);
    assert!(counts.get()[1] > 1);
    assert_eq!(counts.get()[2], 0);
    check_rows(&result, 513, &[1.0]);
}

#[test]
fn headless_sparse_mass_inverse_cancels_inside_factor_and_iteration_then_replays() {
    for (stage, boundary) in [
        (SolverStage::IncompleteCholeskyFactor, 64),
        (SolverStage::SparseIteration, 1),
    ] {
        let control = SolverControl::default();
        let cancel = control.clone();
        let error = with_solver_observer(
            &control,
            move |point| {
                if point.stage == stage && point.completed_steps == boundary {
                    cancel.request_cancel();
                }
            },
            || {
                let result = planned_solve(true, model_with_masses(1, 513, [1.0, 4.0]));
                assert!(result.is_err(), "Engine must not return a partial mode");
                result
            },
        )
        .unwrap_err();
        assert!(error.contains("cancel"), "{error}");
        check_rows(
            &planned_solve(true, model_with_masses(1, 513, [1.0, 4.0])).unwrap(),
            513,
            &[1.0],
        );
    }
}
