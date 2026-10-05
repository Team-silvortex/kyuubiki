use super::*;
use crate::solver_control::{SolverControl, with_solver_observer};
use std::{cell::Cell, rc::Rc};

fn columns() -> Vec<Vec<f64>> {
    vec![vec![1.0, 0.0, 0.0], vec![3.0, 1.0, 0.0]]
}

#[test]
fn modal_triangular_lattice_integer_equivalence_sign_scale_and_exhaustive_control() {
    let original = columns();
    let reduced = QrFit::factor(&original).unwrap().reduced_lattice().unwrap();
    let u = &reduced.transform;
    assert_eq!((u[0][0] * u[1][1] - u[0][1] * u[1][0]).abs(), 1);
    assert!(reduced.shears > 0 && reduced.steps <= MAX_STEPS);
    let transformed: Vec<Vec<_>> = u
        .iter()
        .map(|column| {
            (0..3)
                .map(|i| {
                    column
                        .iter()
                        .zip(&original)
                        .map(|(&q, a)| q as f64 * a[i])
                        .sum()
                })
                .collect()
        })
        .collect();
    let fit = QrFit::factor(&transformed).unwrap();
    for a in [-0.5, 0.0, 0.5] {
        for b in [-0.5, 0.0, 0.5] {
            let rhs = [a, b, 0.0];
            let proposals = fit.quantized_beam(&rhs, 4, 64).unwrap();
            let objective = |x: f64, y: f64| (a - x - 3.0 * y).powi(2) + (b - y).powi(2);
            let actual = proposals
                .iter()
                .map(|v| {
                    let x = u[0][0] as f64 * v[0] + u[1][0] as f64 * v[1];
                    let y = u[0][1] as f64 * v[0] + u[1][1] as f64 * v[1];
                    objective(x, y)
                })
                .fold(f64::INFINITY, f64::min);
            let exhaustive = (-4..=4)
                .flat_map(|x| (-4..=4).map(move |y| (x, y)))
                .map(|(x, y)| objective(x as f64, y as f64))
                .fold(f64::INFINITY, f64::min);
            assert!((actual - exhaustive).abs() < 1e-13);
        }
    }
    for sign in [-1.0, 1.0] {
        for exponent in [-80, 0, 80] {
            let scale = sign * 2.0_f64.powi(exponent);
            let columns: Vec<Vec<_>> = original
                .iter()
                .map(|v| v.iter().map(|a| a * scale).collect())
                .collect();
            let fresh = QrFit::factor(&columns).unwrap().reduced_lattice().unwrap();
            assert_eq!(fresh.transform, reduced.transform);
            assert_eq!(
                (fresh.steps, fresh.shears, fresh.swaps),
                (reduced.steps, reduced.shears, reduced.swaps)
            );
        }
    }
    println!(
        "triangular lattice controls exhaustive_inputs=9 integer_pairs_per_input=81 signed_binary_transforms=6 determinant_abs=1"
    );
}

#[test]
fn modal_triangular_lattice_preflight_step_exhaustion_cancel_and_replay() {
    let fit = QrFit::factor(&columns()).unwrap();
    for limit in [0, MAX_STEPS + 1, usize::MAX] {
        assert!(fit.reduce_with_budget(limit, false).is_err());
        assert!(fit.reduce_with_budget(limit, true).is_err());
    }
    let swapping = QrFit::factor(&[vec![4.0, 0.0, 0.0], vec![0.0, 1.0, 0.0]]).unwrap();
    assert!(
        swapping
            .reduce_with_budget(1, false)
            .unwrap_err()
            .contains("step budget")
    );
    let too_large = QrFit::factor(&[vec![1e110, 0.0, 0.0], vec![0.0, 1e110, 0.0]]).unwrap();
    assert!(too_large.reduced_lattice().unwrap_err().contains("range"));
    let too_small = QrFit::factor(&[vec![1e-145, 0.0, 0.0], vec![0.0, 1e-145, 0.0]]).unwrap();
    assert!(
        too_small
            .reduced_lattice()
            .unwrap_err()
            .contains("retained-entry range")
    );
    let too_sheared = QrFit::factor(&[
        vec![1.0, 0.0, 0.0],
        vec![(2 * MAX_COEFFICIENT) as f64, 1.0, 0.0],
    ])
    .unwrap();
    assert!(
        too_sheared
            .reduced_lattice()
            .unwrap_err()
            .contains("integer bound")
    );
    let healthy = swapping.reduced_lattice().unwrap();
    let partial = swapping.reduce_with_budget(1, true).unwrap();
    assert_eq!(partial.stop, Stop::StepBudget);
    assert_eq!(partial.transform, healthy.transform);
    let partial = too_sheared.bounded_lattice().unwrap();
    assert_eq!(partial.stop, Stop::IntegerBudget);
    assert_eq!(partial.transform, vec![vec![1, 0], vec![0, 1]]);
    assert!(too_large.bounded_lattice().is_err());
    assert!(too_small.bounded_lattice().is_err());
    for step in [0, healthy.steps - 1] {
        let control = SolverControl::default();
        let cancel = control.clone();
        let seen = Rc::new(Cell::new(false));
        let observed = seen.clone();
        let failed = with_solver_observer(
            &control,
            move |p| {
                if p.stage == SolverStage::ModalRoundoffPrepare && p.completed_steps == step as u64
                {
                    observed.set(true);
                    cancel.request_cancel();
                }
            },
            || swapping.reduced_lattice(),
        );
        assert!(seen.get() && failed.unwrap_err().contains("cancel"));
        let fresh = QrFit::factor(&[vec![4.0, 0.0, 0.0], vec![0.0, 1.0, 0.0]])
            .unwrap()
            .reduced_lattice()
            .unwrap();
        assert_eq!(fresh.transform, healthy.transform);
    }
    println!(
        "triangular lattice preflight invalid_limits=3 step_exhaustions=1 range_faults=2 integer_faults=1 cancellations=2 fresh_exact_replays=2"
    );
}
