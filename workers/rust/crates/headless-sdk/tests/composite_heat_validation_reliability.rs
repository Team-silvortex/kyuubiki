use kyuubiki_headless_sdk::{
    composite_heat_cross_validation, composite_heat_cross_validation_for_distributed_load,
    composite_heat_cross_validation_for_regional_loads, composite_heat_mesh_convergence,
    composite_heat_mesh_convergence_for_distributed_load,
    composite_heat_mesh_convergence_for_regional_loads,
    composite_heat_refinement_requests_for_distributed_load,
    composite_heat_refinement_requests_for_regional_loads,
};

const K: [f64; 3] = [390.0, 0.25, 160.0];
const LEVELS: [usize; 4] = [1, 2, 4, 8];

#[test]
fn missing_small_temperature_rise_cannot_pass_against_the_ambient_reference() {
    let power = 1.0e-12;
    let cross = composite_heat_cross_validation_for_distributed_load(K, power, Some(35.0));
    let mesh = composite_heat_mesh_convergence_for_distributed_load(
        K,
        power,
        &LEVELS.map(|level| (level, 35.0)),
    );
    assert_eq!(cross.status, "fail");
    assert_eq!(mesh.status, "fail");
    assert!((cross.relative_error.unwrap() - 1.0).abs() < 1.0e-5);
}

#[test]
fn unresolved_positive_temperature_rise_is_not_a_zero_heat_success() {
    for power in [f64::from_bits(1), 1.0e-25] {
        let cross =
            composite_heat_cross_validation_for_regional_loads(K, [0.0, power, 0.0], Some(35.0));
        let mesh = composite_heat_mesh_convergence_for_regional_loads(
            K,
            [0.0, power, 0.0],
            &LEVELS.map(|level| (level, 35.0)),
        );
        assert_eq!(cross.status, "fail", "power={power}");
        assert_eq!(mesh.status, "fail", "power={power}");
    }
}

#[test]
fn all_layers_require_finite_positive_conductivity_even_if_unheated() {
    for invalid in [0.0, -1.0, f64::INFINITY, f64::NAN] {
        let mut conductivities = K;
        conductivities[0] = invalid;
        assert_eq!(
            composite_heat_cross_validation(conductivities, Some(115.125)).status,
            "fail"
        );
        assert_eq!(
            composite_heat_cross_validation_for_distributed_load(
                conductivities,
                0.02,
                Some(75.125)
            )
            .status,
            "fail"
        );
        assert!(
            composite_heat_refinement_requests_for_distributed_load(conductivities, 0.02).is_err()
        );
        assert!(
            composite_heat_refinement_requests_for_regional_loads(conductivities, [0.0; 3])
                .is_err()
        );
    }
}

#[test]
fn negative_generation_cannot_validate_as_a_cooling_model() {
    let power = -1.0e-5;
    let expected = 35.0 + power * (0.5 * 1000.0 / K[1] + 1000.0 / K[2]);
    assert_eq!(
        composite_heat_cross_validation_for_distributed_load(K, power, Some(expected)).status,
        "fail"
    );
    assert_eq!(
        composite_heat_cross_validation_for_regional_loads(K, [0.0, power, 0.0], Some(expected))
            .status,
        "fail"
    );
}

#[test]
fn mesh_levels_are_validated_before_size_arithmetic() {
    for levels in [
        [1, 2, 4, usize::MAX],
        [1, 2, 4, 0],
        [1, 2, 2, 8],
        [2, 1, 4, 8],
    ] {
        let report = composite_heat_mesh_convergence(K, &levels.map(|level| (level, 115.125)));
        assert_eq!(report.status, "fail", "levels={levels:?}");
    }
}

#[test]
fn missing_samples_and_invalid_samples_are_distinct() {
    for samples in [vec![], vec![(1, 115.125)], vec![(1, 115.125), (2, 115.125)]] {
        assert_eq!(
            composite_heat_mesh_convergence(K, &samples).status,
            "missing"
        );
    }
    for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert_eq!(
            composite_heat_mesh_convergence(K, &LEVELS.map(|level| (level, value))).status,
            "fail"
        );
        assert_eq!(
            composite_heat_cross_validation(K, Some(value)).status,
            "fail"
        );
    }
}

#[test]
fn regional_distribution_rejects_positive_power_lost_to_underflow() {
    assert!(
        composite_heat_refinement_requests_for_regional_loads(K, [f64::from_bits(1), 0.0, 0.0])
            .is_err()
    );
}

#[test]
fn regional_distribution_rejects_nonfinite_total_of_finite_loads() {
    assert!(
        composite_heat_refinement_requests_for_regional_loads(K, [f64::MAX, f64::MAX, 0.0])
            .is_err()
    );
}

#[test]
fn regional_distribution_cannot_hide_small_interface_load_behind_large_layer() {
    assert!(composite_heat_refinement_requests_for_regional_loads(K, [1.0, 1.0e-20, 0.0]).is_err());
}

#[test]
fn zero_generation_is_an_exact_reference_case_not_a_denominator_floor() {
    let samples = LEVELS.map(|level| (level, 35.0));
    let cross = composite_heat_cross_validation_for_regional_loads(K, [0.0; 3], Some(35.0));
    assert_eq!(cross.status, "pass");
    assert_eq!(cross.relative_error, Some(0.0));
    assert_eq!(
        composite_heat_mesh_convergence_for_regional_loads(K, [0.0; 3], &samples).status,
        "pass"
    );
    let next_temperature = f64::from_bits(35.0_f64.to_bits() + 1);
    assert_eq!(
        composite_heat_cross_validation_for_regional_loads(K, [0.0; 3], Some(next_temperature))
            .status,
        "fail"
    );
    assert_eq!(
        composite_heat_cross_validation_for_regional_loads(K, [0.0; 3], None).status,
        "missing"
    );
}

#[test]
fn representable_small_heat_and_a_valid_replay_remain_supported() {
    let power = 1.0e-12;
    let expected = 35.0 + power * (0.5 * 1000.0 / K[1] + 1000.0 / K[2]);
    assert_eq!(
        composite_heat_cross_validation_for_distributed_load(K, power, Some(expected)).status,
        "pass"
    );
    assert_eq!(
        composite_heat_mesh_convergence_for_distributed_load(
            K,
            power,
            &LEVELS.map(|level| (level, expected)),
        )
        .status,
        "pass"
    );
    let requests =
        composite_heat_refinement_requests_for_regional_loads(K, [0.01, 0.02, 0.03]).unwrap();
    for (_, request) in requests {
        assert!(
            (request.nodes.iter().map(|node| node.heat_load).sum::<f64>() - 0.06).abs() < 1e-15
        );
    }
}
