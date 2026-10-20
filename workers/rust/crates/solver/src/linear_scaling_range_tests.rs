use super::*;
use crate::solver_control::{SolverControl, with_solver_observer};

fn power(exponent: i32) -> f64 {
    2.0_f64.powi(exponent)
}

#[test]
fn symmetric_scaling_preserves_couplings_across_intermediate_range_loss() {
    for (coefficient, row_scale, column_scale) in [
        (power(-1000), power(-500), power(500)),
        (1.25 * power(-1000), power(-74), power(74)),
        (power(1000), power(500), power(-500)),
    ] {
        for sign in [1.0, -1.0] {
            let coefficient = sign * coefficient;
            let mut matrix = SparseMatrix::new(2);
            add_at(&mut matrix, 0, 0, 1.0);
            add_at(&mut matrix, 1, 1, 1.0);
            add_at(&mut matrix, 0, 1, coefficient);
            add_at(&mut matrix, 1, 0, coefficient);
            let scales = [row_scale, column_scale];
            let scaled = scaling::scale_sparse_matrix(&matrix, &scales).unwrap();
            assert_eq!(
                scaled.row_entries(0).iter().find(|(col, _)| *col == 1),
                Some(&(1, coefficient)),
                "sparse scaling lost a representable coupling"
            );
            assert_eq!(scaled.row_entries(1)[0], (0, coefficient));

            let compressed = matrix
                .compress_scaled(&scales, SpdPreconditioner::Jacobi)
                .unwrap();
            assert_eq!(compressed.values[1], coefficient);
            assert_eq!(compressed.values[2], coefficient);
        }
    }
}

fn extreme_star(size: usize, strong: usize, weak: [usize; 3]) -> (SparseMatrix, Vec<f64>) {
    let mut matrix = SparseMatrix::new(size);
    let mut rhs = vec![0.0; size];
    for row in 0..size {
        add_at(
            &mut matrix,
            row,
            row,
            if weak.contains(&row) {
                power(-1000)
            } else {
                power(1000)
            },
        );
    }
    for row in weak {
        add_at(&mut matrix, strong, row, power(-1000));
        add_at(&mut matrix, row, strong, power(-1000));
        rhs[row] = 1.0;
    }
    rhs[strong] = 3.0;
    (matrix, rhs)
}

#[test]
fn representable_star_solution_survives_asymmetric_intermediate_underflow() {
    for preconditioner in [
        SpdPreconditioner::Jacobi,
        SpdPreconditioner::SymmetricGaussSeidel,
        SpdPreconditioner::IncompleteCholesky,
    ] {
        for (strong, weak) in [(0, [1, 2, 3]), (1024, [17, 511, 1000])] {
            let (matrix, rhs) = extreme_star(1025, strong, weak);
            let options = SpdSolveOptions {
                preconditioner,
                progress_interval: None,
            };
            let profile = solve_spd_system_profile_with_options(&matrix, &rhs, options.clone())
                .expect("a representable SPD star must survive symmetric scaling");
            assert!(profile.iterations > 0 && profile.iterations <= 16);
            assert_star_solution(&profile.solution, strong, weak, 1.0);
            let prepared = PreparedSpdSolver::factor_with_options(matrix, options).unwrap();
            assert_star_solution(&prepared.solve(&rhs).unwrap(), strong, weak, 1.0);
            let reversed: Vec<_> = rhs.iter().map(|value| -value).collect();
            assert_star_solution(&prepared.solve(&reversed).unwrap(), strong, weak, -1.0);
        }
    }
}

fn assert_star_solution(solution: &[f64], strong: usize, weak: [usize; 3], sign: f64) {
    for (row, &value) in solution.iter().enumerate() {
        if weak.contains(&row) {
            assert!((value / power(1000) - sign).abs() < 1e-12, "weak row {row}");
        } else {
            assert_eq!(value, 0.0, "unloaded or cancelling row {row}");
        }
    }
    // Independently assemble physical forces; a tiny wrong strong displacement matters.
    let strong_force = power(1000) * solution[strong]
        + weak
            .iter()
            .map(|&row| power(-1000) * solution[row])
            .sum::<f64>();
    assert!((strong_force - 3.0 * sign).abs() < 1e-12);
    for row in weak {
        let force = power(-1000) * (solution[row] + solution[strong]);
        assert!((force - sign).abs() < 1e-12);
    }
}

#[test]
fn unresolved_candidate_is_rejected_without_weakening_final_validation() {
    let (matrix, rhs) = extreme_star(1025, 0, [1, 2, 3]);
    let scales = scaling::diagonal_sparse_scaling(&matrix).unwrap();
    let scaled = scaling::scale_sparse_matrix(&matrix, &scales).unwrap();
    let compressed = matrix
        .compress_scaled(&scales, SpdPreconditioner::Jacobi)
        .unwrap();
    let scaled_rhs = scaling::scale_sparse_rhs(&rhs, &scales).unwrap();
    let rhs_scale = power(500);
    let normalized_rhs: Vec<_> = scaled_rhs.iter().map(|value| value / rhs_scale).collect();
    let mut candidate = vec![0.0; 1025];
    candidate[0] = 3.0 * power(-1000);
    candidate[1..4].fill(1.0);
    assert!(
        !compressed
            .satisfies_equilibrium(&scaled_rhs, rhs_scale, &candidate)
            .unwrap()
    );
    assert!(sparse_relative_residual(&scaled, &normalized_rhs, &candidate, 1e-8).is_err());
    candidate[0] = 0.0;
    assert!(
        compressed
            .satisfies_equilibrium(&scaled_rhs, rhs_scale, &candidate)
            .unwrap()
    );
    assert!(sparse_relative_residual(&scaled, &normalized_rhs, &candidate, 1e-8).is_ok());
    candidate[0] = 3.0 * power(-1000);
    candidate[1024] = f64::NAN;
    assert!(
        compressed
            .satisfies_equilibrium(&scaled_rhs, rhs_scale, &candidate)
            .is_err()
    );
}

#[test]
fn matrix_scaling_rejects_invalid_factors_and_genuinely_unrepresentable_coefficients() {
    let mut matrix = SparseMatrix::new(2);
    add_at(&mut matrix, 0, 0, 1.0);
    add_at(&mut matrix, 1, 1, 1.0);
    for factors in [
        vec![],
        vec![1.0],
        vec![1.0; 3],
        vec![0.0, 1.0],
        vec![-1.0, 1.0],
        vec![f64::NAN, 1.0],
        vec![1.0, f64::INFINITY],
    ] {
        assert!(scaling::scale_sparse_matrix(&matrix, &factors).is_err());
        assert!(
            matrix
                .compress_scaled(&factors, SpdPreconditioner::Jacobi)
                .is_err()
        );
        assert!(scaling::average_scaled_diagonal_magnitude(&matrix, &factors).is_err());
    }
    for (coefficient, factors) in [
        (f64::from_bits(1), [0.5, 0.5]),
        (f64::MAX, [2.0, 2.0]),
        (f64::NAN, [1.0, 1.0]),
        (f64::INFINITY, [1.0, 1.0]),
    ] {
        let mut bad = matrix.clone();
        add_at(&mut bad, 0, 1, coefficient);
        assert!(scaling::scale_sparse_matrix(&bad, &factors).is_err());
        assert!(
            bad.compress_scaled(&factors, SpdPreconditioner::Jacobi)
                .is_err()
        );
    }
    assert!(
        matrix
            .compress_scaled(&[1.0; 2], SpdPreconditioner::Jacobi)
            .is_ok()
    );
}

#[test]
fn exactly_representable_subnormal_scaled_couplings_are_retained() {
    let q = f64::from_bits(1);
    for sign in [1.0, -1.0] {
        let mut matrix = SparseMatrix::new(2);
        for row in 0..2 {
            add_at(&mut matrix, row, row, 1.0);
            add_at(&mut matrix, row, 1 - row, sign * 4.0 * q);
        }
        let scaled = scaling::scale_sparse_matrix(&matrix, &[0.5; 2]).unwrap();
        let csr = matrix
            .compress_scaled(&[0.5; 2], SpdPreconditioner::Jacobi)
            .unwrap();
        assert_eq!(scaled.row_entries(0)[1].1, sign * q);
        assert_eq!(scaled.row_entries(1)[0].1, sign * q);
        assert_eq!(csr.values, [0.25, sign * q, sign * q, 0.25]);
    }
}

#[test]
fn wide_range_scaling_remains_cancellable_and_replays_without_partial_output() {
    let size = 2049;
    let mut matrix = SparseMatrix::new(size);
    add_at(&mut matrix, 0, 0, power(1000));
    for row in 1..size {
        add_at(&mut matrix, row, row, power(-1000));
        add_at(&mut matrix, 0, row, power(-1000));
        add_at(&mut matrix, row, 0, power(-1000));
    }
    let factors = scaling::diagonal_sparse_scaling(&matrix).unwrap();
    for step in [0, 1024, size] {
        let control = SolverControl::default();
        let cancel = control.clone();
        let result = with_solver_observer(
            &control,
            move |point| {
                if point.stage == SolverStage::SparseMatrixScaleRow
                    && point.completed_steps == step as u64
                {
                    cancel.request_cancel();
                }
            },
            || {
                let result = scaling::scale_sparse_matrix(&matrix, &factors);
                assert!(
                    result.is_err(),
                    "must not publish a partially scaled matrix"
                );
                result
            },
        );
        assert!(result.err().unwrap().starts_with("solver cancelled"));
        let replay = scaling::scale_sparse_matrix(&matrix, &factors).unwrap();
        assert_eq!(replay.non_zero_count(), matrix.non_zero_count());
        assert!(
            replay.row_entries(0)[1..]
                .iter()
                .all(|&(_, value)| value == power(-1000))
        );
    }
}
