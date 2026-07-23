use super::{IMPERFECTION_AMPLITUDE, element, rotate};
use kyuubiki_protocol::{
    Frame2dElementInput, Frame2dNodeInput, Frame2dStabilityKinematics, Frame2dStabilityPathControl,
    SolveBucklingFrame2dRequest, SolveFrame2dPDeltaRequest, SolveFrame2dRequest,
};

pub(super) fn portal_request(angle: f64) -> SolveFrame2dPDeltaRequest {
    let points = [(0.0, 0.0), (4.0, 0.0), (0.0, 3.0), (4.0, 3.0)];
    let nodes = points
        .into_iter()
        .enumerate()
        .map(|(index, (x, y))| {
            let (x, y) = rotate(x, y, angle);
            let (load_x, load_y) = if index >= 2 {
                rotate(0.0, -80_000.0, angle)
            } else {
                (0.0, 0.0)
            };
            Frame2dNodeInput {
                id: format!("n{index}"),
                x,
                y,
                fix_x: index < 2,
                fix_y: index < 2,
                fix_rz: index < 2,
                load_x,
                load_y,
                moment_z: 0.0,
            }
        })
        .collect();
    let elements = vec![
        element("left-column", 0, 2),
        element("beam", 2, 3),
        element("right-column", 1, 3),
    ];
    let (shape_x, shape_y) = rotate(1.0, 0.0, angle);
    let imperfection_shape = vec![
        0.0, 0.0, 0.0, 0.0, 0.0, 0.0, shape_x, shape_y, 0.0, shape_x, shape_y, 0.0,
    ];
    SolveFrame2dPDeltaRequest {
        buckling: SolveBucklingFrame2dRequest {
            frame: SolveFrame2dRequest { nodes, elements },
            mode_count: Some(3),
        },
        imperfection_amplitude: IMPERFECTION_AMPLITUDE,
        kinematics: Default::default(),
        path_control: Default::default(),
        imperfection_shape: Some(imperfection_shape),
        imperfection_mode_index: None,
        maximum_load_factor: None,
        load_steps: Some(6),
        max_iterations: None,
        tolerance: None,
        max_step_cutbacks: None,
        arc_length_radius: None,
        arc_length_load_scale: None,
        arc_length_target_iterations: None,
        tangent_transition_refinement_steps: None,
        branch_switch: Default::default(),
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

pub(super) fn shallow_arch_request(segments_per_side: usize) -> SolveFrame2dPDeltaRequest {
    let segments_per_side = segments_per_side.max(1);
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
            let x = -1.0 + index as f64 / segments_per_side as f64;
            imperfection_shape.extend([0.0, -branch_position, 0.0]);
            Frame2dNodeInput {
                id: format!("arch-node-{index}"),
                x,
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
        branch_switch: Default::default(),
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
