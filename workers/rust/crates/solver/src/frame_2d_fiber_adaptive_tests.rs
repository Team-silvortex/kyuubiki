use super::*;
use crate::frame_2d_material_p_delta::{CompiledFrame2dFiber, CompiledFrame2dPointMaterial};

fn material(force_scale: f64, length_scale: f64) -> CompiledFrame2dMaterial {
    let stress_scale = force_scale / length_scale.powi(2);
    CompiledFrame2dMaterial {
        yield_strength: 100.0 * stress_scale,
        hardening_ratio: 0.05,
        initial_axial_stress: 0.0,
        section_fibers: (0..32)
            .map(|i| CompiledFrame2dFiber {
                y: (-1.0 + (i as f64 + 0.5) / 16.0) * length_scale,
                area: length_scale.powi(2) / 16.0,
                initial_axial_stress: 0.0,
                material: CompiledFrame2dPointMaterial {
                    youngs_modulus: 1_000.0 * stress_scale,
                    yield_strength: 100.0 * stress_scale,
                    hardening_ratio: 0.05,
                    damage: None,
                },
                uses_material_override: true,
            })
            .collect(),
        fiber_material_ids: vec!["all-fibers".into()],
        longitudinal_integration_points: 2,
        adaptive_longitudinal_integration: true,
        longitudinal_integration_tolerance: 1e-3,
    }
}

fn response(forces: [f64; 3]) -> Frame2dSectionResponse {
    let mut response = elastic_response(1_000.0, 2.0, 2.0 / 3.0, 1.0, 0.0, 0.0, 0.0);
    response.axial_force = forces[0];
    response.moment_i = forces[1];
    response.moment_j = forces[2];
    response
}

#[test]
fn generalized_error_is_stable_across_force_scales() {
    let expected =
        generalized_force_error(&response([10.0, 2.0, 3.0]), &response([10.0, 2.0, 4.0])).unwrap();
    for scale in [1e-200, 1e200] {
        let actual = generalized_force_error(
            &response([10.0 * scale, 2.0 * scale, 3.0 * scale]),
            &response([10.0 * scale, 2.0 * scale, 4.0 * scale]),
        )
        .unwrap();
        assert!(
            actual.is_finite() && (actual - expected).abs() < 1e-12,
            "scale={scale:e}, actual={actual}, expected={expected}"
        );
    }
}

#[test]
fn large_axial_force_cannot_hide_bending_quadrature_error() {
    let actual =
        generalized_force_error(&response([1e12, 0.5, -1.0]), &response([1e12, 1.0, -1.0]))
            .unwrap();
    assert!(actual >= 0.5, "masked bending error: {actual:e}");
}

#[test]
fn unused_parent_strength_cannot_mask_mixed_fiber_error() {
    let baseline = evaluate(1.0, 1.0);
    let mut overridden = material(1.0, 1.0);
    overridden.yield_strength = 1e30;
    let actual = section_response(
        Some(&overridden),
        1_000.0,
        2.0,
        2.0 / 3.0,
        1.0,
        0.03,
        -0.25,
        0.05,
        &Frame2dMaterialHistory::default(),
    )
    .unwrap();
    assert_eq!(
        actual.active_longitudinal_integration_points,
        baseline.active_longitudinal_integration_points
    );
    assert_eq!(
        actual.longitudinal_integration_error,
        baseline.longitudinal_integration_error
    );
}

#[test]
fn adaptive_plastic_front_retains_order_and_diagnostics_across_force_scales() {
    let baseline = evaluate(1.0, 1.0);
    assert!(baseline.active_longitudinal_integration_points > 2);
    for scale in [1e-200, 1e200] {
        let actual = evaluate(scale, 1.0);
        assert_eq!(
            actual.active_longitudinal_integration_points,
            baseline.active_longitudinal_integration_points
        );
        assert!(
            (actual.longitudinal_integration_error.unwrap()
                - baseline.longitudinal_integration_error.unwrap())
            .abs()
                < 1e-10
        );
        for (actual, expected) in [actual.axial_force, actual.moment_i, actual.moment_j]
            .into_iter()
            .zip([baseline.axial_force, baseline.moment_i, baseline.moment_j])
        {
            assert!((actual / scale / expected - 1.0).abs() < 1e-10);
        }
    }
}

#[test]
fn changing_length_units_preserves_adaptive_order_and_error() {
    let baseline = evaluate(1.0, 1.0);
    for scale in [1e-6, 1e6] {
        let actual = evaluate(1.0, scale);
        assert_eq!(
            actual.active_longitudinal_integration_points,
            baseline.active_longitudinal_integration_points
        );
        assert!(
            (actual.longitudinal_integration_error.unwrap()
                - baseline.longitudinal_integration_error.unwrap())
            .abs()
                < 1e-10,
            "scale={scale:e}, actual={:?}, expected={:?}",
            actual.longitudinal_integration_error,
            baseline.longitudinal_integration_error
        );
        assert!((actual.axial_force / baseline.axial_force - 1.0).abs() < 1e-10);
        assert!((actual.moment_i / scale / baseline.moment_i - 1.0).abs() < 1e-10);
        assert!((actual.moment_j / scale / baseline.moment_j - 1.0).abs() < 1e-10);
    }
}

#[test]
fn error_guard_rejects_nonfinite_candidates_instead_of_masking_them() {
    for invalid in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        let reference = response([1.0; 3]);
        for axis in 0..3 {
            let mut forces = [1.0; 3];
            forces[axis] = invalid;
            assert!(generalized_force_error(&response(forces), &reference).is_err());
            assert!(generalized_force_error(&reference, &response(forces)).is_err());
        }
        let mut invalid_sum = response([1.0; 3]);
        invalid_sum.absolute_force_sums[1] = invalid;
        assert!(generalized_force_error(&invalid_sum, &reference).is_err());
    }
    assert_eq!(
        generalized_force_error(&response([f64::MAX; 3]), &response([-f64::MAX; 3])).unwrap(),
        2.0
    );
}

#[test]
fn local_accumulation_roundoff_is_distinct_from_a_resolvable_weak_component() {
    let mut reference = response([0.0; 3]);
    reference.absolute_force_sums = [1.0; 3];
    reference.fiber_point_count = 64;
    let mut roundoff = response([1e-15; 3]);
    roundoff.absolute_force_sums = [1.0; 3];
    roundoff.fiber_point_count = 64;
    assert_eq!(generalized_force_error(&roundoff, &reference).unwrap(), 0.0);
    roundoff.moment_j = 1e-7;
    assert_eq!(generalized_force_error(&roundoff, &reference).unwrap(), 1.0);
}

#[test]
fn a_nonfinite_inactive_high_order_response_is_rejected_with_its_order() {
    let mut material = material(1.0, 1.0);
    for fiber in &mut material.section_fibers {
        fiber.area *= 1e112;
    }
    let initial = section_response(
        Some(&material),
        1_000.0,
        2e112,
        2e112 / 3.0,
        1.0,
        0.0,
        0.0,
        0.0,
        &Frame2dMaterialHistory::default(),
    )
    .unwrap();
    assert_eq!(initial.active_longitudinal_integration_points, 2);
    let mut history = initial.history;
    // A finite injected history makes only the otherwise inactive 12-point force overflow.
    history.fiber_points.last_mut().unwrap().backstress = 1e200;
    let error = section_response(
        Some(&material),
        1_000.0,
        2e112,
        2e112 / 3.0,
        1.0,
        0.0,
        0.0,
        0.0,
        &history,
    )
    .err()
    .unwrap();
    assert!(
        error.contains("order 12") && error.contains("non-finite"),
        "{error}"
    );
    assert!(
        section_response(
            Some(&material),
            1_000.0,
            2e112,
            2e112 / 3.0,
            1.0,
            0.0,
            0.0,
            0.0,
            &Frame2dMaterialHistory::default()
        )
        .is_ok()
    );
}

fn evaluate(force_scale: f64, length_scale: f64) -> Frame2dSectionResponse {
    section_response(
        Some(&material(force_scale, length_scale)),
        1_000.0 * force_scale / length_scale.powi(2),
        2.0 * length_scale.powi(2),
        2.0 / 3.0 * length_scale.powi(4),
        length_scale,
        0.03 * length_scale,
        -0.25,
        0.05,
        &Frame2dMaterialHistory::default(),
    )
    .unwrap()
}
