use crate::frame_2d_corotational::correct_corotational_equilibrium;
use crate::frame_2d_corotational_element::assemble_tangent_and_internal;
use crate::frame_2d_stability::Frame2dStabilitySystem;
use crate::linear_algebra::reduce_sparse_system;
use crate::symmetric_critical_mode::{SymmetricCriticalMode, extract_symmetric_critical_mode};
use crate::symmetric_inertia::assess_symmetric_inertia;
use kyuubiki_protocol::{Frame2dElementInput, Frame2dPDeltaStepResult};

pub(crate) struct TransitionRefinementContext<'a> {
    pub(crate) positions: &'a [(f64, f64)],
    pub(crate) elements: &'a [Frame2dElementInput],
    pub(crate) system: &'a Frame2dStabilitySystem,
    pub(crate) free_dofs: &'a [usize],
    pub(crate) max_iterations: usize,
    pub(crate) tolerance: f64,
    pub(crate) refinement_steps: usize,
}

pub(crate) struct TangentTransitionRefinement {
    pub(crate) load_factor_min: f64,
    pub(crate) load_factor_max: f64,
    pub(crate) refinements: usize,
    pub(crate) critical_load_factor: Option<f64>,
    pub(crate) critical_mode: Option<SymmetricCriticalMode>,
    pub(crate) critical_displacement: Option<Vec<f64>>,
}

struct Endpoint {
    load_factor: f64,
    displacement: Vec<f64>,
    negative_pivots: usize,
}

pub(crate) fn refine_tangent_transition(
    context: &TransitionRefinementContext<'_>,
    lower_step: &Frame2dPDeltaStepResult,
    upper_load_factor: f64,
    upper_displacement: &[f64],
    upper_negative_pivots: usize,
) -> Result<Option<TangentTransitionRefinement>, String> {
    let Some(lower_negative_pivots) = lower_step.tangent_negative_pivots else {
        return Ok(None);
    };
    if context.refinement_steps == 0
        || lower_negative_pivots == upper_negative_pivots
        || lower_step.displacements.len() != upper_displacement.len()
    {
        return Ok(None);
    }
    let mut lower = Endpoint {
        load_factor: lower_step.load_factor,
        displacement: lower_step.displacements.clone(),
        negative_pivots: lower_negative_pivots,
    };
    let mut upper = Endpoint {
        load_factor: upper_load_factor,
        displacement: upper_displacement.to_vec(),
        negative_pivots: upper_negative_pivots,
    };
    let mut refinements = 0;
    for _ in 0..context.refinement_steps {
        let midpoint_load = 0.5 * (lower.load_factor + upper.load_factor);
        if midpoint_load == lower.load_factor || midpoint_load == upper.load_factor {
            break;
        }
        let guess = lower
            .displacement
            .iter()
            .zip(&upper.displacement)
            .map(|(lower, upper)| 0.5 * (lower + upper))
            .collect::<Vec<_>>();
        let Some(midpoint_displacement) = correct_midpoint(
            context,
            &guess,
            &lower.displacement,
            &upper.displacement,
            midpoint_load,
        )?
        else {
            return Ok(None);
        };
        let Some(midpoint_negative_pivots) = negative_pivots(context, &midpoint_displacement)?
        else {
            return Ok(None);
        };
        let midpoint = Endpoint {
            load_factor: midpoint_load,
            displacement: midpoint_displacement,
            negative_pivots: midpoint_negative_pivots,
        };
        if midpoint.negative_pivots == lower.negative_pivots {
            lower = midpoint;
        } else if midpoint.negative_pivots == upper.negative_pivots {
            upper = midpoint;
        } else {
            return Ok(None);
        }
        refinements += 1;
    }
    let lower_mode = endpoint_mode(context, &lower)?;
    let upper_mode = endpoint_mode(context, &upper)?;
    let (critical_load_factor, critical_mode, critical_displacement) =
        closest_mode(&lower, lower_mode, &upper, upper_mode);
    Ok(Some(TangentTransitionRefinement {
        load_factor_min: lower.load_factor.min(upper.load_factor),
        load_factor_max: lower.load_factor.max(upper.load_factor),
        refinements,
        critical_load_factor,
        critical_mode,
        critical_displacement,
    }))
}

fn correct_midpoint(
    context: &TransitionRefinementContext<'_>,
    interpolated: &[f64],
    lower: &[f64],
    upper: &[f64],
    load_factor: f64,
) -> Result<Option<Vec<f64>>, String> {
    for initial in [interpolated, lower, upper] {
        if let Some(displacement) = correct_corotational_equilibrium(
            context.positions,
            context.elements,
            context.system,
            initial,
            load_factor,
            context.max_iterations,
            context.tolerance,
        )? {
            return Ok(Some(displacement));
        }
    }
    Ok(None)
}

fn negative_pivots(
    context: &TransitionRefinementContext<'_>,
    displacement: &[f64],
) -> Result<Option<usize>, String> {
    let (tangent, _) =
        assemble_tangent_and_internal(context.positions, context.elements, displacement)?;
    let (reduced, _, _) = reduce_sparse_system(
        &tangent,
        &context.system.reference_force,
        &context.system.constrained_dofs,
    );
    Ok(assess_symmetric_inertia(&reduced).negative_pivots)
}

fn endpoint_mode(
    context: &TransitionRefinementContext<'_>,
    endpoint: &Endpoint,
) -> Result<Option<SymmetricCriticalMode>, String> {
    let (tangent, _) =
        assemble_tangent_and_internal(context.positions, context.elements, &endpoint.displacement)?;
    let (reduced, _, free) = reduce_sparse_system(
        &tangent,
        &context.system.reference_force,
        &context.system.constrained_dofs,
    );
    debug_assert_eq!(free, context.free_dofs);
    Ok(extract_symmetric_critical_mode(
        &reduced,
        context.free_dofs,
        endpoint.displacement.len(),
    ))
}

fn closest_mode(
    lower_endpoint: &Endpoint,
    lower: Option<SymmetricCriticalMode>,
    upper_endpoint: &Endpoint,
    upper: Option<SymmetricCriticalMode>,
) -> (Option<f64>, Option<SymmetricCriticalMode>, Option<Vec<f64>>) {
    match (lower, upper) {
        (Some(lower), Some(upper))
            if lower.normalized_eigenvalue.abs() <= upper.normalized_eigenvalue.abs() =>
        {
            (
                Some(lower_endpoint.load_factor),
                Some(lower),
                Some(lower_endpoint.displacement.clone()),
            )
        }
        (Some(_), Some(upper)) => (
            Some(upper_endpoint.load_factor),
            Some(upper),
            Some(upper_endpoint.displacement.clone()),
        ),
        (Some(lower), None) => (
            Some(lower_endpoint.load_factor),
            Some(lower),
            Some(lower_endpoint.displacement.clone()),
        ),
        (None, Some(upper)) => (
            Some(upper_endpoint.load_factor),
            Some(upper),
            Some(upper_endpoint.displacement.clone()),
        ),
        (None, None) => (None, None, None),
    }
}
