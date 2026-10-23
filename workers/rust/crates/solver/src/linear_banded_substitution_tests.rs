use super::SymmetricBandCholesky;
use crate::linear_algebra::{SparseMatrix, add_at, sparse_solution_backward_error};
use crate::linear_symmetric_tangent::{SYMMETRIC_BAND_CHOLESKY, solve_symmetric_tangent};
use crate::solver_control::{SolverControl, SolverStage, with_solver_observer};
use std::cell::Cell;
use std::rc::Rc;

fn matrix(rows: &[&[f64]]) -> SparseMatrix {
    let mut matrix = SparseMatrix::new(rows.len());
    for (row, entries) in rows.iter().enumerate() {
        for (column, value) in entries.iter().enumerate() {
            add_at(&mut matrix, row, column, *value);
        }
    }
    matrix
}

#[test]
fn banded_substitution_keeps_finite_large_responses_on_the_sparse_path() {
    let matrix = matrix(&[&[8.0, -7.0], &[-7.0, 8.0]]);
    let factor = SymmetricBandCholesky::try_factor(&matrix, 4)
        .unwrap()
        .unwrap();
    let high = 2.0_f64.powi(1023);
    for sign in [1.0, -1.0, 1.0] {
        let rhs = [sign * high; 2];
        let solution = factor
            .solve(&rhs)
            .expect("finite banded response should survive");
        for value in &solution {
            assert!((value / high - sign).abs() < 1e-14);
        }
        assert!(sparse_solution_backward_error(&matrix, &rhs, &solution, 1e-9).unwrap() < 1e-9);
        let tangent = solve_symmetric_tangent(&matrix, &rhs, 0, "no dense fallback").unwrap();
        assert_eq!(tangent.method, SYMMETRIC_BAND_CHOLESKY);
        assert_eq!(tangent.solution, solution);
    }
}

#[test]
fn banded_substitution_recovers_erased_products_before_division() {
    // L = [[1,0,0], [0,1,0], [0.5,0.5,1]], A = L L^T.
    let matrix = matrix(&[&[1.0, 0.0, 0.5], &[0.0, 1.0, 0.5], &[0.5, 0.5, 1.5]]);
    let factor = SymmetricBandCholesky::try_factor(&matrix, 9)
        .unwrap()
        .unwrap();
    let quantum = f64::from_bits(1);
    for sign in [1.0, -1.0] {
        let rhs = [sign * quantum; 3];
        let solution = factor.solve(&rhs).unwrap();
        assert_eq!(solution, [sign * quantum, sign * quantum, 0.0]);
        assert_eq!(
            sparse_solution_backward_error(&matrix, &rhs, &solution, 1e-9).unwrap(),
            0.0
        );
    }
}

#[test]
fn banded_substitution_rejects_nonzero_response_range_loss_then_replays() {
    let factor = SymmetricBandCholesky::try_factor(&matrix(&[&[4.0]]), 1)
        .unwrap()
        .unwrap();
    for sign in [1.0, -1.0] {
        assert!(factor.solve(&[sign * f64::from_bits(1)]).is_err());
        assert_eq!(factor.solve(&[sign * 4.0]).unwrap(), [sign]);
    }
}

#[test]
fn banded_large_rhs_chain_stays_sparse_with_dense_fallback_disabled() {
    let size = 513;
    let mut matrix = SparseMatrix::new(size);
    for row in 0..size {
        add_at(
            &mut matrix,
            row,
            row,
            if row == 0 || row == size - 1 {
                4.5
            } else {
                8.0
            },
        );
        if row > 0 {
            add_at(&mut matrix, row, row - 1, -3.5);
            add_at(&mut matrix, row - 1, row, -3.5);
        }
    }
    let high = 2.0_f64.powi(1023);
    for sign in [1.0, -1.0] {
        let rhs = vec![sign * high; size];
        let result = solve_symmetric_tangent(&matrix, &rhs, 0, "narrow chain").unwrap();
        assert_eq!(result.method, SYMMETRIC_BAND_CHOLESKY);
        for value in &result.solution {
            assert!((value / high - sign).abs() < 1e-14);
        }
        assert!(
            sparse_solution_backward_error(&matrix, &rhs, &result.solution, 1e-9).unwrap() < 1e-9
        );
    }
}

#[test]
fn banded_transposed_substitution_retains_fused_product_tails() {
    let high = 2.0_f64.powi(1023);
    let delta = 2.0_f64.powi(-27);
    // Packed rows of L = [[1,0,0], [2(1+d),1,0], [-2,0,1]].
    let factor = SymmetricBandCholesky {
        size: 3,
        width: 2,
        lower: vec![1.0, 0.0, 0.0, 1.0, 2.0 * (1.0 + delta), 0.0, 1.0, 0.0, -2.0],
        row_scales: vec![1.0; 3],
    };
    for sign in [1.0, -1.0] {
        let rhs = [0.0, sign * high * (1.0 - delta), sign * high];
        assert_eq!(
            factor.solve(&rhs).unwrap(),
            [sign * 2.0_f64.powi(970), rhs[1], rhs[2]]
        );
    }
}

fn wide_cancelling_factor() -> SymmetricBandCholesky {
    let size = 1026;
    let mut factor = SymmetricBandCholesky {
        size,
        width: size - 1,
        lower: vec![0.0; size * size],
        row_scales: vec![1.0; size],
    };
    // An exact prepared L, built directly to isolate substitution from factorization cost.
    for row in 0..size {
        factor.set(row, row, 1.0);
    }
    factor.set(size - 1, 0, 2.0);
    factor.set(size - 1, 1, -2.0);
    factor
}

#[test]
fn banded_range_passes_cancel_without_poisoning_the_prepared_factor() {
    let factor = wide_cancelling_factor();
    let high = 2.0_f64.powi(1023);
    let mut rhs = vec![0.0; factor.size];
    rhs[..2].fill(high);
    rhs[factor.size - 1] = 1.0;
    let expected = rhs.clone();
    for step in [0, 1024, 1025, 2048, 2050] {
        let control = SolverControl::default();
        let cancel = control.clone();
        let result = with_solver_observer(
            &control,
            move |point| {
                if point.stage == SolverStage::BandedSubstitutionRange
                    && point.completed_steps == step
                {
                    cancel.request_cancel();
                }
            },
            || factor.solve(&rhs),
        );
        assert!(result.is_err());
        assert!(control.was_interrupted());
        let point = control.last_checkpoint().unwrap();
        assert_eq!(point.stage, SolverStage::BandedSubstitutionRange);
        assert_eq!(point.completed_steps, step);
        assert_eq!(factor.solve(&rhs).unwrap(), expected);
    }
    rhs[factor.size - 1] = f64::from_bits(1);
    assert!(
        factor
            .solve(&rhs)
            .unwrap_err()
            .contains("normalization would lose numerical range")
    );
    rhs[factor.size - 1] = 1.0;
    assert_eq!(factor.solve(&rhs).unwrap(), expected);
}

fn original_substitution(factor: &SymmetricBandCholesky, rhs: &[f64]) -> Vec<f64> {
    let mut forward = vec![0.0; factor.size];
    for row in 0..factor.size {
        let correction = (row.saturating_sub(factor.width)..row)
            .map(|column| factor.get(row, column) * forward[column])
            .sum::<f64>();
        forward[row] = (rhs[row] - correction) / factor.get(row, row);
    }
    let mut solution = vec![0.0; factor.size];
    for row in (0..factor.size).rev() {
        let correction = (row + 1..(row + factor.width + 1).min(factor.size))
            .map(|other| factor.get(other, row) * solution[other])
            .sum::<f64>();
        solution[row] = (forward[row] - correction) / factor.get(row, row);
    }
    solution
}

#[test]
fn ordinary_banded_substitution_preserves_bits_and_skips_range_passes() {
    let matrix = matrix(&[&[4.0, 1.0, -0.5], &[1.0, 5.0, 0.75], &[-0.5, 0.75, 6.0]]);
    let factor = SymmetricBandCholesky::try_factor(&matrix, 9)
        .unwrap()
        .unwrap();
    let visits = Rc::new(Cell::new(0));
    let observed = visits.clone();
    with_solver_observer(
        &SolverControl::default(),
        move |point| {
            if point.stage == SolverStage::BandedSubstitutionRange {
                observed.set(observed.get() + 1);
            }
        },
        || {
            for rhs in [[1.0, 0.5, -2.0], [-3.0, 2.0, 4.5], [0.0; 3], [-0.0; 3]] {
                let actual = factor.solve(&rhs)?;
                let expected = original_substitution(&factor, &rhs);
                assert_eq!(
                    actual.iter().map(|x| x.to_bits()).collect::<Vec<_>>(),
                    expected.iter().map(|x| x.to_bits()).collect::<Vec<_>>()
                );
            }
            Ok::<_, String>(())
        },
    )
    .unwrap();
    assert_eq!(visits.get(), 0);
}

#[test]
fn banded_invalid_rhs_and_true_overflow_fail_before_clean_replay() {
    let factor = SymmetricBandCholesky::try_factor(&matrix(&[&[0.25]]), 1)
        .unwrap()
        .unwrap();
    for rhs in [
        vec![],
        vec![1.0, 2.0],
        vec![f64::NAN],
        vec![f64::INFINITY],
        vec![f64::NEG_INFINITY],
        vec![f64::MAX],
    ] {
        assert!(factor.solve(&rhs).is_err());
        assert_eq!(factor.solve(&[0.25]).unwrap(), [1.0]);
    }
}
