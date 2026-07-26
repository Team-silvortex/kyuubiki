use crate::frame_2d_material_p_delta::{
    CompiledFrame2dMaterial, Frame2dMaterialHistory, Frame2dMaterialPointHistory,
};
use longitudinal_quadrature::{ADAPTIVE_POINT_COUNT, adaptive_history_offset, gauss_stations};

#[path = "frame_2d_longitudinal_quadrature.rs"]
mod longitudinal_quadrature;

pub(crate) struct Frame2dSectionResponse {
    pub(crate) axial_force: f64,
    pub(crate) moment_i: f64,
    pub(crate) moment_j: f64,
    pub(crate) tangent: [[f64; 3]; 3],
    pub(crate) history: Frame2dMaterialHistory,
    pub(crate) average_stress: f64,
    pub(crate) average_initial_stress: f64,
    pub(crate) average_plastic_strain: f64,
    pub(crate) average_backstress: f64,
    pub(crate) max_equivalent_plastic_strain: f64,
    pub(crate) fiber_point_count: usize,
    pub(crate) evaluated_fiber_point_count: usize,
    pub(crate) yielded_fiber_point_count: usize,
    pub(crate) active_longitudinal_integration_points: usize,
    pub(crate) longitudinal_integration_error: Option<f64>,
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn section_response(
    material: Option<&CompiledFrame2dMaterial>,
    youngs_modulus: f64,
    area: f64,
    moment_of_inertia: f64,
    length: f64,
    extension: f64,
    phi_i: f64,
    phi_j: f64,
    committed: &Frame2dMaterialHistory,
) -> Frame2dSectionResponse {
    let Some(material) = material else {
        return elastic_response(
            youngs_modulus,
            area,
            moment_of_inertia,
            length,
            extension,
            phi_i,
            phi_j,
        );
    };
    if material.section_fibers.is_empty() {
        return axial_material_response(
            material,
            youngs_modulus,
            area,
            moment_of_inertia,
            length,
            extension,
            phi_i,
            phi_j,
            committed,
        );
    }
    fiber_material_response(
        material,
        youngs_modulus,
        area,
        length,
        extension,
        phi_i,
        phi_j,
        committed,
    )
}

pub(crate) fn committed_effective_axial_tangent(
    material: &CompiledFrame2dMaterial,
    youngs_modulus: f64,
    area: f64,
    committed: &Frame2dMaterialHistory,
) -> f64 {
    if material.section_fibers.is_empty() {
        return positive_tangent_or_elastic(committed.point.tangent_modulus, youngs_modulus);
    }
    let fiber_count = material.section_fibers.len();
    let active_points = if material.adaptive_longitudinal_integration {
        committed.active_longitudinal_integration_points
    } else {
        material.longitudinal_integration_points
    };
    let stations = gauss_stations(active_points);
    let expected_points = if material.adaptive_longitudinal_integration {
        fiber_count * ADAPTIVE_POINT_COUNT
    } else {
        fiber_count * stations.len()
    };
    if committed.fiber_points.len() != expected_points {
        return youngs_modulus;
    }
    let history_offset = if material.adaptive_longitudinal_integration {
        adaptive_history_offset(active_points, fiber_count)
    } else {
        0
    };
    stations
        .iter()
        .enumerate()
        .flat_map(|(station_index, &(_, weight))| {
            material
                .section_fibers
                .iter()
                .enumerate()
                .map(move |(fiber_index, fiber)| {
                    let point_index = history_offset
                        + station_index * material.section_fibers.len()
                        + fiber_index;
                    weight
                        * fiber.area
                        * positive_tangent_or_elastic(
                            committed.fiber_points[point_index].tangent_modulus,
                            youngs_modulus,
                        )
                })
        })
        .sum::<f64>()
        / area
}

fn positive_tangent_or_elastic(tangent_modulus: f64, youngs_modulus: f64) -> f64 {
    if tangent_modulus.is_finite() && tangent_modulus > 0.0 {
        tangent_modulus
    } else {
        youngs_modulus
    }
}

fn elastic_response(
    youngs_modulus: f64,
    area: f64,
    moment_of_inertia: f64,
    length: f64,
    extension: f64,
    phi_i: f64,
    phi_j: f64,
) -> Frame2dSectionResponse {
    let axial_stiffness = youngs_modulus * area / length;
    let bending = youngs_modulus * moment_of_inertia / length;
    Frame2dSectionResponse {
        axial_force: axial_stiffness * extension,
        moment_i: bending * (4.0 * phi_i + 2.0 * phi_j),
        moment_j: bending * (2.0 * phi_i + 4.0 * phi_j),
        tangent: [
            [axial_stiffness, 0.0, 0.0],
            [0.0, 4.0 * bending, 2.0 * bending],
            [0.0, 2.0 * bending, 4.0 * bending],
        ],
        history: Frame2dMaterialHistory::default(),
        average_stress: youngs_modulus * extension / length,
        average_initial_stress: 0.0,
        average_plastic_strain: 0.0,
        average_backstress: 0.0,
        max_equivalent_plastic_strain: 0.0,
        fiber_point_count: 0,
        evaluated_fiber_point_count: 0,
        yielded_fiber_point_count: 0,
        active_longitudinal_integration_points: 0,
        longitudinal_integration_error: None,
    }
}

#[allow(clippy::too_many_arguments)]
fn axial_material_response(
    material: &CompiledFrame2dMaterial,
    youngs_modulus: f64,
    area: f64,
    moment_of_inertia: f64,
    length: f64,
    extension: f64,
    phi_i: f64,
    phi_j: f64,
    committed: &Frame2dMaterialHistory,
) -> Frame2dSectionResponse {
    let response = material.response(
        youngs_modulus,
        extension / length,
        &committed.point,
        material.initial_axial_stress,
    );
    let axial_stiffness = response.tangent_modulus * area / length;
    let bending = youngs_modulus * moment_of_inertia / length;
    Frame2dSectionResponse {
        axial_force: response.stress * area,
        moment_i: bending * (4.0 * phi_i + 2.0 * phi_j),
        moment_j: bending * (2.0 * phi_i + 4.0 * phi_j),
        tangent: [
            [axial_stiffness, 0.0, 0.0],
            [0.0, 4.0 * bending, 2.0 * bending],
            [0.0, 2.0 * bending, 4.0 * bending],
        ],
        history: Frame2dMaterialHistory {
            point: response.history,
            fiber_points: Vec::new(),
            active_longitudinal_integration_points: 0,
            longitudinal_integration_error: None,
        },
        average_stress: response.stress,
        average_initial_stress: material.initial_axial_stress,
        average_plastic_strain: response.history.plastic_strain,
        average_backstress: response.history.backstress,
        max_equivalent_plastic_strain: response.history.equivalent_plastic_strain,
        fiber_point_count: 0,
        evaluated_fiber_point_count: 0,
        yielded_fiber_point_count: 0,
        active_longitudinal_integration_points: 0,
        longitudinal_integration_error: None,
    }
}

#[allow(clippy::too_many_arguments)]
fn fiber_material_response(
    material: &CompiledFrame2dMaterial,
    youngs_modulus: f64,
    area: f64,
    length: f64,
    extension: f64,
    phi_i: f64,
    phi_j: f64,
    committed: &Frame2dMaterialHistory,
) -> Frame2dSectionResponse {
    if material.adaptive_longitudinal_integration {
        return adaptive_fiber_material_response(
            material,
            youngs_modulus,
            area,
            length,
            extension,
            phi_i,
            phi_j,
            committed,
        );
    }
    fiber_material_response_for_stations(
        material,
        youngs_modulus,
        area,
        length,
        extension,
        phi_i,
        phi_j,
        gauss_stations(material.longitudinal_integration_points),
        &committed.fiber_points,
    )
}

#[allow(clippy::too_many_arguments)]
fn fiber_material_response_for_stations(
    material: &CompiledFrame2dMaterial,
    youngs_modulus: f64,
    area: f64,
    length: f64,
    extension: f64,
    phi_i: f64,
    phi_j: f64,
    stations: &[(f64, f64)],
    committed_points: &[Frame2dMaterialPointHistory],
) -> Frame2dSectionResponse {
    let fiber_count = material.section_fibers.len();
    let point_count = fiber_count * stations.len();
    let mut history = Vec::with_capacity(point_count);
    let mut forces = [0.0; 3];
    let mut tangent = [[0.0; 3]; 3];
    let mut average_initial_stress = 0.0;
    let mut average_plastic_strain = 0.0;
    let mut average_backstress = 0.0;
    let mut max_equivalent_plastic_strain = 0.0_f64;
    let mut yielded_fiber_point_count = 0;

    for (station_index, &(xi, weight)) in stations.iter().enumerate() {
        let curvature_i = (-4.0 + 6.0 * xi) / length;
        let curvature_j = (-2.0 + 6.0 * xi) / length;
        for (fiber_index, fiber) in material.section_fibers.iter().enumerate() {
            let point_index = station_index * fiber_count + fiber_index;
            let committed_point = committed_points
                .get(point_index)
                .copied()
                .unwrap_or_default();
            let strain = extension / length + fiber.y * (curvature_i * phi_i + curvature_j * phi_j);
            let response = material.response(
                youngs_modulus,
                strain,
                &committed_point,
                fiber.initial_axial_stress,
            );
            let strain_gradient = [1.0 / length, fiber.y * curvature_i, fiber.y * curvature_j];
            let integration = length * weight * fiber.area;
            for row in 0..3 {
                forces[row] += integration * response.stress * strain_gradient[row];
                for column in 0..3 {
                    tangent[row][column] += integration
                        * response.tangent_modulus
                        * strain_gradient[row]
                        * strain_gradient[column];
                }
            }
            let average_weight = weight * fiber.area / area;
            average_initial_stress += average_weight * fiber.initial_axial_stress;
            average_plastic_strain += average_weight * response.history.plastic_strain;
            average_backstress += average_weight * response.history.backstress;
            max_equivalent_plastic_strain =
                max_equivalent_plastic_strain.max(response.history.equivalent_plastic_strain);
            yielded_fiber_point_count +=
                usize::from(response.history.equivalent_plastic_strain > 0.0);
            history.push(response.history);
        }
    }

    Frame2dSectionResponse {
        axial_force: forces[0],
        moment_i: forces[1],
        moment_j: forces[2],
        tangent,
        history: Frame2dMaterialHistory {
            point: Frame2dMaterialPointHistory::default(),
            fiber_points: history,
            active_longitudinal_integration_points: stations.len(),
            longitudinal_integration_error: None,
        },
        average_stress: forces[0] / area,
        average_initial_stress,
        average_plastic_strain,
        average_backstress,
        max_equivalent_plastic_strain,
        fiber_point_count: point_count,
        evaluated_fiber_point_count: point_count,
        yielded_fiber_point_count,
        active_longitudinal_integration_points: stations.len(),
        longitudinal_integration_error: None,
    }
}

#[allow(clippy::too_many_arguments)]
fn adaptive_fiber_material_response(
    material: &CompiledFrame2dMaterial,
    youngs_modulus: f64,
    area: f64,
    length: f64,
    extension: f64,
    phi_i: f64,
    phi_j: f64,
    committed: &Frame2dMaterialHistory,
) -> Frame2dSectionResponse {
    let fiber_count = material.section_fibers.len();
    let candidate = |point_count: usize| {
        let offset = adaptive_history_offset(point_count, fiber_count);
        let count = point_count * fiber_count;
        let committed_points = committed
            .fiber_points
            .get(offset..offset + count)
            .unwrap_or(&[]);
        fiber_material_response_for_stations(
            material,
            youngs_modulus,
            area,
            length,
            extension,
            phi_i,
            phi_j,
            gauss_stations(point_count),
            committed_points,
        )
    };
    let response_2 = candidate(2);
    let response_3 = candidate(3);
    let response_4 = candidate(4);
    let response_8 = candidate(8);
    let response_12 = candidate(12);
    let error_23 = generalized_force_error(&response_2, &response_3, material, area);
    let error_24 = generalized_force_error(&response_2, &response_4, material, area);
    let error_28 = generalized_force_error(&response_2, &response_8, material, area);
    let error_2_12 = generalized_force_error(&response_2, &response_12, material, area);
    let error_34 = generalized_force_error(&response_3, &response_4, material, area);
    let error_38 = generalized_force_error(&response_3, &response_8, material, area);
    let error_3_12 = generalized_force_error(&response_3, &response_12, material, area);
    let error_48 = generalized_force_error(&response_4, &response_8, material, area);
    let error_4_12 = generalized_force_error(&response_4, &response_12, material, area);
    let error_8_12 = generalized_force_error(&response_8, &response_12, material, area);
    let error_2 = error_23.max(error_24).max(error_28).max(error_2_12);
    let error_3 = error_34.max(error_38).max(error_3_12);
    let error_4 = error_48.max(error_4_12);
    let all_history = response_2
        .history
        .fiber_points
        .iter()
        .chain(&response_3.history.fiber_points)
        .chain(&response_4.history.fiber_points)
        .chain(&response_8.history.fiber_points)
        .chain(&response_12.history.fiber_points)
        .copied()
        .collect();
    let (mut selected, error) = if error_2 <= material.longitudinal_integration_tolerance {
        (response_2, error_2)
    } else if error_3 <= material.longitudinal_integration_tolerance {
        (response_3, error_3)
    } else if error_4 <= material.longitudinal_integration_tolerance {
        (response_4, error_4)
    } else if error_8_12 <= material.longitudinal_integration_tolerance {
        (response_8, error_8_12)
    } else {
        (response_12, error_8_12)
    };
    let active_points = selected.active_longitudinal_integration_points;
    selected.history.fiber_points = all_history;
    selected.history.active_longitudinal_integration_points = active_points;
    selected.history.longitudinal_integration_error = Some(error);
    selected.evaluated_fiber_point_count = ADAPTIVE_POINT_COUNT * fiber_count;
    selected.longitudinal_integration_error = Some(error);
    selected
}

fn generalized_force_error(
    candidate: &Frame2dSectionResponse,
    reference: &Frame2dSectionResponse,
    material: &CompiledFrame2dMaterial,
    area: f64,
) -> f64 {
    let candidate_forces = [
        candidate.axial_force,
        candidate.moment_i,
        candidate.moment_j,
    ];
    let reference_forces = [
        reference.axial_force,
        reference.moment_i,
        reference.moment_j,
    ];
    let difference = candidate_forces
        .iter()
        .zip(reference_forces)
        .map(|(candidate, reference)| (candidate - reference).powi(2))
        .sum::<f64>()
        .sqrt();
    let reference_norm = reference_forces
        .iter()
        .map(|force| force.powi(2))
        .sum::<f64>()
        .sqrt();
    difference / reference_norm.max(material.yield_strength * area * 1.0e-12)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frame_2d_material_p_delta::CompiledFrame2dFiber;

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
        );
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
        );

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
        );

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
            );
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
        let reference =
            dense_reference_forces(youngs_modulus, yield_strength, extension, phi_i, phi_j);
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
            );
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
        );

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
        );

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
        );
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
                })
                .collect(),
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
                })
                .collect(),
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
}

#[cfg(test)]
#[path = "frame_2d_fiber_section_cyclic_reference.rs"]
mod cyclic_reference;
