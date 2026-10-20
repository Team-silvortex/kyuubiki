use super::DenseLu;
use crate::linear_algebra::{PreparedSpdSolver, SparseMatrix, add_at, solve_spd_system_profile};
use crate::solver_control::{SolverControl, SolverStage, with_solver_observer};

#[test]
fn finite_dense_solution_survives_overflowing_unscaled_back_substitution() {
    let high = 2.0_f64.powi(1023);
    let matrix = vec![vec![high, -high * 0.5], vec![-high * 0.5, high]];
    let factor = DenseLu::factor(matrix).unwrap();
    assert_eq!(factor.solve(&[high, high]).unwrap(), [2.0, 2.0]);
    assert_eq!(factor.solve(&[-high, -high]).unwrap(), [-2.0, -2.0]);
}

#[test]
fn subnormal_dense_equations_do_not_round_away_substitution_terms() {
    let quantum = f64::from_bits(1);
    let matrix = vec![
        vec![4.0 * quantum, -2.0 * quantum],
        vec![-2.0 * quantum, 4.0 * quantum],
    ];
    let factor = DenseLu::factor(matrix).unwrap();
    assert_eq!(factor.solve(&[quantum, quantum]).unwrap(), [0.5, 0.5]);
}

#[test]
fn row_scaling_follows_original_rows_through_pivoting_and_repeated_solves() {
    let base = [[0.0, 2.0, 1.0], [1.0, 3.0, 1.0], [2.0, 1.0, 4.0]];
    let scales = [2.0_f64.powi(-1000), 2.0_f64.powi(1000), f64::from_bits(16)];
    for order in [
        [0, 1, 2],
        [0, 2, 1],
        [1, 0, 2],
        [1, 2, 0],
        [2, 0, 1],
        [2, 1, 0],
    ] {
        let matrix = order
            .iter()
            .map(|&row| base[row].map(|value| value * scales[row]).to_vec())
            .collect();
        let factor = DenseLu::factor(matrix).unwrap();
        for sign in [1.0, -1.0, 0.0, 1.0] {
            let expected = [sign, -2.0 * sign, 3.0 * sign];
            let rhs: Vec<_> = order
                .iter()
                .map(|&row| [-1.0, -2.0, 12.0][row] * scales[row] * sign)
                .collect();
            let solution = factor.solve(&rhs).unwrap();
            for (actual, expected) in solution.iter().zip(expected) {
                assert!((actual - expected).abs() <= 8.0 * f64::EPSILON);
            }
            // Independent balance in the original dimensionless equations.
            for (row, coefficients) in base.iter().enumerate() {
                let force: f64 = coefficients.iter().zip(&solution).map(|(a, x)| a * x).sum();
                assert!((force - [-1.0, -2.0, 12.0][row] * sign).abs() <= 32.0 * f64::EPSILON);
            }
        }
    }
}

#[test]
fn direct_and_prepared_dense_spd_paths_validate_original_extreme_equations() {
    // A = L L^T, L = [[1,0,0],[1,1,0],[1,1,1]]: positive definite,
    // fully connected (not the prepared tridiagonal fast path).
    let base = [[1.0, 1.0, 1.0], [1.0, 2.0, 2.0], [1.0, 2.0, 3.0]];
    for scale in [f64::from_bits(1), 2.0_f64.powi(-1000), 2.0_f64.powi(1022)] {
        let mut matrix = SparseMatrix::new(3);
        for (row, entries) in base.iter().enumerate() {
            for (column, value) in entries.iter().enumerate() {
                add_at(&mut matrix, row, column, value * scale);
            }
        }
        let prepared = PreparedSpdSolver::factor(matrix.clone()).unwrap();
        for sign in [1.0, -1.0, 0.0, 1.0] {
            let rhs = [2.0 * scale * sign, 2.0 * scale * sign, 0.0];
            let direct = solve_spd_system_profile(&matrix, &rhs).unwrap();
            assert_eq!(direct.solution, [2.0 * sign, 2.0 * sign, -2.0 * sign]);
            assert_eq!(direct.residual_norm, 0.0);
            assert_eq!(prepared.solve(&rhs).unwrap(), direct.solution);
        }
    }
}

#[test]
fn dense_equilibration_rejects_information_loss_without_poisoning_factors() {
    let high = 2.0_f64.powi(1000);
    let quantum = f64::from_bits(1);
    for lost in [quantum, 2.0_f64.powi(-60) / 3.0] {
        let error = DenseLu::factor(vec![vec![high, lost], vec![0.0, 1.0]])
            .err()
            .unwrap();
        assert!(error.contains("coefficient cannot be scaled without range loss"));
    }
    let strong = DenseLu::factor(vec![vec![high]]).unwrap();
    for lost in [quantum, 2.0_f64.powi(-60) / 3.0] {
        assert!(
            strong
                .solve(&[lost])
                .unwrap_err()
                .contains("right-hand side cannot be scaled")
        );
        assert_eq!(strong.solve(&[high]).unwrap(), [1.0]);
    }
    let weak = DenseLu::factor(vec![vec![quantum]]).unwrap();
    assert!(weak.solve(&[f64::MAX]).unwrap_err().contains("range loss"));
    assert_eq!(weak.solve(&[quantum]).unwrap(), [1.0]);
    for invalid in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert!(strong.solve(&[invalid]).unwrap_err().contains("non-finite"));
        assert!(DenseLu::factor(vec![vec![invalid]]).is_err());
    }
    assert!(strong.solve(&[]).is_err());
    assert_eq!(strong.solve(&[high]).unwrap(), [1.0]);
    assert!(DenseLu::factor(vec![vec![high, high], vec![high, high]]).is_err());
}

#[test]
fn ordinary_dense_equations_keep_the_unscaled_path() {
    for exponent in [-500, 0, 500] {
        let scale = 2.0_f64.powi(exponent);
        let factor =
            DenseLu::factor(vec![vec![scale, -0.5 * scale], vec![-0.5 * scale, scale]]).unwrap();
        assert!(factor.equilibration.is_none());
        assert_eq!(factor.solve(&[scale, scale]).unwrap(), [2.0, 2.0]);
    }
    assert!(
        DenseLu::factor(vec![])
            .unwrap()
            .solve(&[])
            .unwrap()
            .is_empty()
    );
}

fn cancel_at<T>(stage: SolverStage, step: u64, run: impl FnOnce() -> Result<T, String>) {
    let control = SolverControl::default();
    let cancel = control.clone();
    let result = with_solver_observer(
        &control,
        move |point| {
            if point.stage == stage && point.completed_steps >= step {
                cancel.request_cancel();
            }
        },
        run,
    );
    assert!(result.is_err());
    assert!(control.was_interrupted());
    let point = control.last_checkpoint().unwrap();
    assert_eq!(point.stage, stage);
    assert_eq!(point.completed_steps, step);
}

#[test]
fn dense_equilibration_cancels_and_replays_without_partial_state() {
    let size = 65;
    let high = 2.0_f64.powi(1000);
    let matrix: Vec<Vec<f64>> = (0..size)
        .map(|row| {
            (0..size)
                .map(|column| if row == column { high } else { 0.0 })
                .collect()
        })
        .collect();
    for step in [0, 1024, (size * size) as u64] {
        cancel_at(SolverStage::DenseRowScale, step, || {
            DenseLu::factor(matrix.clone())
        });
    }
    let factor = DenseLu::factor(matrix).unwrap();
    let rhs = vec![high; size];
    for step in [0, size as u64] {
        cancel_at(SolverStage::DenseRhsScale, step, || factor.solve(&rhs));
        assert_eq!(factor.solve(&rhs).unwrap(), vec![1.0; size]);
    }
    cancel_at(SolverStage::DenseSubstitution, 2, || factor.solve(&rhs));
    assert_eq!(factor.solve(&rhs).unwrap(), vec![1.0; size]);
}
