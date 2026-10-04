use super::*;
use kyuubiki_protocol::SolveModalFrame2dResult;

#[path = "../../../solver/tests/support/modal_roundoff_reference.rs"]
mod reference;

fn check(value: &Value) {
    let result: SolveModalFrame2dResult = serde_json::from_value(value.clone()).unwrap();
    assert_eq!(result.modes.len(), 1);
    assert_eq!(result.free_dofs.len(), 256);
    let relative = reference::reassembled_residual(&result);
    println!("headless single mode independently reassembled relative={relative:e}");
    assert!(relative <= 1e-8, "{relative:e}");
    assert!((result.modes[0].participation_norm - 1.0).abs() < 1e-10);
}

#[test]
fn headless_bounded_roundoff_admission_preserves_public_json_and_failure_boundaries() {
    for length in [1.0, 1e14] {
        let result = planned_solve(false, model_with_segments(128, length)).unwrap();
        let restored: Value =
            serde_json::from_slice(&serde_json::to_vec(&result).unwrap()).unwrap();
        assert_eq!(restored, result);
        let decoded: SolveModalFrame2dResult = serde_json::from_value(restored.clone()).unwrap();
        assert!(
            decoded.modes[0]
                .shape
                .iter()
                .zip(result["modes"][0]["shape"].as_array().unwrap())
                .all(|(a, b)| a.to_bits() == b.as_f64().unwrap().to_bits())
        );
        check(&restored);
    }
    let error = planned_solve(false, model_with_segments(128, 1e-10)).unwrap_err();
    assert!(
        error.contains("physical modal roundoff recovery failed"),
        "{error}"
    );
    let mut unresolved_spectrum = model_with_segments(128, 1.0);
    unresolved_spectrum["mode_count"] = json!(6);
    let error = planned_solve(false, unresolved_spectrum).unwrap_err();
    assert!(
        error.contains("refinement did not converge within 4 steps"),
        "{error}"
    );
    check(&planned_solve(false, model_with_segments(128, 1.0)).unwrap());
}

#[test]
fn headless_roundoff_phase_cancellation_returns_no_partial_result_and_replays() {
    let baseline = planned_solve(false, model_with_segments(128, 1.0)).unwrap();
    for stage in [
        SolverStage::ModalRoundoffPrepare,
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
            || planned_solve(false, model_with_segments(128, 1.0)),
        )
        .unwrap_err();
        assert!(error.contains("cancel"), "{error}");
        assert_eq!(control.last_checkpoint().unwrap().stage, stage);
        let replay = planned_solve(false, model_with_segments(128, 1.0)).unwrap();
        assert_eq!(replay, baseline);
        check(&replay);
    }
}
