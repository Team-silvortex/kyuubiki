use super::{
    hybrid_tests::{check_published, scaled_bending_parts},
    *,
};
use crate::modal_test_wide::Wide;
use crate::solver_control::{SolverControl, with_solver_observer};
use std::{cell::RefCell, rc::Rc};

#[test]
fn wide_projection_physical_candidates_compare_extreme_coordinate_modes() {
    for step in [1.0, 1e14, 1e-10] {
        let (system, physical) = scaled_bending_parts(128, step);
        let spectrum = crate::modal_frame_spectrum::frame_eigenpairs(&system, Some(1)).unwrap();
        let (value, vector) = &spectrum.pairs[0];
        let (shape, _) = crate::modal_frame_spectrum::checked_mode_shape(
            vector,
            &system.mass,
            &(0..256).collect::<Vec<_>>(),
            256,
        )
        .unwrap();
        let shape = system
            .operator
            .roundoff_comparison_seed(*value, &shape, &system.mass)
            .unwrap();
        let fit =
            wide_projection::Projection::physical(&physical, &system.mass, *value, &shape).unwrap();
        let anchor = shape
            .iter()
            .zip(&system.mass)
            .enumerate()
            .max_by(|(_, (a, ma)), (_, (b, mb))| {
                (a.abs() * ma.sqrt()).total_cmp(&(b.abs() * mb.sqrt()))
            })
            .unwrap()
            .0;
        let full = wide_projection::Projection::physical_partition(
            &physical,
            &system.mass,
            *value,
            (0..shape.len()).filter(|&i| i != anchor).collect(),
        )
        .unwrap();
        for (name, fit) in [("local", &fit), ("anchored", &full)] {
            for rounding in [false, true] {
                let checked = |v: &[f64]| {
                    let applied = system.operator.apply_physical_compensated(v)?;
                    system
                        .operator
                        .physical_residual(*value, v, &system.mass, &applied)
                };
                let result = if rounding {
                    fit.correct_rounding(&shape, 1e-8, checked)
                } else {
                    fit.correct(&shape, 1e-8, checked)
                };
                if name == "local" && step != 1e-10 {
                    let candidate = result.unwrap();
                    let applied = system
                        .operator
                        .apply_physical_compensated(&candidate)
                        .unwrap();
                    let residual = system
                        .operator
                        .physical_residual(*value, &candidate, &system.mass, &applied)
                        .unwrap()
                        .0;
                    let band = if step == 1.0 {
                        9.19e-9..9.22e-9
                    } else {
                        7.35e-9..7.38e-9
                    };
                    assert!(band.contains(&residual), "{residual:e}");
                    println!(
                        "wide projection {name} rounding={rounding} step={step:e}: pass {residual:e}"
                    );
                    check_published(128, step, *value, &candidate);
                } else {
                    let error = result.unwrap_err();
                    assert!(error.contains("unchanged residual gate"), "{error}");
                    let relative = error
                        .split("relative=")
                        .nth(1)
                        .unwrap()
                        .trim_end_matches(')')
                        .parse::<f64>()
                        .unwrap();
                    let band = if step == 1e-10 {
                        1.7e-8..1.8e-8
                    } else if step == 1.0 {
                        2.17e-8..2.18e-8
                    } else {
                        2.01e-8..2.02e-8
                    };
                    assert!(band.contains(&relative), "{error}");
                    println!(
                        "wide projection {name} rounding={rounding} step={step:e}: reject {relative:e}"
                    );
                }
            }
        }
    }
}

fn wide(values: &[f64]) -> Vec<Wide> {
    values.iter().map(|&v| Wide::from(v)).collect()
}

#[test]
fn wide_projection_factor_recovers_independent_nearly_parallel_scaled_columns() {
    let delta = 2.0_f64.powi(-30);
    let left = [1.0; 3];
    let right = [1.0, 1.0 + delta, 1.0 - delta];
    assert!(FineFactor::gram(&[left.to_vec(), right.to_vec()]).is_err());
    let powers = [1.0, 2.0_f64.powi(-200), 2.0_f64.powi(200)];
    for order in [[0, 1, 2], [2, 1, 0], [1, 2, 0]] {
        for scales in [
            (powers[0], powers[0]),
            (powers[1], powers[2]),
            (powers[2], powers[1]),
        ] {
            for signs in [(1.0, 1.0), (-1.0, -1.0), (1.0, -1.0)] {
                let (a, b) = (scales.0 * signs.0, scales.1 * signs.1);
                let columns = [
                    wide(&order.map(|i| left[i] * a)),
                    wide(&order.map(|i| right[i] * b)),
                ];
                let factor = wide_factor::Factor::prepare(&columns).unwrap();
                let rhs = order.map(|i| left[i] + 2.0 * right[i]);
                let coefficients = factor.coefficients(&wide(&rhs)).unwrap();
                assert!((coefficients[0].rounded() * a - 1.0).abs() < 1e-10);
                assert!((coefficients[1].rounded() * b - 2.0).abs() < 1e-10);
                for (i, &target) in rhs.iter().enumerate() {
                    let rebuilt = columns[0][i].rounded() * coefficients[0].rounded()
                        + columns[1][i].rounded() * coefficients[1].rounded();
                    assert!(
                        (rebuilt - target).abs() < 1e-13,
                        "{rebuilt:e} != {target:e}"
                    );
                }
                assert!(
                    factor
                        .project(&wide(&rhs))
                        .unwrap()
                        .iter()
                        .all(|v| v.rounded().abs() < 1e-13)
                );
            }
        }
    }
}

#[test]
fn wide_projection_low_correction_tail_changes_known_final_rounding() {
    let factor = wide_factor::Factor::prepare(&[wide(&[3.0, 0.0])]).unwrap();
    let correction = factor.coefficients(&wide(&[1.0, 0.0])).unwrap()[0];
    assert_ne!(correction.low, 0.0);
    let rounded_first = 0.5 - correction.rounded();
    let rounded_last = Wide::from(0.5).sub(correction).rounded();
    assert_eq!(rounded_last, 1.0 / 6.0);
    assert_ne!(rounded_last, rounded_first);
    let factor =
        wide_factor::Factor::prepare(&[wide(&[1.0, 1.0, 0.0]), wide(&[0.0, 0.0, 1.0])]).unwrap();
    let projected = factor.project(&wide(&[2.0, -2.0, 1.0])).unwrap();
    assert_eq!(
        projected.iter().map(|v| v.rounded()).collect::<Vec<_>>(),
        [2.0, -2.0, 0.0]
    );
}

#[test]
fn wide_projection_factor_rejects_unresolved_rank_range_and_malformed_inputs() {
    let factor = wide_factor::Factor::prepare(&[wide(&[3.0, 0.0])]).unwrap();
    for columns in [
        vec![],
        vec![wide(&[1.0])],
        vec![wide(&[1.0; 257])],
        vec![wide(&[0.0; 2])],
        vec![wide(&[1.0; 3]), wide(&[1.0; 3])],
        vec![wide(&[1.0; 3]), wide(&[1.0; 2])],
        vec![
            wide(&[1.0; 3]),
            wide(&[1.0, 1.0 + 2.0_f64.powi(-50), 1.0 - 2.0_f64.powi(-50)]),
        ],
    ] {
        assert!(wide_factor::Factor::prepare(&columns).is_err());
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
        Wide::from(1e101),
        Wide::from(1e-101),
    ] {
        assert!(wide_factor::Factor::prepare(&[vec![bad; 2]]).is_err());
        assert!(factor.coefficients(&[bad; 2]).is_err());
        assert!(factor.project(&[bad; 2]).is_err());
    }
    assert!(factor.coefficients(&wide(&[1.0])).is_err());
    let seed = [0.5, 0.0];
    for coordinates in [vec![], vec![2], vec![0, 1]] {
        let mut candidate = seed;
        assert!(
            factor
                .choose_rounding(&coordinates, &seed, &mut candidate, &wide(&[1.0, 0.0]))
                .is_err()
        );
        assert_eq!(candidate, seed);
    }
    let mut candidate = seed;
    assert!(
        factor
            .choose_rounding(&[0], &seed, &mut candidate, &wide(&[1.0]))
            .is_err()
    );
    assert_eq!(candidate, seed);
    assert!(
        factor
            .choose_rounding(&[0], &seed[..1], &mut candidate, &wide(&[1.0, 0.0]))
            .is_err()
    );
    assert_eq!(candidate, seed);
    let rhs = wide(&[1.0, 0.0]);
    assert_eq!(factor.coefficients(&rhs).unwrap()[0].rounded(), 1.0 / 3.0);
    assert_eq!(
        rhs.iter().map(|v| v.rounded()).collect::<Vec<_>>(),
        [1.0, 0.0]
    );
}

fn diagonal() -> Vec<Vec<Wide>> {
    (0..4)
        .map(|i| {
            (0..4)
                .map(|j| Wide::from(if i == j { (i + 1) as f64 } else { 0.0 }))
                .collect()
        })
        .collect()
}

#[test]
fn wide_projection_separate_resource_plan_and_physical_input_guards_are_bounded() {
    for size in [2, 4, 128, 256] {
        let plan = wide_projection::Plan::new(size).unwrap();
        assert!(plan.payload_bytes <= 8 * 1024 * 1024);
        assert!(plan.component_visits <= 1_500_000_000);
    }
    let plan = wide_projection::Plan::new(256).unwrap();
    assert_eq!(plan.payload_bytes, 6_553_600);
    assert_eq!(plan.component_visits, 1_476_395_008);
    for size in [0, 1, 257, usize::MAX] {
        assert!(wide_projection::Plan::new(size).is_err());
    }
    for fine in [vec![], vec![0, 1, 2, 3], vec![4], vec![1, 1], vec![2, 0]] {
        assert!(wide_projection::Projection::prepare(diagonal(), fine).is_err());
    }
    assert!(wide_projection::Projection::prepare(vec![wide(&[0.0; 4]); 4], vec![0]).is_err());
    assert!(wide_projection::Projection::prepare(vec![wide(&[1.0; 4]); 3], vec![0]).is_err());
    let matrix = vec![vec![8.0, 2.0], vec![2.0, 5.0]];
    assert!(wide_projection::Projection::physical(&matrix, &[4.0, 1.0], 0.5, &[1.0; 2]).is_ok());
    for mass in [vec![0.0; 2], vec![1.0], vec![1e101; 2]] {
        assert!(wide_projection::Projection::physical(&matrix, &mass, 0.5, &[1.0; 2]).is_err());
    }
    assert!(wide_projection::Projection::physical(&matrix, &[4.0, 1.0], -1.0, &[1.0; 2]).is_err());
    assert!(wide_projection::Projection::physical(&matrix, &[4.0, 1.0], 0.5, &[0.0; 2]).is_err());
    assert!(
        wide_projection::Projection::physical(
            &[vec![1e100, 0.0], vec![0.0, 1e100]],
            &[1e-100; 2],
            0.5,
            &[1.0; 2]
        )
        .is_err()
    );
}

#[test]
fn wide_projection_one_counter_cached_factor_and_fresh_final_certificate_are_enforced() {
    let seed = [1.0; 4];
    let fit = wide_projection::Projection::prepare(diagonal(), vec![0, 1]).unwrap();
    for rounding in [false, true] {
        let visits = Rc::new(RefCell::new(Vec::new()));
        let observed = visits.clone();
        let factors = Rc::new(RefCell::new(0));
        let counted = factors.clone();
        let mut calls = 0;
        let error = with_solver_observer(
            &SolverControl::default(),
            move |p| {
                if p.stage == SolverStage::ModalRoundoffSearch {
                    observed.borrow_mut().push(p.completed_steps);
                }
                if p.stage == SolverStage::DenseFactor {
                    *counted.borrow_mut() += 1;
                }
            },
            || {
                let checked = |_: &[f64]| {
                    calls += 1;
                    Ok((1.0 / (calls + 1) as f64, vec![0.0; 4]))
                };
                if rounding {
                    fit.correct_rounding(&seed, 1e-8, checked)
                } else {
                    fit.correct(&seed, 1e-8, checked)
                }
            },
        )
        .unwrap_err();
        assert!(error.contains("unchanged residual gate"));
        assert_eq!(calls, 74);
        assert_eq!(visits.borrow().as_slice(), (0..74).collect::<Vec<_>>());
        assert_eq!(*factors.borrow(), 0);
        for (relative, residual) in [
            (1.0, vec![0.0; 4]),
            (f64::NAN, vec![0.0; 4]),
            (-1.0, vec![0.0; 4]),
            (0.0, vec![0.0; 3]),
            (0.0, vec![f64::INFINITY; 4]),
        ] {
            let mut calls = 0;
            let checked = |_: &[f64]| {
                calls += 1;
                if calls == 1 {
                    Ok((0.0, vec![0.0; 4]))
                } else {
                    Ok((relative, residual.clone()))
                }
            };
            let error = if rounding {
                fit.correct_rounding(&seed, 1e-8, checked)
            } else {
                fit.correct(&seed, 1e-8, checked)
            }
            .unwrap_err();
            assert!(
                error.contains("unchanged residual gate")
                    || error.contains("finite matching residual certificate")
            );
            assert_eq!(calls, 2);
        }
    }
    assert_eq!(seed, [1.0; 4]);
    for tolerance in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        assert!(fit.correct(&seed, tolerance, |_| unreachable!()).is_err());
    }
    assert!(
        fit.correct(&[f64::NAN; 4], 1e-8, |_| unreachable!())
            .is_err()
    );
    assert!(fit.correct(&[1e101; 4], 1e-8, |_| unreachable!()).is_err());
    assert!(fit.correct(&[1.0; 3], 1e-8, |_| unreachable!()).is_err());
    assert_eq!(
        fit.correct(
            &seed,
            1e-8,
            |_| Err("injected actual-operator error".into())
        )
        .unwrap_err(),
        "injected actual-operator error"
    );
    assert_eq!(
        fit.correct(&seed, 1e-8, |_| Ok((0.0, vec![0.0; 4])))
            .unwrap(),
        seed
    );
}

#[test]
fn wide_projection_preparation_and_cached_search_cancel_without_partial_results_and_replay() {
    let matrix = vec![
        vec![8.0, 2.0, 0.0],
        vec![2.0, 5.0, 1.0],
        vec![0.0, 1.0, 3.0],
    ];
    let before = matrix.clone();
    let mass = [1.0; 3];
    let shape = [1.0; 3];
    for stage in [
        SolverStage::ModalRoundoffPrepare,
        SolverStage::ModalValidation,
        SolverStage::DenseFactor,
        SolverStage::DenseSubstitution,
        SolverStage::ModalVectorScan,
        SolverStage::ModalVectorDot,
        SolverStage::ModalVectorUpdate,
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
            || wide_projection::Projection::physical(&matrix, &mass, 0.5, &shape).map(|_| ()),
        )
        .unwrap_err();
        assert!(error.contains("cancel"), "{stage:?}: {error}");
        assert_eq!(control.last_checkpoint().unwrap().stage, stage);
        assert_eq!(matrix, before);
        assert!(wide_projection::Projection::physical(&matrix, &mass, 0.5, &shape).is_ok());
    }
    let fit = wide_projection::Projection::prepare(diagonal(), vec![0, 1]).unwrap();
    let seed = [1.0; 4];
    for rounding in [false, true] {
        for stage in [
            SolverStage::ModalIteration,
            SolverStage::DenseSubstitution,
            SolverStage::ModalVectorDot,
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
                    let checked = |_: &[f64]| {
                        Ok((
                            if stage == SolverStage::ModalRoundoffValidate {
                                0.0
                            } else {
                                1.0
                            },
                            vec![0.0; 4],
                        ))
                    };
                    if rounding {
                        fit.correct_rounding(&seed, 1e-8, checked)
                    } else {
                        fit.correct(&seed, 1e-8, checked)
                    }
                },
            )
            .unwrap_err();
            assert!(error.contains("cancel"), "{stage:?}: {error}");
            assert_eq!(control.last_checkpoint().unwrap().stage, stage);
            assert_eq!(seed, [1.0; 4]);
            assert_eq!(
                fit.correct_rounding(&seed, 1e-8, |_| Ok((0.0, vec![0.0; 4])))
                    .unwrap(),
                seed
            );
        }
    }
}
