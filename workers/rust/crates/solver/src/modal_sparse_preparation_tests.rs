use super::*;
use crate::linear_algebra::add_at;
use crate::solver_control::{SolverControl, SolverStage, with_solver_observer};
use std::{cell::Cell, rc::Rc};

fn operator(entries: &[(usize, usize, f64)], mass: &[f64]) -> SparseMassNormalizedOperator {
    let mut matrix = SparseMatrix::new(mass.len());
    for &(row, column, value) in entries {
        add_at(&mut matrix, row, column, value);
    }
    SparseMassNormalizedOperator::new(&matrix, mass).unwrap()
}

fn chain(size: usize, uniform: bool, scale: f64) -> SparseMassNormalizedOperator {
    let mut matrix = SparseMatrix::new(size);
    let mut mass = vec![scale; size];
    if uniform {
        mass[size - 1] = 0.5 * scale;
    }
    for row in 0..size {
        let diagonal = if uniform {
            if row + 1 == size { 1.0 } else { 2.0 }
        } else {
            3.0
        };
        add_at(&mut matrix, row, row, diagonal * scale);
        if row > 0 {
            add_at(&mut matrix, row, row - 1, -scale);
            add_at(&mut matrix, row - 1, row, -scale);
        }
    }
    SparseMassNormalizedOperator::new(&matrix, &mass).unwrap()
}

#[test]
fn tridiagonal_preparation_retains_weak_couplings_and_mode_components_in_both_orders() {
    for coupling in [-1e-300, -1e-220, 1e-300, 1e-220] {
        for reversed in [false, true] {
            let (low, high) = if reversed { (1, 0) } else { (0, 1) };
            let mut mass = [0.0; 2];
            mass[low] = 1e200;
            mass[high] = 1e-200;
            let op = operator(
                &[
                    (low, low, 2e200),
                    (high, high, 5e-200),
                    (low, high, coupling),
                    (high, low, coupling),
                ],
                &mass,
            );
            let pair = op
                .smallest_tridiagonal_eigenpair(1e-9)
                .expect("a representable coupling cannot become a disconnected operator")
                .unwrap();
            assert!((pair.eigenvalue / 2.0 - 1.0).abs() < 1e-12);
            let ratio = pair.vector[high] / pair.vector[low];
            let expected = -coupling / 3.0;
            assert!((ratio / expected - 1.0).abs() < 1e-12, "ratio={ratio:e}");
            let applied = op.apply_compensated(&pair.vector).unwrap();
            let residual = stable_l2_norm(
                applied
                    .iter()
                    .zip(&pair.vector)
                    .map(|(a, v)| a - pair.eigenvalue * v),
            );
            assert!(residual / pair.eigenvalue < 1e-9);
        }
    }
}

#[test]
fn tridiagonal_preparation_reports_intrinsic_range_loss_instead_of_declining() {
    for (coupling, mass) in [
        (-f64::from_bits(1), 1e200),
        (-1.0, 1e-320),
        (-1e-300, 1e300),
    ] {
        let op = operator(
            &[(0, 0, 4.0), (1, 1, 2.0), (0, 1, coupling), (1, 0, coupling)],
            &[mass; 2],
        );
        let error = op
            .smallest_tridiagonal_eigenpair(1e-9)
            .expect("range loss must be an error, not an unsupported topology")
            .unwrap_err();
        assert!(error.contains("normalized stiffness"), "{error}");
    }
    assert!(
        chain(6, false, 1.0)
            .smallest_tridiagonal_eigenpair(1e-9)
            .unwrap()
            .is_ok()
    );
}

#[test]
fn tridiagonal_preparation_cannot_ignore_a_mismatched_or_missing_lower_triangle() {
    for (upper, lower) in [(-1.0, -0.25), (1.0, -1.0), (-1.0, 0.0), (0.0, -1.0)] {
        let op = operator(
            &[(0, 0, 3.0), (1, 1, 2.0), (0, 1, upper), (1, 0, lower)],
            &[1.0; 2],
        );
        let error = op
            .smallest_tridiagonal_eigenpair(1e-9)
            .expect("asymmetric coefficients must not disappear into fallback")
            .unwrap_err();
        assert!(error.contains("symmetric"), "{error}");
    }
}

#[test]
fn tridiagonal_preparation_declines_only_genuinely_disconnected_or_nonbanded_topology() {
    for entries in [
        vec![(0, 0, 3.0), (1, 1, 2.0), (2, 2, 4.0)],
        vec![
            (0, 0, 3.0),
            (1, 1, 2.0),
            (2, 2, 4.0),
            (0, 2, -1.0),
            (2, 0, -1.0),
        ],
    ] {
        assert!(
            operator(&entries, &[1.0; 3])
                .smallest_tridiagonal_eigenpair(1e-9)
                .is_none()
        );
    }
}

fn cancel_and_replay(
    op: &SparseMassNormalizedOperator,
    stage: SolverStage,
    completed: u64,
    occurrence: usize,
) {
    let control = SolverControl::default();
    let cancel = control.clone();
    let seen = Rc::new(Cell::new(0));
    let observed = seen.clone();
    let error = with_solver_observer(
        &control,
        move |point| {
            assert_ne!(
                point.stage,
                SolverStage::ModalIteration,
                "must stop before spectral search"
            );
            assert_ne!(
                point.stage,
                SolverStage::SparseMatvec,
                "must stop before residual products"
            );
            if point.stage == stage && point.completed_steps == completed {
                observed.set(observed.get() + 1);
                if observed.get() == occurrence {
                    cancel.request_cancel();
                }
            }
        },
        || {
            let result = op.smallest_tridiagonal_eigenpair(1e-9).unwrap();
            assert!(result.is_err(), "preprocessing must cancel inside its call");
            result
        },
    )
    .unwrap_err();
    assert_eq!(
        seen.get(),
        occurrence,
        "the requested pass must be observed"
    );
    assert!(error.contains("cancel"), "{error}");
    assert_eq!(control.last_checkpoint().unwrap().stage, stage);
    assert_eq!(
        control.last_checkpoint().unwrap().completed_steps,
        completed
    );
    assert!(op.smallest_tridiagonal_eigenpair(1e-9).unwrap().is_ok());
}

#[test]
fn general_tridiagonal_normalization_cancels_before_search_and_replays() {
    let op = chain(129, false, 2.0_f64.powi(-600));
    cancel_and_replay(&op, SolverStage::SparseMatrixScale, 64, 1);
}

#[test]
fn uniform_chain_preparation_cancels_in_each_linear_pass_and_replays() {
    let op = chain(129, true, 1.0);
    for (stage, completed) in [
        (SolverStage::SparseValidateMatrix, 0),
        (SolverStage::SparseValidateMatrix, 64),
        (SolverStage::SparseDiagonalScale, 64),
        (SolverStage::SparseDiagonalMagnitude, 64),
        (SolverStage::ResultFreeDofs, 64),
        (SolverStage::ResultNodeSummary, 64),
        (SolverStage::ResultTotals, 64),
    ] {
        cancel_and_replay(&op, stage, completed, 1);
    }
    cancel_and_replay(&op, SolverStage::SparseValidateMatrix, 64, 2);
}

#[test]
fn tridiagonal_prefix_cancellation_is_not_swallowed_by_a_later_topology_decline() {
    let mut op = chain(129, false, 1.0);
    let mut matrix = SparseMatrix::new(129);
    for row in 0..129 {
        for entry in op.stiffness.row_offsets[row]..op.stiffness.row_offsets[row + 1] {
            add_at(
                &mut matrix,
                row,
                op.stiffness.columns[entry],
                op.stiffness.values[entry],
            );
        }
    }
    add_at(&mut matrix, 128, 0, -0.25);
    op = SparseMassNormalizedOperator::new(&matrix, &[1.0; 129]).unwrap();
    let control = SolverControl::default();
    let cancel = control.clone();
    let error = with_solver_observer(
        &control,
        move |point| {
            if point.stage == SolverStage::SparseValidateMatrix && point.completed_steps == 64 {
                cancel.request_cancel();
            }
        },
        || {
            let result = op
                .smallest_tridiagonal_eigenpair(1e-9)
                .expect("an earlier cancellation cannot be turned into unsupported topology");
            assert!(result.is_err());
            result
        },
    )
    .unwrap_err();
    assert!(error.contains("cancel"), "{error}");
    assert!(op.smallest_tridiagonal_eigenpair(1e-9).is_none());
}

#[test]
fn general_tridiagonal_preparation_matches_discrete_roots_and_directions_across_scales() {
    for scale in [2.0_f64.powi(-600), 1.0, 2.0_f64.powi(600)] {
        for size in [2, 6, 129] {
            let op = chain(size, false, scale);
            let pair = op.smallest_tridiagonal_eigenpair(1e-9).unwrap().unwrap();
            let theta = std::f64::consts::PI / (size + 1) as f64;
            let expected = 3.0 - 2.0 * theta.cos();
            assert!((pair.eigenvalue / expected - 1.0).abs() < 1e-12);
            let norm = (2.0 / (size + 1) as f64).sqrt();
            let dot: f64 = pair
                .vector
                .iter()
                .enumerate()
                .map(|(i, value)| value * norm * ((i + 1) as f64 * theta).sin())
                .sum();
            assert!((dot.abs() - 1.0).abs() < 1e-12);
        }
    }
}
