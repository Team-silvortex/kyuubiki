use super::{smallest_tridiagonal_eigenpair, square_over, sturm_count};
use crate::modal_math::jacobi_eigenpairs;
use crate::solver_control::{SolverControl, SolverStage, with_solver_observer};

#[test]
fn soft_chain_eigenvalue_is_resolved_beyond_the_fixed_bisection_budget() {
    for soft in [1e-24, 1e-100, 1e-200, 1e-300] {
        let off = -std::f64::consts::SQRT_2 * soft;
        // det(A) / lambda_max avoids subtracting two almost equal roots.
        let high = 0.5 * (1.0 + 2.0 * soft + (1.0 - 2.0 * soft).hypot(2.0 * off));
        let expected = (2.0 * soft / high) * (1.0 - soft);
        for diagonal in [[1.0, 2.0 * soft], [2.0 * soft, 1.0]] {
            let pair = smallest_tridiagonal_eigenpair(&diagonal, &[off], 1e-9)
                .unwrap_or_else(|error| panic!("soft={soft}: {error}"));
            assert!((pair.eigenvalue / expected - 1.0).abs() < 1e-12);
        }
    }
}

#[test]
fn sturm_counts_preserve_small_positive_pivots_and_exact_zero_convention() {
    let diagonal = [1e-24, 1.0];
    let off = [1e-15];
    assert_eq!(sturm_count(&diagonal, &off, 0.0).unwrap(), 0);
    assert_eq!(sturm_count(&diagonal, &off, 1e-24 - 2e-30).unwrap(), 0);
    assert_eq!(sturm_count(&diagonal, &off, 1e-24).unwrap(), 1);
    assert_eq!(sturm_count(&diagonal, &off, 2.0).unwrap(), 2);
    assert_eq!(sturm_count(&[1.0, 1.0], &[1.0], 0.0).unwrap(), 1);
}

#[test]
fn sturm_quotient_does_not_square_small_couplings_before_division() {
    assert!((square_over(1e-200, 1e-200) / 1e-200 - 1.0).abs() < 1e-15);
    assert!((square_over(-1e-200, -1e-200) / -1e-200 - 1.0).abs() < 1e-15);
    let tiny = f64::from_bits(1);
    let reference = (1e-10 / tiny.sqrt()).powi(2);
    assert!(reference.is_finite());
    assert!((square_over(1e-10, tiny) / reference - 1.0).abs() < 1e-15);
    assert_eq!(square_over(1.0, f64::INFINITY), 0.0);
}

#[test]
fn general_tridiagonal_modes_match_discrete_sine_references() {
    for size in [2, 3, 17, 128, 257] {
        for scale in [1e-200, 1.0, 1e200] {
            let diagonal = vec![2.0 * scale; size];
            let mut off = vec![-scale; size - 1];
            // Alternating off-diagonal signs are an orthogonal sign transformation.
            for (index, value) in off.iter_mut().enumerate() {
                if index % 2 == 0 {
                    *value = -*value;
                }
            }
            let pair = smallest_tridiagonal_eigenpair(&diagonal, &off, 1e-9).unwrap();
            let theta = std::f64::consts::PI / (size + 1) as f64;
            let expected = scale * (4.0 * (theta * 0.5).sin().powi(2));
            assert!(
                (pair.eigenvalue / expected - 1.0).abs() < 1e-10,
                "size={size}"
            );
            let mut sign = 1.0;
            let reference: Vec<_> = (0..size)
                .map(|index| {
                    if index > 0 {
                        sign *= -off[index - 1].signum();
                    }
                    sign * ((index + 1) as f64 * theta).sin() * (2.0 / (size + 1) as f64).sqrt()
                })
                .collect();
            let dot: f64 = pair.vector.iter().zip(reference).map(|(a, b)| a * b).sum();
            assert!((dot.abs() - 1.0).abs() < 1e-12);
            assert!(pair.iterations > 0 && pair.iterations < 1152);
        }
    }
}

#[test]
fn interior_localized_soft_mode_recovers_from_both_ends() {
    for anchor in [1, 32, 63] {
        let mut diagonal = vec![1.0; 65];
        let mut off = vec![-0.1; 64];
        diagonal[anchor] = 1e-100;
        off[anchor - 1] = -1e-110;
        off[anchor] = -1e-110;
        let pair = smallest_tridiagonal_eigenpair(&diagonal, &off, 1e-9).unwrap();
        assert!((pair.eigenvalue / 1e-100 - 1.0).abs() < 1e-12);
        assert!((pair.vector[anchor].abs() - 1.0).abs() < 1e-12);
        assert!(pair.residual_norm / pair.eigenvalue < 1e-9);
    }
}

#[test]
fn heterogeneous_tridiagonal_lowest_modes_crosscheck_dense_spectra() {
    let mut seed = 0x7bb2_1412_0812_abcd_u64;
    let mut next = || {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        (seed >> 11) as f64 / (1_u64 << 53) as f64
    };
    for size in [2, 3, 7, 16] {
        for _ in 0..12 {
            let mut diagonal: Vec<_> = (0..size).map(|_| 1.0 + next()).collect();
            let off: Vec<_> = (0..size - 1)
                .map(|_| -10.0_f64.powf(4.0 * next() - 2.0))
                .collect();
            for (index, value) in off.iter().enumerate() {
                diagonal[index] += value.abs();
                diagonal[index + 1] += value.abs();
            }
            let mass: Vec<_> = (0..size)
                .map(|_| 10.0_f64.powf(4.0 * next() - 2.0))
                .collect();
            for (value, mass) in diagonal.iter_mut().zip(&mass) {
                *value /= mass;
            }
            let off: Vec<_> = off
                .iter()
                .enumerate()
                .map(|(index, value)| value / mass[index].sqrt() / mass[index + 1].sqrt())
                .collect();
            let mut dense = vec![vec![0.0; size]; size];
            for index in 0..size {
                dense[index][index] = diagonal[index];
                if index + 1 < size {
                    dense[index][index + 1] = off[index];
                    dense[index + 1][index] = off[index];
                }
            }
            let reference = jacobi_eigenpairs(dense).unwrap();
            let pair = smallest_tridiagonal_eigenpair(&diagonal, &off, 1e-9).unwrap();
            assert!((pair.eigenvalue / reference[0].0 - 1.0).abs() < 1e-9);
            let dot: f64 = pair
                .vector
                .iter()
                .zip(&reference[0].1)
                .map(|(a, b)| a * b)
                .sum();
            assert!((dot.abs() - 1.0).abs() < 1e-10);
        }
    }
}

#[test]
fn invalid_tridiagonal_inputs_fail_without_partial_success() {
    for (diagonal, off) in [
        (vec![], vec![]),
        (vec![1.0], vec![1.0]),
        (vec![1.0, 2.0], vec![]),
        (vec![f64::NAN, 1.0], vec![0.5]),
        (vec![1.0, f64::INFINITY], vec![0.5]),
        (vec![1.0, 2.0], vec![f64::NEG_INFINITY]),
        (vec![1.0, 2.0], vec![0.0]),
    ] {
        assert!(smallest_tridiagonal_eigenpair(&diagonal, &off, 1e-9).is_err());
    }
    for tolerance in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        let error = smallest_tridiagonal_eigenpair(&[2.0, 2.0], &[-1.0], tolerance).unwrap_err();
        assert!(error.contains("tolerance"), "{error}");
    }
    assert_eq!(
        smallest_tridiagonal_eigenpair(&[2.0], &[], 1e-9)
            .unwrap()
            .vector,
        vec![1.0]
    );
}

#[test]
fn nonpositive_tridiagonal_modes_are_not_replaced_with_positive_modes() {
    for (diagonal, off) in [
        ([1.0, 1.0], [1.0]),
        ([1.0, 1.0], [2.0]),
        ([-1.0, 2.0], [0.1]),
    ] {
        let error = smallest_tridiagonal_eigenpair(&diagonal, &off, 1e-9).unwrap_err();
        assert!(error.contains("nonpositive"), "{error}");
    }
}

#[test]
fn tridiagonal_range_loss_is_explicit_and_valid_modes_replay() {
    let error = smallest_tridiagonal_eigenpair(&[1e308, 1e-300], &[1e-310], 1e-9).unwrap_err();
    assert!(error.contains("scaling lost"), "{error}");
    let tiny = f64::from_bits(1);
    let error = smallest_tridiagonal_eigenpair(&[tiny, 2.0 * tiny], &[tiny], 1e-9).unwrap_err();
    assert!(error.contains("representable"), "{error}");
    let pair = smallest_tridiagonal_eigenpair(&[2.0 * tiny; 2], &[tiny], 1e-9).unwrap();
    assert_eq!(pair.eigenvalue, tiny);
    assert_eq!(
        smallest_tridiagonal_eigenpair(&[2.0; 2], &[-1.0], 1e-9)
            .unwrap()
            .vector
            .len(),
        2
    );
}

#[test]
fn general_tridiagonal_stages_cancel_in_chunks_and_replay() {
    let diagonal = vec![2.0; 129];
    let off = vec![-1.0; 128];
    for (stage, completed) in [
        (SolverStage::ModalValidation, 64),
        (SolverStage::ModalSweep, 64),
        (SolverStage::ModalIteration, 1),
        (SolverStage::ResultFreeDofs, 64),
        (SolverStage::ResultNodes, 64),
        (SolverStage::ResultNodeSummary, 64),
        (SolverStage::ResultTotals, 64),
    ] {
        let control = SolverControl::default();
        let cancel = control.clone();
        let error = with_solver_observer(
            &control,
            move |point| {
                if point.stage == stage && point.completed_steps == completed {
                    cancel.request_cancel();
                }
            },
            || {
                let result = smallest_tridiagonal_eigenpair(&diagonal, &off, 1e-9);
                assert!(
                    result.is_err(),
                    "{stage:?} must interrupt before scope exit"
                );
                result
            },
        )
        .unwrap_err();
        assert!(error.contains("cancel"), "{error}");
        assert_eq!(control.last_checkpoint().unwrap().stage, stage);
        assert!(smallest_tridiagonal_eigenpair(&diagonal, &off, 1e-9).is_ok());
    }
}

#[test]
fn tiny_positive_sturm_pivots_do_not_change_the_lowest_mode() {
    let diagonal = [1e-24, 1.0];
    let off = [1e-15];
    let expected = 1e-24 - 1e-30;
    for ordered in [diagonal, [diagonal[1], diagonal[0]]] {
        let pair = smallest_tridiagonal_eigenpair(&ordered, &off, 1e-9).unwrap();
        assert!((pair.eigenvalue / expected - 1.0).abs() < 1e-12);
    }
}
