use kyuubiki_protocol::SolveFrame2dPDeltaRequest;
use kyuubiki_solver::solver_control::{SolverControl, SolverStage, with_solver_observer};
use kyuubiki_solver::{solve_frame_2d_p_delta, solve_frame_2d_p_delta_owned};
use serde_json::json;

fn column() -> SolveFrame2dPDeltaRequest {
    serde_json::from_value(json!({
        "buckling": {"frame": {
            "nodes": (0..=8).map(|index| json!({
                "id": format!("n{index}"), "x": 0.0, "y": 0.4 * index as f64,
                "fix_x": index == 0 || index == 8, "fix_y": index == 0,
                "fix_rz": false, "load_x": 0.0,
                "load_y": if index == 8 { -100_000.0 } else { 0.0 }, "moment_z": 0.0
            })).collect::<Vec<_>>(),
            "elements": (0..8).map(|index| json!({
                "id": format!("e{index}"), "node_i": index, "node_j": index + 1,
                "area": 0.01, "youngs_modulus": 205e9, "moment_of_inertia": 7.4e-6,
                "section_modulus": 1e-4
            })).collect::<Vec<_>>()
        }, "mode_count": 1},
        "imperfection_amplitude": 0.0032, "load_steps": 4
    }))
    .unwrap()
}

fn relative(actual: f64, expected: f64, tolerance: f64) {
    assert!(
        actual.is_finite() && expected.is_finite(),
        "{actual}, {expected}"
    );
    assert!(
        (actual / expected - 1.0).abs() < tolerance,
        "{actual}, {expected}"
    );
}

#[test]
fn explicit_imperfection_shape_scaling_does_not_change_the_path() {
    let mut request = column();
    let baseline = solve_frame_2d_p_delta(&request).unwrap();
    for scale in [1e-200, 1e200] {
        request.imperfection_shape = Some(
            baseline
                .initial_imperfection_shape
                .iter()
                .map(|value| value * scale)
                .collect(),
        );
        let result = solve_frame_2d_p_delta(&request).unwrap();
        for (actual, expected) in result.steps.iter().zip(&baseline.steps) {
            relative(
                actual.imperfection_amplification,
                expected.imperfection_amplification,
                1e-9,
            );
            relative(
                actual.max_incremental_displacement,
                expected.max_incremental_displacement,
                1e-9,
            );
        }
    }
}

#[test]
fn tiny_modal_imperfection_retains_secant_amplification() {
    let mut request = column();
    request.imperfection_amplitude = 1e-200;
    let result = solve_frame_2d_p_delta(&request).unwrap();
    for step in &result.steps {
        relative(
            step.imperfection_amplification,
            1.0 / (1.0 - step.critical_factor_ratio),
            1e-7,
        );
    }
}

#[test]
fn large_finite_imperfection_does_not_publish_nonfinite_metrics() {
    let mut request = column();
    request.imperfection_amplitude = 1e155;
    let result = solve_frame_2d_p_delta(&request).unwrap();
    for step in &result.steps {
        assert!(step.residual_norm.is_finite());
        assert!(step.max_incremental_displacement.is_finite());
        relative(
            step.imperfection_amplification,
            1.0 / (1.0 - step.critical_factor_ratio),
            1e-7,
        );
    }
}

#[test]
fn unavailable_extreme_mode_index_returns_error_without_panicking() {
    let mut request = column();
    request.imperfection_mode_index = Some(usize::MAX);
    let outcome = std::panic::catch_unwind(|| solve_frame_2d_p_delta(&request));
    let error = outcome
        .expect("public solve must not panic on an input index")
        .unwrap_err();
    assert!(error.contains("imperfection mode"), "{error}");
    assert!(solve_frame_2d_p_delta(&column()).is_ok());
}

#[test]
fn common_stiffness_and_force_scaling_preserves_the_precritical_path() {
    let original = column();
    let baseline = solve_frame_2d_p_delta(&original).unwrap();
    for scale in [1e-160, 1e160] {
        let mut request = original.clone();
        for node in &mut request.buckling.frame.nodes {
            node.load_y *= scale;
        }
        for element in &mut request.buckling.frame.elements {
            element.youngs_modulus *= scale;
        }
        let result = solve_frame_2d_p_delta(&request).unwrap();
        for (actual, expected) in result.steps.iter().zip(&baseline.steps) {
            assert!(actual.converged && actual.residual_norm.is_finite());
            assert!(actual.residual_norm < 1e-8);
            relative(actual.load_factor, expected.load_factor, 1e-8);
            relative(
                actual.imperfection_amplification,
                expected.imperfection_amplification,
                1e-8,
            );
            relative(
                actual.max_incremental_displacement,
                expected.max_incremental_displacement,
                1e-8,
            );
        }
    }
}

#[test]
fn unusable_shapes_are_errors_and_do_not_poison_a_fresh_path() {
    for (translation, rotation) in [(0.0, 0.0), (0.0, 1.0), (1e-200, 1e308)] {
        let mut request = column();
        let mut shape = vec![0.0; request.buckling.frame.nodes.len() * 3];
        shape[3] = translation;
        shape[5] = rotation;
        request.imperfection_shape = Some(shape);
        let error = solve_frame_2d_p_delta(&request).unwrap_err();
        assert!(error.contains("imperfection"), "{error}");
    }
    assert!(solve_frame_2d_p_delta(&column()).unwrap().converged);
}

#[test]
fn unrepresentable_step_recovery_reports_the_step_and_does_not_poison_replay() {
    let original = column();
    let baseline = solve_frame_2d_p_delta(&original).unwrap();
    let mut extreme = original.clone();
    extreme.imperfection_amplitude = 1e304;
    let error = solve_frame_2d_p_delta(&extreme).unwrap_err();
    assert!(error.contains("p-delta step 1:"), "{error}");
    assert_eq!(solve_frame_2d_p_delta(&original).unwrap(), baseline);
}

#[test]
fn owned_and_explicit_shape_paths_keep_the_same_physics() {
    let mut request = column();
    let baseline = solve_frame_2d_p_delta(&request).unwrap();
    assert_eq!(
        baseline,
        solve_frame_2d_p_delta_owned(request.clone()).unwrap()
    );
    request.imperfection_shape = Some(baseline.initial_imperfection_shape.clone());
    request.imperfection_mode_index = Some(usize::MAX);
    let explicit = solve_frame_2d_p_delta(&request).unwrap();
    for (actual, expected) in explicit.steps.iter().zip(&baseline.steps) {
        relative(
            actual.imperfection_amplification,
            expected.imperfection_amplification,
            1e-10,
        );
    }
}

#[test]
fn cancellation_after_a_completed_step_cannot_be_reported_as_partial_success() {
    let baseline = solve_frame_2d_p_delta(&column()).unwrap();
    for stage in [
        SolverStage::StabilityStep,
        SolverStage::StabilityRecovery,
        SolverStage::ResidualValidate,
    ] {
        let control = SolverControl::default();
        let cancel = control.clone();
        let completed = std::cell::Cell::new(false);
        let error = with_solver_observer(
            &control,
            move |point| {
                if point.stage == SolverStage::StabilityStep && point.completed_steps == 1 {
                    completed.set(true);
                }
                if completed.get() && point.stage == stage && point.completed_steps > 0 {
                    cancel.request_cancel();
                }
            },
            || {
                let result = solve_frame_2d_p_delta(&column());
                assert!(result.is_err(), "ignored in-path cancellation at {stage:?}");
                result
            },
        )
        .unwrap_err();
        assert!(error.contains("cancelled"), "{error}");
        assert_eq!(control.last_checkpoint().unwrap().stage, stage);
        assert_eq!(solve_frame_2d_p_delta(&column()).unwrap(), baseline);
    }
}
