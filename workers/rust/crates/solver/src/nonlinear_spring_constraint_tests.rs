use super::*;
use crate::solver_control::{SolverControl, with_solver_observer};
use kyuubiki_protocol::NonlinearSpring1dNodeInput;

fn nodes() -> Vec<NonlinearSpring1dNodeInput> {
    (0..3)
        .map(|index| NonlinearSpring1dNodeInput {
            id: format!("n{index}"),
            x: index as f64,
            fix_x: index != 1,
            load_x: 0.0,
        })
        .collect()
}

fn elements() -> Vec<NonlinearSpring1dElementInput> {
    vec![NonlinearSpring1dElementInput {
        id: "spring".into(),
        node_i: 0,
        node_j: 1,
        stiffness: 2.0,
        cubic_stiffness: 1.0,
    }]
}

fn evaluate(displacement: &[f64]) -> Result<SpringEquilibrium, String> {
    SpringStep {
        nodes: &nodes(),
        elements: &elements(),
        contacts: &[],
        load_factor: 1.0,
        tolerance: 1e-12,
    }
    .evaluate(displacement)
}

#[test]
fn newton_original_state_rejects_fixed_drift_even_with_zero_free_residual() {
    for value in [f64::from_bits(1), -f64::from_bits(1), 1.0, -1.0] {
        let error = evaluate(&[value, value, 0.0])
            .err()
            .expect("fixed drift must not converge");
        assert!(error.contains("fixed value changed"), "{error}");
    }
    assert_eq!(evaluate(&[-0.0, 0.0, 0.0]).unwrap().norm, 0.0);
}

#[test]
fn newton_original_state_checks_dimensions_including_isolated_fixed_nodes() {
    for displacement in [vec![0.0; 2], vec![0.0; 4]] {
        let error = evaluate(&displacement)
            .err()
            .expect("complete dimensions are required");
        assert!(error.contains("dimensions"), "{error}");
    }
    assert_eq!(evaluate(&[0.0; 3]).unwrap().norm, 0.0);
}

#[test]
fn newton_original_state_checks_nonfinite_values_on_unloaded_fixed_nodes() {
    for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        let error = evaluate(&[0.0, 0.0, value])
            .err()
            .expect("hidden state must be finite");
        assert!(error.contains("non-finite displacement"), "{error}");
    }
    assert_eq!(evaluate(&[0.0; 3]).unwrap().norm, 0.0);
}

fn linearized(scale: f64, contact: bool) -> SpringEquilibrium {
    let mut nodes = nodes();
    nodes[1].load_x = if contact { 35.0 } else { 32.0 } * scale;
    nodes[2].fix_x = false;
    nodes[2].load_x = -27.75 * scale;
    let elements = [
        NonlinearSpring1dElementInput {
            id: "first".into(),
            node_i: 0,
            node_j: 1,
            stiffness: 2.0 * scale,
            cubic_stiffness: scale,
        },
        NonlinearSpring1dElementInput {
            id: "second".into(),
            node_i: 1,
            node_j: 2,
            stiffness: 3.0 * scale,
            cubic_stiffness: 2.0 * scale,
        },
    ];
    let contacts = if contact {
        vec![ContactGap1dContactInput {
            id: "stop".into(),
            node: 1,
            gap: 0.5,
            normal_stiffness: 4.0 * scale,
        }]
    } else {
        vec![]
    };
    SpringStep {
        nodes: &nodes,
        elements: &elements,
        contacts: &contacts,
        load_factor: 1.0,
        tolerance: 1e-12 * scale,
    }
    .evaluate(&[0.0, 1.0, -0.5])
    .unwrap()
}

#[test]
fn newton_original_increment_checks_current_tangent_rhs_and_complete_reconstruction() {
    for exponent in [-600, 0, 600] {
        let scale = 2.0_f64.powi(exponent);
        for contact in [false, true] {
            let mut state = linearized(scale, contact);
            let (reduced, rhs, free) =
                reduce_sparse_system(&state.tangent, &state.residual, &[0]).unwrap();
            let delta = solve_chain_tangent(&reduced, &rhs).unwrap();
            for (actual, expected) in delta.iter().zip([0.25, -0.75]) {
                assert!((actual - expected).abs() < 1e-14);
            }
            let mut scratch = [91.0; 3];
            state
                .validate_increment(&[0], &free, &delta, &mut scratch)
                .unwrap();
            assert_eq!(scratch[0], 0.0);
            assert!(
                state
                    .validate_increment(&[0], &[2, 1], &delta, &mut scratch)
                    .unwrap_err()
                    .contains("row")
            );
            assert!(
                state
                    .validate_increment(&[0], &[0, 2], &delta, &mut scratch)
                    .unwrap_err()
                    .contains("prescribed value changed")
            );
            state
                .validate_increment(&[0], &free, &delta, &mut scratch)
                .unwrap();
            state.residual[1] += scale;
            assert!(
                state
                    .validate_increment(&[0], &free, &delta, &mut scratch)
                    .is_err()
            );
        }
    }
}

#[test]
fn newton_original_increment_accepts_an_invertible_indefinite_contact_switch() {
    let mut nodes = nodes();
    nodes[1].load_x = 2.0;
    nodes[2].fix_x = false;
    let elements = [
        NonlinearSpring1dElementInput {
            id: "softening".into(),
            node_i: 0,
            node_j: 1,
            stiffness: 2.0,
            cubic_stiffness: -1.0,
        },
        NonlinearSpring1dElementInput {
            id: "tail".into(),
            node_i: 1,
            node_j: 2,
            stiffness: 1.0,
            cubic_stiffness: 0.0,
        },
    ];
    let contacts = [ContactGap1dContactInput {
        id: "touching".into(),
        node: 1,
        gap: 1.0,
        normal_stiffness: 5.8125,
    }];
    let state = SpringStep {
        nodes: &nodes,
        elements: &elements,
        contacts: &contacts,
        load_factor: 1.0,
        tolerance: 1e-12,
    }
    .evaluate(&[0.0, 1.0, 1.0])
    .unwrap();
    let (reduced, rhs, free) = reduce_sparse_system(&state.tangent, &state.residual, &[0]).unwrap();
    let delta = solve_chain_tangent(&reduced, &rhs).unwrap();
    assert_eq!(delta, [-1.0, -1.0]);
    state
        .validate_increment(&[0], &free, &delta, &mut [0.0; 3])
        .unwrap();
}

#[test]
fn newton_original_increment_rejects_invalid_dimensions_maps_and_hidden_operands() {
    let mut state = linearized(1.0, false);
    for (free, delta, mut scratch) in [
        (vec![1], vec![0.25, -0.75], vec![0.0; 3]),
        (vec![1, 2], vec![0.25, -0.75], vec![0.0; 2]),
        (vec![1, 3], vec![0.25, -0.75], vec![0.0; 3]),
        (vec![1, usize::MAX], vec![0.25, -0.75], vec![0.0; 3]),
        (vec![1, 2], vec![f64::NAN, -0.75], vec![0.0; 3]),
    ] {
        assert!(
            state
                .validate_increment(&[0], &free, &delta, &mut scratch)
                .is_err()
        );
    }
    state.residual[0] = f64::INFINITY;
    assert!(
        state
            .validate_increment(&[0], &[1, 2], &[0.25, -0.75], &mut [0.0; 3])
            .is_err()
    );
    state.residual[0] = 0.0;
    add_at(&mut state.tangent, 0, 0, f64::NAN);
    assert!(
        state
            .validate_increment(&[0], &[1, 2], &[0.25, -0.75], &mut [0.0; 3])
            .is_err()
    );
    linearized(1.0, false)
        .validate_increment(&[0], &[1, 2], &[0.25, -0.75], &mut [0.0; 3])
        .unwrap();
}

#[test]
fn newton_original_increment_cancels_scratch_index_wide_rows_and_completion_then_replays() {
    let size = 2050;
    let mut tangent = SparseMatrix::new(size);
    for column in 0..size {
        add_at(
            &mut tangent,
            1,
            column,
            if column == 1 { (size - 1) as f64 } else { -1.0 },
        );
        if column != 1 {
            add_at(&mut tangent, column, column, 1.0);
        }
    }
    let mut residual = vec![0.0; size];
    residual[1] = (size - 1) as f64;
    let state = SpringEquilibrium {
        tangent,
        residual,
        norm: (size - 1) as f64,
    };
    let constrained = (0..size).filter(|&dof| dof != 1).collect::<Vec<_>>();
    let mut scratch = vec![91.0; size];
    for (stage, completed) in [
        (SolverStage::ResultFreeDofs, 1024),
        (SolverStage::ConstraintIndex, 64),
        (SolverStage::ConstraintValidate, 0),
        (SolverStage::ConstraintValidate, 64),
        (SolverStage::ConstraintValidateRow, 1024),
        (SolverStage::ConstraintValidate, 2 * size as u64),
    ] {
        let control = SolverControl::default();
        let cancel = control.clone();
        let result = with_solver_observer(
            &control,
            move |point| {
                if point.stage == stage && point.completed_steps == completed {
                    cancel.request_cancel();
                }
            },
            || state.validate_increment(&constrained, &[1], &[1.0], &mut scratch),
        );
        assert!(result.unwrap_err().contains("cancelled"), "{stage:?}");
        assert!(control.was_interrupted());
        state
            .validate_increment(&constrained, &[1], &[1.0], &mut scratch)
            .unwrap();
        assert_eq!(scratch[1], 1.0);
        assert!(
            scratch
                .iter()
                .enumerate()
                .all(|(dof, &value)| dof == 1 || value == 0.0)
        );
    }
}
