use super::*;
use crate::solver_control::{SolverControl, with_solver_observer};
use std::cell::Cell;

fn dot(lhs: &[f64], rhs: &[f64]) -> PositiveDot {
    positive_dot(lhs, rhs).unwrap().expect("positive dot")
}

#[test]
fn ratios_survive_inner_product_underflow_and_overflow() {
    for exponent in [-1000, -600, 0, 600, 1000] {
        let unit = 2.0_f64.powi(exponent);
        let numerator = dot(&[unit, unit], &[unit, unit]);
        let denominator = dot(&[unit], &[unit]);
        assert_eq!(numerator.ratio(denominator), 2.0);
        assert_eq!(denominator.ratio(numerator), 0.5);
    }
    let tiny = dot(&[f64::from_bits(1)], &[f64::from_bits(1)]);
    assert_eq!(tiny.ratio(tiny), 1.0);
}

#[test]
fn crossed_operand_scales_do_not_erase_a_finite_inner_product() {
    let unit = 2.0_f64.powi(1000);
    let inverse = 2.0_f64.powi(-1000);
    let actual = dot(&[unit, inverse], &[inverse, unit]);
    assert_eq!(actual.ratio(dot(&[1.0], &[1.0])), 2.0);
}

#[test]
fn fallback_retains_product_tails_through_cancellation() {
    let unit = 2.0_f64.powi(-600);
    let delta = 2.0_f64.powi(-26);
    // 1 - (1+d)(1-d) = d^2; all unscaled products underflow.
    let actual = dot(
        &[unit, unit * (1.0 + delta)],
        &[unit, -unit * (1.0 - delta)],
    );
    let expected = dot(&[unit * delta], &[unit * delta]);
    assert_eq!(actual.ratio(expected), 1.0);
}

#[test]
fn invalid_or_nonpositive_products_do_not_become_positive() {
    assert!(positive_dot(&[1.0], &[]).is_err());
    for (lhs, rhs) in [
        (vec![], vec![]),
        (vec![0.0], vec![1.0]),
        (vec![-1e-300], vec![1e-300]),
        (vec![1e-300, 1e-300], vec![1e-300, -1e-300]),
        (vec![f64::NAN], vec![1.0]),
        (vec![f64::INFINITY], vec![1.0]),
    ] {
        assert!(positive_dot(&lhs, &rhs).unwrap().is_none());
    }
}

#[test]
fn ordinary_ratios_preserve_the_direct_division_bits() {
    for numerator in [0.125, 1.1, 16.0, 1e100, 1e-100] {
        for denominator in [0.25, 3.7, 32.0, 1e100, 1e-100] {
            let actual = dot(&[numerator], &[1.0]).ratio(dot(&[denominator], &[1.0]));
            assert_eq!(actual.to_bits(), (numerator / denominator).to_bits());
        }
    }
}

#[test]
fn genuinely_unrepresentable_ratios_remain_explicit_iteration_limits() {
    let large = dot(&[1e300], &[1e300]);
    let small = dot(&[1e-300], &[1e-300]);
    assert_eq!(large.ratio(small), f64::INFINITY);
    assert_eq!(small.ratio(large), 0.0);
}

#[test]
fn each_fallback_pass_can_be_cancelled_and_replayed() {
    let values = vec![1e-300; 2049];
    for pass in [2, 3] {
        let control = SolverControl::default();
        let cancel = control.clone();
        let visits = Cell::new(0);
        let result = with_solver_observer(
            &control,
            move |point| {
                if point.stage == SolverStage::PcgDot && point.completed_steps == 0 {
                    visits.set(visits.get() + 1);
                }
                if point.stage == SolverStage::PcgDot
                    && point.completed_steps == 1024
                    && visits.get() == pass
                {
                    cancel.request_cancel();
                }
            },
            || positive_dot(&values, &values),
        );
        assert!(result.err().unwrap().contains("cancelled"));
        assert!(control.was_interrupted());
    }
    let actual = dot(&values, &values);
    let single = dot(&values[..1], &values[..1]);
    assert!((actual.ratio(single) - 2049.0).abs() < 1e-10);
}
