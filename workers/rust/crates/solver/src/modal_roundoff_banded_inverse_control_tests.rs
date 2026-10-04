use super::{Wide, bits, reference::Factor};
use crate::solver_control::{SolverControl, SolverStage, with_solver_observer};
use std::{cell::Cell, rc::Rc};

fn matrix(size: usize, scale: f64) -> Vec<Vec<Wide>> {
    (0..size)
        .map(|i| {
            (0..size)
                .map(|j| {
                    Wide::from(
                        scale
                            * match i.abs_diff(j) {
                                0 => 8.0,
                                1 => -1.0,
                                2 => 0.25,
                                3 => 0.125,
                                _ => 0.0,
                            },
                    )
                })
                .collect()
        })
        .collect()
}

#[test]
fn modal_banded_inverse_known_solutions_and_binary_factor_scaling() {
    for size in [2, 4, 65, 256] {
        for scale in [2.0_f64.powi(-100), 1.0, 2.0_f64.powi(100)] {
            let matrix = matrix(size, scale);
            let factor = Factor::prepare(&matrix).unwrap();
            for sign in [1.0, -1.0] {
                let expected: Vec<_> = (0..size).map(|i| sign * (1 + i % 7) as f64).collect();
                let rhs: Vec<_> = matrix
                    .iter()
                    .map(|row| {
                        row.iter()
                            .zip(&expected)
                            .fold(Wide::default(), |sum, (&a, &x)| {
                                sum.add(a.mul(Wide::from(x)))
                            })
                    })
                    .collect();
                let result = factor.solve(&rhs).unwrap();
                assert_eq!(
                    bits(&result.iter().map(|v| v.rounded()).collect::<Vec<_>>()),
                    bits(&expected)
                );
                let error = result
                    .iter()
                    .zip(&expected)
                    .fold(0.0_f64, |norm, (&v, &x)| {
                        norm.hypot(v.sub(Wide::from(x)).rounded())
                    });
                assert!(error < 1e-25);
            }
        }
    }
    let compact = vec![
        vec![Wide::from(2.0), Wide::from(1.0)],
        vec![Wide::from(1.0), Wide::from(3.0)],
    ];
    let result = Factor::prepare(&compact)
        .unwrap()
        .solve(&[Wide::from(1.0), Wide::from(2.0)])
        .unwrap();
    assert_eq!(
        result.iter().map(|v| v.rounded()).collect::<Vec<_>>(),
        [0.2, 0.6]
    );
}

#[test]
fn modal_banded_inverse_invalid_range_shape_and_iteration_budgets_fail_closed() {
    for bad in [
        vec![],
        vec![vec![Wide::from(1.0)]],
        vec![vec![Wide::from(1.0); 2]],
        matrix(257, 1.0),
        matrix(2, 0.0),
        matrix(2, -1.0),
        matrix(2, 1e-320),
    ] {
        assert!(Factor::prepare(&bad).is_err());
    }
    let mut asymmetric = matrix(4, 1.0);
    asymmetric[1][0] = Wide::from(-0.5);
    assert!(Factor::prepare(&asymmetric).is_err());
    let mut out_of_band = matrix(5, 1.0);
    out_of_band[0][4] = Wide::from(0.25);
    out_of_band[4][0] = Wide::from(0.25);
    assert!(Factor::prepare(&out_of_band).is_err());
    for bad in [
        Wide {
            high: f64::NAN,
            low: 0.0,
        },
        Wide {
            high: 1.0,
            low: f64::INFINITY,
        },
        Wide {
            high: 1e51,
            low: 0.0,
        },
        Wide {
            high: 0.0,
            low: 1.0,
        },
    ] {
        let mut malformed = matrix(2, 1.0);
        malformed[0][0] = bad;
        assert!(Factor::prepare(&malformed).is_err());
    }
    let lost = vec![
        vec![Wide::from(1.0), Wide::from(1e-320)],
        vec![Wide::from(1e-320), Wide::from(1e50)],
    ];
    assert!(
        Factor::prepare(&lost)
            .err()
            .unwrap()
            .contains("lost a nonzero")
    );
    let factor = Factor::prepare(&matrix(2, 1.0)).unwrap();
    assert!(factor.solve(&[]).is_err());
    assert!(
        factor
            .solve(
                &[Wide {
                    high: f64::NAN,
                    low: 0.0
                }; 2]
            )
            .is_err()
    );
    for (seed, steps) in [
        (vec![1.0, 2.0], 0),
        (vec![1.0, 2.0], 5),
        (vec![0.0; 2], 4),
        (vec![f64::NAN; 2], 4),
        (vec![f64::INFINITY; 2], 4),
        (vec![1.0], 4),
    ] {
        assert!(factor.directions(&seed, steps).is_err());
    }
    let tiny = Factor::prepare(&matrix(2, 1e-308)).unwrap();
    assert!(tiny.solve(&[Wide::from(1e50); 2]).is_err());
    assert!(factor.directions(&[1.0, 2.0], 4).is_ok());
}

#[test]
fn modal_banded_inverse_factor_substitution_and_last_iteration_cancellation_replay() {
    let size = 65;
    let matrix = matrix(size, 1.0);
    let original: Vec<_> = matrix
        .iter()
        .flatten()
        .map(|v| [v.high.to_bits(), v.low.to_bits()])
        .collect();
    let seed: Vec<_> = (0..size).map(|i| (1 + i % 7) as f64).collect();
    let factor = Factor::prepare(&matrix).unwrap();
    let baseline = factor.directions(&seed, 4).unwrap();
    for (stage, steps, prepare) in [
        (SolverStage::BandedFactor, 0, true),
        (SolverStage::BandedFactor, size + 17, true),
        (SolverStage::BandedSubstitution, size, false),
        (SolverStage::BandedSubstitution, 2 * size, false),
        (SolverStage::BandedSubstitution, 3 * size, false),
        (SolverStage::ModalIteration, 4, false),
    ] {
        let control = SolverControl::default();
        let cancel = control.clone();
        let observed = Rc::new(Cell::new(false));
        let saw = observed.clone();
        let error = with_solver_observer(
            &control,
            move |point| {
                if point.stage == stage && point.completed_steps == steps as u64 {
                    saw.set(true);
                    cancel.request_cancel();
                }
            },
            || {
                if prepare {
                    Factor::prepare(&matrix).map(|_| ())
                } else {
                    factor.directions(&seed, 4).map(|_| ())
                }
            },
        )
        .unwrap_err();
        assert!(observed.get() && error.contains("cancel"));
        let replay = factor.directions(&seed, 4).unwrap();
        for (actual, expected) in replay.iter().zip(&baseline) {
            assert_eq!(
                actual
                    .iter()
                    .map(|v| [v.high.to_bits(), v.low.to_bits()])
                    .collect::<Vec<_>>(),
                expected
                    .iter()
                    .map(|v| [v.high.to_bits(), v.low.to_bits()])
                    .collect::<Vec<_>>()
            );
        }
    }
    assert_eq!(
        matrix
            .iter()
            .flatten()
            .map(|v| [v.high.to_bits(), v.low.to_bits()])
            .collect::<Vec<_>>(),
        original
    );
}
