use kyuubiki_protocol::{
    Frame2dBranchSwitchSelection, Frame2dElementInput, Frame2dNodeInput,
    Frame2dStabilityKinematics, Frame2dStabilityPathControl, SolveBucklingFrame2dRequest,
    SolveFrame2dPDeltaRequest, SolveFrame2dRequest,
};
use kyuubiki_solver::{solve_buckling_frame_2d, solve_frame_2d_p_delta};

const HOST_DOF_COUNT: usize = 27;

#[test]
fn single_and_multi_mode_requests_retain_the_same_first_factor() {
    let request = shallow_arch_request().buckling;
    let mut single = request.clone();
    single.mode_count = Some(1);
    let single = solve_buckling_frame_2d(&single).expect("single mode should solve");
    let multi = solve_buckling_frame_2d(&request).expect("multiple modes should solve");

    assert_relative(
        single.minimum_load_factor,
        multi.minimum_load_factor,
        1.0e-10,
    );
    assert_relative(
        single.modes[0].load_factor,
        multi.modes[0].load_factor,
        1.0e-10,
    );
}

#[test]
fn unloaded_free_branch_preserves_buckling_and_fixed_load_host_equilibria() {
    let host_request = shallow_arch_request();
    let mut branched_request = shallow_arch_request();
    add_unloaded_free_branch(&mut branched_request);
    let host_buckling =
        solve_buckling_frame_2d(&host_request.buckling).expect("host buckling should solve");
    let branched_buckling = solve_buckling_frame_2d(&branched_request.buckling)
        .expect("branched buckling should solve");

    assert_relative(
        branched_buckling.minimum_load_factor,
        host_buckling.minimum_load_factor,
        1.0e-9,
    );
    for (host_mode, branched_mode) in host_buckling.modes.iter().zip(&branched_buckling.modes) {
        assert_relative(branched_mode.load_factor, host_mode.load_factor, 1.0e-9);
    }
    assert!(
        !branched_buckling
            .element_preloads
            .last()
            .unwrap()
            .active_in_geometric_stiffness
    );

    let host = solve(load_controlled(
        host_request,
        host_buckling.minimum_load_factor,
    ));
    let branched = solve(load_controlled(
        branched_request,
        branched_buckling.minimum_load_factor,
    ));
    assert_eq!(branched.steps.len(), host.steps.len());
    for (host_step, branched_step) in host.steps.iter().zip(&branched.steps) {
        assert_relative(branched_step.load_factor, host_step.load_factor, 2.0e-12);
        assert_host_vector(
            &branched_step.displacements,
            &host_step.displacements,
            2.0e-10,
        );
    }
}

fn solve(request: SolveFrame2dPDeltaRequest) -> kyuubiki_protocol::SolveFrame2dPDeltaResult {
    solve_frame_2d_p_delta(&request).expect("reference topology should solve")
}

fn load_controlled(
    mut request: SolveFrame2dPDeltaRequest,
    critical_factor: f64,
) -> SolveFrame2dPDeltaRequest {
    request.kinematics = Frame2dStabilityKinematics::Corotational;
    request.path_control = Frame2dStabilityPathControl::LoadControl;
    request.maximum_load_factor = Some(critical_factor * 0.8);
    request.load_steps = Some(16);
    request.max_iterations = Some(64);
    request.max_step_cutbacks = Some(12);
    request
}

fn assert_host_vector(branched: &[f64], host: &[f64], tolerance: f64) {
    assert_eq!(host.len(), HOST_DOF_COUNT);
    assert!(
        branched[..HOST_DOF_COUNT]
            .iter()
            .zip(host)
            .map(|(left, right)| (left - right).powi(2))
            .sum::<f64>()
            .sqrt()
            < tolerance
    );
}

fn add_unloaded_free_branch(request: &mut SolveFrame2dPDeltaRequest) {
    let tip = request.buckling.frame.nodes.len();
    request.buckling.frame.nodes.push(Frame2dNodeInput {
        id: "spectator-tip".into(),
        x: 0.0,
        y: 0.3,
        fix_x: false,
        fix_y: false,
        fix_rz: false,
        load_x: 0.0,
        load_y: 0.0,
        moment_z: 0.0,
    });
    request
        .imperfection_shape
        .as_mut()
        .unwrap()
        .extend([0.0, -1.0, 0.0]);
    request.buckling.frame.elements.push(Frame2dElementInput {
        id: "spectator-branch".into(),
        node_i: 4,
        node_j: tip,
        area: 1.0e-3,
        youngs_modulus: 210.0e9,
        moment_of_inertia: 1.0e-8,
        section_modulus: 1.0e-6,
    });
}

fn shallow_arch_request() -> SolveFrame2dPDeltaRequest {
    let segments_per_side = 4;
    let crown = segments_per_side;
    let final_node = segments_per_side * 2;
    let mut imperfection_shape = Vec::with_capacity((final_node + 1) * 3);
    let nodes = (0..=final_node)
        .map(|index| {
            let position = if index <= crown {
                index as f64 / segments_per_side as f64
            } else {
                (final_node - index) as f64 / segments_per_side as f64
            };
            imperfection_shape.extend([0.0, -position, 0.0]);
            Frame2dNodeInput {
                id: format!("arch-node-{index}"),
                x: -1.0 + index as f64 / segments_per_side as f64,
                y: 0.1 * position,
                fix_x: index == 0 || index == final_node,
                fix_y: index == 0 || index == final_node,
                fix_rz: false,
                load_x: 0.0,
                load_y: if index == crown { -1_000.0 } else { 0.0 },
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
            mode_count: Some(6),
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

fn assert_relative(actual: f64, expected: f64, tolerance: f64) {
    let relative = (actual - expected).abs() / expected.abs().max(1.0);
    assert!(
        relative <= tolerance,
        "actual={actual}, expected={expected}, relative={relative}"
    );
}
