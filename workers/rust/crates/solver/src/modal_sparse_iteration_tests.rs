use super::*;
use crate::solver_control::{SolverControl, with_solver_observer};
use std::cell::Cell;

fn rotated_apply(v: &[f64], diagonal: &[f64; 8], inverse: bool) -> Vec<f64> {
    let sign = |i: usize, j: usize| {
        if (i & j).count_ones() % 2 == 0 {
            1.0
        } else {
            -1.0
        }
    };
    let weight = 1.0 / 8.0_f64.sqrt();
    let coefficients: Vec<_> = diagonal
        .iter()
        .enumerate()
        .map(|(j, eigenvalue)| {
            let coefficient: f64 = v
                .iter()
                .enumerate()
                .map(|(i, x)| sign(i, j) * weight * x)
                .sum();
            if inverse {
                coefficient / eigenvalue
            } else {
                coefficient * eigenvalue
            }
        })
        .collect();
    (0..8)
        .map(|i| {
            coefficients
                .iter()
                .enumerate()
                .map(|(j, c)| sign(i, j) * weight * c)
                .sum()
        })
        .collect()
}

#[test]
fn locally_optimal_probes_resolve_rotated_clusters_at_extreme_common_scales() {
    for scale in [1e-200, 1.0, 1e200] {
        for gap in [1e-2, 1e-4, 1e-6, 1e-8] {
            // The uniform starting vector is the exact higher eigenmode (4).
            let diagonal = [4.0, 1.0, 1.0 + gap, 8.0, 12.0, 16.0, 32.0, 64.0].map(|x| x * scale);
            let pair = inverse_power_iteration(
                8,
                InverseIterationOptions::default(),
                |v| Ok(rotated_apply(v, &diagonal, false)),
                |v| Ok(rotated_apply(v, &diagonal, true)),
            )
            .unwrap();
            assert!(
                (pair.eigenvalue / scale - 1.0).abs() < 1e-8,
                "scale={scale:e}, gap={gap:e}, value={:e}",
                pair.eigenvalue
            );
            let image = rotated_apply(&pair.vector, &diagonal, false);
            let residual = stable_l2_norm(
                image
                    .iter()
                    .zip(&pair.vector)
                    .map(|(a, v)| a / scale - pair.eigenvalue / scale * v),
            );
            assert!(residual < 1e-6, "residual={residual:e}");
            assert!(pair.iterations <= 128);
        }
    }
}

#[test]
fn projection_callback_failures_are_not_hidden_by_an_old_ritz_pair() {
    for bad in [Vec::new(), vec![f64::NAN; 8], vec![f64::INFINITY; 8]] {
        let calls = Cell::new(0);
        let diagonal = [1.0, 1.0001, 4.0, 10.0, 25.0, 50.0, 80.0, 100.0];
        let error = inverse_power_iteration(
            8,
            InverseIterationOptions::default(),
            |v| {
                calls.set(calls.get() + 1);
                if calls.get() > 1 {
                    Ok(bad.clone())
                } else {
                    Ok(v.iter().zip(diagonal).map(|(x, d)| x * d).collect())
                }
            },
            |v| Ok(v.iter().zip(diagonal).map(|(x, d)| x / d).collect()),
        )
        .unwrap_err();
        assert!(calls.get() > 1);
        assert!(error.contains("invalid operator vector"), "{error}");
    }
}

#[test]
fn inverse_callback_failure_is_not_convergence() {
    for bad in [
        Vec::new(),
        vec![0.0; 8],
        vec![f64::NAN; 8],
        vec![f64::INFINITY; 8],
    ] {
        let diagonal = [1.0, 1.0001, 4.0, 10.0, 25.0, 50.0, 80.0, 100.0];
        let error = inverse_power_iteration(
            8,
            InverseIterationOptions::default(),
            |v| Ok(v.iter().zip(diagonal).map(|(x, d)| x * d).collect()),
            |_| Ok(bad.clone()),
        )
        .unwrap_err();
        assert!(error.contains("inverse solve returned"), "{error}");
    }
}

#[test]
fn ritz_projection_cancellation_propagates_and_the_same_problem_replays() {
    let problem = || {
        inverse_power_iteration(
            8,
            InverseIterationOptions::default(),
            |v| {
                Ok(v.iter()
                    .enumerate()
                    .map(|(i, x)| x * (i + 1) as f64)
                    .collect())
            },
            |v| {
                Ok(v.iter()
                    .enumerate()
                    .map(|(i, x)| x / (i + 1) as f64)
                    .collect())
            },
        )
    };
    let control = SolverControl::default();
    let cancel = control.clone();
    let error = with_solver_observer(
        &control,
        move |point| {
            if point.stage == SolverStage::ModalSweep {
                cancel.request_cancel();
            }
        },
        || {
            let result = problem();
            assert!(
                result.is_err(),
                "projection must observe cancellation itself"
            );
            result
        },
    )
    .unwrap_err();
    assert!(error.contains("cancel"), "{error}");
    assert!((problem().unwrap().eigenvalue - 1.0).abs() < 1e-10);
}

#[test]
fn unresolved_low_cluster_cannot_exceed_the_configured_iteration_budget() {
    let diagonal = [1.0, 1.0001, 4.0, 10.0, 25.0, 50.0, 80.0, 100.0];
    let solves = Cell::new(0);
    let error = inverse_power_iteration(
        8,
        InverseIterationOptions {
            max_iterations: 1,
            tolerance: 1e-12,
        },
        |v| Ok(v.iter().zip(diagonal).map(|(x, d)| x * d).collect()),
        |v| {
            solves.set(solves.get() + 1);
            Ok(v.iter().zip(diagonal).map(|(x, d)| x / d).collect())
        },
    )
    .unwrap_err();
    assert_eq!(solves.get(), 1);
    assert!(
        error.contains("did not converge within 1 iterations"),
        "{error}"
    );
}

#[test]
fn locally_optimal_projection_does_not_cancel_a_resolved_soft_mode() {
    for soft in [1e-20, 1e-100, 1e-200] {
        let diagonal = [soft, 1.0, 2.0, 3.0, 5.0, 10.0, 25.0, 50.0];
        let pair = inverse_power_iteration(
            8,
            InverseIterationOptions::default(),
            |v| Ok(v.iter().zip(diagonal).map(|(x, d)| x * d).collect()),
            |v| Ok(v.iter().zip(diagonal).map(|(x, d)| x / d).collect()),
        )
        .unwrap();
        assert!((pair.eigenvalue / soft - 1.0).abs() < 1e-10);
        assert!(pair.residual_norm / soft < 1e-6);
    }
}
