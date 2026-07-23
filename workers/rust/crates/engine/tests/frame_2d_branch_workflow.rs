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
        branch_continuation_steps: None,
    }
}
