use crate::frame_2d_branch_switch::{
    BranchSwitchContext, probe_weighted_branch_switches, unavailable_weighted_branch_switches,
};
use crate::symmetric_critical_mode::SymmetricCriticalMode;
use kyuubiki_protocol::{Frame2dBranchSwitchProbeResult, Frame2dBranchSwitchSelection};

pub(crate) const MAX_SUBSPACE_SAMPLES: usize = 16;

#[allow(clippy::too_many_arguments)]
pub(crate) fn probe_subspace_branch_switches(
    context: &BranchSwitchContext<'_>,
    critical_displacement: &[f64],
    primary_displacement: &[f64],
    critical_load_factor: f64,
    critical_modes: &[SymmetricCriticalMode],
    sample_count: usize,
    amplitude: f64,
    selection: Frame2dBranchSwitchSelection,
) -> Vec<Frame2dBranchSwitchProbeResult> {
    subspace_mode_weights(critical_modes.len(), sample_count)
        .into_iter()
        .flat_map(|weights| {
            probe_weighted_branch_switches(
                context,
                critical_displacement,
                primary_displacement,
                critical_load_factor,
                critical_modes,
                &weights,
                amplitude,
                selection,
            )
        })
        .collect()
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
            unavailable_weighted_branch_switches(selection, &weights, amplitude, detail)
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
}
