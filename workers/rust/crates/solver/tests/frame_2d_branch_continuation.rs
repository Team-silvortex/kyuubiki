use kyuubiki_protocol::{
    Frame2dBranchDirection, Frame2dBranchSwitchSelection, Frame2dElementInput, Frame2dNodeInput,
    Frame2dStabilityKinematics, Frame2dStabilityPathControl, SolveBucklingFrame2dRequest,
    SolveFrame2dPDeltaRequest, SolveFrame2dRequest,
};
use kyuubiki_solver::solve_frame_2d_p_delta;

#[test]
fn switched_arch_branches_continue_without_mutating_the_primary_path() {
    let mut request = shallow_arch_request(4, 0.0);
    let probe = solve_frame_2d_p_delta(&request).expect("linear stability probe should solve");
    request.kinematics = Frame2dStabilityKinematics::Corotational;
    request.path_control = Frame2dStabilityPathControl::ArcLength;
    request.maximum_load_factor = Some(probe.buckling_result.minimum_load_factor * 100.0);
    request.load_steps = Some(128);
    request.max_iterations = Some(64);
    request.max_step_cutbacks = Some(12);
    request.branch_switch = Frame2dBranchSwitchSelection::Both;
    request.branch_switch_amplitude = Some(5.0e-3);

    let probe_only =
        solve_frame_2d_p_delta(&request).expect("branch probes should remain inspectable");
    assert!(probe_only.steps.iter().all(|step| {
        step.branch_switch_probes.iter().all(|branch| {
            branch.continuation_steps.is_empty() && branch.continuation_converged.is_none()
        })
    }));
    request.branch_continuation_steps = Some(64);
    let continued =
        solve_frame_2d_p_delta(&request).expect("switched branches should remain inspectable");

    assert_eq!(continued.steps.len(), probe_only.steps.len());
    for (baseline, switched) in probe_only.steps.iter().zip(&continued.steps) {
        assert_relative(switched.load_factor, baseline.load_factor, 1.0e-12);
        assert_eq!(switched.displacements.len(), baseline.displacements.len());
        for (actual, expected) in switched.displacements.iter().zip(&baseline.displacements) {
            assert_relative(*actual, *expected, 1.0e-11);
        }
    }

    let candidate = continued
        .steps
        .iter()
        .find(|step| !step.branch_switch_probes.is_empty())
        .expect("tangent transition should emit switched branches");
    assert_eq!(candidate.branch_switch_probes.len(), 2);
    let mut final_states = Vec::new();
    for (index, branch) in candidate.branch_switch_probes.iter().enumerate() {
        let expected_direction = if index == 0 {
            Frame2dBranchDirection::Positive
        } else {
            Frame2dBranchDirection::Negative
        };
        assert_eq!(branch.direction, expected_direction);
        assert!(branch.distinct_branch, "branch seed: {branch:?}");
        assert_eq!(branch.continuation_converged, Some(true));
        assert_eq!(branch.continuation_failure_detail, None);
        assert_eq!(branch.continuation_steps.len(), 64, "branch: {branch:?}");
        assert!(branch.continuation_steps.iter().all(|step| {
            step.converged
                && step.failure_reason.is_none()
                && step.failure_detail.is_none()
                && step.residual_norm < 1.0e-7
                && step.arc_length_constraint_error < 1.0e-7
                && step.arc_length_radius.is_finite()
                && step.arc_length_radius > 0.0
                && step.load_factor_increment.is_finite()
        }));
        let seed = branch.displacements.as_ref().unwrap();
        let final_state = &branch.continuation_steps.last().unwrap().displacements;
        assert!(
            branch.continuation_steps.iter().all(|step| {
                step.path_event.is_none() && step.tangent_negative_pivots == Some(1)
            })
        );
        assert_eq!(
            branch.continuation_steps[0].tangent_negative_pivot_delta,
            None
        );
        assert!(
            branch
                .continuation_steps
                .iter()
                .skip(1)
                .all(|step| step.tangent_negative_pivot_delta == Some(0))
        );
        assert!(
            branch
                .continuation_steps
                .windows(2)
                .all(|pair| pair[1].load_factor < pair[0].load_factor)
        );
        assert!(distance(seed, final_state) > 1.0e-5);
        final_states.push(final_state);
    }
    assert!(distance(final_states[0], final_states[1]) > 5.0e-3);
}

#[test]
fn branch_continuation_controls_are_bounded_and_require_switching() {
    let mut request = shallow_arch_request(1, 0.0);
    request.branch_continuation_steps = Some(65);
    let error = solve_frame_2d_p_delta(&request)
        .expect_err("excessive branch continuation steps must fail");
    assert!(error.contains("branch_continuation_steps"));

    request.branch_continuation_steps = Some(1);
    let error =
        solve_frame_2d_p_delta(&request).expect_err("continuation without switching must fail");
    assert!(error.contains("requires branch switching"));
}

#[test]
fn switched_branches_are_objective_under_rigid_rotation() {
    let baseline = solve_switched_arch(0.0, 8);
    let angle = 0.731;
    let rotated = solve_switched_arch(angle, 8);
    let baseline_candidate = branch_candidate(&baseline);
    let rotated_candidate = branch_candidate(&rotated);
    assert_eq!(baseline_candidate.len(), 2);
    assert_eq!(rotated_candidate.len(), 2);

    for baseline_branch in baseline_candidate {
        let rotated_seed =
            rotate_displacements(baseline_branch.displacements.as_ref().unwrap(), angle);
        let rotated_branch = rotated_candidate
            .iter()
            .min_by(|left, right| {
                distance(left.displacements.as_ref().unwrap(), &rotated_seed).total_cmp(&distance(
                    right.displacements.as_ref().unwrap(),
                    &rotated_seed,
                ))
            })
            .unwrap();
        assert!(
            distance(
                rotated_branch.displacements.as_ref().unwrap(),
                &rotated_seed
            ) < 2.0e-8
        );
        assert_relative(
            rotated_branch.load_factor.unwrap(),
            baseline_branch.load_factor.unwrap(),
            2.0e-9,
        );
        for (baseline_step, rotated_step) in baseline_branch
            .continuation_steps
            .iter()
            .zip(&rotated_branch.continuation_steps)
        {
            assert_relative(rotated_step.load_factor, baseline_step.load_factor, 2.0e-9);
            assert!(
                distance(
                    &rotated_step.displacements,
                    &rotate_displacements(&baseline_step.displacements, angle)
                ) < 2.0e-8
            );
            assert_eq!(rotated_step.path_event, baseline_step.path_event);
            assert_eq!(
                rotated_step.tangent_negative_pivots,
                baseline_step.tangent_negative_pivots
            );
        }
    }
}

fn solve_switched_arch(
    angle: f64,
    continuation_steps: usize,
) -> kyuubiki_protocol::SolveFrame2dPDeltaResult {
    let mut request = shallow_arch_request(4, angle);
    let probe = solve_frame_2d_p_delta(&request).expect("rotated stability probe should solve");
    request.kinematics = Frame2dStabilityKinematics::Corotational;
    request.path_control = Frame2dStabilityPathControl::ArcLength;
    request.maximum_load_factor = Some(probe.buckling_result.minimum_load_factor * 100.0);
    request.load_steps = Some(128);
    request.max_iterations = Some(64);
    request.max_step_cutbacks = Some(12);
    request.branch_switch = Frame2dBranchSwitchSelection::Both;
    request.branch_switch_amplitude = Some(5.0e-3);
    request.branch_continuation_steps = Some(continuation_steps);
    solve_frame_2d_p_delta(&request).expect("rotated switched branches should solve")
}

fn branch_candidate(
    result: &kyuubiki_protocol::SolveFrame2dPDeltaResult,
) -> &[kyuubiki_protocol::Frame2dBranchSwitchProbeResult] {
    &result
        .steps
        .iter()
        .find(|step| !step.branch_switch_probes.is_empty())
        .expect("transition should produce branches")
        .branch_switch_probes
}

fn shallow_arch_request(segments_per_side: usize, angle: f64) -> SolveFrame2dPDeltaRequest {
    let crown = segments_per_side;
    let final_node = segments_per_side * 2;
    let mut imperfection_shape = Vec::with_capacity((final_node + 1) * 3);
    let nodes = (0..=final_node)
        .map(|index| {
            let branch_position = if index <= crown {
                index as f64 / segments_per_side as f64
            } else {
                (final_node - index) as f64 / segments_per_side as f64
            };
            let (x, y) = rotate(
                -1.0 + index as f64 / segments_per_side as f64,
                0.1 * branch_position,
                angle,
            );
            let (shape_x, shape_y) = rotate(0.0, -branch_position, angle);
            let (load_x, load_y) = if index == crown {
                rotate(0.0, -1_000.0, angle)
            } else {
                (0.0, 0.0)
            };
            imperfection_shape.extend([shape_x, shape_y, 0.0]);
            Frame2dNodeInput {
                id: format!("arch-node-{index}"),
                x,
                y,
                fix_x: index == 0 || index == final_node,
                fix_y: index == 0 || index == final_node,
                fix_rz: false,
                load_x,
                load_y,
                moment_z: 0.0,
            }
        })
        .collect();
    let elements = (0..final_node)
        .map(|index| Frame2dElementInput {
            id: format!("arch-member-{index}"),
            node_i: index,
            node_j: index + 1,
            area: 1.0e-3,
            youngs_modulus: 210.0e9,
            moment_of_inertia: 1.0e-8,
            section_modulus: 1.0e-6,
        })
        .collect();
    SolveFrame2dPDeltaRequest {
        buckling: SolveBucklingFrame2dRequest {
            frame: SolveFrame2dRequest { nodes, elements },
            mode_count: Some(1),
        },
        imperfection_amplitude: 1.0e-3,
        kinematics: Frame2dStabilityKinematics::LinearizedPDelta,
        path_control: Frame2dStabilityPathControl::LoadControl,
        imperfection_shape: Some(imperfection_shape),
        imperfection_mode_index: None,
        maximum_load_factor: None,
        load_steps: Some(8),
        max_iterations: None,
        tolerance: Some(1.0e-8),
        max_step_cutbacks: None,
        arc_length_radius: None,
        arc_length_load_scale: None,
        arc_length_target_iterations: None,
        tangent_transition_refinement_steps: None,
        branch_switch: Frame2dBranchSwitchSelection::Disabled,
        branch_switch_amplitude: None,
        branch_continuation_steps: None,
    }
}

fn distance(left: &[f64], right: &[f64]) -> f64 {
    left.iter()
        .zip(right)
        .map(|(left, right)| (left - right).powi(2))
        .sum::<f64>()
        .sqrt()
}

fn rotate_displacements(values: &[f64], angle: f64) -> Vec<f64> {
    values
        .chunks_exact(3)
        .flat_map(|value| {
            let (x, y) = rotate(value[0], value[1], angle);
            [x, y, value[2]]
        })
        .collect()
}

fn rotate(x: f64, y: f64, angle: f64) -> (f64, f64) {
    let (sine, cosine) = angle.sin_cos();
    (cosine * x - sine * y, sine * x + cosine * y)
}

fn assert_relative(actual: f64, expected: f64, tolerance: f64) {
    let relative = (actual - expected).abs() / expected.abs().max(1.0);
    assert!(
        relative <= tolerance,
        "actual={actual}, expected={expected}, relative={relative}"
    );
}
