use super::*;

const STAGES: [SolverStage; 3] = [
    SolverStage::ModalVectorScan,
    SolverStage::ModalVectorDot,
    SolverStage::ModalVectorUpdate,
];

fn interrupt(stage: SolverStage, solve: impl FnOnce() -> Result<(), String>) {
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
            let result = solve();
            assert!(
                result.is_err(),
                "do not expose modes before the outer scope checks cancellation"
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
fn planar_modal_vector_cancellation_discards_modes_and_replays_independent_physical_checks() {
    let model = bending_chain_with_segments(100, 1.0, 20);
    for stage in STAGES {
        interrupt(stage, || solve_modal_frame_2d(&model).map(|_| ()));
        let result = solve_modal_frame_2d(&model).unwrap();
        complete::check_spectrum_for_segments(&result, 1.0, 20, 100, &polished::ROOTS_100);
        published::check_published_modes(&result, 100);
    }
}

#[test]
fn spatial_modal_vector_cancellation_discards_modes_and_replays_repeated_physical_shapes() {
    let model = complete::spatial::model(100, 1.0);
    let roots: Vec<_> = polished::ROOTS_100
        .iter()
        .take(3)
        .map(|(_, value)| *value)
        .collect();
    for stage in STAGES {
        interrupt(stage, || {
            kyuubiki_solver::solve_modal_frame_3d(&model).map(|_| ())
        });
        let result = kyuubiki_solver::solve_modal_frame_3d(&model).unwrap();
        complete::spatial::check_spatial_result(&result, 100, 1.0, &roots);
    }
}

const FINAL_VALIDATION_STAGES: [SolverStage; 3] = [
    SolverStage::ModalSpectrumNorm,
    SolverStage::ModalShapeScan,
    SolverStage::ModalShapeNorm,
];

#[test]
fn planar_final_modal_validation_discards_modes_and_replays_independent_physical_checks() {
    let model = bending_chain_with_segments(100, 1.0, 20);
    for stage in FINAL_VALIDATION_STAGES {
        interrupt(stage, || solve_modal_frame_2d(&model).map(|_| ()));
        let result = solve_modal_frame_2d(&model).unwrap();
        complete::check_spectrum_for_segments(&result, 1.0, 20, 100, &polished::ROOTS_100);
        published::check_published_modes(&result, 100);
    }
}

#[test]
fn spatial_final_modal_validation_discards_modes_and_replays_repeated_physical_shapes() {
    let model = complete::spatial::model(100, 1.0);
    let roots: Vec<_> = polished::ROOTS_100
        .iter()
        .take(3)
        .map(|(_, value)| *value)
        .collect();
    for stage in FINAL_VALIDATION_STAGES {
        interrupt(stage, || {
            kyuubiki_solver::solve_modal_frame_3d(&model).map(|_| ())
        });
        let result = kyuubiki_solver::solve_modal_frame_3d(&model).unwrap();
        complete::spatial::check_spatial_result(&result, 100, 1.0, &roots);
    }
}

fn interrupt_after_first_published_mode(solve: impl FnOnce() -> Result<(), String>) {
    let mode = Rc::new(Cell::new(0));
    let norms = Rc::new(Cell::new(0));
    let (current, pass) = (mode.clone(), norms.clone());
    let control = SolverControl::default();
    let cancel = control.clone();
    let error = with_solver_observer(
        &control,
        move |point| {
            if point.stage == SolverStage::ResultFreeDofs && point.completed_steps == 0 {
                current.set(current.get() + 1);
            }
            if point.stage == SolverStage::ModalShapeScan && point.completed_steps == 0 {
                pass.set(0);
            }
            if point.stage == SolverStage::ModalShapeNorm {
                if point.completed_steps == 0 {
                    pass.set(pass.get() + 1);
                }
                if current.get() == 2 && pass.get() == 3 && point.completed_steps == 64 {
                    cancel.request_cancel();
                }
            }
        },
        || {
            let result = solve();
            assert!(
                result.is_err(),
                "do not return an earlier mode as partial success"
            );
            result
        },
    )
    .unwrap_err();
    assert!(error.contains("modal_shape_norm after 64 steps"), "{error}");
    assert_eq!(mode.get(), 2);
    assert_eq!(norms.get(), 3);
    assert!(control.was_interrupted());
}

#[test]
fn planar_late_shape_cancellation_discards_prior_modes_on_owned_and_borrowed_routes() {
    let model = bending_chain_with_segments(100, 1.0, 20);
    for owned in [false, true] {
        interrupt_after_first_published_mode(|| {
            if owned {
                solve_modal_frame_2d_owned(model.clone()).map(|_| ())
            } else {
                solve_modal_frame_2d(&model).map(|_| ())
            }
        });
        let result = solve_modal_frame_2d_owned(model.clone()).unwrap();
        complete::check_spectrum_for_segments(&result, 1.0, 20, 100, &polished::ROOTS_100);
        published::check_published_modes(&result, 100);
    }
}

#[test]
fn spatial_late_shape_cancellation_discards_prior_modes_on_owned_and_borrowed_routes() {
    let model = complete::spatial::model(100, 1.0);
    let roots: Vec<_> = polished::ROOTS_100
        .iter()
        .take(3)
        .map(|(_, value)| *value)
        .collect();
    for owned in [false, true] {
        interrupt_after_first_published_mode(|| {
            if owned {
                kyuubiki_solver::solve_modal_frame_3d_owned(model.clone()).map(|_| ())
            } else {
                kyuubiki_solver::solve_modal_frame_3d(&model).map(|_| ())
            }
        });
        let result = kyuubiki_solver::solve_modal_frame_3d_owned(model.clone()).unwrap();
        complete::spatial::check_spatial_result(&result, 100, 1.0, &roots);
    }
}
