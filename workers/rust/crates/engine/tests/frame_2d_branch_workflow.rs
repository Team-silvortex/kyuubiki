use kyuubiki_engine::run_solve_operator;
use kyuubiki_protocol::{
    Frame2dBranchSwitchSelection, Frame2dElementInput, Frame2dNodeInput,
    Frame2dStabilityKinematics, Frame2dStabilityPathControl, SolveBucklingFrame2dRequest,
    SolveFrame2dPDeltaRequest, SolveFrame2dPDeltaResult, SolveFrame2dRequest,
};

#[test]
fn workflow_route_preserves_switched_branch_continuation() {
    let mut request = shallow_arch_request(4);
    let probe = run_solve_operator(
        "solve.frame_2d_p_delta",
        serde_json::to_value(&request).unwrap(),
    )
    .expect("workflow stability probe should solve");
    let critical_factor = probe["buckling_result"]["minimum_load_factor"]
        .as_f64()
        .unwrap();

    request.kinematics = Frame2dStabilityKinematics::Corotational;
    request.path_control = Frame2dStabilityPathControl::ArcLength;
    request.maximum_load_factor = Some(critical_factor * 100.0);
    request.load_steps = Some(128);
    request.max_iterations = Some(64);
    request.max_step_cutbacks = Some(12);
    request.branch_switch = Frame2dBranchSwitchSelection::Both;
    request.branch_switch_amplitude = Some(5.0e-3);
    request.branch_continuation_steps = Some(4);
    request.branch_continuation_min_radius_ratio = Some(0.25);

    let value = run_solve_operator(
        "solve.frame_2d_p_delta",
        serde_json::to_value(&request).unwrap(),
    )
    .expect("workflow switched branches should solve");
    assert_eq!(
        value["_solver_provenance"]["operator_id"],
        "solve.frame_2d_p_delta"
    );
    let result: SolveFrame2dPDeltaResult =
        serde_json::from_value(value).expect("workflow result should preserve the typed contract");
    assert_eq!(result.input.branch_continuation_steps, Some(4));
    assert_eq!(
        result.input.branch_continuation_min_radius_ratio,
        Some(0.25)
    );
    let candidate = result
        .steps
        .iter()
        .find(|step| !step.branch_switch_probes.is_empty())
        .expect("workflow result should retain the branch transition");
    assert_eq!(candidate.branch_switch_probes.len(), 2);
    for branch in &candidate.branch_switch_probes {
        assert!(branch.distinct_branch);
        assert_eq!(branch.continuation_converged, Some(true));
        assert_eq!(branch.continuation_failure_detail, None);
        assert_eq!(branch.continuation_steps.len(), 4);
        assert!(branch.continuation_steps.iter().all(|step| {
            step.converged
                && step.residual_norm < 1.0e-7
                && step.arc_length_constraint_error < 1.0e-7
                && step.path_event.is_none()
                && step.tangent_negative_pivots == Some(1)
        }));
    }
}

#[test]
fn workflow_route_preserves_repeated_mode_combination_branch_families() {
    let mut request = repeated_arch_request(2);
    let probe = run_solve_operator(
        "solve.frame_2d_p_delta",
        serde_json::to_value(&request).unwrap(),
    )
    .expect("workflow twin-arch stability probe should solve");
    let critical_factor = probe["buckling_result"]["minimum_load_factor"]
        .as_f64()
        .unwrap();
    request.kinematics = Frame2dStabilityKinematics::Corotational;
    request.path_control = Frame2dStabilityPathControl::ArcLength;
    request.maximum_load_factor = Some(critical_factor * 100.0);
    request.load_steps = Some(128);
    request.max_iterations = Some(64);
    request.max_step_cutbacks = Some(12);
    request.branch_switch = Frame2dBranchSwitchSelection::Both;
    request.branch_switch_amplitude = Some(5.0e-3);
    request.branch_switch_mode_count = Some(2);
    request.branch_switch_pairwise_combinations = true;
    request.branch_switch_mode_weights = Some(vec![1.0, 1.0]);
    request.branch_continuation_steps = Some(2);

    let value = run_solve_operator(
        "solve.frame_2d_p_delta",
        serde_json::to_value(&request).unwrap(),
    )
    .expect("workflow repeated branch families should solve");
    let result: SolveFrame2dPDeltaResult =
        serde_json::from_value(value).expect("workflow result should preserve repeated modes");
    assert_eq!(result.input.branch_switch_mode_count, Some(2));
    assert!(result.input.branch_switch_pairwise_combinations);
    assert_eq!(
        result.input.branch_switch_mode_weights,
        Some(vec![1.0, 1.0])
    );
    let candidate = result
        .steps
        .iter()
        .find(|step| {
            step.tangent_critical_modes.len() == 2
                && step.branch_switch_probes.len() == 10
                && step
                    .branch_switch_probes
                    .iter()
                    .all(|probe| probe.distinct_branch)
        })
        .expect("workflow should retain individual and pairwise branch families");
    assert_eq!(
        candidate
            .branch_switch_probes
            .iter()
            .take(4)
            .map(|probe| probe.mode_index)
            .collect::<Vec<_>>(),
        vec![0, 0, 1, 1]
    );
    assert!(candidate.branch_switch_probes[4..].iter().all(
        |probe| probe.mode_eigenvalue.is_none()
            && probe.mode_components.len() == 2
            && probe.mode_component_projections.len() == 2
    ));
    let weighted = &candidate.branch_switch_probes[8..];
    assert!(weighted.iter().all(|probe| {
        (probe.mode_components[0].weight - std::f64::consts::FRAC_1_SQRT_2).abs() < 1.0e-12
            && (probe.mode_components[1].weight - std::f64::consts::FRAC_1_SQRT_2).abs() < 1.0e-12
    }));
    assert!(
        candidate
            .branch_switch_probes
            .iter()
            .all(|probe| probe.continuation_converged == Some(true)
                && probe.continuation_steps.len() == 2)
    );
}

#[test]
fn workflow_route_preserves_automatic_three_mode_subspace_fan() {
    let mut request = repeated_arch_request(3);
    let probe = run_solve_operator(
        "solve.frame_2d_p_delta",
        serde_json::to_value(&request).unwrap(),
    )
    .expect("workflow triple-arch stability probe should solve");
    let critical_factor = probe["buckling_result"]["minimum_load_factor"]
        .as_f64()
        .unwrap();
    request.kinematics = Frame2dStabilityKinematics::Corotational;
    request.path_control = Frame2dStabilityPathControl::ArcLength;
    request.maximum_load_factor = Some(critical_factor * 100.0);
    request.load_steps = Some(128);
    request.max_iterations = Some(64);
    request.max_step_cutbacks = Some(12);
    request.branch_switch = Frame2dBranchSwitchSelection::Both;
    request.branch_switch_amplitude = Some(5.0e-3);
    request.branch_switch_mode_count = Some(3);
    request.branch_switch_subspace_sample_count = Some(4);

    let value = run_solve_operator(
        "solve.frame_2d_p_delta",
        serde_json::to_value(&request).unwrap(),
    )
    .expect("workflow automatic subspace fan should solve");
    let result: SolveFrame2dPDeltaResult =
        serde_json::from_value(value).expect("workflow result should preserve the subspace fan");
    assert_eq!(result.input.branch_switch_subspace_sample_count, Some(4));
    let candidate = result
        .steps
        .iter()
        .find(|step| {
            step.tangent_critical_modes.len() == 3
                && step.branch_switch_probes.len() == 14
                && step
                    .branch_switch_probes
                    .iter()
                    .skip(6)
                    .all(|probe| probe.distinct_branch)
        })
        .expect("workflow should retain the automatic three-mode fan");
    assert!(candidate.branch_switch_probes[6..].iter().all(|probe| {
        probe.mode_eigenvalue.is_none()
            && probe.mode_components.len() == 3
            && probe.mode_component_projections.len() == 3
    }));
}

fn repeated_arch_request(copy_count: usize) -> SolveFrame2dPDeltaRequest {
    let mut request = shallow_arch_request(4);
    let source_nodes = request.buckling.frame.nodes.clone();
    let source_elements = request.buckling.frame.elements.clone();
    let source_shape = request.imperfection_shape.clone().unwrap();
    let mut nodes = Vec::new();
    let mut elements = Vec::new();
    let mut shape = Vec::new();
    for copy in 0..copy_count {
        let center = (copy as f64 - (copy_count - 1) as f64 / 2.0) * 2.5;
        let node_offset = nodes.len();
        nodes.extend(source_nodes.iter().cloned().map(|mut node| {
            node.id = format!("arch-{copy}-{}", node.id);
            node.x += center;
            node
        }));
        elements.extend(source_elements.iter().cloned().map(|mut element| {
            element.id = format!("arch-{copy}-{}", element.id);
            element.node_i += node_offset;
            element.node_j += node_offset;
            element
        }));
        shape.extend_from_slice(&source_shape);
    }
    request.buckling.frame = SolveFrame2dRequest { nodes, elements };
    request.imperfection_shape = Some(shape);
    request
}

fn shallow_arch_request(segments_per_side: usize) -> SolveFrame2dPDeltaRequest {
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
            imperfection_shape.extend([0.0, -branch_position, 0.0]);
            Frame2dNodeInput {
                id: format!("arch-node-{index}"),
                x: -1.0 + index as f64 / segments_per_side as f64,
                y: 0.1 * branch_position,
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
        branch_switch_mode_count: None,
        branch_switch_pairwise_combinations: false,
        branch_switch_mode_weights: None,
        branch_switch_subspace_sample_count: None,
        branch_continuation_steps: None,
        branch_continuation_radius: None,
        branch_continuation_min_radius_ratio: None,
    }
}
