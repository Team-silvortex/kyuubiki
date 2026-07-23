use kyuubiki_protocol::{
    Frame2dBranchProbeOrigin, Frame2dBranchSwitchSelection, Frame2dElementInput,
    Frame2dEquilibriumPathEvent, Frame2dNodeInput, Frame2dStabilityKinematics,
    Frame2dStabilityPathControl, SolveBucklingFrame2dRequest, SolveFrame2dPDeltaRequest,
    SolveFrame2dRequest,
};
use kyuubiki_solver::{solve_buckling_frame_2d, solve_frame_2d_p_delta};

const LENGTH: f64 = 1.0;
const COLUMN_SPACING: f64 = 2.0;
const COLUMN_AREA: f64 = 0.1;
const YOUNGS_MODULUS: f64 = 1.0e6;
const COLUMN_INERTIA: f64 = 1.0e-4;
const ASYMMETRIC_COLUMN_INERTIA: f64 = 1.005e-4;
const COUPLING_STIFFNESS: f64 = 20.0;
const ELEMENT_COUNT: usize = 8;
const NODES_PER_COLUMN: usize = ELEMENT_COUNT + 1;

#[test]
fn connected_columns_recover_the_external_symmetric_and_split_mode_references() {
    let request = coupled_euler_request(COLUMN_INERTIA, [1.0, 1.0]);
    let buckling =
        solve_buckling_frame_2d(&request.buckling).expect("coupled buckling should solve");
    let euler = std::f64::consts::PI.powi(2) * YOUNGS_MODULUS * COLUMN_INERTIA / LENGTH.powi(2);
    let antisymmetric_rayleigh =
        euler + 4.0 * COUPLING_STIFFNESS * LENGTH / std::f64::consts::PI.powi(2);

    assert_relative(buckling.modes[0].load_factor, euler, 2.0e-3);
    assert_relative(
        buckling.modes[1].load_factor,
        antisymmetric_rayleigh,
        5.0e-3,
    );
    assert!(midpoint_product(&buckling.modes[0].shape) > 0.0);
    assert!(midpoint_product(&buckling.modes[1].shape) < 0.0);
    assert!(buckling.modes[1].load_factor > buckling.modes[0].load_factor);

    let result =
        solve_frame_2d_p_delta(&request).expect("coupled symmetric branch should remain solvable");
    let candidate = result
        .steps
        .iter()
        .find(|step| !step.branch_switch_probes.is_empty())
        .expect("coupled symmetric transition should emit branches");
    assert_eq!(
        candidate.path_event,
        Some(Frame2dEquilibriumPathEvent::BifurcationCandidate)
    );
    assert_relative(candidate.load_factor, euler, 4.0e-2);
    assert_eq!(candidate.branch_switch_probes.len(), 2);
    for branch in &candidate.branch_switch_probes {
        assert!(branch.distinct_branch, "coupled branch seed: {branch:?}");
        assert_eq!(branch.continuation_converged, Some(true));
        assert_eq!(branch.continuation_steps.len(), 4);
        assert!(branch.continuation_steps.iter().all(|step| {
            step.converged
                && step.residual_norm < 1.0e-7
                && step.arc_length_constraint_error < 1.0e-7
        }));
        let terminal = &branch.continuation_steps.last().unwrap().displacements;
        assert!(midpoint_product(terminal) > 0.0);
        assert!(symmetric_alignment(terminal) > 0.99);
    }
}

#[test]
fn antisymmetric_path_exposes_the_secondary_connected_transition() {
    let result = solve_frame_2d_p_delta(&coupled_euler_request(COLUMN_INERTIA, [1.0, -1.0]))
        .expect("connected antisymmetric path should remain solvable");
    assert!(result.converged, "antisymmetric path: {:?}", result.steps);
    let candidates = result
        .steps
        .iter()
        .filter(|step| !step.branch_switch_probes.is_empty())
        .collect::<Vec<_>>();
    assert_eq!(candidates.len(), 2);
    let euler = std::f64::consts::PI.powi(2) * YOUNGS_MODULUS * COLUMN_INERTIA / LENGTH.powi(2);
    let references = [
        euler,
        euler + 4.0 * COUPLING_STIFFNESS * LENGTH / std::f64::consts::PI.powi(2),
    ];
    for (index, candidate) in candidates.iter().enumerate() {
        assert_eq!(
            candidate.path_event,
            Some(Frame2dEquilibriumPathEvent::BifurcationCandidate)
        );
        assert_eq!(candidate.tangent_negative_pivots, Some(index + 1));
        assert!(
            candidate
                .tangent_critical_eigenvalue
                .is_some_and(|value| value.abs() < 1.0e-8)
        );
        assert_eq!(candidate.branch_switch_probes.len(), 2);
        for branch in &candidate.branch_switch_probes {
            assert!(branch.distinct_branch);
            assert_relative(branch.load_factor.unwrap(), references[index], 3.0e-2);
            assert_eq!(branch.continuation_converged, Some(true));
            assert_eq!(branch.continuation_steps.len(), 4);
            assert!(branch.continuation_steps.iter().all(|step| {
                step.converged
                    && step.residual_norm < 1.0e-7
                    && step.arc_length_constraint_error < 1.0e-7
            }));
            let seed = branch.displacements.as_ref().unwrap();
            let terminal = &branch.continuation_steps.last().unwrap().displacements;
            if index == 0 {
                assert!(midpoint_product(seed) > 0.0);
                assert!(symmetric_alignment(seed) > 0.99);
                assert!(symmetric_alignment(terminal) > 0.99);
            } else {
                assert!(midpoint_product(seed) < 0.0);
                assert!(antisymmetric_alignment(seed) > 0.99);
                assert!(antisymmetric_alignment(terminal) > 0.99);
            }
        }
    }
}

#[test]
fn asymmetric_connected_columns_follow_the_external_mixed_mode_reduction() {
    let references = reduced_mode_references(ASYMMETRIC_COLUMN_INERTIA);
    let low_request = coupled_euler_request(ASYMMETRIC_COLUMN_INERTIA, references[0].1);
    let buckling =
        solve_buckling_frame_2d(&low_request.buckling).expect("asymmetric buckling should solve");
    for (mode, (load, direction)) in buckling.modes.iter().take(2).zip(references) {
        assert_relative(mode.load_factor, load, 5.0e-3);
        assert!(midpoint_alignment(&mode.shape, direction) > 0.98);
        assert!(direction[0].abs() > 0.2 && direction[1].abs() > 0.2);
    }

    let low_result =
        solve_frame_2d_p_delta(&low_request).expect("lower mixed branch should remain solvable");
    let low_candidate = low_result
        .steps
        .iter()
        .find(|step| !step.branch_switch_probes.is_empty())
        .expect("lower mixed transition should emit branches");
    assert_mixed_candidate(low_candidate, references[0], 1);

    let mut high_request = coupled_euler_request(ASYMMETRIC_COLUMN_INERTIA, references[1].1);
    high_request.branch_switch_mode_count = Some(2);
    high_request.branch_switch_pairwise_combinations = true;
    let high_result =
        solve_frame_2d_p_delta(&high_request).expect("upper mixed branch should remain solvable");
    let high_candidates = high_result
        .steps
        .iter()
        .filter(|step| !step.branch_switch_probes.is_empty())
        .collect::<Vec<_>>();
    assert_eq!(high_candidates.len(), 2);
    assert_separated_mixed_candidate(high_candidates[1], references);
}

fn assert_mixed_candidate(
    candidate: &kyuubiki_protocol::Frame2dPDeltaStepResult,
    reference: (f64, [f64; 2]),
    negative_pivots: usize,
) {
    assert_eq!(
        candidate.path_event,
        Some(Frame2dEquilibriumPathEvent::BifurcationCandidate)
    );
    assert_eq!(candidate.tangent_negative_pivots, Some(negative_pivots));
    assert_eq!(candidate.branch_switch_probes.len(), 2);
    for branch in &candidate.branch_switch_probes {
        assert!(branch.distinct_branch);
        assert_relative(branch.load_factor.unwrap(), reference.0, 3.0e-2);
        assert!(midpoint_alignment(branch.displacements.as_ref().unwrap(), reference.1) > 0.98);
        assert_eq!(branch.continuation_converged, Some(true));
        assert_eq!(branch.continuation_steps.len(), 4);
        assert!(branch.continuation_steps.iter().all(|step| {
            step.converged
                && step.residual_norm < 1.0e-7
                && step.arc_length_constraint_error < 1.0e-7
        }));
        assert!(
            midpoint_alignment(
                &branch.continuation_steps.last().unwrap().displacements,
                reference.1
            ) > 0.98
        );
    }
}

fn assert_separated_mixed_candidate(
    candidate: &kyuubiki_protocol::Frame2dPDeltaStepResult,
    references: [(f64, [f64; 2]); 2],
) {
    assert_eq!(
        candidate.path_event,
        Some(Frame2dEquilibriumPathEvent::BifurcationCandidate)
    );
    assert_eq!(candidate.tangent_negative_pivots, Some(2));
    assert_eq!(candidate.tangent_critical_modes.len(), 2);
    assert_eq!(candidate.branch_switch_probes.len(), 8);
    assert_eq!(
        candidate
            .branch_switch_probes
            .iter()
            .filter(|probe| probe.origin == Frame2dBranchProbeOrigin::CriticalMode)
            .count(),
        4
    );
    assert_eq!(
        candidate
            .branch_switch_probes
            .iter()
            .filter(|probe| probe.origin == Frame2dBranchProbeOrigin::PairwiseCombination)
            .count(),
        4
    );

    for (mode_index, mode) in candidate.tangent_critical_modes.iter().enumerate() {
        let direction = midpoint_direction(&mode.shape);
        assert!(direction[0].abs() > 0.2 && direction[1].abs() > 0.2);
        for probe in candidate.branch_switch_probes.iter().filter(|probe| {
            probe.origin == Frame2dBranchProbeOrigin::CriticalMode && probe.mode_index == mode_index
        }) {
            assert!(midpoint_alignment(probe.displacements.as_ref().unwrap(), direction) > 0.98);
        }
    }
    for probe in candidate.branch_switch_probes.iter().filter(|probe| {
        probe.origin == Frame2dBranchProbeOrigin::CriticalMode && probe.mode_index == 0
    }) {
        assert_relative(probe.load_factor.unwrap(), references[1].0, 3.0e-2);
    }

    for branch in candidate
        .branch_switch_probes
        .iter()
        .filter(|probe| probe.origin == Frame2dBranchProbeOrigin::CriticalMode)
    {
        assert!(branch.distinct_branch);
        assert_eq!(branch.continuation_converged, Some(true));
        assert_eq!(branch.continuation_steps.len(), 4);
        assert!(branch.continuation_steps.iter().all(|step| {
            step.converged
                && step.residual_norm < 1.0e-7
                && step.arc_length_constraint_error < 1.0e-7
        }));
    }
    for rejected in candidate
        .branch_switch_probes
        .iter()
        .filter(|probe| probe.origin == Frame2dBranchProbeOrigin::PairwiseCombination)
    {
        assert!(!rejected.equilibrium_converged);
        assert!(!rejected.distinct_branch);
        assert!(rejected.displacements.is_none());
        assert_eq!(rejected.continuation_converged, Some(false));
        assert!(
            rejected
                .failure_detail
                .as_deref()
                .is_some_and(|detail| detail.contains("degenerate critical eigenspace"))
        );
    }
}

fn reduced_mode_references(right_inertia: f64) -> [(f64, [f64; 2]); 2] {
    let left = std::f64::consts::PI.powi(2) * YOUNGS_MODULUS * COLUMN_INERTIA / LENGTH.powi(2);
    let right = std::f64::consts::PI.powi(2) * YOUNGS_MODULUS * right_inertia / LENGTH.powi(2);
    let coupling = 2.0 * COUPLING_STIFFNESS * LENGTH / std::f64::consts::PI.powi(2);
    let center = (left + right + 2.0 * coupling) * 0.5;
    let radius = (((left - right) * 0.5).powi(2) + coupling.powi(2)).sqrt();
    let loads = [center - radius, center + radius];
    loads.map(|load| {
        let ratio = (left + coupling - load) / coupling;
        let norm = 1.0_f64.hypot(ratio);
        (load, [1.0 / norm, ratio / norm])
    })
}

fn coupled_euler_request(
    right_inertia: f64,
    imperfection_weights: [f64; 2],
) -> SolveFrame2dPDeltaRequest {
    let mut nodes = Vec::with_capacity(NODES_PER_COLUMN * 2);
    let mut elements = Vec::with_capacity(ELEMENT_COUNT * 2 + 1);
    let mut imperfection_shape = Vec::with_capacity(NODES_PER_COLUMN * 2 * 3);
    for (column, imperfection_weight) in imperfection_weights.into_iter().enumerate() {
        let node_offset = nodes.len();
        nodes.extend((0..=ELEMENT_COUNT).map(|index| {
            let ratio = index as f64 / ELEMENT_COUNT as f64;
            imperfection_shape.extend([
                imperfection_weight * (std::f64::consts::PI * ratio).sin(),
                0.0,
                0.0,
            ]);
            Frame2dNodeInput {
                id: format!("coupled-column-{column}-node-{index}"),
                x: column as f64 * COLUMN_SPACING,
                y: LENGTH * ratio,
                fix_x: index == 0 || index == ELEMENT_COUNT,
                fix_y: index == 0,
                fix_rz: false,
                load_x: 0.0,
                load_y: if index == ELEMENT_COUNT { -1.0 } else { 0.0 },
                moment_z: 0.0,
            }
        }));
        let moment_of_inertia = if column == 0 {
            COLUMN_INERTIA
        } else {
            right_inertia
        };
        elements.extend((0..ELEMENT_COUNT).map(|index| Frame2dElementInput {
            id: format!("coupled-column-{column}-member-{index}"),
            node_i: node_offset + index,
            node_j: node_offset + index + 1,
            area: COLUMN_AREA,
            youngs_modulus: YOUNGS_MODULUS,
            moment_of_inertia,
            section_modulus: 1.0,
        }));
    }
    elements.push(Frame2dElementInput {
        id: "midpoint-coupler".into(),
        node_i: midpoint_node(0),
        node_j: midpoint_node(1),
        area: COUPLING_STIFFNESS * COLUMN_SPACING / YOUNGS_MODULUS,
        youngs_modulus: YOUNGS_MODULUS,
        moment_of_inertia: 1.0e-12,
        section_modulus: 1.0,
    });
    let euler = std::f64::consts::PI.powi(2) * YOUNGS_MODULUS * COLUMN_INERTIA / LENGTH.powi(2);

    SolveFrame2dPDeltaRequest {
        buckling: SolveBucklingFrame2dRequest {
            frame: SolveFrame2dRequest { nodes, elements },
            mode_count: Some(4),
        },
        imperfection_amplitude: 1.0e-9,
        kinematics: Frame2dStabilityKinematics::Corotational,
        path_control: Frame2dStabilityPathControl::ArcLength,
        imperfection_shape: Some(imperfection_shape),
        imperfection_mode_index: None,
        maximum_load_factor: Some(euler * 1.5),
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
        branch_continuation_steps: Some(4),
        branch_continuation_radius: None,
        branch_continuation_min_radius_ratio: None,
    }
}

fn midpoint_node(column: usize) -> usize {
    column * NODES_PER_COLUMN + ELEMENT_COUNT / 2
}

fn midpoint_product(displacements: &[f64]) -> f64 {
    displacements[midpoint_node(0) * 3] * displacements[midpoint_node(1) * 3]
}

fn symmetric_alignment(displacements: &[f64]) -> f64 {
    let left = displacements[midpoint_node(0) * 3];
    let right = displacements[midpoint_node(1) * 3];
    (left + right).abs() / (2.0_f64.sqrt() * left.hypot(right))
}

fn antisymmetric_alignment(displacements: &[f64]) -> f64 {
    let left = displacements[midpoint_node(0) * 3];
    let right = displacements[midpoint_node(1) * 3];
    (left - right).abs() / (2.0_f64.sqrt() * left.hypot(right))
}

fn midpoint_alignment(displacements: &[f64], direction: [f64; 2]) -> f64 {
    let actual = midpoint_direction(displacements);
    if actual == [0.0, 0.0] {
        return 0.0;
    }
    (actual[0] * direction[0] + actual[1] * direction[1]).abs()
}

fn midpoint_direction(displacements: &[f64]) -> [f64; 2] {
    let left = displacements[midpoint_node(0) * 3];
    let right = displacements[midpoint_node(1) * 3];
    let norm = left.hypot(right);
    if norm <= f64::EPSILON {
        [0.0, 0.0]
    } else {
        [left / norm, right / norm]
    }
}

fn assert_relative(actual: f64, expected: f64, tolerance: f64) {
    let relative = (actual - expected).abs() / expected.abs().max(1.0);
    assert!(
        relative <= tolerance,
        "actual={actual}, expected={expected}, relative={relative}"
    );
}
