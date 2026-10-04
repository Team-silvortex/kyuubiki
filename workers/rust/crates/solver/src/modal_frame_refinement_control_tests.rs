use super::tests::diagonal;
use super::*;
use crate::solver_control::{SolverControl, with_solver_observer};

fn interrupted(stage: SolverStage, work: impl FnOnce() -> Result<(), String>) {
    let control = SolverControl::default();
    let cancel = control.clone();
    let error = with_solver_observer(
        &control,
        move |point| {
            if point.stage == stage && point.completed_steps == 64 {
                cancel.request_cancel();
            }
        },
        || {
            let result = work();
            assert!(
                result.is_err(),
                "cancellation must stop the numerical operation itself"
            );
            result
        },
    )
    .unwrap_err();
    assert!(
        error.contains(&format!("{} after 64 steps", stage.as_str())),
        "{error}"
    );
    assert!(control.was_interrupted());
    assert_eq!(control.last_checkpoint().unwrap().stage, stage);
    assert_eq!(control.last_checkpoint().unwrap().completed_steps, 64);
}

#[test]
fn modal_refinement_residual_scan_cancels_within_a_vector_chunk_and_replays() {
    let vector = vec![1.0; 129];
    let applied = vec![2.0; 129];
    interrupted(SolverStage::ModalVectorScan, || {
        relative_residual(2.0, &vector, &applied).map(|_| ())
    });
    assert_eq!(relative_residual(2.0, &vector, &applied).unwrap(), 0.0);
}

#[test]
fn modal_refinement_basis_dot_cancels_within_a_vector_chunk_and_replays() {
    let mut vector = vec![0.0; 129];
    vector[128] = 1.0;
    let mut basis = vec![0.0; 129];
    basis[0] = 1.0;
    let previous = [(1.0, basis)];
    let mut candidate = vector.clone();
    interrupted(SolverStage::ModalVectorDot, || {
        orthogonalize(&mut candidate, &previous)
    });
    orthogonalize(&mut vector, &previous).unwrap();
    assert_eq!(vector[128], 1.0);
    assert!(vector[..128].iter().all(|value| *value == 0.0));
}

#[test]
fn modal_refinement_binary_scaling_cancels_within_a_vector_chunk_and_replays() {
    let seed = vec![1.0; 129];
    let norm = stable_l2_norm(seed.iter().copied());
    let mut candidate = seed.clone();
    interrupted(SolverStage::ModalVectorUpdate, || {
        scale_binary(&mut candidate, norm).map(|_| ())
    });
    assert_eq!(
        &candidate[64..],
        &seed[64..],
        "do not finish an interrupted update"
    );
    let mut replay = seed.clone();
    let bounded = scale_binary(&mut replay, norm).unwrap();
    assert!((0.5..=1.0).contains(&bounded));
    assert!(replay.iter().all(|value| *value == 0.0625));
}

#[test]
fn modal_smoothing_vector_stage_cancellation_retains_the_seed_and_replays() {
    let system = diagonal(&(0..129).map(|i| 1.0 + i as f64).collect::<Vec<_>>());
    let mut seed = vec![1e-3; 129];
    seed[0] = 1.0;
    let applied = system.operator.apply_compensated(&seed).unwrap();
    let original = rayleigh(&seed, &applied).unwrap();
    for stage in [
        SolverStage::ModalVectorScan,
        SolverStage::ModalVectorDot,
        SolverStage::ModalVectorUpdate,
    ] {
        let mut vector = seed.clone();
        let mut value = original;
        interrupted(stage, || {
            smooth_residual(&system, &mut value, &mut vector, applied.clone(), &[], 1e-8)
                .map(|_| ())
        });
        assert_eq!(vector, seed);
        assert_eq!(value, original);
        smooth_residual(&system, &mut value, &mut vector, applied.clone(), &[], 1e-8).unwrap();
        assert!(
            relative_residual(
                value,
                &vector,
                &system.operator.apply_compensated(&vector).unwrap()
            )
            .unwrap()
                < relative_residual(original, &seed, &applied).unwrap()
        );
    }
}

#[test]
fn modal_refinement_controlled_norm_preserves_the_original_bits_and_chunk_boundaries() {
    for size in [0, 1, 63, 64, 65, 127, 128, 129, 4097] {
        for scale in [1.0, 1e300, 1e-300, f64::from_bits(1)] {
            let values: Vec<_> = (0..size)
                .map(|index| (index % 7) as f64 * scale * if index % 2 == 0 { 1.0 } else { -1.0 })
                .collect();
            assert_eq!(
                vector_norm(values.iter().copied()).unwrap().to_bits(),
                stable_l2_norm(values.iter().copied()).to_bits(),
                "size={size}, scale={scale:e}"
            );
        }
    }
    let points = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
    let observed = points.clone();
    with_solver_observer(
        &SolverControl::default(),
        move |point| {
            assert_eq!(point.stage, SolverStage::ModalVectorScan);
            observed.borrow_mut().push(point.completed_steps);
        },
        || vector_norm(vec![1.0; 129].into_iter()),
    )
    .unwrap();
    assert_eq!(*points.borrow(), [0, 64, 128, 129]);
}

#[test]
fn modal_refinement_cancelled_norm_stops_consuming_terms_in_the_first_chunk() {
    let consumed = std::cell::Cell::new(0);
    interrupted(SolverStage::ModalVectorScan, || {
        vector_norm((0..4097).map(|_| {
            consumed.set(consumed.get() + 1);
            1.0
        }))
        .map(|_| ())
    });
    assert_eq!(consumed.get(), 64);
}

#[test]
fn modal_refinement_controlled_dot_preserves_order_and_rejects_truncated_pairs() {
    let left: Vec<_> = (0..129)
        .map(|index| [1e16, 1.0, -1e16][index % 3])
        .collect();
    let right = vec![1.0; 129];
    assert_eq!(dot(&left, &right).unwrap().to_bits(), 43.0_f64.to_bits());
    assert!(
        dot(&left, &right[..128])
            .unwrap_err()
            .contains("matching dimensions")
    );
    for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert!(dot(&[value], &[1.0]).is_err());
    }
    for (vector, applied) in [
        (vec![], vec![]),
        (vec![0.0], vec![0.0]),
        (vec![1.0], vec![]),
        (vec![1.0], vec![-1.0]),
    ] {
        assert!(rayleigh(&vector, &applied).is_err());
    }
}

fn interrupt_update_after(
    stage: SolverStage,
    completed: u64,
    work: impl FnOnce() -> Result<(), String>,
) {
    let control = SolverControl::default();
    let cancel = control.clone();
    let armed = std::rc::Rc::new(std::cell::Cell::new(false));
    let observed = armed.clone();
    let error = with_solver_observer(
        &control,
        move |point| {
            if point.stage == stage && point.completed_steps == completed {
                observed.set(true);
            }
            if observed.get()
                && point.stage == SolverStage::ModalVectorUpdate
                && point.completed_steps == 64
            {
                cancel.request_cancel();
            }
        },
        || {
            let result = work();
            assert!(
                result.is_err(),
                "the armed numerical update must itself return cancellation"
            );
            result
        },
    )
    .unwrap_err();
    assert!(
        armed.get(),
        "reach the preceding numerical phase before cancellation"
    );
    assert!(
        error.contains("modal_vector_update after 64 steps"),
        "{error}"
    );
    assert_eq!(
        control.last_checkpoint().unwrap().stage,
        SolverStage::ModalVectorUpdate
    );
    assert_eq!(control.last_checkpoint().unwrap().completed_steps, 64);
}

#[test]
fn modal_refinement_projection_update_cancels_after_the_basis_dot_and_replays() {
    let seed = vec![1.0; 129];
    let mut basis = vec![0.0; 129];
    basis[0] = 1.0;
    let previous = [(1.0, basis)];
    let mut candidate = seed.clone();
    interrupt_update_after(SolverStage::ModalVectorDot, 129, || {
        orthogonalize(&mut candidate, &previous)
    });
    assert_eq!(candidate[0], 0.0);
    assert!(candidate[64..].iter().all(|value| *value == 0.0625));
    let mut replay = seed;
    orthogonalize(&mut replay, &previous).unwrap();
    assert_eq!(dot(&replay, &previous[0].1).unwrap(), 0.0);
    assert!((0.5..=1.0).contains(&vector_norm(replay.into_iter()).unwrap()));
}

#[test]
fn modal_refinement_inverse_correction_cancels_after_substitution_and_replays() {
    let system = diagonal(&(0..129).map(|index| 1.0 + index as f64).collect::<Vec<_>>());
    let mut seed = vec![1e-10; 129];
    seed[0] = 1.0;
    let mut pairs = [(1.0, seed.clone())];
    interrupt_update_after(SolverStage::TridiagonalSubstitution, 258, || {
        refine_dense_modes(&system, &mut pairs, 1e-8)
    });
    assert_eq!(pairs[0].0, 1.0);
    assert_eq!(pairs[0].1, seed, "do not commit an interrupted correction");
    let mut replay = [(1.0, seed)];
    refine_dense_modes(&system, &mut replay, 1e-8).unwrap();
    assert!(
        relative_residual(
            replay[0].0,
            &replay[0].1,
            &system.operator.apply_compensated(&replay[0].1).unwrap()
        )
        .unwrap()
            <= 1e-8
    );
}

#[test]
fn modal_inverse_candidate_smoothing_cancels_without_committing_the_correction() {
    let system = diagonal(&(0..129).map(|index| 1.0 + index as f64).collect::<Vec<_>>());
    let mut seed = vec![1e-7; 129];
    seed[0] = 1.0;
    for stage in [
        SolverStage::ModalVectorScan,
        SolverStage::ModalVectorDot,
        SolverStage::ModalVectorUpdate,
    ] {
        let control = SolverControl::default();
        let cancel = control.clone();
        let armed = std::rc::Rc::new(std::cell::Cell::new(false));
        let observed = armed.clone();
        let mut pairs = [(1.0, seed.clone())];
        let error = with_solver_observer(
            &control,
            move |point| {
                if point.stage == SolverStage::ModalValidation && point.completed_steps == 0 {
                    observed.set(true);
                }
                if observed.get() && point.stage == stage && point.completed_steps == 64 {
                    cancel.request_cancel();
                }
            },
            || {
                let result = refine_dense_modes(&system, &mut pairs, 1e-8);
                assert!(result.is_err(), "the operation itself must stop");
                result
            },
        )
        .unwrap_err();
        assert!(armed.get(), "reach the fully corrected private candidate");
        assert!(
            error.contains(&format!("{} after 64 steps", stage.as_str())),
            "{error}"
        );
        assert_eq!(pairs, [(1.0, seed.clone())]);
        assert!(control.was_interrupted());
        let mut replay = [(1.0, seed.clone())];
        refine_dense_modes(&system, &mut replay, 1e-8).unwrap();
        assert!(
            relative_residual(
                replay[0].0,
                &replay[0].1,
                &system.operator.apply_compensated(&replay[0].1).unwrap(),
            )
            .unwrap()
                <= 1e-8
        );
    }
}

#[test]
fn modal_checked_product_reuse_still_polls_before_accepting_a_converged_iterate() {
    let system = diagonal(&(0..129).map(|index| 1.0 + index as f64).collect::<Vec<_>>());
    let mut vector = vec![1e-10; 129];
    vector[0] = 1.0;
    let seed = [(1.0, vector)];
    let mut pairs = seed.clone();
    let control = SolverControl::default();
    let cancel = control.clone();
    let error = with_solver_observer(
        &control,
        move |point| {
            if point.stage == SolverStage::ModalIteration && point.completed_steps == 1 {
                cancel.request_cancel();
            }
        },
        || {
            let result = refine_dense_modes(&system, &mut pairs, 1e-8);
            assert!(
                result.is_err(),
                "do not accept a cached state before polling"
            );
            result
        },
    )
    .unwrap_err();
    assert!(error.contains("modal_iteration after 1 steps"), "{error}");
    assert!(control.was_interrupted());
    // Reach a checked, converged private state, not merely the initial validation.
    assert_ne!(pairs, seed);
    assert!(
        relative_residual(
            pairs[0].0,
            &pairs[0].1,
            &system.operator.apply_compensated(&pairs[0].1).unwrap(),
        )
        .unwrap()
            <= 1e-8
    );
    let mut replay = seed;
    refine_dense_modes(&system, &mut replay, 1e-8).unwrap();
    assert_eq!(
        replay, pairs,
        "a fresh request must not reuse a cancelled state"
    );
}
