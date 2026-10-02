use super::*;

pub(super) fn check_published_modes(result: &SolveModalFrame2dResult, segments: usize) {
    let mut worst = 0.0_f64;
    for mode in &result.modes {
        let relative = published_reference::unit_bending_residual(
            segments,
            mode.eigenvalue_rad_s_squared,
            &mode.shape,
            false,
        );
        assert!(
            relative <= 1e-8,
            "published {}-segment mode {}: {relative:e}",
            segments,
            mode.index
        );
        worst = worst.max(relative);
    }
    eprintln!(
        "published segments={segments}, modes={}, worst residual={worst:e}",
        result.modes.len()
    );
}

#[test]
fn published_bending_modes_pass_an_independent_physical_residual_gate() {
    for segments in [66, 80, 96, 100] {
        let result = solve_modal_frame_2d(&bending_chain_with_segments(segments, 1.0, 6)).unwrap();
        check_published_modes(&result, segments);
    }
}

#[test]
fn published_shape_validation_and_repair_cancel_without_partial_modes_and_replay() {
    for (stage, repair) in [
        (SolverStage::SparseMatvec, false),
        (SolverStage::ResidualValidate, true),
    ] {
        let output = Rc::new(Cell::new(false));
        let polishing = Rc::new(Cell::new(false));
        let seen = Rc::new(Cell::new(false));
        let (out, polish, hit) = (output.clone(), polishing.clone(), seen.clone());
        let control = SolverControl::default();
        let cancel = control.clone();
        let error = with_solver_observer(
            &control,
            move |point| {
                if point.stage == SolverStage::ResultTotals {
                    out.set(true);
                }
                if out.get() && point.stage == SolverStage::ModalValidation {
                    polish.set(true);
                }
                if out.get()
                    && (!repair || polish.get())
                    && point.stage == stage
                    && point.completed_steps == 64
                {
                    hit.set(true);
                    cancel.request_cancel();
                }
            },
            || {
                let result = solve_modal_frame_2d(&bending_chain_with_segments(100, 1.0, 6));
                assert!(
                    result.is_err(),
                    "solve itself must not expose a partial output"
                );
                result
            },
        )
        .unwrap_err();
        assert!(
            seen.get(),
            "must reach the output phase, not just internal refinement"
        );
        assert!(error.contains("cancel"), "{error}");
        let result = solve_modal_frame_2d(&bending_chain_with_segments(100, 1.0, 6)).unwrap();
        check_published_modes(&result, 100);
    }
}
