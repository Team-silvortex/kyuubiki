use super::{
    hybrid_tests::{check_published, scaled_bending_parts},
    *,
};
use crate::solver_control::{SolverControl, with_solver_observer};
use std::{cell::RefCell, rc::Rc};

#[test]
fn joint_rounding_physical_columns_and_triples_compare_extreme_coordinate_modes() {
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
        let mut legacy = system.operator.dense_fallback_matrix().unwrap();
        for (i, row) in legacy.iter_mut().enumerate() {
            for (a, &m) in row.iter_mut().zip(&system.mass) {
                *a /= m.sqrt().recip();
            }
            row[i] -= value * system.mass[i] * system.mass[i].sqrt().recip();
        }
        let direct = joint::physical_columns(&physical, &system.mass, *value).unwrap();
        for (name, matrix) in [("legacy", legacy), ("physical", direct)] {
            let fit = BlockFit::prepare_automatic(matrix, &shape).unwrap();
            for use_joint in [false, true] {
                let checked = |v: &[f64]| {
                    let applied = system.operator.apply_physical_compensated(v)?;
                    system
                        .operator
                        .physical_residual(*value, v, &system.mass, &applied)
                };
                let outcome = if use_joint {
                    joint::correct(&fit, &shape, 1e-8, checked)
                } else {
                    fit.correct(&shape, 1e-8, checked)
                };
                if step == 1e-10 {
                    let error = outcome.unwrap_err();
                    assert!(error.contains("unchanged residual gate"), "{error}");
                    let relative = error
                        .split("relative=")
                        .nth(1)
                        .unwrap()
                        .trim_end_matches(')')
                        .parse::<f64>()
                        .unwrap();
                    assert!((1.7e-8..1.8e-8).contains(&relative), "{error}");
                    println!("joint step={step:e} {name} triples={use_joint}: reject {relative:e}");
                } else {
                    let candidate = outcome.unwrap();
                    let applied = system
                        .operator
                        .apply_physical_compensated(&candidate)
                        .unwrap();
                    let relative = system
                        .operator
                        .physical_residual(*value, &candidate, &system.mass, &applied)
                        .unwrap()
                        .0;
                    let band = match (step, use_joint) {
                        (1.0, false) => 9.1e-9..9.3e-9,
                        (1.0, true) => 8.3e-9..8.5e-9,
                        (_, false) => 7.35e-9..7.4e-9,
                        (_, true) => 7.3e-9..7.35e-9,
                    };
                    assert!(band.contains(&relative), "{relative:e}");
                    println!("joint step={step:e} {name} triples={use_joint}: pass {relative:e}");
                    check_published(128, step, *value, &candidate);
                }
            }
        }
    }
}

fn analytic_matrix() -> Vec<Vec<f64>> {
    vec![
        vec![1.0, 0.0, 0.0, 0.0],
        vec![0.0, 1.0, -0.5, -0.5],
        vec![0.0, 0.0, 0.75_f64.sqrt(), -0.75_f64.sqrt()],
        vec![0.0, 0.125, 0.125, 0.125],
    ]
}

#[test]
fn joint_rounding_three_coordinate_action_crosses_an_analytic_pair_barrier() {
    let seed = [1.5_f64; 4];
    let unit = seed[0].next_up() - seed[0];
    let fit = BlockFit::prepare_coupled(analytic_matrix(), vec![0]).unwrap();
    let groups = joint::triples(&fit.projected).unwrap();
    assert_eq!(groups, [[0, 1, 2]]);
    let mut candidate = seed;
    let mut residual = [0.0, 0.0, 0.0, -0.75 * unit];
    fit.single_sweep(&mut candidate, &mut residual).unwrap();
    fit.pair_sweep(&mut candidate, &mut residual).unwrap();
    assert_eq!(candidate, seed);
    joint::sweep(&fit, &groups, &mut candidate, &mut residual).unwrap();
    assert_eq!(
        candidate,
        [
            seed[0],
            seed[1].next_up().next_up(),
            seed[2].next_up().next_up(),
            seed[3].next_up().next_up()
        ]
    );
    // Independent closed-entry products, not the projected candidate score.
    let checked = |v: &[f64]| {
        let dx = v[1] - seed[1];
        let dy = v[2] - seed[2];
        let dz = v[3] - seed[3];
        let residual = vec![
            v[0] - seed[0],
            dx - 0.5 * (dy + dz),
            0.75_f64.sqrt() * (dy - dz),
            0.125 * (dx + dy + dz) - 0.75 * unit,
        ];
        Ok((
            vector_norm(residual.iter().copied())? / (0.75 * unit),
            residual,
        ))
    };
    let accepted = joint::correct(&fit, &seed, 1e-8, checked).unwrap();
    assert_eq!(accepted, candidate);
    assert_eq!(checked(&accepted).unwrap().0, 0.0);
    assert_eq!(seed, [1.5; 4]);
}

#[test]
fn joint_rounding_graph_and_added_proposal_work_have_separate_bounded_plans() {
    for size in [2, 4, 128, 256] {
        let plan = joint::Plan::new(size).unwrap();
        let base = budget::Plan::new(size).unwrap();
        assert_eq!(plan.groups, 4 * size);
        assert!(plan.payload_bytes > base.payload_bytes && plan.payload_bytes <= 8 * 1024 * 1024);
        assert!(
            plan.component_visits > base.component_visits && plan.component_visits <= 700_000_000
        );
    }
    for size in [0, 1, 257, usize::MAX] {
        assert!(joint::Plan::new(size).is_err());
    }
    assert!(joint::triples(&[]).unwrap().is_empty());
    assert!(joint::triples(&vec![vec![0.0; 3]; 3]).unwrap().is_empty());
    let columns = vec![vec![1.0; 32]; 32];
    let groups = joint::triples(&columns).unwrap();
    assert!(!groups.is_empty() && groups.len() <= 4 * columns.len());
    assert!(groups.windows(2).all(|w| w[0] < w[1]));
    assert!(
        groups
            .iter()
            .all(|g| g[0] < g[1] && g[1] < g[2] && g[2] < columns.len())
    );
    assert_eq!(groups, joint::triples(&columns).unwrap());
    assert!(joint::triples(&vec![vec![1.0; 2]; 257]).is_err());
    assert!(joint::triples(&[vec![1.0], vec![0.0; 2]]).is_err());
    assert!(joint::triples(&[vec![f64::NAN]]).is_err());
}

#[test]
fn joint_rounding_physical_direction_guards_and_known_entries_are_checked() {
    let matrix = vec![vec![8.0, 2.0], vec![2.0, 5.0]];
    assert_eq!(
        joint::physical_columns(&matrix, &[4.0, 1.0], 0.5).unwrap(),
        [vec![3.0, 1.0], vec![2.0, 4.5]]
    );
    for value in [0.0, -1.0, f64::NAN, f64::INFINITY, 1e101, 1e-101] {
        assert!(joint::physical_columns(&matrix, &[4.0, 1.0], value).is_err());
    }
    for mass in [
        vec![],
        vec![0.0; 2],
        vec![f64::NAN; 2],
        vec![1e101; 2],
        vec![1e-101; 2],
    ] {
        assert!(joint::physical_columns(&matrix, &mass, 0.5).is_err());
    }
    for bad in [
        vec![vec![0.0]],
        vec![vec![0.0; 257]; 257],
        vec![vec![1.0], vec![0.0; 2]],
        vec![vec![f64::INFINITY; 2]; 2],
        vec![vec![1e101; 2]; 2],
        vec![vec![1.0, 2.0], vec![3.0, 1.0]],
    ] {
        assert!(joint::physical_columns(&bad, &vec![1.0; bad.len()], 0.5).is_err());
    }
    let tiny = f64::from_bits(1);
    assert!(
        joint::physical_columns(&[vec![1.0, tiny], vec![tiny, 1.0]], &[1e100; 2], 0.5).is_err()
    );
}

#[test]
fn joint_rounding_checks_share_one_counter_and_revalidate_final_candidates() {
    let seed = [1.5; 4];
    let fit = BlockFit::prepare_coupled(analytic_matrix(), vec![0]).unwrap();
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
            joint::correct(&fit, &seed, 1e-8, |_| {
                calls += 1;
                Ok((1.0 / (calls + 1) as f64, vec![0.0; 4]))
            })
        },
    )
    .unwrap_err();
    assert!(error.contains("unchanged residual gate"));
    assert_eq!(calls, 74);
    assert_eq!(visits.borrow().as_slice(), (0..74).collect::<Vec<_>>());
    assert!(calls <= budget::MAX_CHECKS);
    assert_eq!(seed, [1.5; 4]);
    let mut calls = 0;
    let error = joint::correct(&fit, &seed, 1e-8, |_| {
        calls += 1;
        Ok((if calls == 1 { 0.0 } else { 1.0 }, vec![0.0; 4]))
    })
    .unwrap_err();
    assert!(error.contains("unchanged residual gate"));
    assert_eq!(calls, 2);
    for (relative, residual) in [
        (f64::NAN, vec![0.0; 4]),
        (-1.0, vec![0.0; 4]),
        (0.0, vec![0.0; 3]),
        (0.0, vec![f64::INFINITY; 4]),
    ] {
        assert!(joint::correct(&fit, &seed, 1e-8, |_| Ok((relative, residual.clone()))).is_err());
        let mut calls = 0;
        let error = joint::correct(&fit, &seed, 1e-8, |_| {
            calls += 1;
            if calls == 1 {
                Ok((0.0, vec![0.0; 4]))
            } else {
                Ok((relative, residual.clone()))
            }
        })
        .unwrap_err();
        assert!(error.contains("finite matching residual certificate"));
        assert_eq!(calls, 2);
    }
    assert_eq!(
        joint::correct(&fit, &seed, 1e-8, |_| Err(
            "injected actual-operator failure".into()
        ))
        .unwrap_err(),
        "injected actual-operator failure"
    );
    for tolerance in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        assert!(joint::correct(&fit, &seed, tolerance, |_| unreachable!()).is_err());
    }
    assert!(joint::correct(&fit, &[1.5; 3], 1e-8, |_| unreachable!()).is_err());
    assert!(joint::correct(&fit, &[f64::NAN; 4], 1e-8, |_| unreachable!()).is_err());
}

#[test]
fn joint_rounding_each_phase_cancels_without_partial_shapes_and_replays() {
    let seed = [1.5; 4];
    let fit = BlockFit::prepare_coupled(analytic_matrix(), vec![0]).unwrap();
    for stage in [
        SolverStage::ModalRoundoffPrepare,
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
                joint::correct(&fit, &seed, 1e-8, |_| {
                    Ok((
                        if stage == SolverStage::ModalRoundoffValidate {
                            0.0
                        } else {
                            1.0
                        },
                        vec![0.0; 4],
                    ))
                })
            },
        )
        .unwrap_err();
        assert!(error.contains("cancel"), "{stage:?}: {error}");
        assert_eq!(control.last_checkpoint().unwrap().stage, stage);
        assert_eq!(seed, [1.5; 4]);
        assert_eq!(
            joint::correct(&fit, &seed, 1e-8, |_| Ok((0.0, vec![0.0; 4]))).unwrap(),
            seed
        );
    }
    let matrix = vec![vec![8.0, 2.0], vec![2.0, 5.0]];
    let before = matrix.clone();
    let control = SolverControl::default();
    let cancel = control.clone();
    assert!(
        with_solver_observer(
            &control,
            move |p| {
                if p.stage == SolverStage::ModalVectorUpdate {
                    cancel.request_cancel();
                }
            },
            || joint::physical_columns(&matrix, &[4.0, 1.0], 0.5)
        )
        .unwrap_err()
        .contains("cancel")
    );
    assert_eq!(matrix, before);
    assert!(joint::physical_columns(&matrix, &[4.0, 1.0], 0.5).is_ok());
}
