use crate::buckling_frame_2d::solve_buckling_frame_2d;
use crate::frame_2d_arc_length::solve_arc_length_steps;
use crate::frame_2d_corotational::solve_corotational_steps;
use crate::frame_2d_path_events::annotate_path_events;
use crate::frame_2d_stability::assemble_frame_2d_stability;
use crate::linear_algebra::{
    SparseMatrix, add_at, reduce_sparse_system, solve_spd_system_profile_with_options,
};
use crate::linear_banded::SymmetricBandCholesky;
use crate::linear_solver_profile::SpdSolveOptions;
use kyuubiki_protocol::{
    FRAME_2D_P_DELTA_CRITICAL_FACTOR_LIMIT_RATIO, Frame2dBranchSwitchSelection,
    Frame2dImperfectionSource, Frame2dPDeltaStepResult, Frame2dStabilityKinematics,
    Frame2dStabilityPathControl, SolveFrame2dPDeltaRequest, SolveFrame2dPDeltaResult,
};

const DEFAULT_LOAD_STEPS: usize = 10;
const DEFAULT_MAXIMUM_CRITICAL_FACTOR_RATIO: f64 = 0.8;

pub fn solve_frame_2d_p_delta(
    request: &SolveFrame2dPDeltaRequest,
) -> Result<SolveFrame2dPDeltaResult, String> {
    validate_request(request)?;
    let mode_index = request.imperfection_mode_index.unwrap_or(0);
    let mut buckling_request = request.buckling.clone();
    let required_modes = if request.imperfection_shape.is_some() {
        1
    } else {
        mode_index + 1
    };
    buckling_request.mode_count =
        Some(buckling_request.mode_count.unwrap_or(3).max(required_modes));
    let buckling_result = solve_buckling_frame_2d(&buckling_request)?;
    let critical_factor = buckling_result.minimum_load_factor;
    let maximum_load_factor = request
        .maximum_load_factor
        .unwrap_or(critical_factor * DEFAULT_MAXIMUM_CRITICAL_FACTOR_RATIO);
    let maximum_ratio = maximum_load_factor / critical_factor;
    if !(maximum_load_factor.is_finite() && maximum_load_factor > 0.0) {
        return Err("frame 2d p-delta maximum_load_factor must be positive and finite".into());
    }
    if request.path_control == Frame2dStabilityPathControl::LoadControl
        && maximum_ratio >= FRAME_2D_P_DELTA_CRITICAL_FACTOR_LIMIT_RATIO
    {
        return Err(format!(
            "frame 2d p-delta maximum load factor must remain below {:.3} of the first critical factor",
            FRAME_2D_P_DELTA_CRITICAL_FACTOR_LIMIT_RATIO
        ));
    }

    let (imperfection_shape, imperfection_source) = match &request.imperfection_shape {
        Some(shape) => (shape.as_slice(), Frame2dImperfectionSource::ExplicitShape),
        None => (
            buckling_result
                .modes
                .get(mode_index)
                .ok_or_else(|| {
                    format!("frame 2d p-delta imperfection mode {mode_index} is unavailable")
                })?
                .shape
                .as_slice(),
            Frame2dImperfectionSource::BucklingMode,
        ),
    };
    let initial_imperfection_shape = scale_imperfection(
        imperfection_shape,
        request.imperfection_amplitude,
        buckling_request.frame.nodes.len(),
    )?;
    let system = assemble_frame_2d_stability(&buckling_request)?;
    let geometric_imperfection = multiply(&system.geometric, &initial_imperfection_shape);
    let load_steps = request
        .load_steps
        .unwrap_or(DEFAULT_LOAD_STEPS)
        .clamp(1, 128);
    let mut steps = match (request.kinematics, request.path_control) {
        (
            Frame2dStabilityKinematics::LinearizedPDelta,
            Frame2dStabilityPathControl::LoadControl,
        ) => solve_linearized_steps(
            &system,
            &initial_imperfection_shape,
            &geometric_imperfection,
            maximum_load_factor,
            critical_factor,
            load_steps,
        )?,
        (Frame2dStabilityKinematics::Corotational, Frame2dStabilityPathControl::LoadControl) => {
            solve_corotational_steps(
                request,
                &system,
                &initial_imperfection_shape,
                maximum_load_factor,
                critical_factor,
                load_steps,
            )?
        }
        (Frame2dStabilityKinematics::Corotational, Frame2dStabilityPathControl::ArcLength) => {
            solve_arc_length_steps(
                request,
                &system,
                &initial_imperfection_shape,
                maximum_load_factor,
                critical_factor,
                load_steps,
            )?
        }
        (Frame2dStabilityKinematics::LinearizedPDelta, Frame2dStabilityPathControl::ArcLength) => {
            unreachable!("arc-length validation requires corotational kinematics")
        }
    };

    annotate_path_events(&mut steps);
    let final_displacements = steps
        .last()
        .map(|step| step.displacements.clone())
        .unwrap_or_default();
    let max_imperfection_amplification = steps
        .iter()
        .map(|step| step.imperfection_amplification)
        .fold(1.0_f64, f64::max);
    let converged = steps.len() == load_steps && steps.iter().all(|step| step.converged);
    Ok(SolveFrame2dPDeltaResult {
        input: request.clone(),
        buckling_result,
        imperfection_source,
        kinematics: request.kinematics,
        path_control: request.path_control,
        initial_imperfection_shape,
        critical_factor_limit_ratio: FRAME_2D_P_DELTA_CRITICAL_FACTOR_LIMIT_RATIO,
        steps,
        final_displacements,
        max_imperfection_amplification,
        converged,
    })
}

fn solve_linearized_steps(
    system: &crate::frame_2d_stability::Frame2dStabilitySystem,
    initial_imperfection: &[f64],
    geometric_imperfection: &[f64],
    maximum_load_factor: f64,
    critical_factor: f64,
    load_steps: usize,
) -> Result<Vec<Frame2dPDeltaStepResult>, String> {
    let mut steps = Vec::with_capacity(load_steps);
    for step in 1..=load_steps {
        let load_factor = maximum_load_factor * step as f64 / load_steps as f64;
        let tangent = combined_matrix(&system.elastic, &system.geometric, -load_factor);
        let force = system
            .reference_force
            .iter()
            .zip(geometric_imperfection)
            .map(|(external, imperfection)| load_factor * (external + imperfection))
            .collect::<Vec<_>>();
        let (reduced_tangent, reduced_force, free_dofs) =
            reduce_sparse_system(&tangent, &force, &system.constrained_dofs);
        let reduced_displacements = solve_precritical_tangent(&reduced_tangent, &reduced_force)?;
        let displacements = expand(&reduced_displacements, &free_dofs, tangent.size());
        let residual_norm =
            relative_residual(&reduced_tangent, &reduced_displacements, &reduced_force);
        steps.push(Frame2dPDeltaStepResult {
            step,
            load_factor,
            critical_factor_ratio: load_factor / critical_factor,
            iterations: 1,
            converged: true,
            achieved_load_factor: Some(load_factor),
            substeps: 1,
            cutbacks: 0,
            failure_reason: None,
            failure_detail: None,
            arc_length_constraint_error: None,
            arc_length_radius: None,
            load_factor_increment: Some(maximum_load_factor / load_steps as f64),
            path_event: None,
            tangent_stability: None,
            tangent_negative_pivots: None,
            tangent_near_zero_pivots: None,
            tangent_negative_pivot_delta: None,
            tangent_critical_eigenvalue: None,
            tangent_critical_mode_residual: None,
            tangent_critical_mode: None,
            tangent_transition_load_factor_min: None,
            tangent_transition_load_factor_max: None,
            tangent_transition_load_factor_width: None,
            tangent_transition_refinements: None,
            tangent_critical_load_factor: None,
            branch_switch_probes: Vec::new(),
            residual_norm,
            imperfection_amplification: imperfection_amplification(
                initial_imperfection,
                &displacements,
            ),
            max_incremental_displacement: max_translation(&displacements),
            displacements,
        });
    }
    Ok(steps)
}

fn solve_precritical_tangent(matrix: &SparseMatrix, rhs: &[f64]) -> Result<Vec<f64>, String> {
    if let Some(factor) = SymmetricBandCholesky::try_factor(matrix, 8_000_000)? {
        return factor.solve(rhs);
    }
    solve_spd_system_profile_with_options(matrix, rhs, SpdSolveOptions::default())
        .map(|profile| profile.solution)
}

fn validate_request(request: &SolveFrame2dPDeltaRequest) -> Result<(), String> {
    if !(request.imperfection_amplitude.is_finite() && request.imperfection_amplitude > 0.0) {
        return Err("frame 2d p-delta imperfection_amplitude must be positive and finite".into());
    }
    if request.load_steps == Some(0) {
        return Err("frame 2d p-delta load_steps must be positive".into());
    }
    if matches!(request.max_iterations, Some(0 | 257..)) {
        return Err("frame 2d p-delta max_iterations must be between 1 and 256".into());
    }
    if let Some(tolerance) = request.tolerance
        && !(tolerance.is_finite() && tolerance > 0.0)
    {
        return Err("frame 2d p-delta tolerance must be positive and finite".into());
    }
    if matches!(request.max_step_cutbacks, Some(17..)) {
        return Err("frame 2d p-delta max_step_cutbacks must not exceed 16".into());
    }
    if request.path_control == Frame2dStabilityPathControl::ArcLength
        && request.kinematics != Frame2dStabilityKinematics::Corotational
    {
        return Err("frame 2d arc-length control requires corotational kinematics".into());
    }
    if let Some(radius) = request.arc_length_radius
        && !(radius.is_finite() && radius > 0.0)
    {
        return Err("frame 2d arc_length_radius must be positive and finite".into());
    }
    if let Some(scale) = request.arc_length_load_scale
        && !(scale.is_finite() && scale > 0.0)
    {
        return Err("frame 2d arc_length_load_scale must be positive and finite".into());
    }
    if let Some(target) = request.arc_length_target_iterations
        && !(2..=64).contains(&target)
    {
        return Err("frame 2d arc_length_target_iterations must be between 2 and 64".into());
    }
    if let (Some(target), Some(maximum)) =
        (request.arc_length_target_iterations, request.max_iterations)
        && target > maximum
    {
        return Err("frame 2d arc_length_target_iterations must not exceed max_iterations".into());
    }
    if matches!(request.tangent_transition_refinement_steps, Some(21..)) {
        return Err("frame 2d tangent_transition_refinement_steps must not exceed 20".into());
    }
    if request.branch_switch != Frame2dBranchSwitchSelection::Disabled
        && (request.kinematics != Frame2dStabilityKinematics::Corotational
            || request.path_control != Frame2dStabilityPathControl::ArcLength)
    {
        return Err("frame 2d branch switching requires corotational arc-length control".into());
    }
    if let Some(amplitude) = request.branch_switch_amplitude
        && !(amplitude.is_finite() && amplitude > 0.0)
    {
        return Err("frame 2d branch_switch_amplitude must be positive and finite".into());
    }
    if request.branch_switch != Frame2dBranchSwitchSelection::Disabled
        && request.branch_switch_amplitude.is_none()
    {
        return Err("frame 2d branch switching requires branch_switch_amplitude".into());
    }
    if matches!(request.branch_continuation_steps, Some(65..)) {
        return Err("frame 2d branch_continuation_steps must not exceed 64".into());
    }
    if request
        .branch_continuation_steps
        .is_some_and(|steps| steps > 0)
        && request.branch_switch == Frame2dBranchSwitchSelection::Disabled
    {
        return Err("frame 2d branch continuation requires branch switching".into());
    }
    if let Some(shape) = &request.imperfection_shape {
        let expected = request.buckling.frame.nodes.len() * 3;
        if shape.len() != expected {
            return Err(format!(
                "frame 2d p-delta imperfection_shape must contain {expected} values"
            ));
        }
        if shape.iter().any(|value| !value.is_finite()) {
            return Err("frame 2d p-delta imperfection_shape must contain finite values".into());
        }
    }
    Ok(())
}

fn scale_imperfection(mode: &[f64], amplitude: f64, node_count: usize) -> Result<Vec<f64>, String> {
    let maximum = (0..node_count)
        .map(|node| (mode[node * 3].powi(2) + mode[node * 3 + 1].powi(2)).sqrt())
        .fold(0.0_f64, f64::max);
    if !(maximum.is_finite() && maximum > 1.0e-14) {
        return Err("frame 2d p-delta selected mode has no translational imperfection".into());
    }
    Ok(mode
        .iter()
        .map(|value| value * amplitude / maximum)
        .collect())
}

fn combined_matrix(elastic: &SparseMatrix, geometric: &SparseMatrix, scale: f64) -> SparseMatrix {
    let mut result = SparseMatrix::new(elastic.size());
    for row in 0..elastic.size() {
        for &(column, value) in elastic.row_entries(row) {
            add_at(&mut result, row, column, value);
        }
        for &(column, value) in geometric.row_entries(row) {
            add_at(&mut result, row, column, scale * value);
        }
    }
    result
}

fn multiply(matrix: &SparseMatrix, vector: &[f64]) -> Vec<f64> {
    (0..matrix.size())
        .map(|row| {
            matrix
                .row_entries(row)
                .iter()
                .map(|(column, value)| value * vector[*column])
                .sum()
        })
        .collect()
}

fn expand(reduced: &[f64], free_dofs: &[usize], size: usize) -> Vec<f64> {
    let mut result = vec![0.0; size];
    for (index, &dof) in free_dofs.iter().enumerate() {
        result[dof] = reduced[index];
    }
    result
}

fn relative_residual(matrix: &SparseMatrix, solution: &[f64], rhs: &[f64]) -> f64 {
    let residual = multiply(matrix, solution)
        .into_iter()
        .zip(rhs)
        .map(|(left, right)| left - right)
        .map(|value| value * value)
        .sum::<f64>()
        .sqrt();
    let scale = rhs
        .iter()
        .map(|value| value * value)
        .sum::<f64>()
        .sqrt()
        .max(1.0);
    residual / scale
}

fn imperfection_amplification(initial: &[f64], displacement: &[f64]) -> f64 {
    let mut numerator = 0.0;
    let mut denominator = 0.0;
    for node in 0..initial.len() / 3 {
        for offset in 0..2 {
            numerator += initial[node * 3 + offset] * displacement[node * 3 + offset];
            denominator += initial[node * 3 + offset].powi(2);
        }
    }
    1.0 + numerator / denominator.max(f64::MIN_POSITIVE)
}

fn max_translation(displacements: &[f64]) -> f64 {
    (0..displacements.len() / 3)
        .map(|node| (displacements[node * 3].powi(2) + displacements[node * 3 + 1].powi(2)).sqrt())
        .fold(0.0_f64, f64::max)
}
