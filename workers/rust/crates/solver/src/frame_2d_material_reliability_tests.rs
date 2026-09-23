use super::*;

fn point(
    youngs_modulus: f64,
    yield_strength: f64,
    hardening_ratio: f64,
) -> CompiledFrame2dPointMaterial {
    CompiledFrame2dPointMaterial {
        youngs_modulus,
        yield_strength,
        hardening_ratio,
        damage: None,
    }
}

fn axial_material(hardening_ratio: f64) -> CompiledFrame2dMaterial {
    CompiledFrame2dMaterial {
        yield_strength: 100.0,
        hardening_ratio,
        initial_axial_stress: 0.0,
        section_fibers: Vec::new(),
        fiber_material_ids: Vec::new(),
        longitudinal_integration_points: 2,
        adaptive_longitudinal_integration: false,
        longitudinal_integration_tolerance: 1.0e-3,
    }
}

fn relative_close(actual: f64, expected: f64) {
    assert!(actual.is_finite(), "non-finite result: {actual}");
    assert!(
        (actual / expected - 1.0).abs() < 2.0e-14,
        "actual={actual:e}, expected={expected:e}"
    );
}

#[test]
fn return_mapping_preserves_stress_and_tangent_across_material_scales() {
    for modulus in [1.0e-300, 1.0e3, 1.0e300] {
        let material = point(modulus, 0.1 * modulus, 0.05);
        for strain in [-0.2, 0.2] {
            let response = material.response(strain, &Frame2dMaterialPointHistory::default(), 0.0);
            relative_close(response.stress, strain.signum() * modulus * 0.105);
            relative_close(response.tangent_modulus, modulus * 0.05);
            relative_close(response.history.plastic_strain, strain.signum() * 0.095);
            relative_close(
                response.history.backstress,
                strain.signum() * modulus * 0.005,
            );
            relative_close(response.history.equivalent_plastic_strain, 0.095);
        }
    }
}

#[test]
fn near_elastic_hardening_does_not_form_an_unrepresentable_plastic_modulus() {
    let ratio = f64::from_bits(1.0_f64.to_bits() - 1);
    let material = point(1.0e300, 1.0e299, ratio);
    let response = material.response(0.2, &Frame2dMaterialPointHistory::default(), 0.0);
    relative_close(response.stress, 2.0e299);
    relative_close(response.tangent_modulus, 1.0e300 * ratio);
    relative_close(response.history.plastic_strain, 0.1 * (1.0 - ratio));
    relative_close(response.history.backstress, 1.0e299 * ratio);
}

#[test]
fn perfect_plastic_stress_is_not_lost_to_subtraction_cancellation() {
    let material = point(1.0e20, 1.0, 0.0);
    for strain in [-1.0, 1.0] {
        let response = material.response(strain, &Frame2dMaterialPointHistory::default(), 0.0);
        assert_eq!(response.stress, strain);
        assert_eq!(response.tangent_modulus, 0.0);
    }
}

#[test]
fn committed_perfect_plastic_axial_tangent_is_zero_not_elastic() {
    let material = axial_material(0.0);
    let initial = Frame2dMaterialHistory::default();
    assert_eq!(
        committed_effective_axial_tangent(&material, 1_000.0, 1.0, &initial),
        1_000.0
    );
    let loaded = section_response(
        Some(&material),
        1_000.0,
        1.0,
        1.0,
        1.0,
        0.2,
        0.0,
        0.0,
        &initial,
    )
    .unwrap();
    assert!(loaded.history.point.equivalent_plastic_strain > 0.0);
    assert_eq!(
        committed_effective_axial_tangent(&material, 1_000.0, 1.0, &loaded.history),
        0.0
    );
    let unloaded = section_response(
        Some(&material),
        1_000.0,
        1.0,
        1.0,
        1.0,
        0.15,
        0.0,
        0.0,
        &loaded.history,
    )
    .unwrap();
    assert_eq!(
        committed_effective_axial_tangent(&material, 1_000.0, 1.0, &unloaded.history),
        1_000.0
    );
}

#[test]
fn committed_perfect_plastic_fiber_tangent_is_zero_not_elastic() {
    let mut material = axial_material(0.0);
    material.section_fibers = [-0.5, 0.5]
        .into_iter()
        .map(|y| CompiledFrame2dFiber {
            y,
            area: 0.5,
            initial_axial_stress: 0.0,
            material: point(1_000.0, 100.0, 0.0),
            uses_material_override: false,
        })
        .collect();
    for adaptive in [false, true] {
        material.adaptive_longitudinal_integration = adaptive;
        let initial = Frame2dMaterialHistory::default();
        let loaded = section_response(
            Some(&material),
            1_000.0,
            1.0,
            0.25,
            1.0,
            0.2,
            0.0,
            0.0,
            &initial,
        )
        .unwrap();
        assert!(
            loaded
                .history
                .fiber_points
                .iter()
                .all(|point| point.equivalent_plastic_strain > 0.0)
        );
        assert_eq!(
            committed_effective_axial_tangent(&material, 1_000.0, 1.0, &loaded.history),
            0.0
        );
    }
}

#[test]
fn reversed_trial_tangent_matches_finite_difference_at_extreme_scales() {
    for modulus in [1e-280, 1e3, 1e280] {
        let material = point(modulus, modulus * 0.1, 0.05);
        let virgin = Frame2dMaterialPointHistory::default();
        let loaded = material.response(0.2, &virgin, 0.0);
        for strain in [0.15, -0.2, 0.3] {
            let response = material.response(strain, &loaded.history, 0.0);
            let increment = 1e-6;
            let plus = material.response(strain + increment, &loaded.history, 0.0);
            let minus = material.response(strain - increment, &loaded.history, 0.0);
            let numerical = (plus.stress / modulus - minus.stress / modulus) / (2.0 * increment);
            assert!((numerical - response.tangent_modulus / modulus).abs() < 1e-10);
            assert!(
                response.history.equivalent_plastic_strain
                    >= loaded.history.equivalent_plastic_strain
            );
        }
        assert_eq!(virgin.equivalent_plastic_strain, 0.0);
        relative_close(loaded.history.equivalent_plastic_strain, 0.095);
    }
}

#[test]
fn nonfinite_section_values_are_rejected_before_they_can_be_committed() {
    let material = axial_material(0.1);
    let initial = Frame2dMaterialHistory::default();
    let result = section_response(
        Some(&material),
        1e300,
        1.0,
        1.0,
        1.0,
        1e100,
        0.0,
        0.0,
        &initial,
    );
    assert!(result.err().unwrap().contains("non-finite"));
    assert_eq!(initial.point.equivalent_plastic_strain, 0.0);
    assert!(
        section_response(
            Some(&material),
            1_000.0,
            1.0,
            1.0,
            1.0,
            0.2,
            0.0,
            0.0,
            &initial
        )
        .is_ok()
    );
}

#[test]
fn adaptive_inactive_histories_cannot_hide_nonfinite_accumulated_plasticity() {
    let mut material = axial_material(0.1);
    material.section_fibers = [-0.5, 0.5]
        .into_iter()
        .map(|y| CompiledFrame2dFiber {
            y,
            area: 0.5,
            initial_axial_stress: 0.0,
            material: point(1_000.0, 100.0, 0.1),
            uses_material_override: false,
        })
        .collect();
    material.adaptive_longitudinal_integration = true;
    let initial = Frame2dMaterialHistory::default();
    let first = section_response(
        Some(&material),
        1_000.0,
        1.0,
        0.25,
        1.0,
        0.01,
        0.0,
        0.0,
        &initial,
    )
    .unwrap();
    assert_eq!(first.active_longitudinal_integration_points, 2);
    let mut corrupt = first.history;
    corrupt
        .fiber_points
        .last_mut()
        .unwrap()
        .equivalent_plastic_strain = f64::NAN;
    let result = section_response(
        Some(&material),
        1_000.0,
        1.0,
        0.25,
        1.0,
        0.01,
        0.0,
        0.0,
        &corrupt,
    );
    assert!(result.err().unwrap().contains("material history"));
}

#[test]
fn virgin_mixed_fibers_report_their_own_weighted_modulus() {
    let mut material = axial_material(0.1);
    material.section_fibers = [2_000.0, 4_000.0]
        .into_iter()
        .enumerate()
        .map(|(index, modulus)| CompiledFrame2dFiber {
            y: index as f64 - 0.5,
            area: 0.5,
            initial_axial_stress: 0.0,
            material: point(modulus, 100.0, 0.1),
            uses_material_override: true,
        })
        .collect();
    let virgin = Frame2dMaterialHistory::default();
    for adaptive in [false, true] {
        material.adaptive_longitudinal_integration = adaptive;
        assert_eq!(
            committed_effective_axial_tangent(&material, 1_000.0, 1.0, &virgin),
            3_000.0
        );
    }
}
