use crate::frame_2d_p_delta::solve_frame_2d_p_delta;
use kyuubiki_protocol::{
    Frame2dPDeltaPathAttemptResult, Frame2dStabilityKinematics, Frame2dStabilityPathControl,
    SolveFrame2dPDeltaPathRequest, SolveFrame2dPDeltaPathResult, SolveFrame2dPDeltaRequest,
    SolveFrame2dPDeltaResult,
};

const DEFAULT_MAX_SUBDIVISIONS: usize = 4;
const DEFAULT_MINIMUM_STEP_FRACTION: f64 = 1.0 / 256.0;
const MAX_PATH_POINTS: usize = 64;
const MAX_SUBDIVISIONS: usize = 8;

struct AcceptedPoint {
    request: SolveFrame2dPDeltaRequest,
    result: SolveFrame2dPDeltaResult,
}

struct PathControls {
    max_subdivisions: usize,
    minimum_step_fraction: f64,
    minimum_state_overlap: Option<f64>,
    minimum_branch_shape_overlap: Option<f64>,
}

pub fn solve_frame_2d_p_delta_path(
    request: &SolveFrame2dPDeltaPathRequest,
) -> Result<SolveFrame2dPDeltaPathResult, String> {
    solve_path_with(request, &mut solve_frame_2d_p_delta)
}

fn solve_path_with(
    request: &SolveFrame2dPDeltaPathRequest,
    solve_point: &mut impl FnMut(&SolveFrame2dPDeltaRequest) -> Result<SolveFrame2dPDeltaResult, String>,
) -> Result<SolveFrame2dPDeltaPathResult, String> {
    let controls = validate_path_request(request)?;
    let mut attempts = Vec::new();
    let first_request = request.points[0].clone();
    let first = match solve_point(&first_request) {
        Ok(result) => result,
        Err(error) => {
            attempts.push(failed_attempt(0, 1.0, 0, false, error, None));
            return Ok(path_result(request, attempts, 0));
        }
    };
    let first_converged = reusable_result(&first);
    attempts.push(Frame2dPDeltaPathAttemptResult {
        requested_point_index: 0,
        target_fraction: 1.0,
        subdivision_level: 0,
        inserted: false,
        converged: first_converged,
        state_overlap: None,
        branch_shape_overlap: None,
        failure_detail: (!first_converged).then(|| result_failure_detail(&first)),
        result: Some(first.clone()),
    });
    if !first_converged {
        return Ok(path_result(request, attempts, 0));
    }

    let mut accepted = AcceptedPoint {
        request: first_request,
        result: first,
    };
    let mut completed = 1;
    for (index, target) in request.points.iter().enumerate().skip(1) {
        let Some(next) = advance_segment(
            &accepted,
            target,
            index,
            0.0,
            1.0,
            0,
            &controls,
            &mut attempts,
            solve_point,
        )?
        else {
            break;
        };
        accepted = next;
        completed += 1;
    }
    Ok(path_result(request, attempts, completed))
}

#[allow(clippy::too_many_arguments)]
fn advance_segment(
    source: &AcceptedPoint,
    target: &SolveFrame2dPDeltaRequest,
    requested_point_index: usize,
    source_fraction: f64,
    target_fraction: f64,
    subdivision_level: usize,
    controls: &PathControls,
    attempts: &mut Vec<Frame2dPDeltaPathAttemptResult>,
    solve_point: &mut impl FnMut(&SolveFrame2dPDeltaRequest) -> Result<SolveFrame2dPDeltaResult, String>,
) -> Result<Option<AcceptedPoint>, String> {
    let mut attempted_request = target.clone();
    attempted_request.continuation_state = source.result.continuation_state.clone();
    if controls.minimum_branch_shape_overlap.is_some() {
        transport_state_to_branch_shape(&mut attempted_request);
    }
    let solved = solve_point(&attempted_request);
    let state_overlap = solved
        .as_ref()
        .ok()
        .and_then(|result| continuation_state_overlap(&source.result, result));
    let branch_shape_overlap = solved
        .as_ref()
        .ok()
        .and_then(|result| branch_shape_overlap(&attempted_request, result));
    let reusable = solved.as_ref().is_ok_and(reusable_result);
    let identity_accepted = identity_overlap_accepted(
        state_overlap,
        controls.minimum_state_overlap,
        branch_shape_overlap,
        controls.minimum_branch_shape_overlap,
    );
    let converged = reusable && identity_accepted;
    let failure_detail = match &solved {
        Ok(result) if !reusable => Some(result_failure_detail(result)),
        Ok(_) if !identity_accepted => Some(identity_failure_detail(
            state_overlap,
            controls.minimum_state_overlap,
            branch_shape_overlap,
            controls.minimum_branch_shape_overlap,
        )),
        Err(error) => Some(error.clone()),
        _ => None,
    };
    attempts.push(Frame2dPDeltaPathAttemptResult {
        requested_point_index,
        target_fraction,
        subdivision_level,
        inserted: target_fraction < 1.0,
        converged,
        state_overlap,
        branch_shape_overlap,
        failure_detail,
        result: solved.as_ref().ok().cloned(),
    });
    if let Ok(result) = solved
        && converged
    {
        return Ok(Some(AcceptedPoint {
            request: attempted_request,
            result,
        }));
    }

    let width = target_fraction - source_fraction;
    if subdivision_level >= controls.max_subdivisions
        || width * 0.5 < controls.minimum_step_fraction
    {
        return Ok(None);
    }
    let midpoint_fraction = 0.5 * (source_fraction + target_fraction);
    let midpoint_ratio = (midpoint_fraction - source_fraction) / width;
    let midpoint_request = interpolate_request(&source.request, target, midpoint_ratio)?;
    let Some(midpoint) = advance_segment(
        source,
        &midpoint_request,
        requested_point_index,
        source_fraction,
        midpoint_fraction,
        subdivision_level + 1,
        controls,
        attempts,
        solve_point,
    )?
    else {
        return Ok(None);
    };
    advance_segment(
        &midpoint,
        target,
        requested_point_index,
        midpoint_fraction,
        target_fraction,
        subdivision_level + 1,
        controls,
        attempts,
        solve_point,
    )
}

fn validate_path_request(request: &SolveFrame2dPDeltaPathRequest) -> Result<PathControls, String> {
    if !(2..=MAX_PATH_POINTS).contains(&request.points.len()) {
        return Err(format!(
            "frame 2d p-delta path must contain between 2 and {MAX_PATH_POINTS} points"
        ));
    }
    let max_subdivisions = request.max_subdivisions.unwrap_or(DEFAULT_MAX_SUBDIVISIONS);
    if max_subdivisions > MAX_SUBDIVISIONS {
        return Err(format!(
            "frame 2d p-delta path max_subdivisions must not exceed {MAX_SUBDIVISIONS}"
        ));
    }
    let minimum_step_fraction = request
        .minimum_step_fraction
        .unwrap_or(DEFAULT_MINIMUM_STEP_FRACTION);
    if !(minimum_step_fraction.is_finite()
        && minimum_step_fraction > 0.0
        && minimum_step_fraction <= 0.5)
    {
        return Err(
            "frame 2d p-delta path minimum_step_fraction must be finite and in (0, 0.5]".into(),
        );
    }
    if request
        .minimum_state_overlap
        .is_some_and(|overlap| !(overlap.is_finite() && overlap > 0.0 && overlap <= 1.0))
    {
        return Err(
            "frame 2d p-delta path minimum_state_overlap must be finite and in (0, 1]".into(),
        );
    }
    if request
        .minimum_branch_shape_overlap
        .is_some_and(|overlap| !(overlap.is_finite() && overlap > 0.0 && overlap <= 1.0))
    {
        return Err(
            "frame 2d p-delta path minimum_branch_shape_overlap must be finite and in (0, 1]"
                .into(),
        );
    }
    for (index, point) in request.points.iter().enumerate() {
        if point.kinematics != Frame2dStabilityKinematics::Corotational
            || point.path_control != Frame2dStabilityPathControl::ArcLength
        {
            return Err(format!(
                "frame 2d p-delta path point {index} requires corotational arc-length control"
            ));
        }
        if index > 0 && point.continuation_state.is_some() {
            return Err(format!(
                "frame 2d p-delta path point {index} must not provide continuation_state"
            ));
        }
        if index > 0 {
            validate_compatible_models(&request.points[index - 1], point)?;
        }
        if request.minimum_branch_shape_overlap.is_some() && point.imperfection_shape.is_none() {
            return Err(format!(
                "frame 2d p-delta path point {index} requires imperfection_shape when branch shape overlap is enabled"
            ));
        }
    }
    Ok(PathControls {
        max_subdivisions,
        minimum_step_fraction,
        minimum_state_overlap: request.minimum_state_overlap,
        minimum_branch_shape_overlap: request.minimum_branch_shape_overlap,
    })
}

fn validate_compatible_models(
    left: &SolveFrame2dPDeltaRequest,
    right: &SolveFrame2dPDeltaRequest,
) -> Result<(), String> {
    let left_frame = &left.buckling.frame;
    let right_frame = &right.buckling.frame;
    if left_frame.nodes.len() != right_frame.nodes.len()
        || left_frame.elements.len() != right_frame.elements.len()
    {
        return Err("frame 2d p-delta path points must share frame topology".into());
    }
    for (index, (left, right)) in left_frame.nodes.iter().zip(&right_frame.nodes).enumerate() {
        if left.id != right.id
            || left.fix_x != right.fix_x
            || left.fix_y != right.fix_y
            || left.fix_rz != right.fix_rz
        {
            return Err(format!(
                "frame 2d p-delta path node {index} identity and constraints must match"
            ));
        }
    }
    for (index, (left, right)) in left_frame
        .elements
        .iter()
        .zip(&right_frame.elements)
        .enumerate()
    {
        if left.id != right.id || left.node_i != right.node_i || left.node_j != right.node_j {
            return Err(format!(
                "frame 2d p-delta path element {index} identity and connectivity must match"
            ));
        }
    }
    match (&left.imperfection_shape, &right.imperfection_shape) {
        (Some(left), Some(right)) if left.len() == right.len() => {}
        (None, None) if left.imperfection_mode_index == right.imperfection_mode_index => {}
        _ => {
            return Err(
                "frame 2d p-delta path imperfection sources and dimensions must match".into(),
            );
        }
    }
    Ok(())
}

fn interpolate_request(
    left: &SolveFrame2dPDeltaRequest,
    right: &SolveFrame2dPDeltaRequest,
    ratio: f64,
) -> Result<SolveFrame2dPDeltaRequest, String> {
    validate_compatible_models(left, right)?;
    let mut interpolated = right.clone();
    interpolated.imperfection_amplitude = lerp(
        left.imperfection_amplitude,
        right.imperfection_amplitude,
        ratio,
    );
    for (node, (left, right)) in interpolated.buckling.frame.nodes.iter_mut().zip(
        left.buckling
            .frame
            .nodes
            .iter()
            .zip(&right.buckling.frame.nodes),
    ) {
        node.x = lerp(left.x, right.x, ratio);
        node.y = lerp(left.y, right.y, ratio);
        node.load_x = lerp(left.load_x, right.load_x, ratio);
        node.load_y = lerp(left.load_y, right.load_y, ratio);
        node.moment_z = lerp(left.moment_z, right.moment_z, ratio);
    }
    for (element, (left, right)) in interpolated.buckling.frame.elements.iter_mut().zip(
        left.buckling
            .frame
            .elements
            .iter()
            .zip(&right.buckling.frame.elements),
    ) {
        element.area = lerp(left.area, right.area, ratio);
        element.youngs_modulus = lerp(left.youngs_modulus, right.youngs_modulus, ratio);
        element.moment_of_inertia = lerp(left.moment_of_inertia, right.moment_of_inertia, ratio);
        element.section_modulus = lerp(left.section_modulus, right.section_modulus, ratio);
    }
    if let (Some(values), Some(left), Some(right)) = (
        interpolated.imperfection_shape.as_mut(),
        left.imperfection_shape.as_ref(),
        right.imperfection_shape.as_ref(),
    ) {
        for (value, (left, right)) in values.iter_mut().zip(left.iter().zip(right)) {
            *value = lerp(*left, *right, ratio);
        }
    }
    interpolated.continuation_state = None;
    Ok(interpolated)
}

fn reusable_result(result: &SolveFrame2dPDeltaResult) -> bool {
    result.converged && result.continuation_state.is_some()
}

fn continuation_state_overlap(
    left: &SolveFrame2dPDeltaResult,
    right: &SolveFrame2dPDeltaResult,
) -> Option<f64> {
    let left = &left.continuation_state.as_ref()?.displacement_increment;
    let right = &right.continuation_state.as_ref()?.displacement_increment;
    if left.len() != right.len() {
        return None;
    }
    let dot = left
        .iter()
        .zip(right)
        .map(|(left, right)| left * right)
        .sum::<f64>();
    let left_norm = left.iter().map(|value| value * value).sum::<f64>().sqrt();
    let right_norm = right.iter().map(|value| value * value).sum::<f64>().sqrt();
    let denominator = left_norm * right_norm;
    (denominator > f64::EPSILON).then(|| (dot / denominator).abs().clamp(0.0, 1.0))
}

fn branch_shape_overlap(
    request: &SolveFrame2dPDeltaRequest,
    result: &SolveFrame2dPDeltaResult,
) -> Option<f64> {
    let shape = request.imperfection_shape.as_ref()?;
    cosine_overlap_on_shape_support(shape, &result.final_displacements)
}

fn transport_state_to_branch_shape(request: &mut SolveFrame2dPDeltaRequest) {
    let Some(shape) = request.imperfection_shape.as_deref() else {
        return;
    };
    let Some(state) = request.continuation_state.as_mut() else {
        return;
    };
    reorient_on_shape_support(&mut state.displacements, shape);
    reorient_on_shape_support(&mut state.displacement_increment, shape);
}

fn reorient_on_shape_support(values: &mut [f64], shape: &[f64]) {
    if shape.len() != values.len() {
        return;
    }
    let scale = shape
        .iter()
        .map(|value| value.abs())
        .fold(0.0_f64, f64::max);
    if scale <= f64::EPSILON {
        return;
    }
    let support_tolerance = scale * 1.0e-12;
    let (dot, shape_norm, value_norm) = shape
        .iter()
        .zip(values.iter())
        .filter(|(shape, _)| shape.abs() > support_tolerance)
        .fold((0.0, 0.0, 0.0), |(dot, left, right), (shape, value)| {
            (
                dot + shape * value,
                left + shape * shape,
                right + value * value,
            )
        });
    if shape_norm <= f64::EPSILON || value_norm <= f64::EPSILON {
        return;
    }
    let orientation = if dot < 0.0 { -1.0 } else { 1.0 };
    let signed_scale = orientation * (value_norm / shape_norm).sqrt();
    for (value, shape) in values.iter_mut().zip(shape) {
        if shape.abs() > support_tolerance {
            *value = signed_scale * shape;
        }
    }
}

fn identity_overlap_accepted(
    state_overlap: Option<f64>,
    minimum_state_overlap: Option<f64>,
    branch_shape_overlap: Option<f64>,
    minimum_branch_shape_overlap: Option<f64>,
) -> bool {
    if let Some(minimum) = minimum_branch_shape_overlap {
        return branch_shape_overlap.is_some_and(|value| value >= minimum);
    }
    minimum_state_overlap.is_none_or(|minimum| state_overlap.is_some_and(|value| value >= minimum))
}

fn identity_failure_detail(
    state_overlap: Option<f64>,
    minimum_state_overlap: Option<f64>,
    branch_shape_overlap: Option<f64>,
    minimum_branch_shape_overlap: Option<f64>,
) -> String {
    match (minimum_state_overlap, minimum_branch_shape_overlap) {
        (Some(state_minimum), Some(shape_minimum)) => format!(
            "frame 2d p-delta parameter branch shape overlap {:.6} is below required {:.6}; diagnostic state overlap {:.6} against {:.6}",
            branch_shape_overlap.unwrap_or(0.0),
            shape_minimum,
            state_overlap.unwrap_or(0.0),
            state_minimum
        ),
        (Some(minimum), None) => format!(
            "frame 2d p-delta parameter state overlap {:.6} is below required {:.6}",
            state_overlap.unwrap_or(0.0),
            minimum
        ),
        (None, Some(minimum)) => format!(
            "frame 2d p-delta parameter branch shape overlap {:.6} is below required {:.6}",
            branch_shape_overlap.unwrap_or(0.0),
            minimum
        ),
        (None, None) => "frame 2d p-delta parameter identity check failed".into(),
    }
}

fn cosine_overlap_on_shape_support(shape: &[f64], values: &[f64]) -> Option<f64> {
    if shape.len() != values.len() {
        return None;
    }
    let scale = shape
        .iter()
        .map(|value| value.abs())
        .fold(0.0_f64, f64::max);
    if scale <= f64::EPSILON {
        return None;
    }
    let support_tolerance = scale * 1.0e-12;
    let (dot, shape_norm, value_norm) = shape
        .iter()
        .zip(values)
        .filter(|(shape, _)| shape.abs() > support_tolerance)
        .fold((0.0, 0.0, 0.0), |(dot, left, right), (shape, value)| {
            (
                dot + shape * value,
                left + shape * shape,
                right + value * value,
            )
        });
    let denominator = (shape_norm * value_norm).sqrt();
    (denominator > f64::EPSILON).then(|| (dot / denominator).abs().clamp(0.0, 1.0))
}

fn result_failure_detail(result: &SolveFrame2dPDeltaResult) -> String {
    result
        .steps
        .last()
        .and_then(|step| step.failure_detail.clone())
        .unwrap_or_else(|| "frame 2d p-delta path point did not produce a reusable state".into())
}

fn failed_attempt(
    requested_point_index: usize,
    target_fraction: f64,
    subdivision_level: usize,
    inserted: bool,
    failure_detail: String,
    result: Option<SolveFrame2dPDeltaResult>,
) -> Frame2dPDeltaPathAttemptResult {
    Frame2dPDeltaPathAttemptResult {
        requested_point_index,
        target_fraction,
        subdivision_level,
        inserted,
        converged: false,
        state_overlap: None,
        branch_shape_overlap: None,
        failure_detail: Some(failure_detail),
        result,
    }
}

fn path_result(
    request: &SolveFrame2dPDeltaPathRequest,
    attempts: Vec<Frame2dPDeltaPathAttemptResult>,
    completed_point_count: usize,
) -> SolveFrame2dPDeltaPathResult {
    let adaptive_insertion_count = attempts
        .iter()
        .filter(|attempt| attempt.inserted && attempt.converged)
        .count();
    SolveFrame2dPDeltaPathResult {
        input: request.clone(),
        attempts,
        completed_point_count,
        adaptive_insertion_count,
        converged: completed_point_count == request.points.len(),
    }
}

fn lerp(left: f64, right: f64, ratio: f64) -> f64 {
    left + (right - left) * ratio
}

#[cfg(test)]
mod tests {
    use super::*;
    use kyuubiki_protocol::{
        Frame2dBranchSwitchSelection, Frame2dElementInput, Frame2dNodeInput,
        SolveBucklingFrame2dRequest, SolveFrame2dRequest,
    };

    #[test]
    fn target_shape_is_authoritative_and_state_overlap_is_the_fallback() {
        assert!(identity_overlap_accepted(
            Some(0.05),
            Some(0.75),
            Some(0.99),
            Some(0.75)
        ));
        assert!(!identity_overlap_accepted(
            Some(0.99),
            Some(0.75),
            Some(0.05),
            Some(0.75)
        ));
        assert!(!identity_overlap_accepted(
            Some(0.05),
            Some(0.75),
            Some(0.10),
            Some(0.75)
        ));
        assert!(identity_overlap_accepted(
            Some(0.99),
            Some(0.75),
            None,
            None
        ));
        assert!(!identity_overlap_accepted(
            None,
            None,
            Some(0.10),
            Some(0.75)
        ));
    }

    #[test]
    fn branch_shape_overlap_ignores_inactive_dofs() {
        let overlap =
            cosine_overlap_on_shape_support(&[1.0, 0.0, -1.0, 0.0], &[2.0, 1.0e6, -2.0, -1.0e6])
                .unwrap();
        assert!((overlap - 1.0).abs() < 1.0e-12);
    }

    #[test]
    fn state_transport_reorients_only_the_target_shape_support() {
        let mut values = vec![2.0, 7.0, 0.0, 11.0];
        reorient_on_shape_support(&mut values, &[1.0, 0.0, -1.0, 0.0]);
        assert!((values[0] - 2.0_f64.sqrt()).abs() < 1.0e-12);
        assert_eq!(values[1], 7.0);
        assert!((values[2] + 2.0_f64.sqrt()).abs() < 1.0e-12);
        assert_eq!(values[3], 11.0);
    }

    #[test]
    fn failed_parameter_jumps_are_recovered_by_visible_binary_subdivisions() {
        let first = column_request(1.0e-4);
        let template =
            solve_frame_2d_p_delta(&first).expect("continuation result template should solve");
        let target = column_request(2.0e-4);
        let request = SolveFrame2dPDeltaPathRequest {
            points: vec![first, target],
            max_subdivisions: Some(2),
            minimum_step_fraction: Some(1.0 / 16.0),
            minimum_state_overlap: None,
            minimum_branch_shape_overlap: None,
        };
        let mut solve_point = |request: &SolveFrame2dPDeltaRequest| {
            let parameter = request.buckling.frame.elements[0].moment_of_inertia / 1.0e-4;
            if request
                .continuation_state
                .as_ref()
                .is_some_and(|state| (parameter - state.load_factor).abs() > 0.26)
            {
                return Err("synthetic parameter jump exceeded convergence basin".into());
            }
            let mut result = template.clone();
            result.input = request.clone();
            result.converged = true;
            let state = result.continuation_state.as_mut().unwrap();
            state.load_factor = parameter;
            Ok(result)
        };

        let result = solve_path_with(&request, &mut solve_point).unwrap();

        assert!(result.converged);
        assert_eq!(result.completed_point_count, 2);
        assert_eq!(result.adaptive_insertion_count, 3);
        assert_eq!(
            result
                .attempts
                .iter()
                .filter(|attempt| !attempt.converged)
                .count(),
            3
        );
        let accepted_fractions = result
            .attempts
            .iter()
            .filter(|attempt| attempt.converged && attempt.requested_point_index == 1)
            .map(|attempt| attempt.target_fraction)
            .collect::<Vec<_>>();
        assert_eq!(accepted_fractions, vec![0.25, 0.5, 0.75, 1.0]);
    }

    #[test]
    fn topology_changes_are_rejected_before_path_execution() {
        let first = column_request(1.0e-4);
        let mut target = column_request(1.1e-4);
        target.buckling.frame.elements[0].node_j = 2;
        let error = solve_frame_2d_p_delta_path(&SolveFrame2dPDeltaPathRequest {
            points: vec![first, target],
            max_subdivisions: None,
            minimum_step_fraction: None,
            minimum_state_overlap: None,
            minimum_branch_shape_overlap: None,
        })
        .expect_err("path topology mismatch must fail");
        assert!(error.contains("identity and connectivity"));
    }

    fn column_request(moment_of_inertia: f64) -> SolveFrame2dPDeltaRequest {
        let nodes = vec![
            column_node("n0", 0.0, true, true, 0.0),
            column_node("n1", 0.5, false, false, 0.0),
            column_node("n2", 1.0, true, false, -1.0),
        ];
        let elements = (0..2)
            .map(|index| Frame2dElementInput {
                id: format!("e{index}"),
                node_i: index,
                node_j: index + 1,
                area: 0.1,
                youngs_modulus: 1.0e6,
                moment_of_inertia,
                section_modulus: 1.0,
            })
            .collect();
        SolveFrame2dPDeltaRequest {
            buckling: SolveBucklingFrame2dRequest {
                frame: SolveFrame2dRequest { nodes, elements },
                mode_count: Some(1),
            },
            imperfection_amplitude: 1.0e-6,
            kinematics: Frame2dStabilityKinematics::Corotational,
            path_control: Frame2dStabilityPathControl::ArcLength,
            imperfection_shape: Some(vec![0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0]),
            imperfection_mode_index: None,
            maximum_load_factor: Some(500.0),
            load_steps: Some(4),
            max_iterations: Some(32),
            tolerance: Some(1.0e-8),
            max_step_cutbacks: Some(8),
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
            branch_switch_subspace_refinement_levels: None,
            branch_continuation_steps: None,
            branch_continuation_radius: None,
            branch_continuation_min_radius_ratio: None,
            continuation_state: None,
        }
    }

    fn column_node(id: &str, y: f64, fix_x: bool, fix_y: bool, load_y: f64) -> Frame2dNodeInput {
        Frame2dNodeInput {
            id: id.into(),
            x: 0.0,
            y,
            fix_x,
            fix_y,
            fix_rz: false,
            load_x: 0.0,
            load_y,
            moment_z: 0.0,
        }
    }
}
