use kyuubiki_protocol::{
    Frame2dBranchSwitchSelection, Frame2dElementInput, Frame2dNodeInput,
    Frame2dStabilityKinematics, Frame2dStabilityPathControl, SolveBucklingFrame2dRequest,
    SolveFrame2dPDeltaRequest, SolveFrame2dRequest,
};
use kyuubiki_solver::solve_frame_2d_p_delta;

#[test]
fn branch_controls_are_bounded_and_dependency_checked() {
    let mut request = shallow_arch_request();
    request.branch_continuation_steps = Some(65);
    assert_error(&request, "branch_continuation_steps");

    request.branch_continuation_steps = Some(1);
    assert_error(&request, "requires branch switching");

    request.branch_continuation_steps = None;
    request.branch_continuation_radius = Some(0.01);
    assert_error(&request, "requires branch continuation steps");

    request.branch_continuation_steps = Some(1);
    request.branch_continuation_radius = Some(0.0);
    assert_error(&request, "positive and finite");

    request.branch_continuation_radius = Some(f64::NAN);
    assert_error(&request, "positive and finite");

    request.branch_continuation_radius = None;
    request.branch_continuation_steps = None;
    request.branch_switch_mode_count = Some(0);
    assert_error(&request, "between 1 and 4");

    request.branch_switch_mode_count = Some(5);
    assert_error(&request, "between 1 and 4");

    request.branch_switch_mode_count = Some(2);
    assert_error(&request, "requires branch switching");

    request.branch_switch_mode_count = None;
    request.branch_switch_pairwise_combinations = true;
    assert_error(
        &request,
        "branch_switch_pairwise_combinations requires branch switching",
    );

    enable_branch_switching(&mut request);
    request.branch_switch_mode_count = Some(1);
    assert_error(&request, "requires at least two branch-switch modes");

    request.branch_switch_pairwise_combinations = false;
    request.branch_switch_mode_count = Some(3);
    request.branch_switch_mode_weights = Some(vec![1.0, 0.0]);
    assert_error(&request, "must contain exactly 3 weights");

    request.branch_switch_mode_weights = Some(vec![1.0, f64::NAN, -1.0]);
    assert_error(&request, "must be finite");

    request.branch_switch_mode_weights = Some(vec![1.0, 0.0, 0.0]);
    assert_error(&request, "requires at least two nonzero weights");

    request.branch_switch_mode_weights = None;
    request.branch_switch_mode_count = Some(2);
    request.branch_switch_subspace_sample_count = Some(1);
    assert_error(&request, "requires three or four branch-switch modes");

    request.branch_switch_mode_count = Some(3);
    request.branch_switch_subspace_sample_count = Some(0);
    assert_error(&request, "must be between 1 and 4");

    request.branch_switch_subspace_sample_count = Some(5);
    assert_error(&request, "must be between 1 and 4");

    request.branch_switch_mode_count = Some(4);
    request.branch_switch_subspace_sample_count = Some(17);
    assert_error(&request, "must be between 1 and 16");

    request.branch_switch_subspace_sample_count = None;
    request.branch_switch_subspace_refinement_levels = Some(1);
    assert_error(&request, "requires branch_switch_subspace_sample_count");

    request.branch_switch_mode_count = Some(3);
    request.branch_switch_subspace_sample_count = Some(4);
    request.branch_switch_subspace_refinement_levels = Some(0);
    assert_error(&request, "must be between 1 and 2");

    request.branch_switch_subspace_refinement_levels = Some(3);
    assert_error(&request, "must be between 1 and 2");

    request.branch_switch_subspace_refinement_levels = None;
    request.branch_switch_subspace_sample_count = None;
    request.branch_switch = Frame2dBranchSwitchSelection::Disabled;
    request.branch_switch_amplitude = None;
    request.branch_switch_mode_count = None;
    request.branch_continuation_min_radius_ratio = Some(0.0);
    assert_error(&request, "finite and in (0, 1]");

    request.branch_continuation_min_radius_ratio = Some(1.01);
    assert_error(&request, "finite and in (0, 1]");

    request.branch_continuation_min_radius_ratio = Some(f64::NAN);
    assert_error(&request, "finite and in (0, 1]");

    request.branch_continuation_min_radius_ratio = Some(0.25);
    assert_error(&request, "requires branch continuation steps");
}

fn enable_branch_switching(request: &mut SolveFrame2dPDeltaRequest) {
    request.kinematics = Frame2dStabilityKinematics::Corotational;
    request.path_control = Frame2dStabilityPathControl::ArcLength;
    request.branch_switch = Frame2dBranchSwitchSelection::Both;
    request.branch_switch_amplitude = Some(0.01);
}

fn assert_error(request: &SolveFrame2dPDeltaRequest, expected: &str) {
    let error = solve_frame_2d_p_delta(request).expect_err("invalid branch controls must fail");
    assert!(error.contains(expected), "error={error:?}");
}

fn shallow_arch_request() -> SolveFrame2dPDeltaRequest {
    let nodes = vec![
        Frame2dNodeInput {
            id: "left".into(),
            x: -1.0,
            y: 0.0,
            fix_x: true,
            fix_y: true,
            fix_rz: false,
            load_x: 0.0,
            load_y: 0.0,
            moment_z: 0.0,
        },
        Frame2dNodeInput {
            id: "crown".into(),
            x: 0.0,
            y: 0.1,
            fix_x: false,
            fix_y: false,
            fix_rz: false,
            load_x: 0.0,
            load_y: -1_000.0,
            moment_z: 0.0,
        },
        Frame2dNodeInput {
            id: "right".into(),
            x: 1.0,
            y: 0.0,
            fix_x: true,
            fix_y: true,
            fix_rz: false,
            load_x: 0.0,
            load_y: 0.0,
            moment_z: 0.0,
        },
    ];
    let elements = (0..2)
        .map(|index| Frame2dElementInput {
            id: format!("member-{index}"),
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
        imperfection_shape: Some(vec![0.0, 0.0, 0.0, 0.0, -1.0, 0.0, 0.0, 0.0, 0.0]),
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
        branch_switch_mode_count: None,
        branch_switch_pairwise_combinations: false,
        branch_switch_mode_weights: None,
        branch_switch_subspace_sample_count: None,
        branch_switch_subspace_refinement_levels: None,
        branch_continuation_steps: None,
        branch_continuation_radius: None,
        branch_continuation_min_radius_ratio: None,
    }
}
