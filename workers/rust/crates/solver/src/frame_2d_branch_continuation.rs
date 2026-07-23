use crate::frame_2d_arc_length::{ArcLengthAttempt, ArcLengthState, solve_arc_length_step};
use crate::frame_2d_path_events::annotate_branch_path_events;
use crate::frame_2d_stability::Frame2dStabilitySystem;
use kyuubiki_protocol::{
    Frame2dBranchContinuationStepResult, Frame2dBranchSwitchProbeResult, Frame2dElementInput,
    Frame2dPDeltaFailureReason,
};

pub(crate) struct BranchContinuationContext<'a> {
    pub(crate) positions: &'a [(f64, f64)],
    pub(crate) elements: &'a [Frame2dElementInput],
    pub(crate) system: &'a Frame2dStabilitySystem,
    pub(crate) free_dofs: &'a [usize],
    pub(crate) max_iterations: usize,
    pub(crate) tolerance: f64,
    pub(crate) max_cutbacks: usize,
    pub(crate) target_iterations: usize,
    pub(crate) initial_radius: f64,
    pub(crate) min_radius_ratio: Option<f64>,
    pub(crate) load_scale: f64,
}

pub(crate) fn continue_branch_probes(
    context: &BranchContinuationContext<'_>,
    critical_displacement: &[f64],
    critical_load_factor: f64,
    step_count: usize,
    probes: &mut [Frame2dBranchSwitchProbeResult],
) {
    if step_count == 0 {
        return;
    }
    for probe in probes {
        if !probe.distinct_branch {
            probe.continuation_converged = Some(false);
            probe.continuation_failure_detail =
                Some("branch continuation requires a distinct equilibrium seed".into());
            continue;
        }
        let (Some(displacement), Some(load_factor)) =
            (probe.displacements.as_deref(), probe.load_factor)
        else {
            probe.continuation_converged = Some(false);
            probe.continuation_failure_detail =
                Some("branch continuation seed is missing load or displacement state".into());
            continue;
        };
        let displacement_increment = context
            .free_dofs
            .iter()
            .map(|&dof| displacement[dof] - critical_displacement[dof])
            .collect::<Vec<_>>();
        let state = ArcLengthState {
            displacement: displacement.to_vec(),
            load_factor,
            displacement_increment,
            load_increment: load_factor - critical_load_factor,
        };
        match continue_one_branch(context, state, step_count) {
            Ok(steps) => {
                let converged =
                    steps.len() == step_count && steps.iter().all(|step| step.converged);
                probe.continuation_failure_detail = (!converged).then(|| {
                    steps
                        .last()
                        .and_then(|step| step.failure_detail.clone())
                        .unwrap_or_else(|| "branch continuation ended early".into())
                });
                probe.continuation_converged = Some(converged);
                probe.continuation_steps = steps;
            }
            Err(error) => {
                probe.continuation_converged = Some(false);
                probe.continuation_failure_detail =
                    Some(format!("branch continuation failed: {error}"));
            }
        }
    }
}

fn continue_one_branch(
    context: &BranchContinuationContext<'_>,
    mut state: ArcLengthState,
    step_count: usize,
) -> Result<Vec<Frame2dBranchContinuationStepResult>, String> {
    let nominal_radius = context.initial_radius;
    let minimum_radius = minimum_branch_radius(nominal_radius, context.min_radius_ratio);
    let mut current_radius = nominal_radius;
    let mut steps = Vec::with_capacity(step_count);
    for step in 1..=step_count {
        let mut cutbacks = 0;
        let mut total_iterations = 0;
        let attempt = loop {
            let mut attempt = solve_arc_length_step(
                context.positions,
                context.elements,
                context.system,
                context.free_dofs,
                &state,
                current_radius,
                context.load_scale,
                context.max_iterations,
                context.tolerance,
            )?;
            total_iterations += attempt.iterations;
            if attempt.converged {
                break attempt;
            }
            if cutbacks >= context.max_cutbacks {
                attempt.failure_detail =
                    cutback_detail(attempt.failure_reason, attempt.failure_detail.as_deref());
                attempt.failure_reason = Some(Frame2dPDeltaFailureReason::CutbackLimitExhausted);
                break attempt;
            }
            let Some(reduced_radius) = next_cutback_radius(current_radius, minimum_radius) else {
                attempt.failure_detail = Some(radius_floor_detail(
                    attempt.failure_reason,
                    attempt.failure_detail.as_deref(),
                    current_radius,
                    minimum_radius,
                ));
                attempt.failure_reason = Some(Frame2dPDeltaFailureReason::IncrementTooSmall);
                break attempt;
            };
            current_radius = reduced_radius;
            cutbacks += 1;
        };
        let converged = attempt.converged;
        let iterations = attempt.iterations;
        steps.push(step_result(
            step,
            total_iterations,
            cutbacks,
            current_radius,
            &attempt,
        ));
        state = attempt.state;
        if !converged {
            break;
        }
        let adaptation = (context.target_iterations as f64 / iterations as f64)
            .sqrt()
            .clamp(0.5, 2.0);
        current_radius = (current_radius * adaptation).clamp(minimum_radius, nominal_radius);
    }
    annotate_branch_path_events(&mut steps);
    Ok(steps)
}

fn step_result(
    step: usize,
    iterations: usize,
    cutbacks: usize,
    radius: f64,
    attempt: &ArcLengthAttempt,
) -> Frame2dBranchContinuationStepResult {
    Frame2dBranchContinuationStepResult {
        step,
        load_factor: attempt.state.load_factor,
        load_factor_increment: if attempt.converged {
            attempt.state.load_increment
        } else {
            0.0
        },
        iterations,
        converged: attempt.converged,
        cutbacks,
        failure_reason: attempt.failure_reason,
        failure_detail: attempt.failure_detail.clone(),
        residual_norm: attempt.residual_norm,
        arc_length_constraint_error: attempt.constraint_error,
        arc_length_radius: radius,
        tangent_stability: attempt.tangent_stability,
        tangent_negative_pivots: attempt.tangent_negative_pivots,
        tangent_near_zero_pivots: attempt.tangent_near_zero_pivots,
        tangent_negative_pivot_delta: None,
        path_event: None,
        displacements: attempt.state.displacement.clone(),
    }
}

fn cutback_detail(
    reason: Option<Frame2dPDeltaFailureReason>,
    detail: Option<&str>,
) -> Option<String> {
    match (reason, detail) {
        (Some(reason), Some(detail)) => Some(format!("last attempt: {reason:?}: {detail}")),
        (Some(reason), None) => Some(format!("last attempt: {reason:?}")),
        (None, Some(detail)) => Some(detail.to_string()),
        (None, None) => None,
    }
}

fn next_cutback_radius(current_radius: f64, minimum_radius: f64) -> Option<f64> {
    let reduced_radius = current_radius * 0.5;
    (reduced_radius >= minimum_radius).then_some(reduced_radius)
}

fn minimum_branch_radius(nominal_radius: f64, ratio: Option<f64>) -> f64 {
    ratio
        .map(|ratio| nominal_radius * ratio)
        .unwrap_or(f64::EPSILON * nominal_radius.max(1.0))
        .min(nominal_radius)
}

fn radius_floor_detail(
    reason: Option<Frame2dPDeltaFailureReason>,
    detail: Option<&str>,
    current_radius: f64,
    minimum_radius: f64,
) -> String {
    let floor = format!(
        "branch radius floor reached: current={current_radius:.6e}, minimum={minimum_radius:.6e}"
    );
    match cutback_detail(reason, detail) {
        Some(last_attempt) => format!("{floor}; {last_attempt}"),
        None => floor,
    }
}

#[cfg(test)]
mod tests {
    use super::{minimum_branch_radius, next_cutback_radius};

    #[test]
    fn cutback_reaches_but_never_crosses_the_radius_floor() {
        assert_eq!(next_cutback_radius(0.2, 0.05), Some(0.1));
        assert_eq!(next_cutback_radius(0.1, 0.05), Some(0.05));
        assert_eq!(next_cutback_radius(0.05, 0.05), None);
    }

    #[test]
    fn machine_floor_never_exceeds_a_tiny_nominal_radius() {
        let nominal_radius = f64::MIN_POSITIVE;
        assert_eq!(minimum_branch_radius(nominal_radius, None), nominal_radius);
        assert_eq!(
            minimum_branch_radius(nominal_radius, Some(0.25)),
            nominal_radius * 0.25
        );
    }
}
