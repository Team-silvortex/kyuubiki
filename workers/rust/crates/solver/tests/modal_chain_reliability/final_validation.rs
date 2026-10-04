use super::*;

fn cancel_final_validation_and_replay(space: bool) {
    for uniform in [false, true] {
        for scale in [2.0_f64.powi(-600), 1.0, 2.0_f64.powi(600)] {
            let input = model(space, uniform, scale);
            for stage in [
                SolverStage::ModalSpectrumNorm,
                SolverStage::ModalShapeScan,
                SolverStage::ModalShapeNorm,
            ] {
                let control = SolverControl::default();
                let cancel = control.clone();
                let error = with_solver_observer(
                    &control,
                    move |point| {
                        // ModalSweep also marks tridiagonal validation, not just Jacobi.
                        assert_ne!(
                            point.stage,
                            SolverStage::ModalVectorScan,
                            "do not refine densely"
                        );
                        assert_ne!(
                            point.stage,
                            SolverStage::DenseFactor,
                            "do not factor densely"
                        );
                        if point.stage == stage && point.completed_steps == 64 {
                            cancel.request_cancel();
                        }
                    },
                    || {
                        let result = solve(space, input.clone());
                        assert!(result.is_err(), "cancel before returning a public mode");
                        result
                    },
                )
                .unwrap_err();
                assert!(
                    error.contains(&format!("{} after 64 steps", stage.as_str())),
                    "{error}"
                );
                assert_eq!(control.last_checkpoint().unwrap().stage, stage);
                assert_eq!(control.last_checkpoint().unwrap().completed_steps, 64);
                let replay = solve(space, input.clone()).unwrap();
                check_physical_mode(space, uniform, &replay);
                let owned = if space {
                    serde_json::to_value(
                        solve_modal_frame_3d_owned(serde_json::from_value(input.clone()).unwrap())
                            .unwrap(),
                    )
                    .unwrap()
                } else {
                    serde_json::to_value(
                        solve_modal_frame_2d_owned(serde_json::from_value(input.clone()).unwrap())
                            .unwrap(),
                    )
                    .unwrap()
                };
                assert_eq!(owned, replay);
            }
        }
    }
}

#[test]
fn planar_sparse_final_modal_validation_cancels_then_replays_physical_modes() {
    cancel_final_validation_and_replay(false);
}

#[test]
fn spatial_sparse_final_modal_validation_cancels_then_replays_physical_modes() {
    cancel_final_validation_and_replay(true);
}
