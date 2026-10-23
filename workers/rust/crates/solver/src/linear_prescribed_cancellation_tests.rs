use super::{SparseMatrix, add_at, reduce_sparse_system_with_prescribed, solve_spd_system};
use crate::solver_control::{SolverControl, SolverStage, with_solver_observer};

fn boundary_row(diagonal: f64, coefficients: &[f64]) -> SparseMatrix {
    let mut matrix = SparseMatrix::new(coefficients.len() + 1);
    add_at(&mut matrix, 0, 0, diagonal);
    for (index, &value) in coefficients.iter().enumerate() {
        add_at(&mut matrix, 0, index + 1, value);
        add_at(&mut matrix, index + 1, 0, value);
        add_at(&mut matrix, index + 1, index + 1, diagonal);
    }
    matrix
}

#[test]
fn prescribed_cancellation_sparse_recovers_normal_product_tail_before_solving() {
    for power in [-1020, -600, 0, 600] {
        let scale = 2.0_f64.powi(power);
        for delta_power in [-27, -30] {
            if power + 2 * delta_power < -1074 {
                continue;
            }
            let delta = 2.0_f64.powi(delta_power);
            let coefficient = scale * (1.0 + delta);
            let matrix = boundary_row(scale, &[coefficient]);
            for sign in [-1.0, 1.0] {
                let fixed = sign * (1.0 - delta);
                let load = sign * scale;
                assert_eq!(load - coefficient * fixed, 0.0);
                let (reduced, rhs, free) =
                    reduce_sparse_system_with_prescribed(&matrix, &[load, 0.0], &[(1, fixed)])
                        .unwrap();
                assert_eq!(free, [0]);
                assert_eq!(rhs, [sign * scale * (delta * delta)]);
                assert_eq!(
                    solve_spd_system(&reduced, &rhs).unwrap(),
                    [sign * delta * delta]
                );
            }
        }
    }
}

#[test]
fn prescribed_cancellation_sparse_recovers_an_intermediate_zero_before_a_later_term() {
    let delta = 2.0_f64.powi(-27);
    let tail = delta * delta;
    for sign in [-1.0, 1.0] {
        let matrix = boundary_row(1.0, &[1.0 + delta, -tail * 0.5]);
        let (reduced, rhs, _) = reduce_sparse_system_with_prescribed(
            &matrix,
            &[sign, 0.0, 0.0],
            &[(1, sign * (1.0 - delta)), (2, sign)],
        )
        .unwrap();
        assert_eq!(rhs, [sign * tail * 1.5]);
        assert_eq!(solve_spd_system(&reduced, &rhs).unwrap(), rhs);
    }
}

#[test]
fn prescribed_cancellation_sparse_keeps_true_zeros_and_unrelated_quantum_loads() {
    let mut matrix = boundary_row(1.0, &[1.0]);
    matrix.rows.push(vec![(2, 1.0)]);
    let snapshot = matrix.rows.clone();
    let quantum = f64::from_bits(1);
    for sign in [-1.0, 1.0] {
        let (reduced, rhs, free) = reduce_sparse_system_with_prescribed(
            &matrix,
            &[sign, 0.0, sign * quantum],
            &[(1, sign)],
        )
        .unwrap();
        assert_eq!(free, [0, 2]);
        assert_eq!(rhs, [0.0, sign * quantum]);
        assert_eq!(solve_spd_system(&reduced, &rhs).unwrap(), rhs);
    }
    assert_eq!(matrix.rows, snapshot);
}

#[test]
fn prescribed_cancellation_sparse_rejects_unrepresentable_or_unalignable_loads_then_replays() {
    let delta = 2.0_f64.powi(-27);
    let scale = f64::MIN_POSITIVE;
    let matrix = boundary_row(scale, &[scale * (1.0 + delta)]);
    let error = reduce_sparse_system_with_prescribed(&matrix, &[scale, 0.0], &[(1, 1.0 - delta)])
        .unwrap_err();
    assert!(error.contains("range"), "{error}");
    assert_eq!(
        reduce_sparse_system_with_prescribed(&matrix, &[scale, 0.0], &[(1, 0.0)])
            .unwrap()
            .1,
        [scale]
    );
    let large = 2.0_f64.powi(1023);
    let matrix = boundary_row(1.0, &[large, -large]);
    let error = reduce_sparse_system_with_prescribed(
        &matrix,
        &[f64::from_bits(1), 0.0, 0.0],
        &[(1, 1.0), (2, 1.0)],
    )
    .unwrap_err();
    assert!(error.contains("normalization"), "{error}");
    assert_eq!(
        reduce_sparse_system_with_prescribed(&matrix, &[1.0, 0.0, 0.0], &[(1, 1.0), (2, 1.0)])
            .unwrap()
            .1,
        [1.0]
    );
}

#[test]
fn prescribed_cancellation_sparse_cancels_each_wide_replay_phase_then_reuses_inputs() {
    let large = 2.0_f64.powi(60);
    let mut coefficients = vec![1.0; 2048];
    coefficients[0] = large;
    coefficients[1024..2047].fill(-1.0);
    coefficients[2047] = -large;
    let matrix = boundary_row(1.0, &coefficients);
    let snapshot = matrix.rows.clone();
    let mut rhs = vec![0.0; 2049];
    rhs[0] = 1.0;
    let prescribed = (1..=2048).map(|index| (index, 1.0)).collect::<Vec<_>>();
    for (phase, step) in [(1, 1024), (2, 1024), (3, 0), (3, 64), (3, 2048), (3, 4096)] {
        let control = SolverControl::default();
        let cancel = control.clone();
        let entered = std::cell::Cell::new(0);
        let result = with_solver_observer(
            &control,
            move |point| {
                if point.stage == SolverStage::ConstraintReduceRow {
                    if point.completed_steps == 0 {
                        entered.set(entered.get() + 1);
                    }
                    if entered.get() == phase && point.completed_steps == step {
                        cancel.request_cancel();
                    }
                }
            },
            || reduce_sparse_system_with_prescribed(&matrix, &rhs, &prescribed),
        );
        assert!(result.unwrap_err().contains("cancelled"));
        assert!(control.was_interrupted());
        let (reduced, rhs, free) =
            reduce_sparse_system_with_prescribed(&matrix, &rhs, &prescribed).unwrap();
        assert_eq!(free, [0]);
        assert_eq!(rhs, [1.0]);
        assert_eq!(reduced.row_entries(0), [(0, 1.0)]);
    }
    assert_eq!(matrix.rows, snapshot);
}

#[test]
fn prescribed_cancellation_sparse_replays_the_full_row_after_addition_loss() {
    for power in [-600, 0, 600] {
        let scale = 2.0_f64.powi(power);
        let large = scale * 2.0_f64.powi(60);
        let matrix = boundary_row(scale, &[large, -large]);
        for sign in [-1.0, 1.0] {
            let (reduced, rhs, _) = reduce_sparse_system_with_prescribed(
                &matrix,
                &[sign * scale, 0.0, 0.0],
                &[(1, sign), (2, sign)],
            )
            .unwrap();
            assert_eq!(rhs, [sign * scale]);
            assert_eq!(solve_spd_system(&reduced, &rhs).unwrap(), [sign]);
        }
    }
}
