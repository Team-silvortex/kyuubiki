use super::*;
use crate::frame_2d_material_p_delta::{CompiledFrame2dFiber, CompiledFrame2dPointMaterial};

#[test]
fn elastic_fibers_recover_the_discrete_section_stiffness() {
    let material = material_with_initial_stresses([0.0; 4]);
    let response = section_response(
        Some(&material),
        1_000.0,
        1.0,
        0.3125,
        2.0,
        0.02,
        0.01,
        -0.02,
        &Frame2dMaterialHistory::default(),
    )
    .unwrap();
    let bending = 1_000.0 * 0.3125 / 2.0;

    assert_close(response.tangent[0][0], 500.0, 1.0e-12);
    assert_close(response.tangent[1][1], 4.0 * bending, 1.0e-12);
    assert_close(response.tangent[1][2], 2.0 * bending, 1.0e-12);
    assert_close(response.tangent[2][2], 4.0 * bending, 1.0e-12);
    assert_close(response.tangent[0][1], 0.0, 1.0e-12);
    assert_close(response.tangent[0][2], 0.0, 1.0e-12);
    assert_eq!(response.fiber_point_count, 8);
    assert_eq!(response.yielded_fiber_point_count, 0);
}

#[test]
fn distributed_initial_stress_can_have_zero_section_resultants() {
    let material = material_with_initial_stresses([-50.0, 50.0, 50.0, -50.0]);
    let response = section_response(
        Some(&material),
        1_000.0,
        1.0,
        0.3125,
        2.0,
        0.0,
        0.0,
        0.0,
        &Frame2dMaterialHistory::default(),
    )
    .unwrap();

    assert_close(response.axial_force, 0.0, 1.0e-12);
    assert_close(response.moment_i, 0.0, 1.0e-12);
    assert_close(response.moment_j, 0.0, 1.0e-12);
    assert_close(response.average_initial_stress, 0.0, 1.0e-12);
    assert_eq!(response.yielded_fiber_point_count, 0);
}

#[test]
fn asymmetric_axial_bending_yield_creates_a_coupled_consistent_tangent() {
    let material = material_with_initial_stresses([0.0; 4]);
    let committed = Frame2dMaterialHistory::default();
    let response = section_response(
        Some(&material),
        1_000.0,
        1.0,
        0.3125,
        2.0,
        0.16,
        0.35,
        -0.05,
        &committed,
    )
    .unwrap();

    assert!(response.yielded_fiber_point_count > 0);
    assert!(response.yielded_fiber_point_count < response.fiber_point_count);
    assert!(response.tangent[0][1].abs() > 1.0);
    assert_close(response.tangent[0][1], response.tangent[1][0], 1.0e-12);
    assert_eq!(committed.fiber_points.len(), 0);
    assert_eq!(committed.point.equivalent_plastic_strain, 0.0);
    assert!(response.max_equivalent_plastic_strain > 0.0);
}

#[test]
fn rectangular_section_fibers_converge_to_the_analytic_elastoplastic_moment() {
    let youngs_modulus: f64 = 1_000.0;
    let yield_strength: f64 = 100.0;
    let half_depth: f64 = 1.0;
    let curvature = 2.0 * yield_strength / (youngs_modulus * half_depth);
    let elastic_core = yield_strength / (youngs_modulus * curvature);
    let expected_moment = 2.0
        * (youngs_modulus * curvature * elastic_core.powi(3) / 3.0
            + yield_strength * (half_depth.powi(2) - elastic_core.powi(2)) / 2.0);
    let mut previous_error = f64::INFINITY;

    for fiber_count in [4, 8, 16, 32] {
        let material = rectangular_material(fiber_count, half_depth, yield_strength);
        let response = section_response(
            Some(&material),
            youngs_modulus,
            2.0 * half_depth,
            2.0 * half_depth.powi(3) / 3.0,
            1.0,
            0.0,
            -curvature / 2.0,
            curvature / 2.0,
            &Frame2dMaterialHistory::default(),
        )
        .unwrap();
        let error = (response.moment_j - expected_moment).abs();

        assert_close(response.moment_i, -response.moment_j, 1.0e-12);
        assert!(error < previous_error * 0.3);
        previous_error = error;
    }
    assert!(previous_error / expected_moment < 1.0e-3);
}

#[test]
fn longitudinal_gauss_rules_converge_toward_a_dense_plastic_reference() {
    let youngs_modulus = 1_000.0;
    let yield_strength = 100.0;
    let extension = 0.03;
    let phi_i = -0.25;
    let phi_j = 0.05;
    let reference = dense_reference_forces(youngs_modulus, yield_strength, extension, phi_i, phi_j);
    let mut errors = Vec::new();

    for point_count in [2, 3, 4] {
        let mut material = rectangular_material(32, 1.0, yield_strength);
        material.longitudinal_integration_points = point_count;
        let response = section_response(
            Some(&material),
            youngs_modulus,
            2.0,
            2.0 / 3.0,
            1.0,
            extension,
            phi_i,
            phi_j,
            &Frame2dMaterialHistory::default(),
        )
        .unwrap();
        let actual = [response.axial_force, response.moment_i, response.moment_j];
        errors.push(vector_error(actual, reference));
        assert_eq!(response.fiber_point_count, 32 * point_count);
    }

    assert!(errors[1] < errors[0], "errors={errors:?}");
    assert!(errors[2] < errors[1], "errors={errors:?}");
    assert!(errors[2] < 0.5 * errors[0], "errors={errors:?}");
}

#[test]
fn adaptive_longitudinal_integration_uses_two_points_for_elastic_fields() {
    let mut material = rectangular_material(8, 1.0, 100.0);
    material.adaptive_longitudinal_integration = true;
    material.longitudinal_integration_tolerance = 1.0e-12;
    let response = section_response(
        Some(&material),
        1_000.0,
        2.0,
        2.0 / 3.0,
        1.0,
        0.01,
        0.02,
        -0.01,
        &Frame2dMaterialHistory::default(),
    )
    .unwrap();

    assert_eq!(response.active_longitudinal_integration_points, 2);
    assert_eq!(response.fiber_point_count, 16);
    assert_eq!(response.evaluated_fiber_point_count, 232);
    assert_eq!(response.history.fiber_points.len(), 232);
    assert!(response.longitudinal_integration_error.unwrap() < 1.0e-12);
}

#[test]
fn adaptive_longitudinal_integration_promotes_plastic_fronts_to_twelve_points() {
    let mut material = rectangular_material(32, 1.0, 100.0);
    material.adaptive_longitudinal_integration = true;
    material.longitudinal_integration_tolerance = 1.0e-10;
    let first = section_response(
        Some(&material),
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

    assert_eq!(first.active_longitudinal_integration_points, 12);
    assert_eq!(first.fiber_point_count, 384);
    assert_eq!(first.evaluated_fiber_point_count, 928);
    assert_eq!(first.history.fiber_points.len(), 928);
    assert!(first.longitudinal_integration_error.unwrap() > 1.0e-10);

    let second = section_response(
        Some(&material),
        1_000.0,
        2.0,
        2.0 / 3.0,
        1.0,
        0.03,
        -0.25,
        0.05,
        &first.history,
    )
    .unwrap();
    assert_eq!(second.history.fiber_points.len(), 928);
    assert!([2, 3, 4, 8, 12].contains(&second.active_longitudinal_integration_points));
}

fn material_with_initial_stresses(initial_stresses: [f64; 4]) -> CompiledFrame2dMaterial {
    let coordinates = [-0.75, -0.25, 0.25, 0.75];
    CompiledFrame2dMaterial {
        yield_strength: 100.0,
        hardening_ratio: 0.1,
        initial_axial_stress: 0.0,
        section_fibers: coordinates
            .into_iter()
            .zip(initial_stresses)
            .map(|(y, initial_axial_stress)| CompiledFrame2dFiber {
                y,
                area: 0.25,
                initial_axial_stress,
                material: CompiledFrame2dPointMaterial {
                    youngs_modulus: 1_000.0,
                    yield_strength: 100.0,
                    hardening_ratio: 0.1,
                    damage: None,
                },
                uses_material_override: false,
            })
            .collect(),
        fiber_material_ids: Vec::new(),
        longitudinal_integration_points: 2,
        adaptive_longitudinal_integration: false,
        longitudinal_integration_tolerance: 1.0e-3,
    }
}

fn rectangular_material(
    fiber_count: usize,
    half_depth: f64,
    yield_strength: f64,
) -> CompiledFrame2dMaterial {
    let fiber_depth = 2.0 * half_depth / fiber_count as f64;
    CompiledFrame2dMaterial {
        yield_strength,
        hardening_ratio: 0.0,
        initial_axial_stress: 0.0,
        section_fibers: (0..fiber_count)
            .map(|index| CompiledFrame2dFiber {
                y: -half_depth + (index as f64 + 0.5) * fiber_depth,
                area: fiber_depth,
                initial_axial_stress: 0.0,
                material: CompiledFrame2dPointMaterial {
                    youngs_modulus: 1_000.0,
                    yield_strength,
                    hardening_ratio: 0.0,
                    damage: None,
                },
                uses_material_override: false,
            })
            .collect(),
        fiber_material_ids: Vec::new(),
        longitudinal_integration_points: 2,
        adaptive_longitudinal_integration: false,
        longitudinal_integration_tolerance: 1.0e-3,
    }
}

fn dense_reference_forces(
    youngs_modulus: f64,
    yield_strength: f64,
    extension: f64,
    phi_i: f64,
    phi_j: f64,
) -> [f64; 3] {
    let material = rectangular_material(32, 1.0, yield_strength);
    let sample_count = 50_000;
    let mut forces = [0.0; 3];
    for sample in 0..sample_count {
        let xi = (sample as f64 + 0.5) / sample_count as f64;
        let curvature_i = -4.0 + 6.0 * xi;
        let curvature_j = -2.0 + 6.0 * xi;
        for fiber in &material.section_fibers {
            let strain = extension + fiber.y * (curvature_i * phi_i + curvature_j * phi_j);
            let stress = (youngs_modulus * strain).clamp(-yield_strength, yield_strength);
            let gradient = [1.0, fiber.y * curvature_i, fiber.y * curvature_j];
            for row in 0..3 {
                forces[row] += fiber.area * stress * gradient[row] / sample_count as f64;
            }
        }
    }
    forces
}

fn vector_error(actual: [f64; 3], expected: [f64; 3]) -> f64 {
    actual
        .iter()
        .zip(expected)
        .map(|(actual, expected)| (actual - expected).powi(2))
        .sum::<f64>()
        .sqrt()
}

fn assert_close(actual: f64, expected: f64, tolerance: f64) {
    assert!(
        (actual - expected).abs() <= tolerance,
        "actual={actual:.12e}, expected={expected:.12e}"
    );
}
