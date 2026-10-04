use super::*;
use crate::solver_control::{SolverControl, with_solver_observer};
use std::{cell::Cell, rc::Rc};

#[test]
fn hybrid_qr_retains_analytic_solutions_under_binary_column_scales_and_row_orders() {
    let original = [[1.0, 2.0, -3.0, 0.0], [2.0, -1.0, 0.0, 3.0]];
    for order in [[0, 1, 2, 3], [3, 2, 1, 0], [1, 3, 0, 2]] {
        for exponents in [[0, 0], [-400, 400], [400, -400]] {
            for signs in [[1.0, -1.0], [-1.0, 1.0]] {
                let scales = [
                    signs[0] * 2.0_f64.powi(exponents[0]),
                    signs[1] * 2.0_f64.powi(exponents[1]),
                ];
                let columns: Vec<Vec<_>> = (0..2)
                    .map(|j| order.iter().map(|&i| original[j][i] * scales[j]).collect())
                    .collect();
                let rhs: Vec<_> = order
                    .iter()
                    .map(|&i| 1.25 * original[0][i] - 0.75 * original[1][i])
                    .collect();
                let actual = QrFit::factor(&columns).unwrap().solve(&rhs).unwrap();
                assert!((actual[0] * scales[0] - 1.25).abs() < 2e-15);
                assert!((actual[1] * scales[1] + 0.75).abs() < 2e-15);
                for (i, &target) in rhs.iter().enumerate() {
                    let product = actual[0].mul_add(columns[0][i], actual[1] * columns[1][i]);
                    assert!((product - target).abs() < 1e-14);
                }
            }
        }
    }
}

#[test]
fn hybrid_qr_rejects_invalid_rank_range_and_preflights_before_factorization() {
    for columns in [
        vec![],
        vec![vec![1.0]],
        vec![vec![1.0; 2]; 2],
        vec![vec![1.0; 4]; 2],
        vec![vec![0.0; 4]],
        vec![vec![1.0; 4], vec![1.0; 3]],
        vec![vec![f64::NAN; 4]],
        vec![vec![1e100, 1e-320, 0.0]],
    ] {
        assert!(QrFit::factor(&columns).is_err());
    }
    let factors = Rc::new(Cell::new(0));
    let counts = factors.clone();
    with_solver_observer(
        &SolverControl::default(),
        move |p| {
            if p.stage == SolverStage::DenseFactor {
                counts.set(counts.get() + 1);
            }
        },
        || {
            assert!(QrFit::factor(&[vec![1.0; 257]]).is_err());
            Ok::<(), String>(())
        },
    )
    .unwrap();
    assert_eq!(factors.get(), 0);
}

fn columns() -> Vec<Vec<f64>> {
    vec![
        vec![1.0; 128],
        (0..128).map(|i| f64::from(i >= 64)).collect(),
    ]
}

#[test]
fn hybrid_qr_factor_and_substitution_cancellation_preserve_borrowed_data_and_replay() {
    let columns = columns();
    let rhs: Vec<_> = (0..128)
        .map(|i| columns[0][i] + 2.0 * columns[1][i])
        .collect();
    let factor = QrFit::factor(&columns).unwrap();
    let expected = factor.solve(&rhs).unwrap();
    for (stage, steps) in [
        (SolverStage::DenseFactor, 1),
        (SolverStage::ModalVectorUpdate, 64),
        (SolverStage::DenseSubstitution, 1),
    ] {
        let control = SolverControl::default();
        let cancel = control.clone();
        let error = with_solver_observer(
            &control,
            move |p| {
                if p.stage == stage && p.completed_steps == steps {
                    cancel.request_cancel();
                }
            },
            || {
                if stage == SolverStage::DenseSubstitution {
                    factor.solve(&rhs)
                } else {
                    QrFit::factor(&columns).map(|_| Vec::new())
                }
            },
        )
        .unwrap_err();
        assert!(error.contains("cancel"), "{error}");
        assert_eq!(control.last_checkpoint().unwrap().stage, stage);
        assert_eq!(columns, self::columns());
        assert!(
            factor
                .solve(&rhs)
                .unwrap()
                .iter()
                .zip(&expected)
                .all(|(a, b)| a.to_bits() == b.to_bits())
        );
    }
}

#[test]
fn hybrid_qr_cached_factor_survives_invalid_rhs_without_refactorization() {
    let counts = Rc::new(Cell::new(0));
    let observed = counts.clone();
    with_solver_observer(
        &SolverControl::default(),
        move |p| {
            if p.stage == SolverStage::DenseFactor && p.completed_steps == 0 {
                observed.set(observed.get() + 1);
            }
        },
        || {
            let factor = QrFit::factor(&columns()).unwrap();
            let rhs = vec![1.0; 128];
            let baseline = factor.solve(&rhs).unwrap();
            assert!(factor.solve(&[1.0; 2]).is_err());
            assert!(factor.solve(&[f64::INFINITY; 128]).is_err());
            assert_eq!(factor.solve(&rhs).unwrap(), baseline);
            Ok::<(), String>(())
        },
    )
    .unwrap();
    assert_eq!(counts.get(), 1);
}
