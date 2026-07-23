use kyuubiki_protocol::{Frame2dEquilibriumPathEvent, Frame2dPDeltaStepResult};

pub(crate) fn annotate_path_events(steps: &mut [Frame2dPDeltaStepResult]) {
    annotate_limit_points(steps);
    annotate_tangent_transitions(steps);
}

fn annotate_limit_points(steps: &mut [Frame2dPDeltaStepResult]) {
    for index in 1..steps.len() {
        if !(steps[index - 1].converged && steps[index].converged) {
            continue;
        }
        let Some(previous) = steps[index - 1].load_factor_increment else {
            continue;
        };
        let Some(current) = steps[index].load_factor_increment else {
            continue;
        };
        let scale = steps[index - 1]
            .load_factor
            .abs()
            .max(steps[index].load_factor.abs())
            .max(1.0);
        let threshold = 1.0e-12 * scale;
        if previous > threshold && current < -threshold {
            steps[index - 1].path_event = Some(Frame2dEquilibriumPathEvent::LimitPointMaximum);
        } else if previous < -threshold && current > threshold {
            steps[index - 1].path_event = Some(Frame2dEquilibriumPathEvent::LimitPointMinimum);
        }
    }
}

fn annotate_tangent_transitions(steps: &mut [Frame2dPDeltaStepResult]) {
    for index in 1..steps.len() {
        if !(steps[index - 1].converged && steps[index].converged) {
            continue;
        }
        let (Some(previous), Some(current)) = (
            steps[index - 1].tangent_negative_pivots,
            steps[index].tangent_negative_pivots,
        ) else {
            continue;
        };
        let delta = current as i32 - previous as i32;
        steps[index].tangent_negative_pivot_delta = Some(delta);
        if delta != 0 && !near_limit_point(steps, index) {
            steps[index].path_event = Some(Frame2dEquilibriumPathEvent::BifurcationCandidate);
        }
    }
}

fn near_limit_point(steps: &[Frame2dPDeltaStepResult], transition_end: usize) -> bool {
    let start = transition_end.saturating_sub(2);
    let end = (transition_end + 1).min(steps.len().saturating_sub(1));
    steps[start..=end].iter().any(|step| {
        matches!(
            step.path_event,
            Some(
                Frame2dEquilibriumPathEvent::LimitPointMaximum
                    | Frame2dEquilibriumPathEvent::LimitPointMinimum
            )
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use kyuubiki_protocol::Frame2dTangentStability;

    #[test]
    fn inertia_change_without_load_reversal_marks_a_bifurcation_candidate() {
        let mut steps = vec![step(1, 0.1, 0), step(2, 0.1, 1)];
        annotate_path_events(&mut steps);
        assert_eq!(steps[1].tangent_negative_pivot_delta, Some(1));
        assert_eq!(
            steps[1].path_event,
            Some(Frame2dEquilibriumPathEvent::BifurcationCandidate)
        );
    }

    #[test]
    fn inertia_change_at_a_limit_point_is_not_mislabeled_as_bifurcation() {
        let mut steps = vec![step(1, 0.1, 0), step(2, -0.1, 1)];
        annotate_path_events(&mut steps);
        assert_eq!(
            steps[0].path_event,
            Some(Frame2dEquilibriumPathEvent::LimitPointMaximum)
        );
        assert_eq!(steps[1].tangent_negative_pivot_delta, Some(1));
        assert_eq!(steps[1].path_event, None);
    }

    fn step(step: usize, load_increment: f64, negative_pivots: usize) -> Frame2dPDeltaStepResult {
        Frame2dPDeltaStepResult {
            step,
            load_factor: step as f64 * load_increment,
            critical_factor_ratio: 0.0,
            iterations: 1,
            converged: true,
            achieved_load_factor: None,
            substeps: 1,
            cutbacks: 0,
            failure_reason: None,
            failure_detail: None,
            arc_length_constraint_error: None,
            arc_length_radius: None,
            load_factor_increment: Some(load_increment),
            path_event: None,
            tangent_stability: Some(Frame2dTangentStability::PositiveDefinite),
            tangent_negative_pivots: Some(negative_pivots),
            tangent_near_zero_pivots: Some(0),
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
            residual_norm: 0.0,
            imperfection_amplification: 1.0,
            max_incremental_displacement: 0.0,
            displacements: Vec::new(),
        }
    }
}
