use super::solve_with_prescribed;
use crate::solver_control::{SolverControl, SolverStage, with_solver_observer};

#[test]
fn prescribed_cancellation_chain_recovers_normal_product_tail_on_either_boundary() {
    for power in [-1022, -600, 0, 600] {
        let scale = 2.0_f64.powi(power);
        let delta = 2.0_f64.powi(-27);
        let coefficient = scale * (1.0 + delta);
        for sign in [-1.0, 1.0] {
            for fixed_index in [0, 1] {
                let fixed = sign * (1.0 - delta);
                let mut rhs = [0.0; 2];
                rhs[1 - fixed_index] = sign * scale;
                let solution = solve_with_prescribed(
                    &[scale; 2],
                    &[coefficient],
                    &[coefficient],
                    &rhs,
                    &[(fixed_index, fixed)],
                )
                .unwrap();
                assert_eq!(solution[fixed_index], fixed);
                assert_eq!(solution[1 - fixed_index], sign * delta * delta);
            }
        }
    }
}

#[test]
fn prescribed_cancellation_chain_recovers_an_intermediate_zero_before_a_later_boundary() {
    let delta = 2.0_f64.powi(-27);
    let tail = delta * delta;
    for sign in [-1.0, 1.0] {
        let result = solve_with_prescribed(
            &[1.0; 3],
            &[1.0 + delta, 0.0],
            &[0.0, -tail * 0.5],
            &[0.0, sign, 0.0],
            &[(0, sign * (1.0 - delta)), (2, sign)],
        )
        .unwrap();
        assert_eq!(result[1], sign * tail * 1.5);
    }
}

#[test]
fn prescribed_cancellation_chain_keeps_exact_zeros_and_independent_quantum_loads() {
    let quantum = f64::from_bits(1);
    for sign in [-1.0, 1.0] {
        assert_eq!(
            solve_with_prescribed(
                &[1.0; 3],
                &[1.0, 0.0],
                &[0.0; 2],
                &[0.0, sign, sign * quantum],
                &[(0, sign)],
            )
            .unwrap(),
            [sign, 0.0, sign * quantum]
        );
    }
}

#[test]
fn prescribed_cancellation_chain_rejects_irreversible_alignment_then_replays() {
    let large = 2.0_f64.powi(1023);
    let error = solve_with_prescribed(
        &[1.0; 3],
        &[large, 0.0],
        &[0.0, -large],
        &[0.0, f64::from_bits(1), 0.0],
        &[(0, 1.0), (2, 1.0)],
    )
    .unwrap_err();
    assert!(error.contains("normalization"), "{error}");
    assert_eq!(
        solve_with_prescribed(
            &[1.0; 3],
            &[large, 0.0],
            &[0.0, -large],
            &[0.0, 1.0, 0.0],
            &[(0, 1.0), (2, 1.0)],
        )
        .unwrap(),
        [1.0; 3]
    );
}

#[test]
fn prescribed_cancellation_chain_cancels_recovery_and_publication_then_replays() {
    let delta = 2.0_f64.powi(-27);
    let fixed = 1.0 - delta;
    for (stage, step) in [
        (SolverStage::ConstraintReduce, 0),
        (SolverStage::ConstraintReduce, 1),
        (SolverStage::ConstraintReduce, 2),
        (SolverStage::ResultPrescribed, 2),
    ] {
        let control = SolverControl::default();
        let cancel = control.clone();
        let solve = || {
            solve_with_prescribed(
                &[1.0; 2],
                &[1.0 + delta],
                &[0.0],
                &[0.0, 1.0],
                &[(0, fixed)],
            )
        };
        let result = with_solver_observer(
            &control,
            move |point| {
                if point.stage == stage && point.completed_steps == step {
                    cancel.request_cancel();
                }
            },
            solve,
        );
        assert!(result.unwrap_err().contains("cancelled"));
        assert!(control.was_interrupted());
        assert_eq!(solve().unwrap(), [fixed, delta * delta]);
    }
}

#[test]
fn prescribed_cancellation_chain_replays_both_boundaries_after_addition_loss() {
    for power in [-600, 0, 600] {
        let scale = 2.0_f64.powi(power);
        let large = scale * 2.0_f64.powi(60);
        for sign in [-1.0, 1.0] {
            let result = solve_with_prescribed(
                &[scale; 3],
                &[large, 0.0],
                &[0.0, -large],
                &[0.0, sign * scale, 0.0],
                &[(0, sign), (2, sign)],
            )
            .unwrap();
            assert_eq!(result, [sign; 3]);
        }
    }
}
