use super::PrescribedSystemValidator;
use crate::linear_algebra::{SparseMatrix, add_at, reduce_sparse_system, solve_spd_system};
use crate::solver_control::{SolverControl, SolverStage, with_solver_observer};

fn spring(scale: f64) -> SparseMatrix {
    let mut matrix = SparseMatrix::new(2);
    for (row, column, value) in [(0, 0, scale), (0, 1, -scale), (1, 0, -scale), (1, 1, scale)] {
        add_at(&mut matrix, row, column, value);
    }
    matrix
}

#[test]
fn homogeneous_constraints_check_original_spring_equilibrium_and_zero_identity() {
    for power in [-600, 0, 600] {
        let scale = 2.0_f64.powi(power);
        let matrix = spring(scale);
        for sign in [-1.0, 1.0] {
            let rhs = [17.0 * scale, sign * scale];
            let (reduced, force, free) = reduce_sparse_system(&matrix, &rhs, &[0]).unwrap();
            let solved = solve_spd_system(&reduced, &force).unwrap();
            assert_eq!(free, [1]);
            assert_eq!(solved, [sign]);
            let gate = PrescribedSystemValidator::new_homogeneous(&matrix, &[0]).unwrap();
            gate.validate(&rhs, &[-0.0, sign]).unwrap();
            assert!(
                gate.validate(&rhs, &[f64::from_bits(1), sign])
                    .unwrap_err()
                    .contains("prescribed value changed")
            );
            assert!(
                gate.validate(&rhs, &[0.0, 2.0 * sign])
                    .unwrap_err()
                    .contains("row 1")
            );
            assert!(
                gate.validate(&[17.0 * scale, 2.0 * sign * scale], &[0.0, sign])
                    .is_err()
            );
        }
    }
}

#[test]
fn homogeneous_constraints_match_explicit_zero_prescriptions_including_reactions() {
    for constrained in [vec![], vec![0], vec![0, 0], vec![0, 1], vec![1, 0, 1]] {
        let matrix = spring(1.0);
        let explicit = constrained
            .iter()
            .map(|&index| (index, 0.0))
            .collect::<Vec<_>>();
        let zero = PrescribedSystemValidator::new_homogeneous(&matrix, &constrained).unwrap();
        let reference = PrescribedSystemValidator::new(&matrix, &explicit).unwrap();
        for (rhs, solution) in [
            ([0.0; 2], [0.0; 2]),
            ([3.0, -3.0], [0.0; 2]),
            ([-1.0, 1.0], [0.0, 1.0]),
            ([0.0; 2], [f64::from_bits(1), 0.0]),
        ] {
            assert_eq!(
                zero.validate(&rhs, &solution).is_ok(),
                reference.validate(&rhs, &solution).is_ok()
            );
        }
        if constrained.contains(&0) && constrained.contains(&1) {
            zero.validate(&[100.0, -100.0], &[0.0; 2]).unwrap();
        }
    }
    let empty = SparseMatrix::new(0);
    PrescribedSystemValidator::new_homogeneous(&empty, &[])
        .unwrap()
        .validate(&[], &[])
        .unwrap();
}

#[test]
fn homogeneous_constraints_do_not_hide_wrong_weak_dofs_with_fixed_zero_roundoff() {
    for (coefficient, wrong) in [(4.0, f64::from_bits(1)), (1.0e308, 1.0e-16)] {
        let mut matrix = SparseMatrix::new(2);
        add_at(&mut matrix, 0, 0, 1.0);
        add_at(&mut matrix, 0, 1, coefficient);
        add_at(&mut matrix, 1, 1, 1.0);
        let gate = PrescribedSystemValidator::new_homogeneous(&matrix, &[1]).unwrap();
        assert!(
            gate.validate(&[0.0; 2], &[wrong, 0.0])
                .unwrap_err()
                .contains("row 0")
        );
        gate.validate(&[0.0; 2], &[0.0; 2]).unwrap();
    }
}

#[test]
fn homogeneous_constraints_validate_full_input_and_out_of_range_supports() {
    let matrix = spring(1.0);
    for constrained in [vec![2], vec![usize::MAX], vec![0, 0, 2]] {
        assert!(PrescribedSystemValidator::new_homogeneous(&matrix, &constrained).is_err());
    }
    let gate = PrescribedSystemValidator::new_homogeneous(&matrix, &[0, 1]).unwrap();
    assert!(gate.validate(&[f64::INFINITY, 0.0], &[0.0; 2]).is_err());
    assert!(gate.validate(&[0.0; 2], &[0.0, f64::NAN]).is_err());
    assert!(gate.validate(&[0.0], &[0.0; 2]).is_err());
    for entries in [
        vec![(2, 1.0)],
        vec![(1, 1.0), (0, 1.0)],
        vec![(0, f64::NAN)],
    ] {
        let mut matrix = spring(1.0);
        matrix.rows[0] = entries;
        assert!(PrescribedSystemValidator::new_homogeneous(&matrix, &[0, 1]).is_err());
    }
}

#[test]
fn homogeneous_constraints_cancel_long_support_index_then_replay() {
    let matrix = spring(1.0);
    let fixed = vec![0; 2048];
    for step in [64, 2048] {
        let control = SolverControl::default();
        let cancel = control.clone();
        let result = with_solver_observer(
            &control,
            move |point| {
                if point.stage == SolverStage::ConstraintIndex && point.completed_steps == step {
                    cancel.request_cancel();
                }
            },
            || PrescribedSystemValidator::new_homogeneous(&matrix, &fixed).map(|_| ()),
        );
        assert!(result.unwrap_err().contains("cancelled"));
        assert!(control.was_interrupted());
        PrescribedSystemValidator::new_homogeneous(&matrix, &fixed)
            .unwrap()
            .validate(&[0.0; 2], &[0.0; 2])
            .unwrap();
    }
}

#[test]
fn homogeneous_constraints_cancel_wide_original_rows_then_replay() {
    let mut matrix = SparseMatrix::new(2049);
    add_at(&mut matrix, 0, 0, 1.0);
    for column in 1..2049 {
        add_at(&mut matrix, 0, column, 1.0e308);
    }
    let fixed = (1..2049).collect::<Vec<_>>();
    let gate = PrescribedSystemValidator::new_homogeneous(&matrix, &fixed).unwrap();
    let mut rhs = vec![0.0; 2049];
    rhs[0] = 1.0;
    let solution = rhs.clone();
    for (stage, step) in [
        (SolverStage::ConstraintValidateRow, 1024),
        (SolverStage::ConstraintValidate, 4098),
    ] {
        let control = SolverControl::default();
        let cancel = control.clone();
        let result = with_solver_observer(
            &control,
            move |point| {
                if point.stage == stage && point.completed_steps == step {
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
