use super::*;

const TRAJECTORY_STEPS: usize = 64;
const TRAJECTORY_RADIUS: f64 = 2.0e-3;

#[test]
fn external_elastica_reduction_recovers_analytic_limits() {
    assert_relative(
        complete_elliptic_k(0.0),
        std::f64::consts::PI * 0.5,
        1.0e-14,
    );
    assert_relative(complete_elliptic_k(0.5), 1.685_750_354_812_596, 1.0e-13);

    let reference = reduced_mode_references(ASYMMETRIC_COLUMN_INERTIA)[0];
    let external = solve_external_trajectory_point(1.0e-7, reference.1);
    assert_relative(external.load, reference.0, 1.0e-8);
    assert!(
        (external.direction[0] * reference.1[0] + external.direction[1] * reference.1[1]).abs()
            > 1.0 - 1.0e-10
    );
    assert!(external.normalized_residual < 1.0e-12);
}

#[test]
fn asymmetric_connected_postcritical_trajectory_tracks_external_elastica_reduction() {
    let references = reduced_mode_references(ASYMMETRIC_COLUMN_INERTIA);
    let request = trajectory_request(coupled_euler_request(
        ASYMMETRIC_COLUMN_INERTIA,
        references[0].1,
    ));
    correlate_trajectory("direct coupling", &request, references[0]);
}

#[test]
fn branched_series_coupler_preserves_the_external_postcritical_trajectory() {
    let reference = reduced_mode_references(ASYMMETRIC_COLUMN_INERTIA)[0];
    let request = trajectory_request(branched_series_coupler_request(reference.1));
    let buckling =
        solve_buckling_frame_2d(&request.buckling).expect("branched series topology should solve");
    assert_relative(buckling.minimum_load_factor, reference.0, 5.0e-3);
    assert!(midpoint_alignment(&buckling.modes[0].shape, reference.1) > 0.98);
    assert!(
        !buckling
            .element_preloads
            .last()
            .unwrap()
            .active_in_geometric_stiffness
    );
    correlate_trajectory("branched series coupling", &request, reference);
}

fn trajectory_request(mut request: SolveFrame2dPDeltaRequest) -> SolveFrame2dPDeltaRequest {
    request.branch_continuation_steps = Some(TRAJECTORY_STEPS);
    request.branch_continuation_radius = Some(TRAJECTORY_RADIUS);
    request.branch_continuation_min_radius_ratio = Some(0.25);
    request
}

fn correlate_trajectory(
    label: &str,
    request: &SolveFrame2dPDeltaRequest,
    reference: (f64, [f64; 2]),
) {
    let result =
        solve_frame_2d_p_delta(&request).expect("asymmetric postcritical trajectory should solve");
    let candidate = result
        .steps
        .iter()
        .filter(|step| !step.branch_switch_probes.is_empty())
        .min_by(|left, right| {
            (left.load_factor - reference.0)
                .abs()
                .total_cmp(&(right.load_factor - reference.0).abs())
        })
        .expect("lower asymmetric transition should emit a switched branch");
    let branch = candidate
        .branch_switch_probes
        .iter()
        .filter(|probe| {
            probe.origin == Frame2dBranchProbeOrigin::CriticalMode
                && probe.continuation_converged == Some(true)
        })
        .max_by(|left, right| {
            midpoint_alignment(left.displacements.as_ref().unwrap(), reference.1).total_cmp(
                &midpoint_alignment(right.displacements.as_ref().unwrap(), reference.1),
            )
        })
        .unwrap_or_else(|| {
            panic!(
                "lower mixed branch should continue: candidate_load={}, reference_load={}, probes={:?}",
                candidate.load_factor,
                reference.0,
                candidate
                    .branch_switch_probes
                    .iter()
                    .map(|probe| (
                        probe.origin,
                        probe.mode_index,
                        probe.equilibrium_converged,
                        probe.distinct_branch,
                        probe.continuation_converged,
                        probe.failure_detail.as_deref(),
                        probe.continuation_failure_detail.as_deref(),
                    ))
                    .collect::<Vec<_>>()
            )
        });
    let seed_direction = midpoint_direction(branch.displacements.as_ref().unwrap());
    let seed_alignment = midpoint_alignment(branch.displacements.as_ref().unwrap(), reference.1);
    assert!(
        seed_alignment > 0.98,
        "seed alignment={seed_alignment}, actual={seed_direction:?}, reference={:?}, candidates={:?}",
        reference.1,
        result
            .steps
            .iter()
            .filter(|step| !step.branch_switch_probes.is_empty())
            .map(|step| (
                step.load_factor,
                step.tangent_critical_modes
                    .iter()
                    .map(|mode| midpoint_direction(&mode.shape))
                    .collect::<Vec<_>>(),
                step.branch_switch_probes
                    .iter()
                    .map(|probe| (
                        probe.mode_index,
                        probe.displacements.as_deref().map(midpoint_direction),
                        probe.continuation_converged,
                    ))
                    .collect::<Vec<_>>(),
            ))
            .collect::<Vec<_>>()
    );
    assert_eq!(branch.continuation_steps.len(), TRAJECTORY_STEPS);

    let mut external_direction = reference.1;
    let seed_radius = midpoint_radius(branch.displacements.as_ref().unwrap());
    let mut previous_radius = seed_radius;
    let mut maximum_load_error = 0.0_f64;
    let mut minimum_direction_alignment = 1.0_f64;
    for step in &branch.continuation_steps {
        assert!(step.converged);
        assert!(step.residual_norm < 1.0e-7);
        assert!(step.arc_length_constraint_error < 1.0e-7);
        let radius = midpoint_radius(&step.displacements);
        assert!(radius > previous_radius);
        previous_radius = radius;

        let external = solve_external_trajectory_point(radius, external_direction);
        external_direction = external.direction;
        let load_error = (step.load_factor - external.load).abs() / external.load.abs().max(1.0);
        let alignment = midpoint_alignment(&step.displacements, external.direction);
        maximum_load_error = maximum_load_error.max(load_error);
        minimum_direction_alignment = minimum_direction_alignment.min(alignment);
        assert!(
            external.normalized_residual < 1.0e-10,
            "external residual={} at radius={radius}",
            external.normalized_residual
        );
    }

    println!(
        "{label}: points={}, seed_radius={seed_radius:.8e}, terminal_radius={previous_radius:.8e}, max_load_error={maximum_load_error:.8e}, min_direction_alignment={minimum_direction_alignment:.8e}",
        branch.continuation_steps.len()
    );
    assert!(
        previous_radius > seed_radius + 5.0 * TRAJECTORY_RADIUS,
        "seed radius={seed_radius}, terminal radius={previous_radius}, maximum load error={maximum_load_error}, minimum direction alignment={minimum_direction_alignment}"
    );
    assert!(
        maximum_load_error < 0.03,
        "maximum external load error={maximum_load_error}"
    );
    assert!(
        minimum_direction_alignment > 0.999,
        "minimum external direction alignment={minimum_direction_alignment}"
    );
}

fn branched_series_coupler_request(imperfection_weights: [f64; 2]) -> SolveFrame2dPDeltaRequest {
    let mut request = coupled_euler_request(ASYMMETRIC_COLUMN_INERTIA, imperfection_weights);
    let direct_coupler = request
        .buckling
        .frame
        .elements
        .iter()
        .position(|element| element.id == "midpoint-coupler-0-1")
        .expect("direct coupler should exist");
    request.buckling.frame.elements.remove(direct_coupler);

    let center = request.buckling.frame.nodes.len();
    request.buckling.frame.nodes.push(Frame2dNodeInput {
        id: "series-coupler-center".into(),
        x: COLUMN_SPACING * 0.5,
        y: LENGTH * 0.5,
        fix_x: false,
        fix_y: false,
        fix_rz: false,
        load_x: 0.0,
        load_y: 0.0,
        moment_z: 0.0,
    });
    let center_weight = 0.5 * (imperfection_weights[0] + imperfection_weights[1]);
    request
        .imperfection_shape
        .as_mut()
        .unwrap()
        .extend([center_weight, 0.0, 0.0]);

    let half_length = COLUMN_SPACING * 0.5;
    let series_area = 2.0 * COUPLING_STIFFNESS * half_length / YOUNGS_MODULUS;
    for (id, node_i, node_j) in [
        ("series-coupler-left", midpoint_node(0), center),
        ("series-coupler-right", center, midpoint_node(1)),
    ] {
        request.buckling.frame.elements.push(Frame2dElementInput {
            id: id.into(),
            node_i,
            node_j,
            area: series_area,
            youngs_modulus: YOUNGS_MODULUS,
            moment_of_inertia: 1.0e-12,
            section_modulus: 1.0,
        });
    }

    let spectator = request.buckling.frame.nodes.len();
    request.buckling.frame.nodes.push(Frame2dNodeInput {
        id: "series-coupler-spectator".into(),
        x: COLUMN_SPACING * 0.5,
        y: LENGTH * 0.75,
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
        .extend([center_weight, 0.0, 0.0]);
    request.buckling.frame.elements.push(Frame2dElementInput {
        id: "unloaded-free-spectator".into(),
        node_i: center,
        node_j: spectator,
        area: 1.0e-3,
        youngs_modulus: YOUNGS_MODULUS,
        moment_of_inertia: 1.0e-8,
        section_modulus: 1.0,
    });
    request.branch_switch_amplitude = Some(1.0e-4);
    request
}

#[derive(Clone, Copy)]
struct ExternalTrajectoryPoint {
    load: f64,
    direction: [f64; 2],
    normalized_residual: f64,
}

fn solve_external_trajectory_point(
    radius: f64,
    initial_direction: [f64; 2],
) -> ExternalTrajectoryPoint {
    let mut angle = initial_direction[1].atan2(initial_direction[0]);
    for _ in 0..40 {
        let residual = external_direction_residual(radius, angle);
        if residual.abs() < 1.0e-11 {
            break;
        }
        let delta = 1.0e-6;
        let derivative = (external_direction_residual(radius, angle + delta)
            - external_direction_residual(radius, angle - delta))
            / (2.0 * delta);
        assert!(derivative.is_finite() && derivative.abs() > 1.0e-9);
        angle -= (residual / derivative).clamp(-0.2, 0.2);
    }
    let direction = [angle.cos(), angle.sin()];
    let amplitudes = [radius * direction[0], radius * direction[1]];
    let coupling = reduced_coupling_load();
    let left_load = pinned_elastica_load(COLUMN_INERTIA, amplitudes[0].abs())
        + coupling * (amplitudes[0] - amplitudes[1]) / amplitudes[0];
    let right_load = pinned_elastica_load(ASYMMETRIC_COLUMN_INERTIA, amplitudes[1].abs())
        + coupling * (amplitudes[1] - amplitudes[0]) / amplitudes[1];
    let load = 0.5 * (left_load + right_load);
    ExternalTrajectoryPoint {
        load,
        direction,
        normalized_residual: (left_load - right_load).abs() / load.abs().max(1.0),
    }
}

fn external_direction_residual(radius: f64, angle: f64) -> f64 {
    let left = radius * angle.cos();
    let right = radius * angle.sin();
    pinned_elastica_load(COLUMN_INERTIA, left.abs())
        - pinned_elastica_load(ASYMMETRIC_COLUMN_INERTIA, right.abs())
        + reduced_coupling_load() * (left / right - right / left)
}

fn reduced_coupling_load() -> f64 {
    2.0 * COUPLING_STIFFNESS * LENGTH / std::f64::consts::PI.powi(2)
}

pub(super) fn pinned_elastica_load(moment_of_inertia: f64, midpoint_amplitude: f64) -> f64 {
    let normalized_amplitude = midpoint_amplitude / LENGTH;
    if normalized_amplitude <= f64::EPSILON {
        return std::f64::consts::PI.powi(2) * YOUNGS_MODULUS * moment_of_inertia / LENGTH.powi(2);
    }
    let mut lower = 0.0;
    let mut upper = 0.8;
    assert!(upper / complete_elliptic_k(upper) > normalized_amplitude);
    for _ in 0..80 {
        let midpoint = 0.5 * (lower + upper);
        if midpoint / complete_elliptic_k(midpoint) < normalized_amplitude {
            lower = midpoint;
        } else {
            upper = midpoint;
        }
    }
    let elliptic = complete_elliptic_k(0.5 * (lower + upper));
    4.0 * elliptic * elliptic * YOUNGS_MODULUS * moment_of_inertia / LENGTH.powi(2)
}

fn complete_elliptic_k(modulus: f64) -> f64 {
    let mut arithmetic = 1.0;
    let mut geometric = (1.0 - modulus * modulus).sqrt();
    for _ in 0..32 {
        let next_arithmetic = 0.5 * (arithmetic + geometric);
        let next_geometric = (arithmetic * geometric).sqrt();
        arithmetic = next_arithmetic;
        geometric = next_geometric;
        if (arithmetic - geometric).abs() < 1.0e-15 {
            break;
        }
    }
    std::f64::consts::PI / (2.0 * arithmetic)
}

fn midpoint_radius(displacements: &[f64]) -> f64 {
    let left = displacements[midpoint_node(0) * 3];
    let right = displacements[midpoint_node(1) * 3];
    left.hypot(right)
}
