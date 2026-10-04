use super::*;
use crate::solver_control::{SolverControl, with_solver_observer};
use std::{cell::RefCell, rc::Rc};

fn identity(size: usize) -> Vec<Vec<Wide>> {
    (0..size)
        .map(|i| (0..size).map(|j| Wide::from(f64::from(i == j))).collect())
        .collect()
}

fn centers(size: usize) -> Vec<Wide> {
    let unit = 1.5_f64.next_up() - 1.5;
    (0..size)
        .map(|i| Wide::from(1.5).add(Wide::from(unit * (i + 1) as f64 / (size + 1) as f64)))
        .collect()
}

fn run(
    fit: &Search,
    center: &[Wide],
    partitioned: bool,
    checked: impl FnMut(&[f64]) -> Result<(f64, Vec<f64>), String>,
) -> Result<Vec<f64>, String> {
    if partitioned {
        fit.correct_partitioned(
            center,
            &(0..center.len()).step_by(2).collect::<Vec<_>>(),
            1e-8,
            checked,
        )
    } else {
        fit.correct(center, 1e-8, checked)
    }
}

fn same_wide(a: &[Wide], b: &[Wide]) -> bool {
    a.iter()
        .zip(b)
        .all(|(a, b)| a.high.to_bits() == b.high.to_bits() && a.low.to_bits() == b.low.to_bits())
}

#[test]
fn coherent_phase_eight_coordinate_move_crosses_an_independent_local_barrier() {
    let mut matrix = vec![vec![Wide::default(); 9]; 9];
    matrix[0][0] = Wide::from(1.0);
    for (i, row) in matrix.iter_mut().enumerate().take(8).skip(1) {
        row[i] = Wide::from(1.0);
        row[i + 1] = Wide::from(-1.0);
    }
    matrix[8][1..].fill(Wide::from(1.0 / 32.0));
    for permutation in [
        [0, 1, 2, 3, 4, 5, 6, 7, 8],
        [8, 3, 0, 2, 7, 1, 6, 4, 5],
        [1, 5, 3, 7, 0, 2, 4, 6, 8],
    ] {
        for sign in [1.0, -1.0] {
            let seed = [sign * 1.5_f64; 9];
            let unit = seed[0].next_up() - seed[0];
            let checked = |v: &[f64]| {
                let mut d = [0.0; 9];
                for (i, &j) in permutation.iter().enumerate() {
                    d[j] = v[i] - seed[i];
                }
                let mut closed = [0.0; 9];
                closed[0] = d[0];
                for i in 1..8 {
                    closed[i] = d[i] - d[i + 1];
                }
                closed[8] = d[1..].iter().sum::<f64>() / 32.0 - unit / 4.0;
                let residual = permutation.map(|i| closed[i]).to_vec();
                let relative = super::super::vector_norm(residual.iter().copied())? / (unit / 4.0);
                Ok((relative, residual))
            };
            let permuted: Vec<Vec<_>> = permutation
                .iter()
                .map(|&i| permutation.iter().map(|&j| matrix[i][j]).collect())
                .collect();
            let local =
                super::super::physical_neighborhood::Neighborhood::prepare(permuted.clone(), 4)
                    .unwrap();
            let error = local.correct(&seed, 1e-8, checked).unwrap_err();
            assert!(error.contains("relative=1e0"), "{error}");
            // A deliberately approximate phase hint, not an exact continuous solution.
            let center: Vec<_> = permutation
                .iter()
                .map(|&i| {
                    Wide::from(seed[0]).add(Wide::from(if i == 0 { 0.0 } else { unit / 4.0 }))
                })
                .collect();
            let fit = Search::prepare(permuted).unwrap();
            let expected = permutation.map(|i| if i == 0 { seed[0] } else { seed[0].next_up() });
            for partitioned in [false, true] {
                let accepted = run(&fit, &center, partitioned, checked).unwrap();
                assert_eq!(accepted, expected);
                assert_eq!(checked(&accepted).unwrap().0, 0.0);
            }
            for trial in 0..3_usize.pow(8) {
                let mut encoded = trial;
                let d = std::array::from_fn::<_, 8, _>(|_| {
                    let next = [0.0, unit, -unit][encoded % 3];
                    encoded /= 3;
                    next
                });
                if d.iter().filter(|&&v| v != 0.0).count() <= 4 {
                    let score = d.windows(2).map(|w| (w[0] - w[1]).powi(2)).sum::<f64>()
                        + (d.iter().sum::<f64>() / 32.0 - unit / 4.0).powi(2);
                    assert!(score >= (unit / 4.0).powi(2));
                }
            }
            assert_eq!(seed, [sign * 1.5; 9]);
        }
    }
}

#[test]
fn coherent_phase_two_axes_recover_a_vector_outside_the_one_axis_family() {
    let fit = Search::prepare(identity(4)).unwrap();
    let center = centers(4);
    let brackets = Brackets::new(&center).unwrap();
    let expected = [1.5_f64.next_up(), 1.5, 1.5_f64.next_up(), 1.5];
    let unit = 1.5_f64.next_up() - 1.5;
    let checked = |v: &[f64]| {
        let residual: Vec<_> = v
            .iter()
            .zip(expected)
            .map(|(&v, target)| v - target)
            .collect();
        Ok((
            super::super::vector_norm(residual.iter().copied())? / unit,
            residual,
        ))
    };
    let residual: Vec<_> = checked(&brackets.nearest)
        .unwrap()
        .1
        .into_iter()
        .map(Wide::from)
        .collect();
    let one = fit.rank(&brackets, &residual).unwrap();
    assert!(one.iter().all(|p| p.vector != expected));
    let two = fit.rank_partitioned(&brackets, &residual, &[0, 2]).unwrap();
    assert_eq!(two[0].vector, expected);
    assert_eq!(two[0].score.rounded(), 0.0);
    for candidate in one.iter().chain(&two) {
        let (_, closed) = checked(&candidate.vector).unwrap();
        let square = closed.iter().map(|v| v * v).sum::<f64>();
        assert_eq!(candidate.score.rounded(), square);
    }
    assert!(
        run(&fit, &center, false, checked)
            .unwrap_err()
            .contains("unchanged residual gate")
    );
    assert_eq!(run(&fit, &center, true, checked).unwrap(), expected);
    assert_eq!(checked(&expected).unwrap().0, 0.0);
}

#[test]
fn coherent_phase_brackets_keep_fraction_tails_ties_and_exact_coordinates() {
    let unit = 1.5_f64.next_up() - 1.5;
    let hint = Wide::from(1.5).add(Wide::from(unit / 4.0));
    let brackets = Brackets::new(&[hint, hint, Wide::from(2.0)]).unwrap();
    assert_eq!(brackets.events.len(), 2);
    assert_eq!(brackets.lower[2], brackets.upper[2]);
    let fit = Search::prepare(identity(3)).unwrap();
    let ranked = fit.rank(&brackets, &[Wide::default(); 3]).unwrap();
    assert_eq!(ranked.len(), 1);
    assert_eq!(
        ranked[0].vector,
        [1.5_f64.next_up(), 1.5_f64.next_up(), 2.0]
    );
    let half = [
        Wide::from(1.5).add(Wide::from(unit / 2.0)),
        Wide::from(1.5_f64.next_up()).add(Wide::from(unit / 2.0)),
    ];
    let tied = Brackets::new(&half).unwrap();
    let two = Search::prepare(identity(2)).unwrap();
    let patterns = two.rank(&tied, &[Wide::default(); 2]).unwrap();
    assert!(patterns.iter().all(|p| p.vector != tied.nearest));
    assert_eq!(
        two.correct(&half, 1e-8, |_| Ok((0.0, vec![0.0; 2])))
            .unwrap(),
        tied.nearest
    );
    let tails = [
        Wide {
            high: 1.5,
            low: -unit * 2.0_f64.powi(-60),
        },
        Wide {
            high: 1.5,
            low: -unit * 2.0_f64.powi(-61),
        },
    ];
    let brackets = Brackets::new(&tails).unwrap();
    assert_eq!(
        brackets.events.iter().map(|e| e.1).collect::<Vec<_>>(),
        [1, 0]
    );
    assert_eq!(brackets.events[0].0.high, brackets.events[1].0.high);
    assert!(brackets.events[0].0.low > brackets.events[1].0.low);
    assert_eq!(
        brackets.events[0].0.rounded(),
        brackets.events[1].0.rounded()
    );
    let parts = super::super::wide::unit_shape_parts(&[Wide::from(3.0), Wide::from(4.0)]).unwrap();
    assert_eq!(
        parts.iter().map(|v| v.rounded()).collect::<Vec<_>>(),
        [0.6, 0.8]
    );
    assert!(parts.iter().any(|v| v.low != 0.0));
}

#[test]
fn coherent_phase_dimensions_ranges_partitions_and_resource_caps_preflight() {
    for size in [2, 9, 128, 256] {
        let plan = Plan::new(size).unwrap();
        assert!(plan.payload_bytes <= 8 * 1024 * 1024);
        assert!(plan.component_visits <= 40_000_000);
        assert!(plan.certificates <= MAX_CERTIFICATES);
    }
    let maximum = Plan::new(256).unwrap();
    assert_eq!(maximum.payload_bytes, 4_718_592);
    assert_eq!(maximum.component_visits, 38_797_312);
    assert_eq!(maximum.patterns, 257);
    assert_eq!(maximum.combinations, 16_641);
    assert_eq!(maximum.certificates, 66);
    for size in [0, 1, 257, usize::MAX] {
        assert!(Plan::new(size).is_err());
    }
    for matrix in [
        vec![],
        vec![vec![Wide::from(1.0)]],
        vec![vec![Wide::from(1.0); 2]; 3],
        identity(257),
    ] {
        assert!(Search::prepare(matrix).is_err());
    }
    let fit = Search::prepare(identity(3)).unwrap();
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
            high: 0.0,
            low: 1.0,
        },
        Wide::from(1e51),
        Wide::from(1e-51),
    ] {
        let mut matrix = identity(3);
        matrix[0][0] = bad;
        assert!(Search::prepare(matrix).is_err());
        assert!(fit.correct(&[bad; 3], 1e-8, |_| unreachable!()).is_err());
    }
    for tolerance in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        assert!(
            fit.correct(&centers(3), tolerance, |_| unreachable!())
                .is_err()
        );
    }
    assert!(fit.correct(&centers(2), 1e-8, |_| unreachable!()).is_err());
    for indices in [vec![], vec![0, 1, 2], vec![0, 0], vec![1, 0], vec![3]] {
        assert!(
            fit.correct_partitioned(&centers(3), &indices, 1e-8, |_| unreachable!())
                .is_err()
        );
    }
    let unresolved = [Wide {
        high: 1.5,
        low: f64::from_bits(1),
    }; 3];
    assert!(
        fit.correct(&unresolved, 1e-8, |_| unreachable!())
            .unwrap_err()
            .contains("fraction is not resolved")
    );
    let zero = Search::prepare(vec![vec![Wide::default(); 3]; 3]).unwrap();
    assert_eq!(
        zero.correct(&[Wide::default(); 3], 1e-8, |_| Ok((0.0, vec![0.0; 3])))
            .unwrap(),
        [0.0; 3]
    );
}

#[test]
fn coherent_phase_shortlist_uses_one_counter_and_requires_a_fresh_final_certificate() {
    let fit = Search::prepare(identity(128)).unwrap();
    let center = centers(128);
    for partitioned in [false, true] {
        let visits = Rc::new(RefCell::new(Vec::new()));
        let observed = visits.clone();
        let mut calls = 0;
        let error = with_solver_observer(
            &SolverControl::default(),
            move |p| {
                if p.stage == SolverStage::ModalRoundoffSearch {
                    observed.borrow_mut().push(p.completed_steps);
                }
            },
            || {
                run(&fit, &center, partitioned, |_| {
                    calls += 1;
                    Ok((1.0 / (calls + 1) as f64, vec![0.0; 128]))
                })
            },
        )
        .unwrap_err();
        assert!(error.contains("unchanged residual gate"), "{error}");
        assert_eq!(calls, MAX_CERTIFICATES);
        assert_eq!(
            visits.borrow().as_slice(),
            (0..MAX_CERTIFICATES as u64).collect::<Vec<_>>()
        );
        let mut calls = 0;
        let error = run(&fit, &center, partitioned, |_| {
            calls += 1;
            Ok((if calls == 2 { 0.0 } else { 1.0 }, vec![0.0; 128]))
        })
        .unwrap_err();
        assert!(error.contains("unchanged residual gate"), "{error}");
        assert_eq!(
            calls, 3,
            "a passing searched candidate still needs a fresh final check"
        );
        for (relative, residual) in [
            (1.0, vec![0.0; 128]),
            (f64::NAN, vec![0.0; 128]),
            (-1.0, vec![0.0; 128]),
            (0.0, vec![0.0; 127]),
            (0.0, vec![f64::INFINITY; 128]),
            (0.0, vec![1e51; 128]),
            (0.0, vec![1e-51; 128]),
        ] {
            let mut calls = 0;
            let error = run(&fit, &center, partitioned, |_| {
                calls += 1;
                if calls == 1 {
                    Ok((0.0, vec![0.0; 128]))
                } else {
                    Ok((relative, residual.clone()))
                }
            })
            .unwrap_err();
            assert!(
                error.contains("unchanged residual gate")
                    || error.contains("finite matching certificate"),
                "{error}"
            );
            assert_eq!(calls, 2);
        }
    }
}

#[test]
fn coherent_phase_prediction_cannot_override_actual_failure_or_mutate_the_center() {
    let fit = Search::prepare(identity(3)).unwrap();
    let center = centers(3);
    let before = center.clone();
    let nearest: Vec<_> = center.iter().map(|v| v.rounded()).collect();
    let mut target = nearest.clone();
    target[1] = target[1].next_up();
    let unit = target[1] - nearest[1];
    for partitioned in [false, true] {
        let mut seen = Vec::new();
        let error = run(&fit, &center, partitioned, |v| {
            seen.push(v.to_vec());
            Ok((if v == nearest { 1.0 } else { 2.0 }, vec![0.0, -unit, 0.0]))
        })
        .unwrap_err();
        assert!(error.contains("relative=1e0"), "{error}");
        assert!(seen.iter().any(|v| v != &nearest));
        assert_eq!(
            seen[1], target,
            "zero predicted score cannot certify the target"
        );
        assert_eq!(seen.last().unwrap(), &nearest);
        for (relative, residual) in [(f64::NAN, vec![0.0; 3]), (1.0, vec![0.0; 2])] {
            let mut calls = 0;
            let error = run(&fit, &center, partitioned, |_| {
                calls += 1;
                Ok((relative, residual.clone()))
            })
            .unwrap_err();
            assert!(error.contains("finite matching certificate"));
            assert_eq!(calls, 1);
        }
        for fail_at in [1, 2] {
            let mut calls = 0;
            let error = run(&fit, &center, partitioned, |_| {
                calls += 1;
                if calls == fail_at {
                    Err("actual operator failure".into())
                } else {
                    Ok((1.0, vec![0.0; 3]))
                }
            })
            .unwrap_err();
            assert_eq!(error, "actual operator failure");
            assert_eq!(calls, fail_at);
        }
        assert!(same_wide(&center, &before));
        assert_eq!(
            run(&fit, &center, partitioned, |_| Ok((0.0, vec![0.0; 3]))).unwrap(),
            nearest
        );
    }
}

#[test]
fn coherent_phase_preparation_and_search_cancel_without_partial_shapes_and_replay() {
    let matrix = identity(3);
    let before = matrix.clone();
    for stage in [
        SolverStage::ModalRoundoffPrepare,
        SolverStage::ModalVectorScan,
    ] {
        let control = SolverControl::default();
        let cancel = control.clone();
        let error = with_solver_observer(
            &control,
            move |p| {
                if p.stage == stage {
                    cancel.request_cancel();
                }
            },
            || Search::prepare(matrix.clone()).map(|_| ()),
        )
        .unwrap_err();
        assert!(error.contains("cancel"), "{stage:?}: {error}");
        assert_eq!(control.last_checkpoint().unwrap().stage, stage);
        assert!(matrix.iter().zip(&before).all(|(a, b)| same_wide(a, b)));
        assert!(Search::prepare(matrix.clone()).is_ok());
    }
    let fit = Search::prepare(matrix).unwrap();
    let center = centers(3);
    let before = center.clone();
    let target = [1.5_f64.next_up(); 3];
    let checked = |v: &[f64]| {
        let residual: Vec<_> = v
            .iter()
            .zip(target)
            .map(|(&v, target)| v - target)
            .collect();
        Ok((
            super::super::vector_norm(residual.iter().copied())? / (1.5_f64.next_up() - 1.5),
            residual,
        ))
    };
    for partitioned in [false, true] {
        for stage in [
            SolverStage::ModalRoundoffPrepare,
            SolverStage::ModalShapeScan,
            SolverStage::ModalVectorDot,
            SolverStage::ModalVectorScan,
            SolverStage::ModalVectorUpdate,
            SolverStage::ModalIteration,
            SolverStage::ModalRoundoffSearch,
            SolverStage::ModalRoundoffValidate,
        ] {
            let control = SolverControl::default();
            let cancel = control.clone();
            let error = with_solver_observer(
                &control,
                move |p| {
                    if p.stage == stage {
                        cancel.request_cancel();
                    }
                },
                || {
                    run(&fit, &center, partitioned, |_| {
                        Ok((
                            if stage == SolverStage::ModalRoundoffValidate {
                                0.0
                            } else {
                                1.0
                            },
                            vec![0.0; 3],
                        ))
                    })
                },
            )
            .unwrap_err();
            assert!(error.contains("cancel"), "{stage:?}: {error}");
            assert_eq!(control.last_checkpoint().unwrap().stage, stage);
            assert!(same_wide(&center, &before));
            assert_eq!(run(&fit, &center, partitioned, checked).unwrap(), target);
        }
    }
}
