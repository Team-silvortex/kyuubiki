use kyuubiki_protocol::{
    Frame2dBranchDirection, Frame2dBranchProbeOrigin, Frame2dBranchSwitchSelection,
    Frame2dElementInput, Frame2dNodeInput, Frame2dStabilityKinematics, Frame2dStabilityPathControl,
    SolveBucklingFrame2dRequest, SolveFrame2dPDeltaRequest, SolveFrame2dRequest,
};
use kyuubiki_solver::solve_frame_2d_p_delta;

const LENGTH: f64 = 1.0;
const AREA: f64 = 0.1;
const YOUNGS_MODULUS: f64 = 1.0e6;
const MOMENT_OF_INERTIA: f64 = 1.0e-4;
const COARSE_ELEMENT_COUNT: usize = 8;
const FINE_ELEMENT_COUNT: usize = 16;

#[test]
fn euler_column_switches_into_two_postcritical_branches_at_the_analytic_load() {
    let coarse = solve_frame_2d_p_delta(&euler_branch_request(COARSE_ELEMENT_COUNT))
        .expect("coarse Euler path should solve");
    let result = solve_frame_2d_p_delta(&euler_branch_request(FINE_ELEMENT_COUNT))
        .expect("fine Euler path should solve");
    assert!(result.converged, "Euler primary path: {:?}", result.steps);

    let coarse_candidate = branch_candidate(&coarse);
    let candidate = result
        .steps
        .iter()
        .find(|step| !step.branch_switch_probes.is_empty())
        .expect("Euler tangent transition should emit branch probes");
    let analytic =
        std::f64::consts::PI.powi(2) * YOUNGS_MODULUS * MOMENT_OF_INERTIA / LENGTH.powi(2);
    let coarse_seed_load = mean_seed_load(coarse_candidate);
    let fine_seed_load = mean_seed_load(candidate);
    assert_relative(coarse.buckling_result.minimum_load_factor, analytic, 2.0e-3);
    assert_relative(result.buckling_result.minimum_load_factor, analytic, 2.0e-3);
    assert_relative(candidate.load_factor, analytic, 2.0e-2);
    assert_relative(fine_seed_load, analytic, 2.0e-2);
    assert!((fine_seed_load - analytic).abs() < (coarse_seed_load - analytic).abs());
    assert_eq!(
        candidate.path_event,
        Some(kyuubiki_protocol::Frame2dEquilibriumPathEvent::BifurcationCandidate)
    );
    assert!(
        candidate
            .tangent_critical_eigenvalue
            .is_some_and(|value| value.abs() < 1.0e-8)
    );
    assert!(
        candidate
            .tangent_critical_mode_residual
            .is_some_and(|value| value < 1.0e-8)
    );
    assert_eq!(candidate.branch_switch_probes.len(), 2);

    for (index, branch) in candidate.branch_switch_probes.iter().enumerate() {
        assert_eq!(
            branch.direction,
            if index == 0 {
                Frame2dBranchDirection::Positive
            } else {
                Frame2dBranchDirection::Negative
            }
        );
        assert!(branch.distinct_branch, "Euler branch seed: {branch:?}");
        assert_relative(
            branch.mode_projection.unwrap(),
            if index == 0 { 1.0e-3 } else { -1.0e-3 },
            1.0e-8,
        );
        assert_eq!(branch.continuation_converged, Some(true));
        assert_eq!(branch.continuation_steps.len(), 8);
        assert!(branch.continuation_steps.iter().all(|step| {
            step.converged
                && step.residual_norm < 1.0e-7
                && step.arc_length_constraint_error < 1.0e-7
        }));
    }
    let midpoint_x = |branch: &kyuubiki_protocol::Frame2dBranchSwitchProbeResult| {
        branch.continuation_steps.last().unwrap().displacements[FINE_ELEMENT_COUNT / 2 * 3]
    };
    assert!(
        midpoint_x(&candidate.branch_switch_probes[0])
            * midpoint_x(&candidate.branch_switch_probes[1])
            < 0.0
    );
}

#[test]
fn repeated_euler_modes_cover_local_same_and_opposite_branch_families() {
    let mut request = euler_network_request(COARSE_ELEMENT_COUNT, 2);
    request.branch_switch_mode_count = Some(2);
    request.branch_switch_pairwise_combinations = true;
    request.branch_continuation_steps = Some(4);
    let result =
        solve_frame_2d_p_delta(&request).expect("repeated Euler subspace should remain solvable");
    let analytic =
        std::f64::consts::PI.powi(2) * YOUNGS_MODULUS * MOMENT_OF_INERTIA / LENGTH.powi(2);

    assert_relative(
        result.buckling_result.modes[0].load_factor,
        analytic,
        2.0e-3,
    );
    assert_relative(
        result.buckling_result.modes[1].load_factor,
        analytic,
        2.0e-3,
    );
    assert_relative(
        result.buckling_result.modes[0].load_factor,
        result.buckling_result.modes[1].load_factor,
        1.0e-10,
    );
    let candidate = result
        .steps
        .iter()
        .find(|step| {
            step.tangent_critical_modes.len() == 2
                && step.branch_switch_probes.len() == 8
                && step
                    .branch_switch_probes
                    .iter()
                    .all(|probe| probe.distinct_branch)
        })
        .expect("repeated Euler transition should expose eight signed probes");
    assert!(candidate.tangent_critical_modes.iter().all(|mode| {
        mode.normalized_eigenvalue.abs() < 1.0e-8 && mode.normalized_residual < 1.0e-8
    }));
    assert_relative(
        candidate.tangent_critical_modes[0].normalized_eigenvalue,
        candidate.tangent_critical_modes[1].normalized_eigenvalue,
        1.0e-10,
    );

    let positive_directions = candidate
        .branch_switch_probes
        .iter()
        .filter(|probe| probe.direction == Frame2dBranchDirection::Positive)
        .map(|probe| {
            assert!(matches!(
                probe.origin,
                Frame2dBranchProbeOrigin::CriticalMode
                    | Frame2dBranchProbeOrigin::PairwiseCombination
            ));
            assert_relative(probe.load_factor.unwrap(), analytic, 3.0e-2);
            assert_eq!(probe.continuation_converged, Some(true));
            assert_eq!(probe.continuation_steps.len(), 4);
            assert!(probe.continuation_steps.iter().all(|step| {
                step.converged
                    && step.residual_norm < 1.0e-7
                    && step.arc_length_constraint_error < 1.0e-7
            }));
            physical_midpoint_direction(probe, COARSE_ELEMENT_COUNT)
        })
        .collect::<Vec<_>>();
    assert_eq!(positive_directions.len(), 4);

    let diagonal = std::f64::consts::FRAC_1_SQRT_2;
    for target in [
        [1.0, 0.0],
        [0.0, 1.0],
        [diagonal, diagonal],
        [diagonal, -diagonal],
    ] {
        let best_alignment = positive_directions
            .iter()
            .map(|direction| (direction[0] * target[0] + direction[1] * target[1]).abs())
            .fold(0.0_f64, f64::max);
        assert!(
            best_alignment > 0.9,
            "target={target:?}, best={best_alignment}"
        );
    }
}

fn physical_midpoint_direction(
    probe: &kyuubiki_protocol::Frame2dBranchSwitchProbeResult,
    element_count: usize,
) -> [f64; 2] {
    let displacements = probe.displacements.as_ref().unwrap();
    let first = displacements[element_count / 2 * 3];
    let second = displacements[(element_count + 1 + element_count / 2) * 3];
    let norm = first.hypot(second);
    assert!(norm > 1.0e-8);
    [first / norm, second / norm]
}

fn branch_candidate(
    result: &kyuubiki_protocol::SolveFrame2dPDeltaResult,
) -> &kyuubiki_protocol::Frame2dPDeltaStepResult {
    result
        .steps
        .iter()
        .find(|step| !step.branch_switch_probes.is_empty())
        .expect("Euler tangent transition should emit branch probes")
}

fn mean_seed_load(candidate: &kyuubiki_protocol::Frame2dPDeltaStepResult) -> f64 {
    candidate
        .branch_switch_probes
        .iter()
        .map(|probe| probe.load_factor.unwrap())
        .sum::<f64>()
        / candidate.branch_switch_probes.len() as f64
}

fn euler_branch_request(element_count: usize) -> SolveFrame2dPDeltaRequest {
    euler_network_request(element_count, 1)
}

fn euler_network_request(element_count: usize, column_count: usize) -> SolveFrame2dPDeltaRequest {
    let mut nodes = Vec::with_capacity((element_count + 1) * column_count);
    let mut elements = Vec::with_capacity(element_count * column_count);
    let mut imperfection_shape = Vec::with_capacity((element_count + 1) * column_count * 3);
    for column in 0..column_count {
        let node_offset = nodes.len();
        nodes.extend((0..=element_count).map(|index| {
            let ratio = index as f64 / element_count as f64;
            imperfection_shape.extend([(std::f64::consts::PI * ratio).sin(), 0.0, 0.0]);
            Frame2dNodeInput {
                id: format!("column-{column}-node-{index}"),
                x: column as f64 * 2.0,
                y: LENGTH * ratio,
                fix_x: index == 0 || index == element_count,
                fix_y: index == 0,
                fix_rz: false,
                load_x: 0.0,
                load_y: if index == element_count { -1.0 } else { 0.0 },
                moment_z: 0.0,
            }
        }));
        elements.extend((0..element_count).map(|index| Frame2dElementInput {
            id: format!("column-{column}-member-{index}"),
            node_i: node_offset + index,
            node_j: node_offset + index + 1,
            area: AREA,
            youngs_modulus: YOUNGS_MODULUS,
            moment_of_inertia: MOMENT_OF_INERTIA,
            section_modulus: 1.0,
        }));
    }
    let analytic =
        std::f64::consts::PI.powi(2) * YOUNGS_MODULUS * MOMENT_OF_INERTIA / LENGTH.powi(2);

    SolveFrame2dPDeltaRequest {
        buckling: SolveBucklingFrame2dRequest {
            frame: SolveFrame2dRequest { nodes, elements },
            mode_count: Some(3),
        },
        imperfection_amplitude: 1.0e-9,
        kinematics: Frame2dStabilityKinematics::Corotational,
        path_control: Frame2dStabilityPathControl::ArcLength,
        imperfection_shape: Some(imperfection_shape),
        imperfection_mode_index: None,
        maximum_load_factor: Some(analytic * 2.0),
        load_steps: Some(128),
        max_iterations: Some(64),
        tolerance: Some(1.0e-8),
        max_step_cutbacks: Some(12),
        arc_length_radius: None,
        arc_length_load_scale: None,
        arc_length_target_iterations: None,
        tangent_transition_refinement_steps: Some(16),
        branch_switch: Frame2dBranchSwitchSelection::Both,
        branch_switch_amplitude: Some(1.0e-3),
        branch_switch_mode_count: Some(1),
        branch_switch_pairwise_combinations: false,
        branch_switch_mode_weights: None,
        branch_switch_subspace_sample_count: None,
        branch_switch_subspace_refinement_levels: None,
        branch_continuation_steps: Some(8),
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
