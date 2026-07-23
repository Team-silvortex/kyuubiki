use super::*;

#[test]
fn connected_triplet_recovers_a_repeated_mixed_critical_subspace() {
    let request = coupled_column_network_request(
        &[COLUMN_INERTIA; 3],
        &[1.0, -1.0, 0.0],
        &[
            (0, 1, COUPLING_STIFFNESS),
            (1, 2, COUPLING_STIFFNESS),
            (0, 2, COUPLING_STIFFNESS),
        ],
    );
    let buckling =
        solve_buckling_frame_2d(&request.buckling).expect("connected triplet should solve");
    let euler = std::f64::consts::PI.powi(2) * YOUNGS_MODULUS * COLUMN_INERTIA / LENGTH.powi(2);
    let repeated = euler + 6.0 * COUPLING_STIFFNESS * LENGTH / std::f64::consts::PI.powi(2);

    assert_relative(buckling.modes[0].load_factor, euler, 2.0e-3);
    assert_relative(buckling.modes[1].load_factor, repeated, 5.0e-3);
    assert_relative(buckling.modes[2].load_factor, repeated, 5.0e-3);
    assert_relative(
        buckling.modes[1].load_factor,
        buckling.modes[2].load_factor,
        1.0e-9,
    );
    let symmetric = triplet_midpoint_direction(&buckling.modes[0].shape);
    assert!(symmetric.iter().sum::<f64>().abs() > 1.7);
    assert!(symmetric.iter().all(|value| value.abs() > 0.5));
    for mode in &buckling.modes[1..=2] {
        let direction = triplet_midpoint_direction(&mode.shape);
        assert!(direction.iter().filter(|value| value.abs() > 0.2).count() >= 2);
        let sum = direction.iter().sum::<f64>().abs();
        assert!(sum < 1.0e-4, "mixed direction={direction:?}, sum={sum}");
    }
    let mixed_dot = triplet_midpoint_dot(&buckling.modes[1].shape, &buckling.modes[2].shape).abs();
    assert!(mixed_dot < 1.0e-4, "mixed midpoint dot={mixed_dot}");

    let mut nonlinear = request;
    nonlinear.branch_switch_mode_count = Some(2);
    nonlinear.branch_switch_pairwise_combinations = true;
    nonlinear.branch_continuation_steps = Some(2);
    let result = solve_frame_2d_p_delta(&nonlinear)
        .expect("connected triplet repeated transition should remain solvable");
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
        .expect("connected triplet should expose eight repeated-subspace probes");
    assert_relative(candidate.load_factor, repeated, 4.0e-2);
    assert!(candidate.tangent_critical_modes.iter().all(|mode| {
        mode.normalized_eigenvalue.abs() < 1.0e-8 && mode.normalized_residual < 1.0e-8
    }));
    assert!(
        (candidate.tangent_critical_modes[0].normalized_eigenvalue
            - candidate.tangent_critical_modes[1].normalized_eigenvalue)
            .abs()
            < 1.0e-8
    );
    for mode in &candidate.tangent_critical_modes {
        let direction = triplet_midpoint_direction(&mode.shape);
        assert!(direction.iter().filter(|value| value.abs() > 0.2).count() >= 2);
    }
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
    for probe in &candidate.branch_switch_probes {
        assert_eq!(probe.continuation_converged, Some(true));
        assert_eq!(probe.continuation_steps.len(), 2);
        assert!(probe.continuation_steps.iter().all(|step| {
            step.converged
                && step.residual_norm < 1.0e-7
                && step.arc_length_constraint_error < 1.0e-7
        }));
    }
}

#[test]
fn connected_triplet_two_parameter_unfolding_tracks_the_reduced_spectrum() {
    let cases = [
        (0.0, 0.0),
        (0.005, 0.0),
        (0.0, 0.5),
        (0.005, 0.5),
        (-0.005, -0.5),
    ];
    for (inertia_skew, coupling_skew) in cases {
        let inertias = [
            COLUMN_INERTIA,
            COLUMN_INERTIA,
            COLUMN_INERTIA * (1.0 + inertia_skew),
        ];
        let couplers = [
            (0, 1, COUPLING_STIFFNESS * (1.0 + coupling_skew)),
            (1, 2, COUPLING_STIFFNESS),
            (0, 2, COUPLING_STIFFNESS),
        ];
        let request = coupled_column_network_request(&inertias, &[1.0, -1.0, 0.0], &couplers);
        let buckling =
            solve_buckling_frame_2d(&request.buckling).expect("unfolded triplet should solve");
        let reference = reduced_triplet_eigenvalues(&inertias, &couplers);
        for (mode, expected) in buckling.modes.iter().take(3).zip(reference) {
            assert_relative(mode.load_factor, expected, 5.0e-3);
        }

        let actual_split = (buckling.modes[2].load_factor - buckling.modes[1].load_factor).abs();
        let reference_split = (reference[2] - reference[1]).abs();
        assert_relative(actual_split, reference_split, 5.0e-3);
        if inertia_skew == 0.0 && coupling_skew == 0.0 {
            assert!(actual_split / reference[1] < 1.0e-8);
        } else {
            assert!(actual_split / reference[1] > 1.0e-4);
        }
    }
}

#[test]
fn connected_triplet_parameter_path_retains_the_nonlinear_mixed_branch_identity() {
    let path = [
        (-0.005, 0.0),
        (-0.003_535, 0.353_55),
        (0.0, 0.5),
        (0.003_535, 0.353_55),
        (0.005, 0.0),
    ];
    let mut previous_reference: Option<[f64; 3]> = None;
    let mut previous_critical: Option<[f64; 3]> = None;
    for (inertia_skew, coupling_skew) in path {
        let inertias = [
            COLUMN_INERTIA,
            COLUMN_INERTIA,
            COLUMN_INERTIA * (1.0 + inertia_skew),
        ];
        let couplers = [
            (0, 1, COUPLING_STIFFNESS),
            (1, 2, COUPLING_STIFFNESS),
            (0, 2, COUPLING_STIFFNESS * (1.0 + coupling_skew)),
        ];
        let eigenpairs = reduced_triplet_eigenpairs(&inertias, &couplers);
        let reference = if let Some(previous) = previous_reference {
            eigenpairs[1..]
                .iter()
                .copied()
                .max_by(|left, right| {
                    direction_dot(previous, left.1)
                        .abs()
                        .total_cmp(&direction_dot(previous, right.1).abs())
                })
                .unwrap()
        } else {
            eigenpairs[2]
        };
        let mut request = coupled_column_network_request(&inertias, &reference.1, &couplers);
        request.branch_continuation_steps = Some(2);
        let result =
            solve_frame_2d_p_delta(&request).expect("unfolded mixed branch should remain solvable");
        let candidate = result
            .steps
            .iter()
            .filter(|step| {
                step.branch_switch_probes.len() == 2 && !step.tangent_critical_modes.is_empty()
            })
            .max_by(|left, right| {
                triplet_alignment(&left.tangent_critical_modes[0].shape, reference.1).total_cmp(
                    &triplet_alignment(&right.tangent_critical_modes[0].shape, reference.1),
                )
            })
            .expect("unfolded parameter point should expose a signed branch pair");

        assert_relative(candidate.load_factor, reference.0, 4.0e-2);
        let critical_direction =
            triplet_midpoint_direction(&candidate.tangent_critical_modes[0].shape);
        let reference_alignment =
            triplet_alignment(&candidate.tangent_critical_modes[0].shape, reference.1);
        assert!(
            reference_alignment > 0.9,
            "reference alignment={reference_alignment} at inertia_skew={inertia_skew}, coupling_skew={coupling_skew}, reference={:?}, critical={critical_direction:?}",
            reference.1
        );
        if let Some(previous) = previous_reference {
            let overlap = direction_dot(previous, reference.1).abs();
            assert!(
                overlap > 0.75,
                "reference overlap={overlap} at inertia_skew={inertia_skew}, coupling_skew={coupling_skew}"
            );
        }
        if let Some(previous) = previous_critical {
            let overlap = direction_dot(previous, critical_direction).abs();
            assert!(
                overlap > 0.75,
                "critical overlap={overlap} at inertia_skew={inertia_skew}, coupling_skew={coupling_skew}"
            );
        }
        for probe in &candidate.branch_switch_probes {
            assert!(probe.distinct_branch);
            let displacement = probe.displacements.as_ref().unwrap();
            let alignment = triplet_alignment(displacement, critical_direction);
            assert!(
                alignment > 0.98,
                "inertia_skew={inertia_skew}, coupling_skew={coupling_skew}, load={}, reference_load={}, alignment={alignment}, actual={:?}, critical={critical_direction:?}",
                candidate.load_factor,
                reference.0,
                triplet_midpoint_direction(displacement),
            );
            assert_eq!(probe.continuation_converged, Some(true));
            assert_eq!(probe.continuation_steps.len(), 2);
            assert!(probe.continuation_steps.iter().all(|step| {
                step.converged
                    && step.residual_norm < 1.0e-7
                    && step.arc_length_constraint_error < 1.0e-7
            }));
        }
        previous_reference = Some(reference.1);
        previous_critical = Some(critical_direction);
    }
}

#[test]
fn connected_triplet_state_seed_crosses_the_repeated_parameter_point() {
    let source_inertias = [
        COLUMN_INERTIA,
        COLUMN_INERTIA,
        COLUMN_INERTIA * (1.0 - 0.005),
    ];
    let symmetric_couplers = [
        (0, 1, COUPLING_STIFFNESS),
        (1, 2, COUPLING_STIFFNESS),
        (0, 2, COUPLING_STIFFNESS),
    ];
    let source_reference = reduced_triplet_eigenpairs(&source_inertias, &symmetric_couplers)[2].1;
    let mut source =
        coupled_column_network_request(&source_inertias, &source_reference, &symmetric_couplers);
    source.branch_continuation_steps = Some(3);
    let source_result = solve_frame_2d_p_delta(&source).expect("source mixed branch should solve");
    let source_probe = source_result
        .steps
        .iter()
        .flat_map(|step| &step.branch_switch_probes)
        .filter(|probe| probe.continuation_converged == Some(true))
        .max_by(|left, right| {
            triplet_alignment(left.displacements.as_ref().unwrap(), source_reference).total_cmp(
                &triplet_alignment(right.displacements.as_ref().unwrap(), source_reference),
            )
        })
        .expect("source point should expose a continued mixed branch");
    let source_state = state_from_probe(source_probe);
    let radius = source_probe
        .continuation_steps
        .last()
        .unwrap()
        .arc_length_radius;

    let mut repeated = coupled_column_network_request(
        &[COLUMN_INERTIA; 3],
        &source_reference,
        &symmetric_couplers,
    );
    configure_state_seeded_run(&mut repeated, source_state, radius);
    let repeated_result =
        solve_frame_2d_p_delta(&repeated).expect("state should cross the repeated parameter point");
    assert!(repeated_result.converged);
    assert!(repeated_result.steps.iter().all(|step| {
        step.converged
            && step.residual_norm < 1.0e-7
            && step.arc_length_constraint_error.unwrap() < 1.0e-7
    }));
    assert!(
        repeated_result
            .continuation_state_correction_norm
            .is_some_and(|norm| norm.is_finite() && norm >= 0.0)
    );
    assert!(
        triplet_alignment(&repeated_result.final_displacements, source_reference) > 0.9,
        "repeated point lost the incoming physical branch"
    );

    let target_inertias = [
        COLUMN_INERTIA,
        COLUMN_INERTIA,
        COLUMN_INERTIA * (1.0 + 0.005),
    ];
    let target_reference = reduced_triplet_eigenpairs(&target_inertias, &symmetric_couplers)[1..]
        .iter()
        .copied()
        .max_by(|left, right| {
            direction_dot(source_reference, left.1)
                .abs()
                .total_cmp(&direction_dot(source_reference, right.1).abs())
        })
        .unwrap()
        .1;
    let mut target =
        coupled_column_network_request(&target_inertias, &target_reference, &symmetric_couplers);
    configure_state_seeded_run(
        &mut target,
        repeated_result.continuation_state.unwrap(),
        radius,
    );
    let target_result =
        solve_frame_2d_p_delta(&target).expect("state should continue after mode-order exchange");
    assert!(target_result.converged);
    assert!(target_result.steps.iter().all(|step| {
        step.converged
            && step.residual_norm < 1.0e-7
            && step.arc_length_constraint_error.unwrap() < 1.0e-7
    }));
    assert!(
        triplet_alignment(&target_result.final_displacements, target_reference) > 0.85,
        "state-seeded branch did not retain physical identity after mode-order exchange"
    );
}

fn state_from_probe(
    probe: &kyuubiki_protocol::Frame2dBranchSwitchProbeResult,
) -> Frame2dPDeltaContinuationState {
    let steps = &probe.continuation_steps;
    let last = steps.last().unwrap();
    let previous = &steps[steps.len() - 2];
    Frame2dPDeltaContinuationState {
        displacements: last.displacements.clone(),
        load_factor: last.load_factor,
        displacement_increment: last
            .displacements
            .iter()
            .zip(&previous.displacements)
            .map(|(last, previous)| last - previous)
            .collect(),
        load_factor_increment: last.load_factor - previous.load_factor,
    }
}

fn configure_state_seeded_run(
    request: &mut SolveFrame2dPDeltaRequest,
    state: Frame2dPDeltaContinuationState,
    radius: f64,
) {
    request.load_steps = Some(2);
    request.arc_length_radius = Some(radius);
    request.branch_switch = Frame2dBranchSwitchSelection::Disabled;
    request.branch_switch_amplitude = None;
    request.branch_switch_mode_count = None;
    request.branch_continuation_steps = None;
    request.continuation_state = Some(state);
}

fn reduced_triplet_eigenvalues(
    inertias: &[f64; 3],
    couplers: &[(usize, usize, f64); 3],
) -> [f64; 3] {
    symmetric_eigenvalues_3x3(reduced_triplet_matrix(inertias, couplers))
}

fn reduced_triplet_eigenpairs(
    inertias: &[f64; 3],
    couplers: &[(usize, usize, f64); 3],
) -> [(f64, [f64; 3]); 3] {
    let matrix = reduced_triplet_matrix(inertias, couplers);
    symmetric_eigenvalues_3x3(matrix).map(|value| (value, null_direction(matrix, value)))
}

fn reduced_triplet_matrix(
    inertias: &[f64; 3],
    couplers: &[(usize, usize, f64); 3],
) -> [[f64; 3]; 3] {
    let mut matrix = [[0.0; 3]; 3];
    for (index, inertia) in inertias.iter().enumerate() {
        matrix[index][index] =
            std::f64::consts::PI.powi(2) * YOUNGS_MODULUS * inertia / LENGTH.powi(2);
    }
    for &(left, right, stiffness) in couplers {
        let reduced_stiffness = 2.0 * stiffness * LENGTH / std::f64::consts::PI.powi(2);
        matrix[left][left] += reduced_stiffness;
        matrix[right][right] += reduced_stiffness;
        matrix[left][right] -= reduced_stiffness;
        matrix[right][left] -= reduced_stiffness;
    }
    matrix
}

fn symmetric_eigenvalues_3x3(matrix: [[f64; 3]; 3]) -> [f64; 3] {
    let center = (matrix[0][0] + matrix[1][1] + matrix[2][2]) / 3.0;
    let diagonal_spread = (matrix[0][0] - center).powi(2)
        + (matrix[1][1] - center).powi(2)
        + (matrix[2][2] - center).powi(2);
    let off_diagonal_energy =
        2.0 * (matrix[0][1].powi(2) + matrix[0][2].powi(2) + matrix[1][2].powi(2));
    let scale = ((diagonal_spread + off_diagonal_energy) / 6.0).sqrt();
    if scale <= f64::EPSILON {
        return [center; 3];
    }

    let mut normalized = matrix;
    for (row, values) in normalized.iter_mut().enumerate() {
        for value in values.iter_mut() {
            *value /= scale;
        }
        values[row] -= center / scale;
    }
    let determinant = normalized[0][0]
        * (normalized[1][1] * normalized[2][2] - normalized[1][2] * normalized[2][1])
        - normalized[0][1]
            * (normalized[1][0] * normalized[2][2] - normalized[1][2] * normalized[2][0])
        + normalized[0][2]
            * (normalized[1][0] * normalized[2][1] - normalized[1][1] * normalized[2][0]);
    let phase = (0.5 * determinant).clamp(-1.0, 1.0).acos() / 3.0;
    let largest = center + 2.0 * scale * phase.cos();
    let smallest = center + 2.0 * scale * (phase + 2.0 * std::f64::consts::PI / 3.0).cos();
    let middle = 3.0 * center - smallest - largest;
    [smallest, middle, largest]
}

fn null_direction(matrix: [[f64; 3]; 3], eigenvalue: f64) -> [f64; 3] {
    let mut shifted = matrix;
    for (index, row) in shifted.iter_mut().enumerate() {
        row[index] -= eigenvalue;
    }
    let candidates = [
        cross(shifted[0], shifted[1]),
        cross(shifted[0], shifted[2]),
        cross(shifted[1], shifted[2]),
    ];
    let mut direction = candidates
        .into_iter()
        .max_by(|left, right| squared_norm(*left).total_cmp(&squared_norm(*right)))
        .unwrap();
    let norm = squared_norm(direction).sqrt();
    for value in &mut direction {
        *value /= norm;
    }
    if direction
        .iter()
        .find(|value| value.abs() > 1.0e-12)
        .is_some_and(|value| *value < 0.0)
    {
        direction = direction.map(|value| -value);
    }
    direction
}

fn cross(left: [f64; 3], right: [f64; 3]) -> [f64; 3] {
    [
        left[1] * right[2] - left[2] * right[1],
        left[2] * right[0] - left[0] * right[2],
        left[0] * right[1] - left[1] * right[0],
    ]
}

fn squared_norm(vector: [f64; 3]) -> f64 {
    vector.iter().map(|value| value * value).sum()
}

fn direction_dot(left: [f64; 3], right: [f64; 3]) -> f64 {
    left.iter()
        .zip(right)
        .map(|(left, right)| left * right)
        .sum()
}

fn triplet_midpoint_direction(displacements: &[f64]) -> [f64; 3] {
    let mut values = [0.0; 3];
    for (column, value) in values.iter_mut().enumerate() {
        *value = displacements[midpoint_node(column) * 3];
    }
    let norm = values.iter().map(|value| value * value).sum::<f64>().sqrt();
    values.map(|value| value / norm)
}

fn triplet_midpoint_dot(left: &[f64], right: &[f64]) -> f64 {
    let left = triplet_midpoint_direction(left);
    let right = triplet_midpoint_direction(right);
    left.iter()
        .zip(right)
        .map(|(left, right)| left * right)
        .sum()
}

fn triplet_alignment(displacements: &[f64], direction: [f64; 3]) -> f64 {
    triplet_midpoint_direction(displacements)
        .iter()
        .zip(direction)
        .map(|(left, right)| left * right)
        .sum::<f64>()
        .abs()
}
