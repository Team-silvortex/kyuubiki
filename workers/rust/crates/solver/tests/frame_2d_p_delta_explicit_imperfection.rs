use kyuubiki_protocol::{
    Frame2dBranchDirection, Frame2dBranchSwitchSelection, Frame2dElementInput,
    Frame2dEquilibriumPathEvent, Frame2dImperfectionSource, Frame2dNodeInput,
    Frame2dPDeltaFailureReason, Frame2dStabilityKinematics, Frame2dStabilityPathControl,
    Frame2dTangentStability, SolveBucklingFrame2dRequest, SolveFrame2dPDeltaRequest,
    SolveFrame2dRequest,
};
use kyuubiki_solver::solve_frame_2d_p_delta;

const IMPERFECTION_AMPLITUDE: f64 = 0.012;

#[test]
fn explicit_portal_imperfection_is_objective_under_rigid_rotation() {
    let baseline = solve_frame_2d_p_delta(&portal_request(0.0))
        .expect("baseline imperfect portal should solve");
    let rotated = solve_frame_2d_p_delta(&portal_request(0.731))
        .expect("rotated imperfect portal should solve");

    assert_eq!(
        baseline.imperfection_source,
        Frame2dImperfectionSource::ExplicitShape
    );
    assert_eq!(
        rotated.imperfection_source,
        Frame2dImperfectionSource::ExplicitShape
    );
    assert_relative(
        max_translation(&baseline.initial_imperfection_shape),
        IMPERFECTION_AMPLITUDE,
        1.0e-12,
    );
    assert_relative(
        rotated.buckling_result.minimum_load_factor,
        baseline.buckling_result.minimum_load_factor,
        2.0e-8,
    );
    assert_eq!(rotated.steps.len(), baseline.steps.len());
    for (rotated_step, baseline_step) in rotated.steps.iter().zip(&baseline.steps) {
        assert_relative(
            rotated_step.imperfection_amplification,
            baseline_step.imperfection_amplification,
            2.0e-8,
        );
        assert_relative(
            max_translation(&rotated_step.displacements),
            max_translation(&baseline_step.displacements),
            2.0e-8,
        );
        assert!(rotated_step.residual_norm < 1.0e-8);
    }
}

#[test]
fn rejects_malformed_explicit_imperfection_fields() {
    let mut wrong_length = portal_request(0.0);
    wrong_length.imperfection_shape = Some(vec![0.0; 3]);
    let error = solve_frame_2d_p_delta(&wrong_length)
        .expect_err("wrong explicit imperfection length must fail");
    assert!(error.contains("must contain 12 values"));

    let mut non_finite = portal_request(0.0);
    non_finite.imperfection_shape.as_mut().unwrap()[6] = f64::NAN;
    let error = solve_frame_2d_p_delta(&non_finite)
        .expect_err("non-finite explicit imperfection must fail");
    assert!(error.contains("must contain finite values"));

    let mut zero_shape = portal_request(0.0);
    zero_shape.imperfection_shape = Some(vec![0.0; 12]);
    let error =
        solve_frame_2d_p_delta(&zero_shape).expect_err("zero explicit imperfection must fail");
    assert!(error.contains("has no translational imperfection"));

    let mut invalid_iterations = portal_request(0.0);
    invalid_iterations.max_iterations = Some(0);
    assert!(
        solve_frame_2d_p_delta(&invalid_iterations)
            .expect_err("zero Newton iterations must fail")
            .contains("max_iterations")
    );

    let mut invalid_tolerance = portal_request(0.0);
    invalid_tolerance.tolerance = Some(f64::NAN);
    assert!(
        solve_frame_2d_p_delta(&invalid_tolerance)
            .expect_err("non-finite tolerance must fail")
            .contains("tolerance")
    );

    let mut excessive_cutbacks = portal_request(0.0);
    excessive_cutbacks.max_step_cutbacks = Some(17);
    assert!(
        solve_frame_2d_p_delta(&excessive_cutbacks)
            .expect_err("excessive cutbacks must fail")
            .contains("max_step_cutbacks")
    );

    let mut incompatible_control = portal_request(0.0);
    incompatible_control.path_control = Frame2dStabilityPathControl::ArcLength;
    let error = solve_frame_2d_p_delta(&incompatible_control)
        .expect_err("linearized arc-length control must fail");
    assert!(error.contains("requires corotational"));

    incompatible_control.kinematics = Frame2dStabilityKinematics::Corotational;
    incompatible_control.arc_length_radius = Some(0.0);
    let error = solve_frame_2d_p_delta(&incompatible_control)
        .expect_err("zero arc-length radius must fail");
    assert!(error.contains("arc_length_radius"));

    incompatible_control.arc_length_radius = None;
    incompatible_control.arc_length_load_scale = Some(f64::NAN);
    let error = solve_frame_2d_p_delta(&incompatible_control)
        .expect_err("non-finite arc-length scale must fail");
    assert!(error.contains("arc_length_load_scale"));

    incompatible_control.arc_length_load_scale = None;
    incompatible_control.arc_length_target_iterations = Some(1);
    let error = solve_frame_2d_p_delta(&incompatible_control)
        .expect_err("too-small arc-length iteration target must fail");
    assert!(error.contains("arc_length_target_iterations"));

    incompatible_control.arc_length_target_iterations = Some(8);
    incompatible_control.max_iterations = Some(4);
    let error = solve_frame_2d_p_delta(&incompatible_control)
        .expect_err("arc-length target above Newton limit must fail");
    assert!(error.contains("must not exceed max_iterations"));

    incompatible_control.arc_length_target_iterations = None;
    incompatible_control.max_iterations = None;
    incompatible_control.tangent_transition_refinement_steps = Some(21);
    let error = solve_frame_2d_p_delta(&incompatible_control)
        .expect_err("excessive transition refinement must fail");
    assert!(error.contains("tangent_transition_refinement_steps"));

    incompatible_control.tangent_transition_refinement_steps = None;
    incompatible_control.branch_switch = Frame2dBranchSwitchSelection::Both;
    let error = solve_frame_2d_p_delta(&incompatible_control)
        .expect_err("branch switching without amplitude must fail");
    assert!(error.contains("branch_switch_amplitude"));

    incompatible_control.branch_switch_amplitude = Some(0.0);
    let error = solve_frame_2d_p_delta(&incompatible_control)
        .expect_err("zero branch-switch amplitude must fail");
    assert!(error.contains("branch_switch_amplitude"));
}

#[test]
fn corotational_portal_converges_and_is_objective() {
    let mut baseline_request = portal_request(0.0);
    baseline_request.kinematics = Frame2dStabilityKinematics::Corotational;
    let mut rotated_request = portal_request(0.731);
    rotated_request.kinematics = Frame2dStabilityKinematics::Corotational;

    let baseline = solve_frame_2d_p_delta(&baseline_request)
        .expect("baseline corotational portal should solve");
    let rotated =
        solve_frame_2d_p_delta(&rotated_request).expect("rotated corotational portal should solve");

    assert!(baseline.converged);
    assert!(rotated.converged);
    assert_eq!(
        baseline.kinematics,
        Frame2dStabilityKinematics::Corotational
    );
    assert!(baseline.steps.iter().all(|step| step.converged));
    assert!(baseline.steps.iter().any(|step| step.iterations > 1));
    for (rotated_step, baseline_step) in rotated.steps.iter().zip(&baseline.steps) {
        assert_relative(
            rotated_step.imperfection_amplification,
            baseline_step.imperfection_amplification,
            2.0e-6,
        );
        assert_relative(
            max_translation(&rotated_step.displacements),
            max_translation(&baseline_step.displacements),
            2.0e-6,
        );
    }
}

#[test]
fn corotational_portal_matches_linearized_response_at_low_load() {
    let mut probe = portal_request(0.0);
    probe.load_steps = Some(1);
    let critical_factor = solve_frame_2d_p_delta(&probe)
        .expect("critical-factor probe should solve")
        .buckling_result
        .minimum_load_factor;

    let mut linearized_request = portal_request(0.0);
    linearized_request.load_steps = Some(1);
    linearized_request.maximum_load_factor = Some(0.005 * critical_factor);
    let mut corotational_request = linearized_request.clone();
    corotational_request.kinematics = Frame2dStabilityKinematics::Corotational;

    let linearized = solve_frame_2d_p_delta(&linearized_request)
        .expect("low-load linearized portal should solve");
    let corotational = solve_frame_2d_p_delta(&corotational_request)
        .expect("low-load corotational portal should solve");

    assert!(corotational.converged);
    assert_relative(
        corotational.steps[0].imperfection_amplification,
        linearized.steps[0].imperfection_amplification,
        2.0e-3,
    );
}

#[test]
fn adaptive_cutback_recovers_a_difficult_corotational_step() {
    let critical_factor = solve_frame_2d_p_delta(&portal_request(0.0))
        .expect("critical-factor probe should solve")
        .buckling_result
        .minimum_load_factor;
    let mut request = portal_request(0.0);
    request.kinematics = Frame2dStabilityKinematics::Corotational;
    request.maximum_load_factor = Some(0.9 * critical_factor);
    request.load_steps = Some(1);
    request.max_iterations = Some(4);
    request.tolerance = Some(1.0e-8);
    request.max_step_cutbacks = Some(0);

    let without_cutback =
        solve_frame_2d_p_delta(&request).expect("failed equilibrium should remain inspectable");
    assert!(!without_cutback.converged);
    assert_eq!(without_cutback.steps[0].achieved_load_factor, Some(0.0));
    assert_eq!(
        without_cutback.steps[0].failure_reason,
        Some(Frame2dPDeltaFailureReason::CutbackLimitExhausted)
    );
    assert!(
        without_cutback.steps[0]
            .failure_detail
            .as_deref()
            .unwrap()
            .contains("MaximumIterations")
    );

    request.max_step_cutbacks = Some(8);
    let adaptive = solve_frame_2d_p_delta(&request).expect("adaptive equilibrium should solve");
    let step = &adaptive.steps[0];
    assert!(adaptive.converged);
    assert!(step.converged);
    assert!(step.cutbacks > 0);
    assert!(step.substeps > 1);
    assert_eq!(step.failure_reason, None);
    assert_eq!(step.failure_detail, None);
    assert_relative(
        step.achieved_load_factor.unwrap(),
        step.load_factor,
        1.0e-12,
    );
}

#[test]
fn arc_length_path_crosses_the_screening_limit_and_is_objective() {
    let critical_factor = solve_frame_2d_p_delta(&portal_request(0.0))
        .expect("portal probe should solve")
        .buckling_result
        .minimum_load_factor;
    let mut request = portal_request(0.0);
    request.kinematics = Frame2dStabilityKinematics::Corotational;
    request.path_control = Frame2dStabilityPathControl::ArcLength;
    request.maximum_load_factor = Some(critical_factor * 6.0);
    request.load_steps = Some(96);
    request.max_iterations = Some(64);

    let result =
        solve_frame_2d_p_delta(&request).expect("arc-length path should remain inspectable");
    assert!(result.converged, "arc-length steps: {:?}", result.steps);
    assert_eq!(result.path_control, Frame2dStabilityPathControl::ArcLength);
    assert!(
        result
            .steps
            .iter()
            .any(|step| step.critical_factor_ratio > 0.95),
        "path ratios: {:?}",
        result
            .steps
            .iter()
            .map(|step| step.critical_factor_ratio)
            .collect::<Vec<_>>()
    );
    assert!(result.steps.iter().all(|step| step.residual_norm < 1.0e-7));
    assert!(result.steps.iter().all(|step| {
        step.arc_length_constraint_error
            .is_some_and(|error| error < 1.0e-7)
    }));
    assert!(result.steps.iter().all(|step| step.path_event.is_none()));

    let mut rotated_request = portal_request(0.731);
    rotated_request.kinematics = Frame2dStabilityKinematics::Corotational;
    rotated_request.path_control = Frame2dStabilityPathControl::ArcLength;
    rotated_request.maximum_load_factor = request.maximum_load_factor;
    rotated_request.load_steps = request.load_steps;
    rotated_request.max_iterations = request.max_iterations;
    let rotated =
        solve_frame_2d_p_delta(&rotated_request).expect("rotated arc-length path should solve");
    assert!(rotated.converged);
    for (baseline, rotated) in result.steps.iter().zip(&rotated.steps) {
        assert_relative(rotated.load_factor, baseline.load_factor, 2.0e-8);
        assert_relative(
            max_translation(&rotated.displacements),
            max_translation(&baseline.displacements),
            2.0e-8,
        );
    }
}

#[test]
fn arc_length_cutback_recovers_an_oversized_radius() {
    let critical_factor = solve_frame_2d_p_delta(&portal_request(0.0))
        .expect("portal probe should solve")
        .buckling_result
        .minimum_load_factor;
    let mut request = portal_request(0.0);
    request.kinematics = Frame2dStabilityKinematics::Corotational;
    request.path_control = Frame2dStabilityPathControl::ArcLength;
    request.maximum_load_factor = Some(critical_factor * 6.0);
    request.load_steps = Some(1);
    request.max_iterations = Some(4);
    request.tolerance = Some(1.0e-8);
    request.max_step_cutbacks = Some(0);

    let failed = solve_frame_2d_p_delta(&request).expect("failed arc step should be inspectable");
    assert!(!failed.converged);
    assert_eq!(
        failed.steps[0].failure_reason,
        Some(Frame2dPDeltaFailureReason::CutbackLimitExhausted)
    );
    let nominal_radius = failed.steps[0].arc_length_radius.unwrap();

    request.max_step_cutbacks = Some(8);
    let recovered = solve_frame_2d_p_delta(&request).expect("arc cutback should recover");
    let step = &recovered.steps[0];
    assert!(recovered.converged, "recovered step: {step:?}");
    assert!(step.cutbacks > 0);
    assert!(step.arc_length_radius.unwrap() < nominal_radius);
    assert_eq!(step.failure_reason, None);
    assert_eq!(step.failure_detail, None);
}

#[test]
fn arc_length_radius_tracks_the_target_iteration_count() {
    let critical_factor = solve_frame_2d_p_delta(&portal_request(0.0))
        .expect("portal probe should solve")
        .buckling_result
        .minimum_load_factor;
    let mut request = portal_request(0.0);
    request.kinematics = Frame2dStabilityKinematics::Corotational;
    request.path_control = Frame2dStabilityPathControl::ArcLength;
    request.maximum_load_factor = Some(critical_factor * 2.0);
    request.load_steps = Some(12);
    request.max_iterations = Some(32);
    request.arc_length_target_iterations = Some(2);

    let result = solve_frame_2d_p_delta(&request).expect("adaptive radius path should solve");
    assert!(
        result.converged,
        "adaptive radius steps: {:?}",
        result.steps
    );
    let radii = result
        .steps
        .iter()
        .map(|step| step.arc_length_radius.unwrap())
        .collect::<Vec<_>>();
    assert!(radii.iter().skip(1).any(|radius| *radius < radii[0]));
    assert!(radii.windows(2).all(|pair| pair[1] <= pair[0] * 2.0));
}

#[test]
fn arc_length_matches_shallow_arch_limit_point_and_descending_branch() {
    let probe =
        solve_frame_2d_p_delta(&shallow_arch_request(1)).expect("shallow arch probe should solve");
    let mut request = shallow_arch_request(1);
    request.kinematics = Frame2dStabilityKinematics::Corotational;
    request.path_control = Frame2dStabilityPathControl::ArcLength;
    request.maximum_load_factor = Some(probe.buckling_result.minimum_load_factor * 100.0);
    request.load_steps = Some(128);
    request.max_iterations = Some(64);
    request.max_step_cutbacks = Some(12);

    let result = solve_frame_2d_p_delta(&request).expect("shallow arch path should be inspectable");
    let factors = result
        .steps
        .iter()
        .map(|step| step.load_factor)
        .collect::<Vec<_>>();
    let crown_displacements = result
        .steps
        .iter()
        .map(|step| step.displacements[4])
        .collect::<Vec<_>>();
    assert!(result.converged, "shallow arch steps: {:?}", result.steps);
    assert!(factors.iter().all(|factor| factor.is_finite()));
    let (peak_index, peak_factor) = factors
        .iter()
        .copied()
        .enumerate()
        .max_by(|left, right| left.1.total_cmp(&right.1))
        .unwrap();
    let pin_jointed_peak = pin_jointed_shallow_arch_peak_factor();
    assert!(peak_index > 0 && peak_index + 1 < factors.len());
    assert_relative(peak_factor, pin_jointed_peak, 2.0e-2);
    assert_eq!(
        result.steps[peak_index].path_event,
        Some(Frame2dEquilibriumPathEvent::LimitPointMaximum)
    );
    assert_eq!(
        result.steps[0].tangent_stability,
        Some(Frame2dTangentStability::PositiveDefinite)
    );
    assert!(result.steps[peak_index + 1..].iter().any(|step| {
        step.tangent_stability == Some(Frame2dTangentStability::Indefinite)
            && step.tangent_negative_pivots.is_some_and(|count| count > 0)
    }));
    assert_eq!(result.steps[0].tangent_negative_pivot_delta, None);
    assert!(result.steps.iter().skip(1).all(|step| {
        step.tangent_negative_pivot_delta.is_some()
            && step.path_event != Some(Frame2dEquilibriumPathEvent::BifurcationCandidate)
    }));
    assert!(factors[peak_index + 1..].iter().any(|factor| *factor < 0.0));
    assert!(crown_displacements.last().unwrap() < &-0.1);
    assert!(result.steps.iter().all(|step| step.residual_norm < 1.0e-7));
    assert!(result.steps.iter().all(|step| {
        step.arc_length_constraint_error
            .is_some_and(|error| error < 1.0e-7)
    }));
}

#[test]
fn segmented_shallow_arch_resolves_the_member_instability_branch() {
    let mut critical_factors = Vec::new();
    let mut peak_factors = Vec::new();
    let mut bifurcation_candidate_factors = Vec::new();
    for segments_per_side in [2, 4, 8] {
        let probe = solve_frame_2d_p_delta(&shallow_arch_request(segments_per_side))
            .expect("segmented shallow arch probe should solve");
        let mut request = shallow_arch_request(segments_per_side);
        request.kinematics = Frame2dStabilityKinematics::Corotational;
        request.path_control = Frame2dStabilityPathControl::ArcLength;
        request.maximum_load_factor = Some(probe.buckling_result.minimum_load_factor * 100.0);
        request.load_steps = Some(128);
        request.max_iterations = Some(64);
        request.max_step_cutbacks = Some(12);

        let result =
            solve_frame_2d_p_delta(&request).expect("segmented shallow arch should be inspectable");
        assert!(result.converged, "segmented steps: {:?}", result.steps);
        let factors = result
            .steps
            .iter()
            .map(|step| step.load_factor)
            .collect::<Vec<_>>();
        let (peak_index, peak_factor) = factors
            .iter()
            .copied()
            .enumerate()
            .max_by(|left, right| left.1.total_cmp(&right.1))
            .unwrap();
        critical_factors.push(probe.buckling_result.minimum_load_factor);
        peak_factors.push(peak_factor);
        assert!(peak_index > 0 && peak_index + 1 < factors.len());
        assert_eq!(
            result.steps[peak_index].path_event,
            Some(Frame2dEquilibriumPathEvent::LimitPointMaximum)
        );
        assert_eq!(
            result.steps[0].tangent_stability,
            Some(Frame2dTangentStability::PositiveDefinite)
        );
        assert!(result.steps[peak_index + 1..].iter().any(|step| {
            step.tangent_stability == Some(Frame2dTangentStability::Indefinite)
                && step.tangent_negative_pivots.is_some_and(|count| count > 0)
        }));
        assert_eq!(result.steps[0].tangent_negative_pivot_delta, None);
        let bifurcation_candidates = result
            .steps
            .iter()
            .filter(|step| {
                step.path_event == Some(Frame2dEquilibriumPathEvent::BifurcationCandidate)
            })
            .collect::<Vec<_>>();
        assert_eq!(
            bifurcation_candidates.len(),
            1,
            "segments={segments_per_side}, candidates={bifurcation_candidates:?}"
        );
        let candidate = bifurcation_candidates[0];
        assert_eq!(candidate.step, 2);
        assert_eq!(candidate.tangent_negative_pivot_delta, Some(1));
        assert!(candidate.load_factor > critical_factors.last().copied().unwrap());
        assert!(candidate.load_factor < peak_factor);
        assert!(
            candidate
                .tangent_critical_eigenvalue
                .is_some_and(|value| value.abs() < 1.0e-8)
        );
        assert!(
            candidate
                .tangent_critical_mode_residual
                .is_some_and(|residual| residual < 5.0e-8)
        );
        let mode = candidate.tangent_critical_mode.as_ref().unwrap();
        assert_eq!(mode.len(), request.buckling.frame.nodes.len() * 3);
        assert_relative(
            mode.iter().map(|value| value * value).sum::<f64>().sqrt(),
            1.0,
            1.0e-10,
        );
        for (node, shape) in request
            .buckling
            .frame
            .nodes
            .iter()
            .zip(mode.chunks_exact(3))
        {
            assert!(!node.fix_x || shape[0] == 0.0);
            assert!(!node.fix_y || shape[1] == 0.0);
            assert!(!node.fix_rz || shape[2] == 0.0);
        }
        let bracket_min = candidate.tangent_transition_load_factor_min.unwrap();
        let bracket_max = candidate.tangent_transition_load_factor_max.unwrap();
        let bracket_width = candidate.tangent_transition_load_factor_width.unwrap();
        let critical_load = candidate.tangent_critical_load_factor.unwrap();
        assert_eq!(candidate.tangent_transition_refinements, Some(12));
        assert!(bracket_min < bracket_max);
        assert_relative(bracket_max - bracket_min, bracket_width, 1.0e-12);
        assert!(bracket_min <= critical_load && critical_load <= bracket_max);
        assert!(critical_load < peak_factor);
        let coarse_width =
            (candidate.load_factor - result.steps[candidate.step - 2].load_factor).abs();
        assert!(bracket_width < coarse_width / 4_000.0);
        bifurcation_candidate_factors.push(critical_load);
        assert!(factors[peak_index + 1..].iter().any(|factor| *factor < 0.0));
        assert!(result.steps.iter().all(|step| step.residual_norm < 1.0e-7));
        assert!(result.steps.iter().all(|step| {
            step.arc_length_constraint_error
                .is_some_and(|error| error < 1.0e-7)
        }));
    }
    assert_relative(critical_factors[2], critical_factors[1], 1.0e-3);
    assert!(peak_factors.windows(2).all(|pair| pair[1] < pair[0]));
    assert_relative(peak_factors[2], peak_factors[1], 1.0e-1);
    assert!(
        bifurcation_candidate_factors
            .windows(2)
            .all(|pair| pair[1] < pair[0])
    );
    assert_relative(
        bifurcation_candidate_factors[2],
        bifurcation_candidate_factors[1],
        5.0e-2,
    );
    assert!(peak_factors[0] < pin_jointed_shallow_arch_peak_factor() * 0.15);
}

#[test]
fn modal_constraint_probes_both_member_instability_branches() {
    let mut request = shallow_arch_request(4);
    let probe =
        solve_frame_2d_p_delta(&request).expect("segmented shallow arch probe should solve");
    request.kinematics = Frame2dStabilityKinematics::Corotational;
    request.path_control = Frame2dStabilityPathControl::ArcLength;
    request.maximum_load_factor = Some(probe.buckling_result.minimum_load_factor * 100.0);
    request.load_steps = Some(128);
    request.max_iterations = Some(64);
    request.max_step_cutbacks = Some(12);
    request.branch_switch = Frame2dBranchSwitchSelection::Both;
    request.branch_switch_amplitude = Some(5.0e-3);

    let result =
        solve_frame_2d_p_delta(&request).expect("branch-switch probes should be inspectable");
    let candidate = result
        .steps
        .iter()
        .find(|step| !step.branch_switch_probes.is_empty())
        .expect("tangent transition should produce branch probes");
    assert_eq!(candidate.branch_switch_probes.len(), 2);
    assert_eq!(
        candidate.branch_switch_probes[0].direction,
        Frame2dBranchDirection::Positive
    );
    assert_eq!(
        candidate.branch_switch_probes[1].direction,
        Frame2dBranchDirection::Negative
    );
    for branch in &candidate.branch_switch_probes {
        assert!(
            branch.equilibrium_converged,
            "branch probe should converge: {branch:?}"
        );
        assert!(
            branch.primary_equilibrium_converged,
            "branch probe: {branch:?}"
        );
        assert!(branch.distinct_branch, "branch probe: {branch:?}");
        assert!(branch.load_factor.is_some_and(f64::is_finite));
        assert!(
            branch
                .residual_norm
                .is_some_and(|residual| residual < 1.0e-7)
        );
        assert!(
            branch
                .modal_constraint_error
                .is_some_and(|error| error < 1.0e-7)
        );
        assert!(
            branch
                .mode_projection
                .is_some_and(|projection| projection.abs() >= 4.5e-3)
        );
        assert!(
            branch
                .primary_displacement_distance
                .is_some_and(|distance| distance >= 2.5e-3)
        );
        assert_eq!(
            branch.displacements.as_ref().map(Vec::len),
            Some(request.buckling.frame.nodes.len() * 3)
        );
    }
}

fn pin_jointed_shallow_arch_peak_factor() -> f64 {
    let half_span: f64 = 1.0;
    let height: f64 = 0.099;
    let axial_rigidity: f64 = 210.0e9 * 1.0e-3;
    let reference_load: f64 = 1_000.0;
    let initial_length = half_span.hypot(height);
    (0..=20_000)
        .map(|index| height * index as f64 / 20_000.0)
        .map(|current_height| {
            let current_length = half_span.hypot(current_height);
            let compression = axial_rigidity * (initial_length - current_length) / initial_length;
            2.0 * compression * current_height / (current_length * reference_load)
        })
        .fold(0.0, f64::max)
}

fn portal_request(angle: f64) -> SolveFrame2dPDeltaRequest {
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
    }
}

fn shallow_arch_request(segments_per_side: usize) -> SolveFrame2dPDeltaRequest {
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
    }
}

fn element(id: &str, node_i: usize, node_j: usize) -> Frame2dElementInput {
    Frame2dElementInput {
        id: id.to_string(),
        node_i,
        node_j,
        area: 0.012,
        youngs_modulus: 210.0e9,
        moment_of_inertia: 9.0e-5,
        section_modulus: 6.0e-4,
    }
}

fn rotate(x: f64, y: f64, angle: f64) -> (f64, f64) {
    let (sine, cosine) = angle.sin_cos();
    (cosine * x - sine * y, sine * x + cosine * y)
}

fn max_translation(values: &[f64]) -> f64 {
    (0..values.len() / 3)
        .map(|node| (values[node * 3].powi(2) + values[node * 3 + 1].powi(2)).sqrt())
        .fold(0.0_f64, f64::max)
}

fn assert_relative(actual: f64, expected: f64, tolerance: f64) {
    let relative = (actual - expected).abs() / expected.abs().max(1.0);
    assert!(
        relative <= tolerance,
        "actual={actual}, expected={expected}, relative={relative}"
    );
}
