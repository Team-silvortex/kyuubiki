use super::{
    coupling_resolved, jacobi_connected_eigenpairs, jacobi_eigenpairs,
    jacobi_eigenpairs_with_max_sweeps,
};
use crate::linear_algebra::stable_l2_norm;
use crate::solver_control::{SolverControl, SolverStage, with_solver_observer};

fn check_eigenpairs(matrix: &[Vec<f64>], pairs: &[(f64, Vec<f64>)]) {
    assert_eq!(pairs.len(), matrix.len());
    let scale = matrix
        .iter()
        .flatten()
        .copied()
        .map(f64::abs)
        .fold(0.0_f64, f64::max);
    for (index, (value, vector)) in pairs.iter().enumerate() {
        assert!(value.is_finite());
        assert!((stable_l2_norm(vector.iter().copied()) - 1.0).abs() < 1e-12);
        let residual = stable_l2_norm(matrix.iter().zip(vector).map(|(row, component)| {
            row.iter().zip(vector).map(|(a, b)| a * b).sum::<f64>() - value * component
        }));
        let relative = if *value == 0.0 {
            residual / scale
        } else {
            residual / value.abs()
        };
        assert!(
            relative < 1e-11,
            "mode={index}, lambda={value:e}, relative={relative:e}"
        );
        for (_, other) in pairs.iter().skip(index + 1) {
            let dot: f64 = vector.iter().zip(other).map(|(a, b)| a * b).sum();
            assert!(dot.abs() < 1e-12);
        }
    }
    assert!(pairs.windows(2).all(|pair| pair[0].0 <= pair[1].0));
}

#[test]
fn jacobi_convergence_cannot_hide_a_soft_mode_coupling_behind_a_stiff_neighbor() {
    for soft in [1e-24, 1e-100, 1e-200, 1e-300] {
        let coupling = -std::f64::consts::SQRT_2 * soft;
        assert!(!coupling_resolved(coupling, 1.0, 2.0 * soft));
        assert!(!coupling_resolved(coupling, 2.0 * soft, 1.0));
    }
}

#[test]
fn complete_jacobi_spectrum_retains_small_components_and_relative_residuals() {
    for soft in [1e-24, 1e-100, 1e-200, 1e-300] {
        let coupling = -std::f64::consts::SQRT_2 * soft;
        for diagonal in [[1.0, 2.0 * soft], [2.0 * soft, 1.0]] {
            let matrix = vec![vec![diagonal[0], coupling], vec![coupling, diagonal[1]]];
            let pairs = jacobi_eigenpairs(matrix.clone()).unwrap();
            for (value, vector) in &pairs {
                let residual = stable_l2_norm(matrix.iter().zip(vector).map(|(row, component)| {
                    row.iter().zip(vector).map(|(a, b)| a * b).sum::<f64>() - value * component
                }));
                assert!(
                    residual / value.abs() < 1e-12,
                    "soft={soft:e}, residual={residual:e}"
                );
            }
            assert!((pairs[0].0 / (2.0 * soft) - 1.0).abs() < 1e-12);
            assert!((pairs[1].0 - 1.0).abs() < 1e-12);
            let stiff_index = usize::from(diagonal[0] != 1.0);
            let tiny_component = pairs[0].1[stiff_index].abs();
            assert!((tiny_component / (-coupling) - 1.0).abs() < 1e-12);
        }
    }
}

#[test]
fn jacobi_stopping_rule_is_scale_symmetric_and_has_no_absolute_floor() {
    for scale in [1e-310, 1e-200, 1.0, 1e200] {
        for sign in [-1.0, 1.0] {
            let (a, b) = (4.0 * scale, sign * scale);
            assert!(coupling_resolved(0.5e-12 * scale, a, b));
            assert!(coupling_resolved(-0.5e-12 * scale, b, a));
            assert!(!coupling_resolved(2e-12 * scale, a, b));
            assert!(!coupling_resolved(-2e-12 * scale, b, a));
        }
    }
    let tiny = f64::from_bits(1);
    assert!(!coupling_resolved(tiny, 1.0, 1e-318));
    assert!(!coupling_resolved(tiny, 0.0, 1.0));
    assert!(!coupling_resolved(-tiny, 0.0, 0.0));
    assert!(coupling_resolved(-0.0, 0.0, -1.0));
}

#[test]
fn complete_spectra_retain_zero_and_negative_modes_without_regularization() {
    for sign in [-1.0, 1.0] {
        for diagonal in [[-1.0, 2e-100], [-1.0, -2e-100]] {
            let matrix = vec![
                vec![diagonal[0], sign * 1e-100],
                vec![sign * 1e-100, diagonal[1]],
            ];
            let pairs = jacobi_eigenpairs(matrix.clone()).unwrap();
            check_eigenpairs(&matrix, &pairs);
            assert!((pairs[0].0 + 1.0).abs() < 1e-12);
            assert!((pairs[1].0 / diagonal[1] - 1.0).abs() < 1e-12);
        }
        let matrix = vec![vec![0.0, sign * 1e-200], vec![sign * 1e-200, 0.0]];
        let pairs = jacobi_eigenpairs(matrix.clone()).unwrap();
        check_eigenpairs(&matrix, &pairs);
        assert!((pairs[0].0 / -1e-200 - 1.0).abs() < 1e-12);
        assert!((pairs[1].0 / 1e-200 - 1.0).abs() < 1e-12);
    }
    let matrix = vec![vec![1.0, -1.0], vec![-1.0, 1.0]];
    let pairs = jacobi_eigenpairs(matrix.clone()).unwrap();
    check_eigenpairs(&matrix, &pairs);
    assert_eq!(pairs[0].0, 0.0);
    assert!((pairs[1].0 - 2.0).abs() < 1e-12);
}

#[test]
fn connected_soft_modes_keep_permutation_and_sign_invariant_spectra() {
    let soft = [1e-24, 1e-100, 1e-200];
    let mut base = vec![vec![0.0; 4]; 4];
    base[0][0] = 1.0;
    for (index, value) in soft.into_iter().enumerate() {
        base[index + 1][index + 1] = 2.0 * value;
        base[0][index + 1] = -value;
        base[index + 1][0] = -value;
    }
    let signs = [1.0, -1.0, 1.0, -1.0];
    for order in [[0, 1, 2, 3], [3, 0, 2, 1], [2, 3, 1, 0]] {
        let matrix: Vec<Vec<_>> = order
            .iter()
            .map(|&row| {
                order
                    .iter()
                    .map(|&column| signs[row] * base[row][column] * signs[column])
                    .collect()
            })
            .collect();
        let pairs = jacobi_eigenpairs(matrix.clone()).unwrap();
        check_eigenpairs(&matrix, &pairs);
        for ((value, _), expected) in pairs.iter().zip([2e-200, 2e-100, 2e-24, 1.0]) {
            assert!((value / expected - 1.0).abs() < 1e-12);
        }
    }
}

#[test]
fn complete_soft_spectra_survive_common_scales_and_preserve_orthogonality() {
    for scale in [1e-200, 1.0, 1e200] {
        let matrix = vec![
            vec![scale, -1e-100 * scale],
            vec![-1e-100 * scale, 2e-100 * scale],
        ];
        let pairs = jacobi_eigenpairs(matrix.clone()).unwrap();
        check_eigenpairs(&matrix, &pairs);
        assert!((pairs[0].0 / (2e-100 * scale) - 1.0).abs() < 1e-12);
        assert!((pairs[1].0 / scale - 1.0).abs() < 1e-12);
        assert!((pairs[0].1[0].abs() / 1e-100 - 1.0).abs() < 1e-12);
    }
}

#[test]
fn required_soft_mode_rotations_cannot_bypass_the_sweep_budget() {
    let matrix = vec![vec![1.0, -1e-100], vec![-1e-100, 2e-100]];
    let error = jacobi_eigenpairs_with_max_sweeps(matrix.clone(), 0).unwrap_err();
    assert!(
        error.contains("did not converge within 0 sweeps"),
        "{error}"
    );
    let pairs = jacobi_eigenpairs_with_max_sweeps(matrix.clone(), 1).unwrap();
    check_eigenpairs(&matrix, &pairs);
    let diagonal = vec![vec![1.0, 0.0], vec![0.0, 1e-100]];
    assert!(jacobi_eigenpairs_with_max_sweeps(diagonal, 0).is_ok());
}

#[test]
fn soft_mode_jacobi_sweep_cancellation_propagates_before_success_and_replays() {
    let matrix = vec![vec![1.0, -1e-100], vec![-1e-100, 2e-100]];
    let control = SolverControl::default();
    let cancel = control.clone();
    let error = with_solver_observer(
        &control,
        move |point| {
            if point.stage == SolverStage::ModalSweep && point.completed_steps == 1 {
                cancel.request_cancel();
            }
        },
        || {
            let result = jacobi_connected_eigenpairs(matrix.clone(), 40);
            assert!(
                result.is_err(),
                "the rotation sweep must observe cancellation before scope exit"
            );
            result
        },
    )
    .unwrap_err();
    assert!(error.contains("cancel"), "{error}");
    assert_eq!(control.last_checkpoint().unwrap().completed_steps, 1);
    let pairs = jacobi_connected_eigenpairs(matrix.clone(), 40).unwrap();
    check_eigenpairs(&matrix, &pairs);
}
