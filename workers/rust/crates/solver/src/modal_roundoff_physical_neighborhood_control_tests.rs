use super::*;
use crate::solver_control::{SolverControl, with_solver_observer};
use std::{cell::RefCell, rc::Rc};

fn barrier() -> Vec<Vec<Wide>> {
    [
        [1.0, 0.0, 0.0, 0.0, 0.0],
        [0.0, 1.0, -1.0, 0.0, 0.0],
        [0.0, 0.0, 0.0, 1.0, -1.0],
        [0.0, 1.0, 1.0, -1.0, -1.0],
        [0.0, 0.125, 0.125, 0.125, 0.125],
    ]
    .iter()
    .map(|r| r.iter().map(|&v| Wide::from(v)).collect())
    .collect()
}

#[test]
fn physical_neighborhood_four_coordinates_cross_a_known_three_coordinate_barrier() {
    for permutation in [[0, 1, 2, 3, 4], [4, 2, 0, 3, 1], [1, 3, 4, 0, 2]] {
        for sign in [1.0, -1.0] {
            let seed = [1.5_f64 * sign; 5];
            let unit = seed[0].next_up() - seed[0];
            let matrix = barrier();
            let permuted: Vec<Vec<_>> = permutation
                .iter()
                .map(|&i| permutation.iter().map(|&j| matrix[i][j]).collect())
                .collect();
            let checked = |v: &[f64]| {
                let mut delta = [0.0; 5];
                for (i, &j) in permutation.iter().enumerate() {
                    delta[j] = v[i] - seed[i];
                }
                let closed = [
                    delta[0],
                    delta[1] - delta[2],
                    delta[3] - delta[4],
                    delta[1] + delta[2] - delta[3] - delta[4],
                    0.125 * (delta[1] + delta[2] + delta[3] + delta[4]) - 0.5 * unit,
                ];
                let residual = permutation.map(|i| closed[i]).to_vec();
                let relative = super::super::vector_norm(residual.iter().copied())? / (0.5 * unit);
                Ok((relative, residual))
            };
            for width in [1, 2, 3] {
                let fit = Neighborhood::prepare(permuted.clone(), width).unwrap();
                let error = fit.correct(&seed, 1e-8, checked).unwrap_err();
                assert!(error.contains("relative=1e0"), "{error}");
            }
            let fit = Neighborhood::prepare(permuted, 4).unwrap();
            let accepted = fit.correct(&seed, 1e-8, checked).unwrap();
            let expected = permutation.map(|i| if i == 0 { seed[0] } else { seed[0].next_up() });
            assert_eq!(accepted, expected);
            assert_eq!(checked(&accepted).unwrap().0, 0.0);
            assert_eq!(seed, [1.5 * sign; 5]);
        }
    }
}

#[test]
fn physical_neighborhood_graph_range_and_resource_preflights_are_bounded() {
    for size in [2, 5, 128, 256] {
        for width in 1..=4 {
            let plan = Plan::new(size, width).unwrap();
            assert!(plan.payload_bytes <= 8 * 1024 * 1024);
            assert!(plan.component_visits <= 250_000_000);
        }
    }
    let maximum = Plan::new(256, 4).unwrap();
    assert_eq!(maximum.payload_bytes, 5_242_880);
    assert_eq!(maximum.component_visits, 218_103_808);
    for size in [0, 1, 257, usize::MAX] {
        assert!(Plan::new(size, 4).is_err());
    }
    for width in [0, 5, usize::MAX] {
        assert!(Plan::new(5, width).is_err());
    }
    for matrix in [
        vec![],
        vec![vec![Wide::from(1.0)]],
        vec![vec![Wide::from(1.0); 2]; 3],
        vec![vec![Wide::from(1.0); 257]; 257],
    ] {
        assert!(Neighborhood::prepare(matrix, 4).is_err());
    }
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
        let mut matrix = barrier();
        matrix[0][0] = bad;
        assert!(Neighborhood::prepare(matrix, 4).is_err());
    }
    for width in 1..=4 {
        let a = Neighborhood::prepare(barrier(), width).unwrap();
        let b = Neighborhood::prepare(barrier(), width).unwrap();
        assert_eq!(a.families, b.families);
        assert!(a.families.iter().map(Vec::len).sum::<usize>() <= 35);
        for family in &a.families {
            assert!(family.windows(2).all(|w| w[0] < w[1]));
            assert!(family.iter().all(|g| !g.is_empty()
                && g.len() <= width
                && g.windows(2).all(|w| w[0] < w[1])
                && g.iter().all(|&i| i < 5)));
        }
    }
    let zeros = Neighborhood::prepare(vec![vec![Wide::default(); 3]; 3], 4).unwrap();
    assert!(zeros.families[1].is_empty());
    assert_eq!(
        zeros
            .correct(&[0.0; 3], 1e-8, |_| Ok((0.0, vec![0.0; 3])))
            .unwrap(),
        [0.0; 3]
    );
}

#[test]
fn physical_neighborhood_quadratic_proposals_match_independent_closed_products() {
    let fit = Neighborhood::prepare(barrier(), 4).unwrap();
    let mut seed = [1.5_f64; 5];
    let unit = seed[0].next_up() - seed[0];
    for trial in 0..81 {
        let mut encoded = trial;
        let d = std::array::from_fn::<_, 4, _>(|_| {
            let next = [0.0, unit, -unit][encoded % 3];
            encoded /= 3;
            next
        });
        if d.iter().filter(|&&v| v != 0.0).count() <= 3 {
            let closed = [
                d[0] - d[1],
                d[2] - d[3],
                d[0] + d[1] - d[2] - d[3],
                0.125 * d.iter().sum::<f64>() - 0.5 * unit,
            ];
            assert!(closed.iter().map(|v| v * v).sum::<f64>() >= 0.25 * unit * unit);
        }
    }
    let mut residual = vec![Wide::default(); 5];
    residual[4] = Wide::from(-0.5 * unit);
    fit.choose(&[1, 2, 3], &mut seed, &mut residual).unwrap();
    assert_eq!(seed, [1.5; 5]);
    fit.choose(&[1, 2, 3, 4], &mut seed, &mut residual).unwrap();
    assert_eq!(
        seed,
        [
            1.5,
            1.5_f64.next_up(),
            1.5_f64.next_up(),
            1.5_f64.next_up(),
            1.5_f64.next_up()
        ]
    );
    assert!(residual.iter().all(|v| v.rounded() == 0.0));
    // Independent exact binary entries certify the simultaneous move.
    let d = [seed[1] - 1.5, seed[2] - 1.5, seed[3] - 1.5, seed[4] - 1.5];
    assert_eq!(
        [
            d[0] - d[1],
            d[2] - d[3],
            d[0] + d[1] - d[2] - d[3],
            0.125 * d.iter().sum::<f64>() - 0.5 * unit
        ],
        [0.0; 4]
    );
}

#[test]
fn physical_neighborhood_one_counter_reanchors_and_rechecks_the_final_certificate() {
    let fit = Neighborhood::prepare(barrier(), 4).unwrap();
    let seed = [1.5; 5];
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
            fit.correct(&seed, 1e-8, |_| {
                calls += 1;
                Ok((1.0 / (calls + 1) as f64, vec![0.0; 5]))
            })
        },
    )
    .unwrap_err();
    assert!(error.contains("unchanged residual gate"));
    assert_eq!(calls, MAX_CERTIFICATES);
    assert_eq!(
        visits.borrow().as_slice(),
        (0..MAX_CERTIFICATES as u64).collect::<Vec<_>>()
    );
    for (relative, residual) in [
        (1.0, vec![0.0; 5]),
        (f64::NAN, vec![0.0; 5]),
        (-1.0, vec![0.0; 5]),
        (0.0, vec![0.0; 4]),
        (0.0, vec![f64::INFINITY; 5]),
        (0.0, vec![1e51; 5]),
    ] {
        let mut calls = 0;
        let error = fit
            .correct(&seed, 1e-8, |_| {
                calls += 1;
                if calls == 1 {
                    Ok((0.0, vec![0.0; 5]))
                } else {
                    Ok((relative, residual.clone()))
                }
            })
            .unwrap_err();
        assert!(
            error.contains("unchanged residual gate")
                || error.contains("finite matching certificate")
        );
        assert_eq!(calls, 2);
    }
    for tolerance in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        assert!(fit.correct(&seed, tolerance, |_| unreachable!()).is_err());
    }
    for invalid in [
        vec![1.5; 4],
        vec![f64::NAN; 5],
        vec![1e51; 5],
        vec![1e-51; 5],
    ] {
        assert!(fit.correct(&invalid, 1e-8, |_| unreachable!()).is_err());
    }
    assert_eq!(
        fit.correct(&seed, 1e-8, |_| Err("actual operator failure".into()))
            .unwrap_err(),
        "actual operator failure"
    );
    // A proposal may predict improvement, but a worse true certificate is rejected.
    let unit = seed[0].next_up() - seed[0];
    let mut calls = 0;
    let mut changed = false;
    let error = fit
        .correct(&seed, 1e-8, |v| {
            calls += 1;
            changed |= v != seed;
            let mut residual = vec![0.0; 5];
            residual[4] = -0.5 * unit;
            Ok((if v == seed { 1.0 } else { 2.0 }, residual))
        })
        .unwrap_err();
    assert!(changed && calls <= MAX_CERTIFICATES);
    assert!(error.contains("relative=1e0"), "{error}");
    let mut expected = seed;
    expected[0] = expected[0].next_up();
    let mut seen = Vec::new();
    let error = fit
        .correct(&seed, 1e-8, |v| {
            seen.push(v.to_vec());
            if seen.len() == 1 {
                let mut residual = vec![0.0; 5];
                residual[0] = -unit;
                Ok((1.0, residual))
            } else {
                Ok((0.5, vec![0.0; 5]))
            }
        })
        .unwrap_err();
    assert!(error.contains("relative=5e-1"), "{error}");
    assert_eq!(seen.len(), 14);
    assert_eq!(seen[0], seed);
    assert!(
        seen[1..].iter().all(|v| v == &expected),
        "accepted residual must replace the old prediction"
    );
    for (relative, residual) in [(f64::NAN, vec![0.0; 5]), (1.0, vec![0.0; 4])] {
        let mut calls = 0;
        assert!(
            fit.correct(&seed, 1e-8, |_| {
                calls += 1;
                Ok((relative, residual.clone()))
            })
            .unwrap_err()
            .contains("finite matching certificate")
        );
        assert_eq!(calls, 1);
    }
    let mut calls = 0;
    assert_eq!(
        fit.correct(&seed, 1e-8, |_| {
            calls += 1;
            if calls == 1 {
                Ok((1.0, vec![0.0; 5]))
            } else {
                Err("later actual operator failure".into())
            }
        })
        .unwrap_err(),
        "later actual operator failure"
    );
    assert_eq!(calls, 2);
    assert_eq!(seed, [1.5; 5]);
    assert_eq!(
        fit.correct(&seed, 1e-8, |_| Ok((0.0, vec![0.0; 5])))
            .unwrap(),
        seed
    );
}

#[test]
fn physical_neighborhood_preparation_and_search_cancel_without_partial_shapes_and_replay() {
    let matrix = barrier();
    let before = matrix.clone();
    for stage in [
        SolverStage::ModalRoundoffPrepare,
        SolverStage::ModalVectorDot,
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
            || Neighborhood::prepare(matrix.clone(), 4).map(|_| ()),
        )
        .unwrap_err();
        assert!(error.contains("cancel"), "{stage:?}: {error}");
        assert_eq!(control.last_checkpoint().unwrap().stage, stage);
        assert!(
            matrix
                .iter()
                .flatten()
                .zip(before.iter().flatten())
                .all(|(a, b)| a.high == b.high && a.low == b.low)
        );
        assert!(Neighborhood::prepare(matrix.clone(), 4).is_ok());
    }
    let fit = Neighborhood::prepare(matrix, 4).unwrap();
    let seed = [1.5; 5];
    for stage in [
        SolverStage::ModalIteration,
        SolverStage::ModalVectorDot,
        SolverStage::ModalVectorScan,
        SolverStage::ModalVectorUpdate,
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
                fit.correct(&seed, 1e-8, |_| {
                    Ok((
                        if stage == SolverStage::ModalRoundoffValidate {
                            0.0
                        } else {
                            1.0
                        },
                        vec![0.0; 5],
                    ))
                })
            },
        )
        .unwrap_err();
        assert!(error.contains("cancel"), "{stage:?}: {error}");
        assert_eq!(control.last_checkpoint().unwrap().stage, stage);
        assert_eq!(seed, [1.5; 5]);
        assert_eq!(
            fit.correct(&seed, 1e-8, |_| Ok((0.0, vec![0.0; 5])))
                .unwrap(),
            seed
        );
    }
}
