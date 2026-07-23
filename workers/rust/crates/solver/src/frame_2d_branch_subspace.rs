use crate::frame_2d_branch_switch::{
    BranchSwitchContext, mark_probe_origin, probe_weighted_branch_switches,
    unavailable_weighted_branch_switches,
};
use crate::symmetric_critical_mode::SymmetricCriticalMode;
use kyuubiki_protocol::{
    Frame2dBranchProbeOrigin, Frame2dBranchSwitchProbeResult, Frame2dBranchSwitchSelection,
};

pub(crate) const MAX_SUBSPACE_SAMPLES: usize = 16;
pub(crate) const MAX_SUBSPACE_REFINEMENT_LEVELS: usize = 2;

struct DirectionFamily {
    weights: Vec<f64>,
    probes: Vec<Frame2dBranchSwitchProbeResult>,
}

struct RefinementDirection {
    weights: Vec<f64>,
    parent_angle_radians: f64,
    parent: ResponseBoundary,
}

#[derive(Clone)]
struct ResponseBoundary {
    left_weights: Vec<f64>,
    left_signature: ResponseSignature,
    right_weights: Vec<f64>,
    right_signature: ResponseSignature,
}

type ResponseSignature = Vec<(bool, bool, bool)>;

#[allow(clippy::too_many_arguments)]
pub(crate) fn probe_subspace_branch_switches(
    context: &BranchSwitchContext<'_>,
    critical_displacement: &[f64],
    primary_displacement: &[f64],
    critical_load_factor: f64,
    critical_modes: &[SymmetricCriticalMode],
    sample_count: usize,
    refinement_levels: usize,
    amplitude: f64,
    selection: Frame2dBranchSwitchSelection,
) -> Vec<Frame2dBranchSwitchProbeResult> {
    let mut families = subspace_mode_weights(critical_modes.len(), sample_count)
        .into_iter()
        .map(|weights| DirectionFamily {
            probes: probe_subspace_direction(
                context,
                critical_displacement,
                primary_displacement,
                critical_load_factor,
                critical_modes,
                &weights,
                amplitude,
                selection,
                Frame2dBranchProbeOrigin::AutomaticSubspace,
                0,
                None,
            ),
            weights,
        })
        .collect::<Vec<_>>();
    let mut boundaries = response_boundaries(&families);
    for level in 1..=refinement_levels {
        let existing_directions = families
            .iter()
            .map(|family| family.weights.clone())
            .collect::<Vec<_>>();
        let refinements = refinement_directions(&boundaries, &existing_directions, sample_count);
        if refinements.is_empty() {
            break;
        }
        let mut child_boundaries = Vec::new();
        for refinement in refinements {
            let probes = probe_subspace_direction(
                context,
                critical_displacement,
                primary_displacement,
                critical_load_factor,
                critical_modes,
                &refinement.weights,
                amplitude,
                selection,
                Frame2dBranchProbeOrigin::AdaptiveSubspace,
                level,
                Some(refinement.parent_angle_radians),
            );
            child_boundaries.extend(refined_boundaries(&refinement, response_signature(&probes)));
            families.push(DirectionFamily {
                weights: refinement.weights,
                probes,
            });
        }
        boundaries = child_boundaries;
    }
    families
        .into_iter()
        .flat_map(|family| family.probes)
        .collect()
}

#[allow(clippy::too_many_arguments)]
fn probe_subspace_direction(
    context: &BranchSwitchContext<'_>,
    critical_displacement: &[f64],
    primary_displacement: &[f64],
    critical_load_factor: f64,
    critical_modes: &[SymmetricCriticalMode],
    weights: &[f64],
    amplitude: f64,
    selection: Frame2dBranchSwitchSelection,
    origin: Frame2dBranchProbeOrigin,
    refinement_level: usize,
    parent_angle_radians: Option<f64>,
) -> Vec<Frame2dBranchSwitchProbeResult> {
    let mut probes = mark_probe_origin(
        probe_weighted_branch_switches(
            context,
            critical_displacement,
            primary_displacement,
            critical_load_factor,
            critical_modes,
            weights,
            amplitude,
            selection,
        ),
        origin,
        Some(refinement_level),
    );
    for probe in &mut probes {
        probe.subspace_parent_angle_radians = parent_angle_radians;
    }
    probes
}

pub(crate) fn unavailable_subspace_branch_switches(
    selection: Frame2dBranchSwitchSelection,
    mode_count: usize,
    sample_count: usize,
    amplitude: f64,
    detail: &str,
) -> Vec<Frame2dBranchSwitchProbeResult> {
    subspace_mode_weights(mode_count, sample_count)
        .into_iter()
        .flat_map(|weights| {
            mark_probe_origin(
                unavailable_weighted_branch_switches(selection, &weights, amplitude, detail),
                Frame2dBranchProbeOrigin::AutomaticSubspace,
                Some(0),
            )
        })
        .collect()
}

pub(crate) fn available_subspace_sample_count(mode_count: usize) -> usize {
    match mode_count {
        3 => 4,
        4 => MAX_SUBSPACE_SAMPLES,
        _ => 0,
    }
}

fn subspace_mode_weights(mode_count: usize, sample_count: usize) -> Vec<Vec<f64>> {
    let mut directions = Vec::new();
    enumerate_ternary_directions(mode_count, 0, &mut vec![0; mode_count], &mut directions);
    directions.sort_by(|left, right| {
        active_count(right)
            .cmp(&active_count(left))
            .then_with(|| right.cmp(left))
    });
    directions
        .into_iter()
        .take(sample_count)
        .map(|direction| {
            let norm = (active_count(&direction) as f64).sqrt();
            direction
                .into_iter()
                .map(|weight| weight as f64 / norm)
                .collect()
        })
        .collect()
}

fn response_signature(probes: &[Frame2dBranchSwitchProbeResult]) -> ResponseSignature {
    probes
        .iter()
        .map(|probe| {
            (
                probe.equilibrium_converged,
                probe.primary_equilibrium_converged,
                probe.distinct_branch,
            )
        })
        .collect()
}

fn response_boundaries(families: &[DirectionFamily]) -> Vec<ResponseBoundary> {
    let directions = families
        .iter()
        .map(|family| family.weights.clone())
        .collect::<Vec<_>>();
    let signatures = families
        .iter()
        .map(|family| response_signature(&family.probes))
        .collect::<Vec<_>>();
    response_boundaries_from(&directions, &signatures)
}

fn response_boundaries_from(
    directions: &[Vec<f64>],
    signatures: &[ResponseSignature],
) -> Vec<ResponseBoundary> {
    let mut boundaries = Vec::new();
    for left in 0..directions.len() {
        for right in left + 1..directions.len() {
            let left_signature = signatures[left].clone();
            let right_signature = signatures[right].clone();
            if left_signature != right_signature {
                boundaries.push(ResponseBoundary {
                    left_weights: directions[left].clone(),
                    left_signature,
                    right_weights: directions[right].clone(),
                    right_signature,
                });
            }
        }
    }
    boundaries
}

fn refinement_directions(
    boundaries: &[ResponseBoundary],
    existing_directions: &[Vec<f64>],
    limit: usize,
) -> Vec<RefinementDirection> {
    let mut ordered = boundaries.iter().collect::<Vec<_>>();
    ordered.sort_by(|left, right| boundary_angle(left).total_cmp(&boundary_angle(right)));
    let mut refinements = Vec::new();
    for boundary in ordered {
        let Some(candidate) = projective_midpoint(&boundary.left_weights, &boundary.right_weights)
        else {
            continue;
        };
        if projectively_unique(&candidate, existing_directions)
            && refinements.iter().all(|refinement: &RefinementDirection| {
                dot(&candidate, &refinement.weights).abs() < 1.0 - 1.0e-12
            })
        {
            refinements.push(RefinementDirection {
                weights: candidate,
                parent_angle_radians: boundary_angle(boundary),
                parent: boundary.clone(),
            });
            if refinements.len() == limit {
                break;
            }
        }
    }
    refinements
}

fn refined_boundaries(
    refinement: &RefinementDirection,
    midpoint_signature: ResponseSignature,
) -> Vec<ResponseBoundary> {
    let midpoint = refinement.weights.clone();
    let mut boundaries = Vec::new();
    if refinement.parent.left_signature != midpoint_signature {
        boundaries.push(ResponseBoundary {
            left_weights: refinement.parent.left_weights.clone(),
            left_signature: refinement.parent.left_signature.clone(),
            right_weights: midpoint.clone(),
            right_signature: midpoint_signature.clone(),
        });
    }
    if midpoint_signature != refinement.parent.right_signature {
        boundaries.push(ResponseBoundary {
            left_weights: midpoint,
            left_signature: midpoint_signature,
            right_weights: refinement.parent.right_weights.clone(),
            right_signature: refinement.parent.right_signature.clone(),
        });
    }
    boundaries
}

fn boundary_angle(boundary: &ResponseBoundary) -> f64 {
    dot(&boundary.left_weights, &boundary.right_weights)
        .abs()
        .clamp(-1.0, 1.0)
        .acos()
}

fn projective_midpoint(left: &[f64], right: &[f64]) -> Option<Vec<f64>> {
    if left.len() != right.len() {
        return None;
    }
    let orientation = if dot(left, right) < 0.0 { -1.0 } else { 1.0 };
    let mut midpoint = left
        .iter()
        .zip(right)
        .map(|(left, right)| left + orientation * right)
        .collect::<Vec<_>>();
    let norm = dot(&midpoint, &midpoint).sqrt();
    if !(norm.is_finite() && norm > f64::EPSILON) {
        return None;
    }
    for weight in &mut midpoint {
        *weight /= norm;
    }
    if midpoint
        .iter()
        .find(|weight| weight.abs() > 1.0e-15)
        .is_some_and(|weight| *weight < 0.0)
    {
        for weight in &mut midpoint {
            *weight = -*weight;
        }
    }
    Some(midpoint)
}

fn projectively_unique(candidate: &[f64], existing: &[Vec<f64>]) -> bool {
    existing
        .iter()
        .all(|direction| dot(candidate, direction).abs() < 1.0 - 1.0e-12)
}

fn dot(left: &[f64], right: &[f64]) -> f64 {
    left.iter()
        .zip(right)
        .map(|(left, right)| left * right)
        .sum()
}

fn enumerate_ternary_directions(
    mode_count: usize,
    index: usize,
    current: &mut [i8],
    output: &mut Vec<Vec<i8>>,
) {
    if index == mode_count {
        let first = current.iter().find(|weight| **weight != 0);
        if active_count(current) >= 3 && first.is_some_and(|weight| *weight > 0) {
            output.push(current.to_vec());
        }
        return;
    }
    for weight in [-1, 0, 1] {
        current[index] = weight;
        enumerate_ternary_directions(mode_count, index + 1, current, output);
    }
}

fn active_count(direction: &[i8]) -> usize {
    direction.iter().filter(|weight| **weight != 0).count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn three_mode_fan_is_complete_normalized_and_deterministic() {
        let directions = subspace_mode_weights(3, 4);
        let scale = 3.0_f64.sqrt();
        assert_eq!(
            directions,
            vec![
                vec![1.0 / scale, 1.0 / scale, 1.0 / scale],
                vec![1.0 / scale, 1.0 / scale, -1.0 / scale],
                vec![1.0 / scale, -1.0 / scale, 1.0 / scale],
                vec![1.0 / scale, -1.0 / scale, -1.0 / scale],
            ]
        );
        assert!(directions.iter().all(|direction| {
            (direction.iter().map(|weight| weight * weight).sum::<f64>() - 1.0).abs() < 1.0e-15
        }));
    }

    #[test]
    fn four_mode_fan_is_bounded_unique_and_full_dimensional_first() {
        let directions = subspace_mode_weights(4, MAX_SUBSPACE_SAMPLES);
        assert_eq!(directions.len(), MAX_SUBSPACE_SAMPLES);
        assert!(
            directions[..8]
                .iter()
                .all(|direction| direction.iter().all(|weight| *weight != 0.0))
        );
        for (index, direction) in directions.iter().enumerate() {
            assert!(directions[index + 1..].iter().all(|other| {
                direction
                    .iter()
                    .zip(other)
                    .map(|(left, right)| left * right)
                    .sum::<f64>()
                    .abs()
                    < 1.0 - 1.0e-12
            }));
        }
    }

    #[test]
    fn refinement_targets_nearest_response_boundary_without_duplicates() {
        let directions = subspace_mode_weights(3, 4);
        let signatures = vec![
            vec![(true, true, true)],
            vec![(true, true, true)],
            vec![(false, false, false)],
            vec![(false, false, false)],
        ];
        let boundaries = response_boundaries_from(&directions, &signatures);
        let refinements = refinement_directions(&boundaries, &directions, 4);
        assert!(!refinements.is_empty());
        assert!(refinements.len() <= 4);
        for (index, refinement) in refinements.iter().enumerate() {
            assert!((dot(&refinement.weights, &refinement.weights) - 1.0).abs() < 1.0e-12);
            assert!(projectively_unique(&refinement.weights, &directions));
            assert!(
                refinements[index + 1..].iter().all(|other| {
                    dot(&refinement.weights, &other.weights).abs() < 1.0 - 1.0e-12
                })
            );
            assert!(refinement.parent_angle_radians > 0.0);
        }
        let first = &refinements[0];
        let children = refined_boundaries(first, first.parent.left_signature.clone());
        let mut existing = directions;
        existing.push(first.weights.clone());
        let second = refinement_directions(&children, &existing, 1);
        assert_eq!(second.len(), 1);
        assert!(
            (2.0 * second[0].parent_angle_radians - first.parent_angle_radians).abs() < 1.0e-12
        );
    }
}
