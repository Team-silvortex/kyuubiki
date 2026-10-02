use super::tests::diagonal;
use super::*;
use crate::solver_control::{SolverControl, with_solver_observer};
use std::{cell::Cell, rc::Rc};

#[test]
fn binary_modal_scaling_preserves_direction_bits_across_normal_and_subnormal_ranges() {
    for scale in [1.0, 1e300, 1e-300, f64::from_bits(1)] {
        let original = [3.0 * scale, 4.0 * scale];
        let mut vector = original;
        let norm = stable_l2_norm(vector);
        let bounded = scale_binary(&mut vector, norm).unwrap();
        assert!((0.5..=1.0).contains(&bounded));
        let factor = original[0] / vector[0];
        for (before, after) in original.iter().zip(vector) {
            assert_eq!(before.to_bits(), (after * factor).to_bits());
        }
    }
    let mut vector = [3.0, 4.0];
    assert_eq!(scale_binary(&mut vector, 5.0).unwrap(), 0.625);
    assert_eq!(
        vector,
        [0.375, 0.5],
        "do not round through unit normalization"
    );
}

#[test]
fn binary_modal_scaling_rejects_invalid_norms_and_component_loss() {
    for norm in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        assert!(scale_binary(&mut [1.0], norm).is_err());
    }
    assert!(scale_binary(&mut [2.0, f64::from_bits(1)], 2.0).is_err());
    assert!(scale_binary(&mut [f64::NAN], 1.0).is_err());
}

#[test]
fn modal_orthogonalization_and_rayleigh_use_nonunit_basis_norms() {
    let mut vector = [0.1, 3.0, 4.0];
    orthogonalize(&mut vector, &[(1.0, vec![2.0, 0.0, 0.0])]).unwrap();
    assert_eq!(vector, [0.0, 0.375, 0.5]);
    assert!((rayleigh(&vector, &[0.0, 0.75, 2.0]) - 3.28).abs() < 1e-14);
    for root in [1e308, 1e-308] {
        let mut vector = [1.5, 0.0];
        orthogonalize(&mut vector, &[]).unwrap();
        assert!((rayleigh(&vector, &[root * vector[0], 0.0]) / root - 1.0).abs() < 1e-14);
    }
    for basis in [vec![1.0], vec![0.0; 3], vec![f64::NAN; 3]] {
        assert!(orthogonalize(&mut [0.0, 0.75, 1.0], &[(1.0, basis)]).is_err());
    }
}

#[test]
fn modal_smoothing_lowers_the_true_residual_at_extreme_common_scales() {
    for scale in [1.0, 1e200, 1e-200] {
        let system = diagonal(&[scale, 2.0 * scale, 4.0 * scale]);
        let mut vector = vec![1.0, 1e-3, 1e-3];
        let applied = system.operator.apply_compensated(&vector).unwrap();
        let mut value = rayleigh(&vector, &applied);
        let before = relative_residual(value, &vector, &applied).unwrap();
        smooth_residual(&system, &mut value, &mut vector, &applied, &[], 1e-8).unwrap();
        let after = relative_residual(
            value,
            &vector,
            &system.operator.apply_compensated(&vector).unwrap(),
        )
        .unwrap();
        assert!(
            after < before * 0.5,
            "scale={scale:e}, before={before:e}, after={after:e}"
        );
    }
}

#[test]
fn modal_smoothing_keeps_a_converged_or_unimproved_seed_unchanged() {
    for (values, seed, tolerance) in [
        ([1.0, 2.0], vec![1.0, 0.0], 1e-8),
        ([1.0, 3.0], vec![1.0, 1.0], 1e-18),
    ] {
        let system = diagonal(&values);
        let applied = system.operator.apply_compensated(&seed).unwrap();
        let original = rayleigh(&seed, &applied);
        let mut value = original;
        let mut vector = seed.clone();
        smooth_residual(&system, &mut value, &mut vector, &applied, &[], tolerance).unwrap();
        assert_eq!(vector, seed);
        assert_eq!(value, original);
    }
}

#[test]
fn modal_smoothing_cancellation_retains_the_seed_and_replays() {
    let system = diagonal(&(0..129).map(|i| 1.0 + i as f64).collect::<Vec<_>>());
    let mut seed = vec![1e-3; 129];
    seed[0] = 1.0;
    let applied = system.operator.apply_compensated(&seed).unwrap();
    let original = rayleigh(&seed, &applied);
    let mut value = original;
    let mut vector = seed.clone();
    let control = SolverControl::default();
    let cancel = control.clone();
    let error = with_solver_observer(
        &control,
        move |point| {
            if point.stage == SolverStage::SparseMatvec && point.completed_steps == 64 {
                cancel.request_cancel();
            }
        },
        || {
            let result = smooth_residual(&system, &mut value, &mut vector, &applied, &[], 1e-8);
            assert!(result.is_err());
            result
        },
    )
    .unwrap_err();
    assert!(error.contains("cancel"));
    assert_eq!(vector, seed);
    assert_eq!(value, original);
    smooth_residual(&system, &mut value, &mut vector, &applied, &[], 1e-8).unwrap();
    assert!(
        relative_residual(
            value,
            &vector,
            &system.operator.apply_compensated(&vector).unwrap()
        )
        .unwrap()
            < relative_residual(original, &seed, &applied).unwrap()
    );
}

#[test]
fn modal_smoothing_cannot_add_hidden_inverse_steps_or_retries() {
    for budget in [0, 1] {
        let counts = Rc::new(Cell::new([0, 0]));
        let observed = counts.clone();
        let mut pairs = [(1.0, vec![1.0, 1e-3, 1e-3])];
        let system = diagonal(&[1.0, 2.0, 3.0]);
        let error = with_solver_observer(
            &SolverControl::default(),
            move |point| {
                let mut counts = observed.get();
                if point.stage == SolverStage::ModalValidation && point.completed_steps == 0 {
                    counts[0] += 1;
                }
                if point.stage == SolverStage::DenseFactor && point.completed_steps == 0 {
                    counts[1] += 1;
                }
                observed.set(counts);
            },
            || refine_with_budget(&system, &mut pairs, 1e-8, budget),
        )
        .unwrap_err();
        assert!(error.contains(&format!("within {budget} steps")), "{error}");
        // The diagonal inverse has a prepared tridiagonal backend, not dense LU.
        assert_eq!(counts.get(), [budget, 0]);
    }
}
