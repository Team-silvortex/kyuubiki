use super::*;
use kyuubiki_solver::solver_control::{SolverControl, SolverStage, with_solver_observer};
use std::{cell::Cell, rc::Rc};

const SEGMENTS: usize = 129;

fn model(space: bool, scale: f64) -> Value {
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
            let mut element = input["elements"][0].clone();
            element["id"] = json!(format!("beam-{index}"));
            element["node_i"] = json!(index);
            element["node_j"] = json!(index + 1);
            element
        })
        .collect();
    input["nodes"] = json!(nodes);
    input["elements"] = json!(elements);
    input
}

fn check_published_chain(space: bool, result: &Value) {
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
    for (index, &value) in shape.iter().enumerate() {
        assert!(value.is_finite());
        if index < dofs || index % dofs != 0 {
            assert_eq!(value, 0.0);
        }
    }
    let expected = 4.0
        * (std::f64::consts::PI / (4.0 * SEGMENTS as f64))
            .sin()
            .powi(2);
    let lambda = eigenvalue(result);
    assert!((lambda / expected - 1.0).abs() < 1e-12);
    assert!((shape.iter().map(|value| value * value).sum::<f64>() - 1.0).abs() < 1e-10);
    let mut residual = 0.0;
    let mut force_norm = 0.0;
    let mut mass_norm = 0.0;
    for node in 1..=SEGMENTS {
        let current = shape[node * dofs];
        let left = shape[(node - 1) * dofs];
        let (stiffness, weighted_mass) = if node == SEGMENTS {
            (current - left, 0.5 * current)
        } else {
            (2.0 * current - left - shape[(node + 1) * dofs], current)
        };
        residual += (stiffness - lambda * weighted_mass).powi(2);
        force_norm += stiffness.powi(2);
        mass_norm += (lambda * weighted_mass).powi(2);
    }
    // Cancel the common physical scale analytically in this independent K*phi-lambda*M*phi check.
    let relative = residual.sqrt() / force_norm.sqrt().max(mass_norm.sqrt());
    assert!(relative < 1e-9, "published axial residual={relative:e}");
}

fn cancel_and_replay(space: bool) {
    for scale in [2.0_f64.powi(-600), 2.0_f64.powi(600)] {
        let input = model(space, scale);
        for occurrence in [1, 2] {
            let control = SolverControl::default();
            let cancel = control.clone();
            let seen = Rc::new(Cell::new(0));
            let observed = seen.clone();
            let error = with_solver_observer(
                &control,
                move |point| {
                    assert_ne!(
                        point.stage,
                        SolverStage::ModalValidation,
                        "must stop during the first product"
                    );
                    if point.stage == SolverStage::SparseMatvec && point.completed_steps == 64 {
                        observed.set(observed.get() + 1);
                        if observed.get() == occurrence {
                            cancel.request_cancel();
                        }
                    }
                },
                || {
                    let result = solve(space, input.clone());
                    assert!(
                        result.is_err(),
                        "no successful result may escape before scope exit"
                    );
                    result.map(|_| ())
                },
            )
            .expect_err("the public solver must report cancellation");
            assert_eq!(seen.get(), occurrence);
            assert!(error.contains("cancel"), "{error}");
            check_published_chain(space, &solve(space, input.clone()).unwrap());
        }
    }
}

#[test]
fn planar_extreme_scale_sparse_products_cancel_then_replay_physical_modes() {
    cancel_and_replay(false);
}

#[test]
fn spatial_extreme_scale_sparse_products_cancel_then_replay_physical_modes() {
    cancel_and_replay(true);
}
