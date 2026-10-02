use super::*;
use kyuubiki_solver::solve_modal_frame_3d;

fn cancel_during_normalization<T>(solve: impl FnOnce() -> Result<T, String>) {
    let control = SolverControl::default();
    let cancel = control.clone();
    let reached = Rc::new(Cell::new(false));
    let seen = reached.clone();
    let error = with_solver_observer(
        &control,
        move |point| {
            assert_ne!(
                point.stage,
                SolverStage::ModalSweep,
                "must stop before eigen-search"
            );
            assert_ne!(
                point.stage,
                SolverStage::DenseFactor,
                "must stop before inverse preparation"
            );
            if point.stage == SolverStage::SparseMatrixScale && point.completed_steps == 64 {
                seen.set(true);
                cancel.request_cancel();
            }
        },
        || {
            let result = solve();
            assert!(
                result.is_err(),
                "solve must fail before the observer scope exits"
            );
            result.map(|_| ())
        },
    )
    .expect_err("the solver control scope must report cancellation");
    assert!(reached.get());
    assert!(error.contains("cancel"), "{error}");
}

#[test]
fn dense_planar_normalization_cancels_before_eigen_search_and_replays_physical_modes() {
    let model = bending_chain_with_segments(66, 1.0, 6);
    cancel_during_normalization(|| solve_modal_frame_2d(&model));
    let replay = solve_modal_frame_2d(&model).unwrap();
    assert_eq!(replay.modes.len(), 6);
    published::check_published_modes(&replay, 66);
}

#[test]
fn dense_spatial_normalization_cancels_before_eigen_search_and_replays_repeated_modes() {
    let model = complete::spatial::model(66, 1.0);
    cancel_during_normalization(|| solve_modal_frame_3d(&model));
    let replay = solve_modal_frame_3d(&model).unwrap();
    assert_eq!(replay.modes.len(), 6);
    for pair in replay.modes.chunks_exact(2) {
        assert!(
            (pair[0].eigenvalue_rad_s_squared / pair[1].eigenvalue_rad_s_squared - 1.0).abs()
                < 1e-8
        );
        for mode in pair {
            let relative = published_reference::unit_bending_residual(
                66,
                mode.eigenvalue_rad_s_squared,
                &mode.shape,
                true,
            );
            assert!(relative <= 1e-8, "published spatial residual={relative:e}");
        }
    }
}
