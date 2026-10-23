use super::PrescribedSystemValidator;
use crate::linear_algebra::{
    SparseMatrix, add_at, reduce_sparse_system_with_prescribed, solve_spd_system,
    sparse_solution_backward_error,
};
use crate::solver_control::{SolverControl, SolverStage, with_solver_observer};

fn diagonal_system() -> SparseMatrix {
    let mut matrix = SparseMatrix::new(3);
    for (row, value) in [2.0, 3.0, 5.0].into_iter().enumerate() {
        add_at(&mut matrix, row, row, value);
    }
    matrix
}

#[test]
fn original_constraints_reject_wrong_reconstruction_or_load_after_reduced_success() {
    let matrix = diagonal_system();
    let rhs = [2.0, 123.0, 15.0];
    let prescribed = [(1, 7.0)];
    let (reduced, reduced_rhs, free) =
        reduce_sparse_system_with_prescribed(&matrix, &rhs, &prescribed).unwrap();
    let solution = solve_spd_system(&reduced, &reduced_rhs).unwrap();
    assert_eq!(free, [0, 2]);
    assert_eq!(solution, [1.0, 3.0]);
    let gate = PrescribedSystemValidator::new(&matrix, &prescribed).unwrap();
    gate.validate(&rhs, &[1.0, 7.0, 3.0]).unwrap();
    assert!(
        gate.validate(&rhs, &[3.0, 7.0, 1.0])
            .unwrap_err()
            .contains("row 0")
    );
    assert!(
        gate.validate(&[2.0, 123.0, 16.0], &[1.0, 7.0, 3.0])
            .unwrap_err()
            .contains("row 2")
    );
    gate.validate(&rhs, &[1.0, 7.0, 3.0]).unwrap();
}

#[test]
fn original_constraints_check_fixed_identity_without_forbidding_reactions() {
    let matrix = diagonal_system();
    let q = f64::from_bits(1);
    for value in [q, -q, 1.0, 1.0e200] {
        let gate = PrescribedSystemValidator::new(&matrix, &[(1, value), (1, value)]).unwrap();
        gate.validate(&[2.0, -10.0, 15.0], &[1.0, value, 3.0])
            .unwrap();
        let changed = if value.abs() < 1.0 {
            0.0
        } else {
            value * (1.0 + f64::EPSILON)
        };
        assert!(
            gate.validate(&[2.0, -10.0, 15.0], &[1.0, changed, 3.0])
                .unwrap_err()
                .contains("prescribed value changed")
        );
    }
    let gate = PrescribedSystemValidator::new(&matrix, &[(0, -0.0), (1, 8.0), (2, 9.0)]).unwrap();
    gate.validate(&[500.0; 3], &[0.0, 8.0, 9.0]).unwrap();
    let empty = SparseMatrix::new(0);
    PrescribedSystemValidator::new(&empty, &[])
        .unwrap()
        .validate(&[], &[])
        .unwrap();
}

#[test]
fn original_constraints_fixed_inputs_cannot_lend_their_unknown_rounding_budget() {
    for (coefficient, wrong) in [(4.0, f64::from_bits(1)), (1.0e308, 1.0e-16)] {
        let mut matrix = SparseMatrix::new(2);
        add_at(&mut matrix, 0, 0, 1.0);
        add_at(&mut matrix, 0, 1, coefficient);
        add_at(&mut matrix, 1, 1, 1.0);
        let solution = [wrong, 0.0];
        assert_eq!(
            sparse_solution_backward_error(&matrix, &[0.0; 2], &solution, 1.0e-8).unwrap(),
            0.0
        );
        let gate = PrescribedSystemValidator::new(&matrix, &[(1, 0.0)]).unwrap();
        assert!(
            gate.validate(&[0.0; 2], &solution)
                .unwrap_err()
                .contains("row 0")
        );
        gate.validate(&[0.0; 2], &[0.0; 2]).unwrap();
    }
}

#[test]
fn original_constraints_keep_valid_quantized_unknowns_but_reject_erased_loads() {
    let q = f64::from_bits(1);
    let mut matrix = SparseMatrix::new(2);
    add_at(&mut matrix, 0, 0, 3.0);
    add_at(&mut matrix, 0, 1, -1.0);
    add_at(&mut matrix, 1, 1, 1.0);
    for sign in [-1.0, 1.0] {
        let gate = PrescribedSystemValidator::new(&matrix, &[(1, sign * q)]).unwrap();
        gate.validate(&[sign * 3.0 * q, 0.0], &[sign * q; 2])
            .unwrap();
    }
    let mut matrix = SparseMatrix::new(1);
    add_at(&mut matrix, 0, 0, 4.0);
    let gate = PrescribedSystemValidator::new(&matrix, &[]).unwrap();
    assert!(gate.validate(&[q], &[0.0]).is_err());
    gate.validate(&[4.0 * q], &[q]).unwrap();
}

#[test]
fn original_constraints_validate_hidden_data_dimensions_and_constraint_indices() {
    let matrix = diagonal_system();
    for fixed in [
        vec![(3, 0.0)],
        vec![(1, f64::NAN)],
        vec![(1, 1.0), (1, 2.0)],
    ] {
        assert!(PrescribedSystemValidator::new(&matrix, &fixed).is_err());
    }
    let gate = PrescribedSystemValidator::new(&matrix, &[(0, 0.0), (1, 0.0), (2, 0.0)]).unwrap();
    for (rhs, solution) in [
        (vec![], vec![0.0; 3]),
        (vec![0.0; 3], vec![0.0; 4]),
        (vec![0.0, f64::INFINITY, 0.0], vec![0.0; 3]),
        (vec![0.0; 3], vec![0.0, f64::NAN, 0.0]),
    ] {
        assert!(gate.validate(&rhs, &solution).is_err());
    }
    for entries in [
        vec![(3, 1.0)],
        vec![(0, 1.0), (0, 1.0)],
        vec![(1, 1.0), (0, 1.0)],
        vec![(0, f64::NAN)],
    ] {
        let mut matrix = diagonal_system();
        matrix.rows[0] = entries;
        assert!(PrescribedSystemValidator::new(&matrix, &[(0, 0.0)]).is_err());
    }
}

#[test]
fn original_constraints_reject_nonrepresentable_residual_and_alignment_loss() {
    let mut matrix = SparseMatrix::new(3);
    for (column, value) in [(0, 1.0), (1, 4.0), (2, -2.0)] {
        add_at(&mut matrix, 0, column, value);
    }
    let large = 2.0_f64.powi(1023);
    let gate = PrescribedSystemValidator::new(&matrix, &[(1, large), (2, large)]).unwrap();
    assert!(
        gate.validate(&[0.0; 3], &[0.0, large, large])
            .unwrap_err()
            .contains("representable range")
    );
    let mut matrix = SparseMatrix::new(2);
    add_at(&mut matrix, 0, 0, 0.5);
    add_at(&mut matrix, 0, 1, 1.0);
    let q = f64::from_bits(1);
    let gate = PrescribedSystemValidator::new(&matrix, &[(1, 1.0)]).unwrap();
    assert!(
        gate.validate(&[1.0, 0.0], &[q, 1.0])
            .unwrap_err()
            .contains("normalization")
    );
}

#[test]
fn original_constraints_wide_plain_and_range_validation_cancel_then_replay() {
    for range in [false, true] {
        let mut matrix = SparseMatrix::new(2049);
        add_at(&mut matrix, 0, 0, 1.0);
        for column in 1..2049 {
            add_at(&mut matrix, 0, column, if column == 2 { -2.0 } else { 2.0 });
        }
        let mut solution = vec![0.0; 2049];
        solution[0] = 1.0;
        if range {
            solution[1..3].fill(2.0_f64.powi(1023));
        }
        let prescribed = (1..2049)
            .map(|index| (index, solution[index]))
            .collect::<Vec<_>>();
        let gate = PrescribedSystemValidator::new(&matrix, &prescribed).unwrap();
        let mut rhs = vec![0.0; 2049];
        rhs[0] = 1.0;
        let mut points = vec![
            (SolverStage::ConstraintValidate, 64, 0),
            (SolverStage::ConstraintValidateRow, 1024, 1),
            (SolverStage::ConstraintValidate, 4098, 0),
        ];
        if range {
            points.extend([
                (SolverStage::ConstraintValidateRow, 1024, 2),
                (SolverStage::ConstraintValidateRow, 3073, 2),
            ]);
        }
        for (stage, step, phase) in points {
            let control = SolverControl::default();
            let cancel = control.clone();
            let entered = std::cell::Cell::new(0);
            let result = with_solver_observer(
                &control,
                move |point| {
                    if point.stage == SolverStage::ConstraintValidateRow
                        && point.completed_steps == 0
                    {
                        entered.set(entered.get() + 1);
                    }
                    if point.stage == stage
                        && point.completed_steps == step
                        && (phase == 0 || entered.get() == phase)
                    {
                        cancel.request_cancel();
                    }
                },
                || gate.validate(&rhs, &solution),
            );
            assert!(result.unwrap_err().contains("cancelled"));
            assert!(control.was_interrupted());
            gate.validate(&rhs, &solution).unwrap();
        }
    }
}
