use super::*;

#[test]
fn unavailable_bidirectional_probe_is_explicit() {
    let probes =
        unavailable_branch_switches(Frame2dBranchSwitchSelection::Both, 2, 0.01, "unavailable");
    assert_eq!(probes.len(), 2);
    assert!(probes.iter().all(|probe| probe.mode_index == 2));
    assert!(probes.iter().all(|probe| {
        probe.mode_components.len() == 1
            && probe.mode_components[0].mode_index == 2
            && probe.mode_components[0].normalized_eigenvalue.is_none()
            && probe.mode_components[0].weight == 1.0
    }));
    assert_eq!(probes[0].direction, Frame2dBranchDirection::Positive);
    assert_eq!(probes[1].direction, Frame2dBranchDirection::Negative);
    assert!(
        probes
            .iter()
            .all(|probe| !probe.equilibrium_converged && probe.failure_detail.is_some())
    );
}

#[test]
fn pairwise_mode_directions_are_unit_normalized_and_attributed() {
    let left = vec![1.0, 0.0];
    let right = vec![0.0, 1.0];
    let (sum, left_weight, right_weight) = combine_modes(&left, &right, 1.0).unwrap();
    assert!(
        sum.iter()
            .all(|value| { (value - std::f64::consts::FRAC_1_SQRT_2).abs() < f64::EPSILON })
    );
    assert!((left_weight - std::f64::consts::FRAC_1_SQRT_2).abs() < f64::EPSILON);
    assert!((right_weight - std::f64::consts::FRAC_1_SQRT_2).abs() < f64::EPSILON);

    let (difference, _, right_weight) = combine_modes(&left, &right, -1.0).unwrap();
    assert!((difference[0] - std::f64::consts::FRAC_1_SQRT_2).abs() < f64::EPSILON);
    assert!((difference[1] + std::f64::consts::FRAC_1_SQRT_2).abs() < f64::EPSILON);
    assert!((right_weight + std::f64::consts::FRAC_1_SQRT_2).abs() < f64::EPSILON);
}

#[test]
fn unavailable_pairwise_families_remain_visible() {
    let probes = unavailable_pairwise_branch_switches(
        Frame2dBranchSwitchSelection::Both,
        2,
        0.01,
        "unavailable",
    );
    assert_eq!(probes.len(), 4);
    assert!(probes.iter().all(|probe| {
        probe.mode_eigenvalue.is_none()
            && probe.mode_components.len() == 2
            && !probe.equilibrium_converged
    }));
}

#[test]
fn arbitrary_weights_are_stably_unit_normalized() {
    let normalized = normalize_weights(&[f64::MAX, f64::MAX / 2.0, -f64::MAX / 2.0]).unwrap();
    assert_relative(normalized[0], 2.0_f64.sqrt() / 3.0_f64.sqrt());
    assert_relative(normalized[1], 1.0 / 6.0_f64.sqrt());
    assert_relative(normalized[2], -1.0 / 6.0_f64.sqrt());
    assert_relative(normalized.iter().map(|weight| weight * weight).sum(), 1.0);
}

#[test]
fn unavailable_weighted_family_preserves_component_attribution() {
    let probes = unavailable_weighted_branch_switches(
        Frame2dBranchSwitchSelection::Both,
        &[1.0, 2.0, -2.0],
        0.01,
        "unavailable",
    );
    assert_eq!(probes.len(), 2);
    assert!(probes.iter().all(|probe| {
        probe.mode_index == 0
            && probe.mode_eigenvalue.is_none()
            && probe.mode_components.len() == 3
            && !probe.equilibrium_converged
    }));
    assert_relative(probes[0].mode_components[0].weight, 1.0 / 3.0);
    assert_relative(probes[0].mode_components[1].weight, 2.0 / 3.0);
    assert_relative(probes[0].mode_components[2].weight, -2.0 / 3.0);
}

fn assert_relative(actual: f64, expected: f64) {
    assert!((actual - expected).abs() <= 1.0e-15);
}
