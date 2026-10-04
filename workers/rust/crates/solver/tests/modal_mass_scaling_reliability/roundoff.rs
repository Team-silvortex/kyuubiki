use super::*;

#[path = "../support/modal_roundoff_reference.rs"]
mod reference;
use reference::reassembled_residual;

fn check(result: &SolveModalFrame2dResult, segments: usize, length: f64) {
    assert_eq!(result.modes.len(), 1);
    assert_eq!(result.free_dofs.len(), 2 * segments);
    let mode = &result.modes[0];
    let mut shape = mode.shape.clone();
    for (i, v) in shape.iter_mut().enumerate() {
        if i % 3 == 2 {
            *v *= length;
        }
    }
    let relative = reassembled_residual(result);
    println!(
        "public first mode segments={segments} length={length:e} physical relative={relative:e}"
    );
    assert!(relative <= 1e-8, "{relative:e}");
    if length == 1.0 {
        if !result.input.nodes[0].fix_y {
            shape = shape.chunks_exact(3).rev().flatten().copied().collect();
        }
        assert!(
            published_reference::unit_bending_residual(
                segments,
                mode.eigenvalue_rad_s_squared,
                &shape,
                false
            ) <= 1e-8
        );
    }
    assert!((mode.participation_norm - 1.0).abs() < 1e-10);
    assert!((mode.natural_frequency_hz * mode.period_s - 1.0).abs() < 1e-12);
    for (node, values) in result.input.nodes.iter().zip(mode.shape.chunks_exact(3)) {
        if node.fix_y {
            assert_eq!(values[1], 0.0);
        }
        if node.fix_rz {
            assert_eq!(values[2], 0.0);
        }
    }
    assert!(mode.shape.chunks_exact(3).all(|v| v[0] == 0.0));
}

#[test]
fn bounded_single_mode_roundoff_recovery_publishes_checked_shapes_through_public_json() {
    for segments in [80, 100, 128] {
        for length in [1.0, 1e14, 1e-10] {
            if segments == 128 && length == 1e-10 {
                continue;
            }
            let result =
                solve_modal_frame_2d_owned(bending_chain_with_segments(segments, length, 1))
                    .unwrap_or_else(|e| panic!("segments={segments} length={length:e}: {e}"));
            let restored: SolveModalFrame2dResult =
                serde_json::from_slice(&serde_json::to_vec(&result).unwrap()).unwrap();
            assert_eq!(result, restored);
            assert!(
                result.modes[0]
                    .shape
                    .iter()
                    .zip(&restored.modes[0].shape)
                    .all(|(a, b)| a.to_bits() == b.to_bits())
            );
            check(&restored, segments, length);
        }
    }
}

#[test]
fn bounded_single_mode_roundoff_rejects_unresolved_scaled_geometry_without_relaxation() {
    let request = bending_chain_with_segments(128, 1e-10, 1);
    for owned in [false, true] {
        let error = if owned {
            solve_modal_frame_2d_owned(request.clone())
        } else {
            solve_modal_frame_2d(&request)
        }
        .unwrap_err();
        println!("unresolved public single mode: {error}");
        assert!(error.contains("modal roundoff recovery failed"), "{error}");
        assert!(error.contains("unchanged residual gate"), "{error}");
        check(
            &solve_modal_frame_2d(&bending_chain_with_segments(128, 1.0, 1)).unwrap(),
            128,
            1.0,
        );
    }
}

#[test]
fn bounded_single_mode_roundoff_recovery_preserves_reversed_node_and_member_order() {
    for segments in [80, 100, 128] {
        let base = solve_modal_frame_2d(&bending_chain_with_segments(segments, 1.0, 1)).unwrap();
        let mut request = bending_chain_with_segments(segments, 1.0, 1);
        request.nodes.reverse();
        for element in &mut request.elements {
            element.node_i = segments - element.node_i;
            element.node_j = segments - element.node_j;
        }
        request.elements.reverse();
        let result = solve_modal_frame_2d_owned(request).unwrap();
        check(&result, segments, 1.0);
        assert!(
            (result.modes[0].eigenvalue_rad_s_squared / base.modes[0].eigenvalue_rad_s_squared
                - 1.0)
                .abs()
                < 1e-8
        );
        let actual: Vec<_> = result.modes[0]
            .shape
            .chunks_exact(3)
            .rev()
            .flatten()
            .copied()
            .collect();
        let expected = &base.modes[0].shape;
        let sign = actual
            .iter()
            .zip(expected)
            .map(|(a, b)| a * b)
            .sum::<f64>()
            .signum();
        let distance = actual
            .iter()
            .zip(expected)
            .map(|(a, b)| (a - sign * b).powi(2))
            .sum::<f64>()
            .sqrt();
        assert!(distance < 1e-8, "direction error={distance:e}");
    }
}

#[test]
fn bounded_public_roundoff_recovery_cancels_without_results_and_replays_fresh() {
    let request = bending_chain_with_segments(128, 1.0, 1);
    let baseline = solve_modal_frame_2d(&request).unwrap();
    let stages = [
        SolverStage::ModalRoundoffPrepare,
        SolverStage::ModalRoundoffSearch,
        SolverStage::ModalRoundoffValidate,
    ];
    for stage in stages {
        let control = SolverControl::default();
        let cancel = control.clone();
        let error = with_solver_observer(
            &control,
            move |p| {
                if p.stage == stage {
                    cancel.request_cancel();
                }
            },
            || solve_modal_frame_2d(&request),
        )
        .unwrap_err();
        assert!(error.contains("cancel"), "{error}");
        assert_eq!(solve_modal_frame_2d(&request).unwrap(), baseline);
        check(&baseline, 128, 1.0);
    }
}

#[test]
fn bounded_public_physical_roundoff_cancels_after_normalized_admission_and_replays() {
    let request = bending_chain_with_segments(128, 1.0, 1);
    let baseline = solve_modal_frame_2d(&request).unwrap();
    for stage in [
        SolverStage::ModalRoundoffPrepare,
        SolverStage::ModalRoundoffSearch,
        SolverStage::ModalRoundoffValidate,
    ] {
        let control = SolverControl::default();
        let cancel = control.clone();
        let normalized_done = Cell::new(false);
        let error = with_solver_observer(
            &control,
            move |p| {
                if normalized_done.get() && p.stage == stage {
                    cancel.request_cancel();
                }
                if p.stage == SolverStage::ModalRoundoffValidate {
                    normalized_done.set(true);
                }
            },
            || solve_modal_frame_2d_owned(request.clone()),
        )
        .expect_err("physical phase must cancel");
        assert!(error.contains("cancel"), "{error}");
        assert_eq!(control.last_checkpoint().unwrap().stage, stage);
        let replay = solve_modal_frame_2d_owned(request.clone()).unwrap();
        assert_eq!(replay, baseline);
        check(&replay, 128, 1.0);
    }
}

#[test]
fn bounded_spatial_single_mode_keeps_the_physical_gate_after_public_json() {
    for segments in [40, 64] {
        let planar = solve_modal_frame_2d(&bending_chain_with_segments(segments, 1.0, 1)).unwrap();
        let mut request = complete::spatial::model(segments, 1.0);
        request.mode_count = Some(1);
        let result = kyuubiki_solver::solve_modal_frame_3d(&request).unwrap();
        let restored: kyuubiki_protocol::SolveModalFrame3dResult =
            serde_json::from_slice(&serde_json::to_vec(&result).unwrap()).unwrap();
        assert_eq!(restored, result);
        assert_eq!(restored.free_dofs.len(), 4 * segments);
        assert_eq!(restored.modes.len(), 1);
        let mode = &restored.modes[0];
        assert!(
            mode.shape
                .iter()
                .zip(&result.modes[0].shape)
                .all(|(a, b)| a.to_bits() == b.to_bits())
        );
        assert!(
            (mode.eigenvalue_rad_s_squared / planar.modes[0].eigenvalue_rad_s_squared - 1.0).abs()
                < 1e-8
        );
        assert!(
            published_reference::unit_bending_residual(
                segments,
                mode.eigenvalue_rad_s_squared,
                &mode.shape,
                true
            ) <= 1e-8
        );
        assert!((mode.participation_norm - 1.0).abs() < 1e-10);
    }
}
