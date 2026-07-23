use kyuubiki_protocol::{
    Frame2dBranchDirection, Frame2dBranchSwitchSelection, Frame2dElementInput,
    Frame2dEquilibriumPathEvent, Frame2dNodeInput, Frame2dStabilityKinematics,
    Frame2dStabilityPathControl, SolveBucklingFrame2dRequest, SolveFrame2dPDeltaRequest,
    SolveFrame2dRequest,
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
fn explicit_branch_radius_explores_a_deeper_equilibrium_segment() {
    let mut request = configured_switched_arch(0.0, 64);
    request.branch_continuation_radius = Some(0.02);
    request.branch_continuation_min_radius_ratio = Some(0.25);
    let result =
        solve_frame_2d_p_delta(&request).expect("explicit branch continuation radius should solve");
    let branches = branch_candidate(&result);
    assert_eq!(branches.len(), 2);
    for branch in branches {
        assert_eq!(branch.continuation_converged, Some(true));
        assert_eq!(branch.continuation_steps.len(), 64);
        assert_relative(
            branch.continuation_steps[0].arc_length_radius,
            0.02,
            1.0e-12,
        );
        assert!(branch.continuation_steps.iter().all(|step| {
            step.converged
                && step.failure_reason.is_none()
                && step.residual_norm < 1.0e-7
                && step.arc_length_constraint_error < 1.0e-7
                && (0.005..=0.02).contains(&step.arc_length_radius)
        }));
        let minimum = branch
            .continuation_steps
            .iter()
            .min_by(|left, right| left.load_factor.total_cmp(&right.load_factor))
            .unwrap();
        assert!(minimum.load_factor < -1.4);
        assert!(branch.continuation_steps.last().unwrap().load_factor > minimum.load_factor + 2.0);
        let events = branch
            .continuation_steps
            .iter()
            .filter_map(|step| step.path_event.map(|event| (step, event)))
            .collect::<Vec<_>>();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].1, Frame2dEquilibriumPathEvent::LimitPointMinimum);
        assert_relative(events[0].0.load_factor, minimum.load_factor, 1.0e-12);
    }
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

#[test]
fn branched_arch_continuation_is_objective_and_contains_local_seed_failures() {
    let angle = 0.417;
    let baseline = solve_branched_arch(0.0);
    let rotated = solve_branched_arch(angle);
    assert_relative(
        rotated.buckling_result.minimum_load_factor,
        baseline.buckling_result.minimum_load_factor,
        3.0e-9,
    );
    assert!(
        baseline
            .steps
            .iter()
            .filter(|step| !step.branch_switch_probes.is_empty())
            .count()
            >= 2
    );
    let baseline_candidate = fully_switched_candidate(&baseline);
    let rotated_candidate = fully_switched_candidate(&rotated);
    assert_eq!(baseline_candidate.len(), 2);
    assert_eq!(rotated_candidate.len(), 2);

    for baseline_branch in baseline_candidate {
        assert_eq!(baseline_branch.continuation_converged, Some(true));
        assert_eq!(baseline_branch.continuation_steps.len(), 16);
        assert!(baseline_branch.continuation_steps.iter().all(|step| {
            step.converged
                && step.failure_reason.is_none()
                && step.residual_norm < 1.0e-7
                && step.arc_length_constraint_error < 1.0e-7
        }));
        assert_eq!(
            baseline_branch.continuation_steps[0].displacements.len(),
            30
        );
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
            ) < 3.0e-8
        );
        for (baseline_step, rotated_step) in baseline_branch
            .continuation_steps
            .iter()
            .zip(&rotated_branch.continuation_steps)
        {
            assert_relative(rotated_step.load_factor, baseline_step.load_factor, 3.0e-9);
            assert_eq!(rotated_step.path_event, baseline_step.path_event);
            assert_eq!(
                rotated_step.tangent_negative_pivots,
                baseline_step.tangent_negative_pivots
            );
            assert!(
                distance(
                    &rotated_step.displacements,
                    &rotate_displacements(&baseline_step.displacements, angle)
                ) < 3.0e-8
            );
        }
    }

    let isolated = baseline
        .steps
        .iter()
        .flat_map(|step| &step.branch_switch_probes)
        .find(|branch| !branch.distinct_branch)
        .expect("secondary transition should retain its non-distinct seed");
    assert_eq!(isolated.continuation_converged, Some(false));
    assert!(isolated.continuation_steps.is_empty());
    assert!(
        isolated
            .continuation_failure_detail
            .as_deref()
            .unwrap()
            .contains("distinct equilibrium seed")
    );
    assert!(baseline.steps.last().unwrap().converged);
}

#[test]
fn twin_arch_repeated_modes_drive_individual_and_pairwise_branch_families() {
    let mut request = configured_switched_request(repeated_arch_request(2), 4);
    request.branch_switch_mode_count = Some(2);
    request.branch_switch_pairwise_combinations = true;
    let result = solve_frame_2d_p_delta(&request).expect("twin-arch path should remain available");
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
        .expect("repeated transition should expose individual and pairwise branch families");
    let first = &candidate.tangent_critical_modes[0];
    let second = &candidate.tangent_critical_modes[1];
    assert_eq!(first.mode_index, 0);
    assert_eq!(second.mode_index, 1);
    assert_relative(
        first.normalized_eigenvalue,
        second.normalized_eigenvalue,
        1.0e-12,
    );
    assert!(first.normalized_residual < 1.0e-8);
    assert!(second.normalized_residual < 1.0e-8);
    let modal_dot = first
        .shape
        .iter()
        .zip(&second.shape)
        .map(|(left, right)| left * right)
        .sum::<f64>();
    assert!(modal_dot.abs() < 1.0e-10);
    assert_eq!(
        candidate.tangent_critical_eigenvalue,
        Some(first.normalized_eigenvalue)
    );
    assert_eq!(candidate.tangent_critical_mode.as_ref(), Some(&first.shape));

    for (index, probe) in candidate.branch_switch_probes[..4].iter().enumerate() {
        assert_eq!(probe.mode_index, index / 2);
        assert_eq!(
            probe.direction,
            if index % 2 == 0 {
                Frame2dBranchDirection::Positive
            } else {
                Frame2dBranchDirection::Negative
            }
        );
        assert_eq!(
            probe.mode_eigenvalue,
            Some(candidate.tangent_critical_modes[index / 2].normalized_eigenvalue)
        );
        assert_eq!(probe.mode_components.len(), 1);
        assert_eq!(probe.mode_components[0].mode_index, index / 2);
        assert_relative(probe.mode_components[0].weight, 1.0, 1.0e-12);
        assert_eq!(probe.mode_component_projections.len(), 1);
        assert_relative(
            probe.mode_component_projections[0],
            probe.mode_projection.unwrap(),
            1.0e-10,
        );
    }
    for (index, probe) in candidate.branch_switch_probes[4..].iter().enumerate() {
        assert_eq!(probe.mode_index, 0);
        assert_eq!(probe.mode_eigenvalue, None);
        assert_eq!(probe.mode_components.len(), 2);
        assert_eq!(probe.mode_components[0].mode_index, 0);
        assert_eq!(probe.mode_components[1].mode_index, 1);
        assert_eq!(probe.mode_component_projections.len(), 2);
        assert_relative(
            probe.mode_components[0].weight,
            std::f64::consts::FRAC_1_SQRT_2,
            1.0e-12,
        );
        let expected_right_sign = if index < 2 { 1.0 } else { -1.0 };
        assert_relative(
            probe.mode_components[1].weight,
            expected_right_sign * std::f64::consts::FRAC_1_SQRT_2,
            1.0e-12,
        );
        let direction_sign = if probe.direction == Frame2dBranchDirection::Positive {
            1.0
        } else {
            -1.0
        };
        for (component, projection) in probe
            .mode_components
            .iter()
            .zip(&probe.mode_component_projections)
        {
            assert_relative(
                *projection,
                direction_sign * probe.seed_amplitude * component.weight,
                1.0e-8,
            );
        }
    }
    for probe in &candidate.branch_switch_probes {
        assert_eq!(probe.continuation_converged, Some(true));
        assert_eq!(probe.continuation_steps.len(), 4);
        assert!(probe.continuation_steps.iter().all(|step| {
            step.converged
                && step.residual_norm < 1.0e-7
                && step.arc_length_constraint_error < 1.0e-7
        }));
    }
    let first_terminal = &candidate.branch_switch_probes[0]
        .continuation_steps
        .last()
        .unwrap()
        .displacements;
    let second_terminal = &candidate.branch_switch_probes[2]
        .continuation_steps
        .last()
        .unwrap()
        .displacements;
    assert!(distance(first_terminal, second_terminal) > 5.0e-3);
}

#[test]
fn triple_arch_repeated_modes_drive_an_arbitrary_weighted_branch_family() {
    let mut request = configured_switched_request(repeated_arch_request(3), 2);
    request.branch_switch_mode_count = Some(3);
    request.branch_switch_mode_weights = Some(vec![1.0, 2.0, -2.0]);
    let result =
        solve_frame_2d_p_delta(&request).expect("triple-arch path should remain available");
    let candidate = result
        .steps
        .iter()
        .find(|step| {
            step.tangent_critical_modes.len() == 3
                && step.branch_switch_probes.len() == 8
                && step
                    .branch_switch_probes
                    .iter()
                    .skip(6)
                    .all(|probe| probe.distinct_branch)
        })
        .expect("repeated transition should expose a three-mode weighted branch family");
    for probe in &candidate.branch_switch_probes[6..] {
        assert_eq!(probe.mode_index, 0);
        assert_eq!(probe.mode_eigenvalue, None);
        assert_eq!(probe.mode_components.len(), 3);
        assert_eq!(probe.mode_component_projections.len(), 3);
        let direction = if probe.direction == Frame2dBranchDirection::Positive {
            1.0
        } else {
            -1.0
        };
        for (component, expected_weight) in
            probe
                .mode_components
                .iter()
                .zip([1.0 / 3.0, 2.0 / 3.0, -2.0 / 3.0])
        {
            assert_relative(component.weight, expected_weight, 1.0e-12);
        }
        let combined_projection = probe
            .mode_components
            .iter()
            .zip(&probe.mode_component_projections)
            .map(|(component, projection)| component.weight * projection)
            .sum::<f64>();
        assert_relative(combined_projection, probe.mode_projection.unwrap(), 1.0e-10);
        assert_relative(
            probe.mode_projection.unwrap(),
            direction * probe.seed_amplitude,
            1.0e-8,
        );
        assert_eq!(probe.continuation_converged, Some(true));
        assert_eq!(probe.continuation_steps.len(), 2);
    }
}

#[test]
fn triple_arch_repeated_modes_drive_the_bounded_automatic_subspace_fan() {
    let mut request = configured_switched_request(repeated_arch_request(3), 2);
    request.branch_switch_mode_count = Some(3);
    request.branch_switch_subspace_sample_count = Some(4);
    let result = solve_frame_2d_p_delta(&request).expect("subspace fan should remain available");
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
        .expect("three-mode transition should expose four signed direction families");
    let fan = &candidate.branch_switch_probes[6..];
    for family in fan.chunks_exact(2) {
        assert_eq!(family[0].direction, Frame2dBranchDirection::Positive);
        assert_eq!(family[1].direction, Frame2dBranchDirection::Negative);
        assert_eq!(family[0].mode_components.len(), 3);
        assert_eq!(family[0].mode_component_projections.len(), 3);
        assert!(family[0].mode_components[0].weight > 0.0);
        for probe in family {
            let combined_projection = probe
                .mode_components
                .iter()
                .zip(&probe.mode_component_projections)
                .map(|(component, projection)| component.weight * projection)
                .sum::<f64>();
            assert_relative(combined_projection, probe.mode_projection.unwrap(), 1.0e-10);
            assert_eq!(probe.continuation_converged, Some(true));
            assert_eq!(probe.continuation_steps.len(), 2);
        }
    }
    assert!(fan.chunks_exact(2).enumerate().all(|(index, family)| {
        fan.chunks_exact(2).skip(index + 1).all(|other| {
            family[0]
                .mode_components
                .iter()
                .zip(&other[0].mode_components)
                .map(|(left, right)| left.weight * right.weight)
                .sum::<f64>()
                .abs()
                < 1.0 - 1.0e-12
        })
    }));
}

fn solve_switched_arch(
    angle: f64,
    continuation_steps: usize,
) -> kyuubiki_protocol::SolveFrame2dPDeltaResult {
    let request = configured_switched_arch(angle, continuation_steps);
    solve_frame_2d_p_delta(&request).expect("rotated switched branches should solve")
}

fn configured_switched_arch(angle: f64, continuation_steps: usize) -> SolveFrame2dPDeltaRequest {
    configured_switched_request(shallow_arch_request(4, angle), continuation_steps)
}

fn solve_branched_arch(angle: f64) -> kyuubiki_protocol::SolveFrame2dPDeltaResult {
    let mut request = shallow_arch_request(4, angle);
    let branch_tip = request.buckling.frame.nodes.len();
    let (x, y) = rotate(0.0, 0.3, angle);
    let (shape_x, shape_y) = rotate(0.0, -1.0, angle);
    request.buckling.frame.nodes.push(Frame2dNodeInput {
        id: "arch-branch-tip".into(),
        x,
        y,
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
        .extend([shape_x, shape_y, 0.0]);
    request.buckling.frame.elements.push(Frame2dElementInput {
        id: "arch-branch".into(),
        node_i: 4,
        node_j: branch_tip,
        area: 1.0e-3,
        youngs_modulus: 210.0e9,
        moment_of_inertia: 1.0e-8,
        section_modulus: 1.0e-6,
    });
    let mut request = configured_switched_request(request, 16);
    request.branch_continuation_radius = Some(0.02);
    solve_frame_2d_p_delta(&request).expect("branched arch should preserve its primary path")
}

fn configured_switched_request(
    mut request: SolveFrame2dPDeltaRequest,
    continuation_steps: usize,
) -> SolveFrame2dPDeltaRequest {
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
    request
}

fn repeated_arch_request(copy_count: usize) -> SolveFrame2dPDeltaRequest {
    let mut request = shallow_arch_request(4, 0.0);
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

fn fully_switched_candidate(
    result: &kyuubiki_protocol::SolveFrame2dPDeltaResult,
) -> &[kyuubiki_protocol::Frame2dBranchSwitchProbeResult] {
    &result
        .steps
        .iter()
        .find(|step| {
            step.branch_switch_probes.len() == 2
                && step.branch_switch_probes.iter().all(|branch| {
                    branch.distinct_branch && branch.continuation_converged == Some(true)
                })
        })
        .expect("complex topology should retain one fully switched transition")
        .branch_switch_probes
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
        branch_switch_mode_count: None,
        branch_switch_pairwise_combinations: false,
        branch_switch_mode_weights: None,
        branch_switch_subspace_sample_count: None,
        branch_continuation_steps: None,
        branch_continuation_radius: None,
        branch_continuation_min_radius_ratio: None,
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
