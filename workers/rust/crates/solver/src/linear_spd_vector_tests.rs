use super::*;
use crate::linear_spd::control_tests::interrupted;
use crate::solver_control::{SolverControl, with_solver_control};

fn reference_norm(values: &[f64]) -> f64 {
    let mut scale = 0.0;
    let mut sum_squares = 1.0;
    for value in values
        .iter()
        .map(|value| value.abs())
        .filter(|value| *value > 0.0)
    {
        if scale < value {
            sum_squares = 1.0 + sum_squares * (scale / value).powi(2);
            scale = value;
        } else {
            sum_squares += (value / scale).powi(2);
        }
    }
    if scale == 0.0 {
        0.0
    } else {
        scale * sum_squares.sqrt()
    }
}

fn reference_dot(lhs: &[f64], rhs: &[f64]) -> f64 {
    let mut sum = 0.0;
    for index in 0..lhs.len() {
        sum += lhs[index] * rhs[index];
    }
    sum
}

fn reference_update(x: &mut [f64], r: &mut [f64], p: &[f64], ap: &[f64], alpha: f64) -> f64 {
    let mut sum = 0.0;
    for index in 0..x.len() {
        x[index] += alpha * p[index];
        r[index] -= alpha * ap[index];
        sum += r[index] * r[index];
    }
    sum
}

fn same_bits(actual: &[f64], expected: &[f64]) {
    assert_eq!(actual.len(), expected.len());
    for (index, (&actual, &expected)) in actual.iter().zip(expected).enumerate() {
        assert!(
            actual.to_bits() == expected.to_bits() || actual.is_nan() && expected.is_nan(),
            "element {index}: {actual:?} != {expected:?}"
        );
    }
}

#[test]
fn all_vector_passes_poll_entry_interior_and_short_final_blocks() {
    for size in [0, 1, 63, 1023, 1024, 1025, 2049] {
        for steps in [0, size.min(VECTOR_CHUNK), size] {
            let lhs = vec![2.0; size];
            let rhs = vec![3.0; size];
            interrupted(SolverStage::PcgRhsScale, steps, || rhs_scale(&rhs));
            interrupted(SolverStage::PcgRhsNormalize, steps, || {
                normalize_rhs(&rhs, 3.0)
            });
            interrupted(SolverStage::PcgDirectionCopy, steps, || {
                copy_direction(&lhs, &mut vec![0.0; size])
            });
            interrupted(SolverStage::PcgDot, steps, || dot(&lhs, &rhs));
            interrupted(SolverStage::PcgNorm, steps, || l2_norm(&rhs));
            interrupted(SolverStage::PcgVectorUpdate, steps, || {
                update_solution_residual(&mut vec![0.0; size], &mut rhs.clone(), &lhs, &rhs, 0.25)
            });
            interrupted(SolverStage::PcgResidualUpdate, steps, || {
                update_residual(&rhs, 3.0, &lhs, &mut vec![0.0; size])
            });
            interrupted(SolverStage::PcgDirectionUpdate, steps, || {
                update_direction(&mut lhs.clone(), &rhs, 0.5)
            });
            interrupted(SolverStage::PcgSolutionScale, steps, || {
                rescale_solution(lhs.clone(), 3.0)
            });
        }
    }
}

#[test]
fn vector_passes_retain_original_order_and_nonfinite_behavior() {
    for size in [0, 1, 1023, 1024, 1025, 4097] {
        let values: Vec<f64> = (0..size)
            .map(|i| [1e100, -1e100, 1e-100, -0.0, 3.5][i % 5])
            .collect();
        let other: Vec<_> = (0..size)
            .map(|i| [1.0, -0.5, 1e-200, 0.0, 2.0][i % 5])
            .collect();
        let scale = values.iter().map(|v| v.abs()).fold(0.0, f64::max);
        same_bits(&[rhs_scale(&values).unwrap()], &[scale]);
        same_bits(
            &[dot(&values, &other).unwrap()],
            &[reference_dot(&values, &other)],
        );
        same_bits(&[l2_norm(&values).unwrap()], &[reference_norm(&values)]);
        let normalized: Vec<_> = values.iter().map(|v| v / 7.0).collect();
        same_bits(&normalize_rhs(&values, 7.0).unwrap(), &normalized);
        same_bits(
            &rescale_solution(values.clone(), 7.0).unwrap(),
            &values.iter().map(|v| v * 7.0).collect::<Vec<_>>(),
        );
        let mut copied = vec![f64::NAN; size];
        copy_direction(&values, &mut copied).unwrap();
        same_bits(&copied, &values);
        let mut direction = values.clone();
        update_direction(&mut direction, &other, 0.125).unwrap();
        same_bits(
            &direction,
            &values
                .iter()
                .zip(&other)
                .map(|(p, z)| z + 0.125 * p)
                .collect::<Vec<_>>(),
        );

        let (mut x, mut r) = (values.clone(), values.clone());
        let (mut expected_x, mut expected_r) = (values.clone(), values.clone());
        let expected = reference_update(&mut expected_x, &mut expected_r, &values, &other, 0.25);
        let actual = update_solution_residual(&mut x, &mut r, &values, &other, 0.25).unwrap();
        same_bits(&[actual], &[expected]);
        same_bits(&x, &expected_x);
        same_bits(&r, &expected_r);
        let mut expected_sum = 0.0;
        for index in 0..size {
            expected_r[index] = values[index] / 7.0 - other[index];
            expected_sum += expected_r[index] * expected_r[index];
        }
        same_bits(
            &[update_residual(&values, 7.0, &other, &mut r).unwrap()],
            &[expected_sum],
        );
        same_bits(&r, &expected_r);
    }
    for values in [
        vec![0.0, -0.0],
        vec![f64::NAN, 2.0],
        vec![f64::INFINITY],
        vec![f64::INFINITY, f64::INFINITY],
        vec![1e300, 1e-300],
    ] {
        same_bits(&[l2_norm(&values).unwrap()], &[reference_norm(&values)]);
        same_bits(
            &[rhs_scale(&values).unwrap()],
            &[values.iter().map(|v| v.abs()).fold(0.0, f64::max)],
        );
    }
}

#[test]
fn interrupted_update_exposes_only_discardable_prefix_scratch() {
    let (mut x, mut r) = (vec![0.0; 2049], vec![2.0; 2049]);
    interrupted(SolverStage::PcgVectorUpdate, 1024, || {
        update_solution_residual(&mut x, &mut r, &vec![1.0; 2049], &vec![2.0; 2049], 0.25)
    });
    assert!(x[..1024].iter().all(|v| *v == 0.25));
    assert!(r[..1024].iter().all(|v| *v == 1.5));
    assert!(x[1024..].iter().all(|v| *v == 0.0));
    assert!(r[1024..].iter().all(|v| *v == 2.0));
}

#[test]
fn a_requested_token_does_not_touch_vector_scratch() {
    let control = SolverControl::default();
    control.request_cancel();
    let mut direction = vec![5.0; 2049];
    let result = with_solver_control(&control, || {
        let result = copy_direction(&vec![1.0; 2049], &mut direction);
        assert!(result.is_err());
        result
    });
    assert!(result.is_err());
    assert_eq!(direction, vec![5.0; 2049]);
}

#[path = "linear_spd_vector_benchmark.rs"]
mod benchmark;
