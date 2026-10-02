use super::*;
use crate::linear_algebra::add_at;
use crate::solver_control::{SolverControl, SolverStage, with_solver_observer};
use std::{cell::Cell, rc::Rc};

fn system(mass: [f64; 3]) -> ReducedSparseModalSystem {
    let mut stiffness = SparseMatrix::new(3);
    for i in 0..3 {
        for j in 0..3 {
            let coefficient = if i == j { 1.4 } else { -0.2 };
            add_at(
                &mut stiffness,
                i,
                j,
                mass[i].sqrt() * coefficient * mass[j].sqrt(),
            );
        }
    }
    reduce_sparse_modal_system(&stiffness, &mass, &[]).unwrap()
}

fn check_solution(inverse: &PreparedSpdSolver, rhs: &[f64; 3]) {
    let solution = inverse.solve(rhs).unwrap();
    assert_eq!(solution.len(), 3);
    for (value, input) in solution.iter().zip(rhs) {
        let expected = 0.625 * input + 0.125 * rhs.iter().sum::<f64>();
        assert!((value - expected).abs() < 1e-10);
    }
}

#[test]
fn inverse_mass_coordinates_do_not_make_a_well_conditioned_operator_singular() {
    for mass in [
        [1.0, 1.0, 1.0],
        [1e-24, 1.0, 1e24],
        [1e24, 1.0, 1e-24],
        [1e-240, 1.0, 1e240],
        [1e240, 1.0, 1e-240],
    ] {
        // A = 1.6 I - 0.2 uu^T has spectrum {1, 1.6, 1.6}; K = sqrt(M) A sqrt(M).
        let inverse = system(mass).prepare_normalized_inverse().unwrap();
        for rhs in [[1.0, 1.0, 1.0], [1.0, 0.0, 0.0], [1.0, -2.0, 3.0]] {
            check_solution(&inverse, &rhs);
        }
    }
}

#[test]
fn normalized_inverse_prepares_once_and_reuses_multiple_right_hand_sides() {
    let preparations = Rc::new(Cell::new(0));
    let observed = preparations.clone();
    with_solver_observer(
        &SolverControl::default(),
        move |point| {
            if point.stage == SolverStage::DenseFactor && point.completed_steps == 0 {
                observed.set(observed.get() + 1);
            }
        },
        || {
            let inverse = system([1e-24, 1.0, 1e24]).prepare_normalized_inverse()?;
            for rhs in [[1.0, 1.0, 1.0], [1.0, 0.0, 0.0], [1.0, -2.0, 3.0]] {
                check_solution(&inverse, &rhs);
            }
            Ok::<(), String>(())
        },
    )
    .unwrap();
    assert_eq!(preparations.get(), 1);
}

#[test]
fn a_cancelled_normalized_inverse_solve_does_not_poison_the_prepared_factor() {
    let inverse = system([1e-24, 1.0, 1e24])
        .prepare_normalized_inverse()
        .unwrap();
    let control = SolverControl::default();
    let cancel = control.clone();
    let error = with_solver_observer(
        &control,
        move |point| {
            if point.stage == SolverStage::DenseSubstitution && point.completed_steps == 1 {
                cancel.request_cancel();
            }
        },
        || {
            let result = inverse.solve(&[1.0, 0.0, 0.0]);
            assert!(result.is_err());
            result
        },
    )
    .unwrap_err();
    assert!(error.contains("cancel"), "{error}");
    check_solution(&inverse, &[1.0, 0.0, 0.0]);
}

#[test]
fn normalized_stiffness_preserves_couplings_after_avoidable_intermediate_underflow() {
    let mut stiffness = SparseMatrix::new(2);
    for (i, j, value) in [
        (0, 0, 1e200),
        (1, 1, 1e-200),
        (0, 1, -1e-300),
        (1, 0, -1e-300),
    ] {
        add_at(&mut stiffness, i, j, value);
    }
    let operator = SparseMassNormalizedOperator::new(&stiffness, &[1e200, 1e-200]).unwrap();
    let normalized = operator.normalized_stiffness().unwrap();
    for row in 0..2 {
        let value = normalized
            .row_entries(row)
            .iter()
            .find(|(column, _)| *column != row)
            .unwrap()
            .1;
        assert!((value / -1e-300 - 1.0).abs() < 1e-14);
    }
}

#[test]
fn unrepresentable_normalized_entries_fail_without_silently_cutting_couplings() {
    for mass in [[1e100, 1e100], [1e-320, 1e-320]] {
        let mut stiffness = SparseMatrix::new(2);
        for (i, j, value) in [(0, 0, 1.0), (1, 1, 1.0), (0, 1, 1e-300), (1, 0, 1e-300)] {
            add_at(&mut stiffness, i, j, value);
        }
        let system = reduce_sparse_modal_system(&stiffness, &mass, &[]).unwrap();
        let error = match system.prepare_normalized_inverse() {
            Ok(_) => panic!("unrepresentable normalized operator must not be prepared"),
            Err(error) => error,
        };
        assert!(error.contains("normalized stiffness"), "{error}");
    }
    check_solution(
        &system([1.0, 1.0, 1.0])
            .prepare_normalized_inverse()
            .unwrap(),
        &[1.0; 3],
    );
}
