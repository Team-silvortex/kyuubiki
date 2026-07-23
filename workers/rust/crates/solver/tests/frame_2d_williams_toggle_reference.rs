use kyuubiki_protocol::{
    Frame2dElementInput, Frame2dEquilibriumPathEvent, Frame2dNodeInput, Frame2dStabilityKinematics,
    Frame2dStabilityPathControl, SolveBucklingFrame2dRequest, SolveFrame2dPDeltaRequest,
    SolveFrame2dRequest,
};
use kyuubiki_solver::solve_frame_2d_p_delta;

const HALF_SPAN_IN: f64 = 12.943;
const RISE_IN: f64 = 0.386;
const WIDTH_IN: f64 = 0.753;
const DEPTH_IN: f64 = 0.243;
const YOUNGS_MODULUS_PSI: f64 = 10.3e6;
const ELEMENTS_PER_SIDE: usize = 5;

#[test]
fn williams_toggle_frame_matches_the_external_snap_through_limit() {
    let result =
        solve_frame_2d_p_delta(&williams_toggle_request()).expect("toggle frame should solve");
    assert!(result.converged, "toggle path: {:?}", result.steps);

    let (peak_index, peak) = result
        .steps
        .iter()
        .enumerate()
        .max_by(|left, right| left.1.load_factor.total_cmp(&right.1.load_factor))
        .expect("toggle path should contain steps");
    assert_eq!(
        peak.path_event,
        Some(Frame2dEquilibriumPathEvent::LimitPointMaximum)
    );
    assert_relative(peak.load_factor, 34.0392, 5.0e-2);
    assert!(result.steps.last().unwrap().load_factor < peak.load_factor);
    assert!(
        result.steps[peak_index + 1..]
            .iter()
            .all(|step| step.load_factor_increment.is_some_and(|value| value < 0.0))
    );
    assert!(result.steps.iter().all(|step| step.residual_norm < 1.0e-7));
    assert!(result.steps.iter().all(|step| {
        step.arc_length_constraint_error
            .is_some_and(|error| error < 1.0e-7)
    }));
}

fn williams_toggle_request() -> SolveFrame2dPDeltaRequest {
    let node_count = ELEMENTS_PER_SIDE * 2 + 1;
    let mut imperfection_shape = Vec::with_capacity(node_count * 3);
    let nodes = (0..node_count)
        .map(|index| {
            let (x, y) = if index <= ELEMENTS_PER_SIDE {
                let ratio = index as f64 / ELEMENTS_PER_SIDE as f64;
                (-HALF_SPAN_IN * (1.0 - ratio), RISE_IN * ratio)
            } else {
                let ratio = (index - ELEMENTS_PER_SIDE) as f64 / ELEMENTS_PER_SIDE as f64;
                (HALF_SPAN_IN * ratio, RISE_IN * (1.0 - ratio))
            };
            let fixed = index == 0 || index + 1 == node_count;
            let apex = index == apex_node();
            imperfection_shape.extend([0.0, if apex { -1.0 } else { 0.0 }, 0.0]);
            Frame2dNodeInput {
                id: format!("toggle-node-{index}"),
                x,
                y,
                fix_x: fixed,
                fix_y: fixed,
                fix_rz: fixed,
                load_x: 0.0,
                load_y: if apex { -1.0 } else { 0.0 },
                moment_z: 0.0,
            }
        })
        .collect();
    let area = WIDTH_IN * DEPTH_IN;
    let moment_of_inertia = WIDTH_IN * DEPTH_IN.powi(3) / 12.0;
    let section_modulus = WIDTH_IN * DEPTH_IN.powi(2) / 6.0;
    let elements = (0..node_count - 1)
        .map(|index| Frame2dElementInput {
            id: format!("toggle-member-{index}"),
            node_i: index,
            node_j: index + 1,
            area,
            youngs_modulus: YOUNGS_MODULUS_PSI,
            moment_of_inertia,
            section_modulus,
        })
        .collect();

    SolveFrame2dPDeltaRequest {
        buckling: SolveBucklingFrame2dRequest {
            frame: SolveFrame2dRequest { nodes, elements },
            mode_count: Some(3),
        },
        imperfection_amplitude: 1.0e-10,
        kinematics: Frame2dStabilityKinematics::Corotational,
        path_control: Frame2dStabilityPathControl::ArcLength,
        imperfection_shape: Some(imperfection_shape),
        imperfection_mode_index: None,
        maximum_load_factor: Some(100.0),
        load_steps: Some(128),
        max_iterations: Some(64),
        tolerance: Some(1.0e-8),
        max_step_cutbacks: Some(12),
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
        branch_switch_subspace_refinement_levels: None,
        branch_continuation_steps: None,
        branch_continuation_radius: None,
        branch_continuation_min_radius_ratio: None,
    }
}

fn apex_node() -> usize {
    ELEMENTS_PER_SIDE
}

fn assert_relative(actual: f64, expected: f64, tolerance: f64) {
    let relative = (actual - expected).abs() / expected.abs().max(1.0);
    assert!(
        relative <= tolerance,
        "actual={actual}, expected={expected}, relative={relative}"
    );
}
