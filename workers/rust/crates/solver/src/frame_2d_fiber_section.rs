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
    absolute_force_sums: [f64; 3],
    pub(crate) tangent: [[f64; 3]; 3],
    pub(crate) history: Frame2dMaterialHistory,
    pub(crate) average_stress: f64,
    pub(crate) average_initial_stress: f64,
    pub(crate) average_plastic_strain: f64,
    pub(crate) average_backstress: f64,
    pub(crate) max_equivalent_plastic_strain: f64,
    pub(crate) max_fiber_damage: f64,
    pub(crate) fiber_point_count: usize,
    pub(crate) evaluated_fiber_point_count: usize,
    pub(crate) yielded_fiber_point_count: usize,
    pub(crate) damaged_fiber_point_count: usize,
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
) -> Result<Frame2dSectionResponse, String> {
    let response = if let Some(material) = material {
        if material.section_fibers.is_empty() {
            axial_material_response(
                material,
                youngs_modulus,
                area,
                moment_of_inertia,
                length,
                extension,
                phi_i,
                phi_j,
                committed,
            )
        } else {
            fiber_material_response(
                material,
                youngs_modulus,
                area,
                length,
                extension,
                phi_i,
                phi_j,
                committed,
            )?
        }
    } else {
        elastic_response(
            youngs_modulus,
            area,
            moment_of_inertia,
            length,
            extension,
            phi_i,
            phi_j,
        )
    };
    response.validate()?;
    Ok(response)
}

pub(crate) fn committed_effective_axial_tangent(
    material: &CompiledFrame2dMaterial,
    youngs_modulus: f64,
    area: f64,
    committed: &Frame2dMaterialHistory,
) -> f64 {
    if material.section_fibers.is_empty() {
        return committed_point_tangent(&committed.point, youngs_modulus);
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
        return material
            .section_fibers
            .iter()
            .map(|fiber| (fiber.area / area) * fiber.material.youngs_modulus)
            .sum();
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
                        * (fiber.area / area)
                        * committed_point_tangent(
                            &committed.fiber_points[point_index],
                            fiber.material.youngs_modulus,
                        )
                })
        })
        .sum::<f64>()
}

fn committed_point_tangent(point: &Frame2dMaterialPointHistory, youngs_modulus: f64) -> f64 {
    // A virgin point uses the default zero; a yielded perfect-plastic point truly has zero tangent.
    if point.tangent_modulus == 0.0 && point.equivalent_plastic_strain == 0.0 {
        youngs_modulus
    } else {
        point.tangent_modulus
    }
}

impl Frame2dSectionResponse {
    fn validate(&self) -> Result<(), String> {
        let scalars = [
            self.axial_force,
            self.moment_i,
            self.moment_j,
            self.average_stress,
            self.average_initial_stress,
            self.average_plastic_strain,
            self.average_backstress,
            self.max_equivalent_plastic_strain,
            self.max_fiber_damage,
        ];
        if scalars
            .iter()
            .chain(self.tangent.iter().flatten())
            .chain(&self.absolute_force_sums)
            .any(|value| !value.is_finite())
            || self
                .longitudinal_integration_error
                .is_some_and(|error| !error.is_finite())
        {
            return Err("frame 2d section response is non-finite".into());
        }
        // Inspect every candidate history, including inactive adaptive quadrature rules.
        for point in std::iter::once(&self.history.point).chain(&self.history.fiber_points) {
            if [
                point.plastic_strain,
                point.backstress,
                point.equivalent_plastic_strain,
                point.damage,
                point.tangent_modulus,
            ]
            .iter()
            .any(|value| !value.is_finite())
                || point.equivalent_plastic_strain < 0.0
                || !(0.0..1.0).contains(&point.damage)
            {
                return Err("frame 2d section material history is invalid or non-finite".into());
            }
        }
        Ok(())
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
        absolute_force_sums: [0.0; 3],
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
        max_fiber_damage: 0.0,
        fiber_point_count: 0,
        evaluated_fiber_point_count: 0,
        yielded_fiber_point_count: 0,
        damaged_fiber_point_count: 0,
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
        absolute_force_sums: [0.0; 3],
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
        max_fiber_damage: response.history.damage,
        fiber_point_count: 0,
        evaluated_fiber_point_count: 0,
        yielded_fiber_point_count: 0,
        damaged_fiber_point_count: 0,
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
) -> Result<Frame2dSectionResponse, String> {
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
    Ok(fiber_material_response_for_stations(
        material,
        youngs_modulus,
        area,
        length,
        extension,
        phi_i,
        phi_j,
        gauss_stations(material.longitudinal_integration_points),
        &committed.fiber_points,
    ))
}

#[allow(clippy::too_many_arguments)]
fn fiber_material_response_for_stations(
    material: &CompiledFrame2dMaterial,
    _youngs_modulus: f64,
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
    let mut absolute_force_sums = [0.0; 3];
    let mut tangent = [[0.0; 3]; 3];
    let mut average_initial_stress = 0.0;
    let mut average_plastic_strain = 0.0;
    let mut average_backstress = 0.0;
    let mut max_equivalent_plastic_strain = 0.0_f64;
    let mut max_fiber_damage = 0.0_f64;
    let mut yielded_fiber_point_count = 0;
    let mut damaged_fiber_point_count = 0;

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
            let response =
                fiber
                    .material
                    .response(strain, &committed_point, fiber.initial_axial_stress);
            let strain_gradient = [1.0 / length, fiber.y * curvature_i, fiber.y * curvature_j];
            let integration = length * weight * fiber.area;
            for row in 0..3 {
                let force = integration * response.stress * strain_gradient[row];
                forces[row] += force;
                absolute_force_sums[row] += force.abs();
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
            max_fiber_damage = max_fiber_damage.max(response.history.damage);
            yielded_fiber_point_count +=
                usize::from(response.history.equivalent_plastic_strain > 0.0);
            damaged_fiber_point_count += usize::from(response.history.damage > 0.0);
            history.push(response.history);
        }
    }

    Frame2dSectionResponse {
        axial_force: forces[0],
        moment_i: forces[1],
        moment_j: forces[2],
        absolute_force_sums,
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
        max_fiber_damage,
        fiber_point_count: point_count,
        evaluated_fiber_point_count: point_count,
        yielded_fiber_point_count,
        damaged_fiber_point_count,
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
) -> Result<Frame2dSectionResponse, String> {
    let fiber_count = material.section_fibers.len();
    let candidate = |point_count: usize| {
        let offset = adaptive_history_offset(point_count, fiber_count);
        let count = point_count * fiber_count;
        let committed_points = committed
            .fiber_points
            .get(offset..offset + count)
            .unwrap_or(&[]);
        let response = fiber_material_response_for_stations(
            material,
            youngs_modulus,
            area,
            length,
            extension,
            phi_i,
            phi_j,
            gauss_stations(point_count),
            committed_points,
        );
        response.validate().map_err(|error| {
            format!("adaptive longitudinal integration order {point_count}: {error}")
        })?;
        Ok::<_, String>(response)
    };
    let response_2 = candidate(2)?;
    let response_3 = candidate(3)?;
    let response_4 = candidate(4)?;
    let response_8 = candidate(8)?;
    let response_12 = candidate(12)?;
    let error_23 = generalized_force_error(&response_2, &response_3)?;
    let error_24 = generalized_force_error(&response_2, &response_4)?;
    let error_28 = generalized_force_error(&response_2, &response_8)?;
    let error_2_12 = generalized_force_error(&response_2, &response_12)?;
    let error_34 = generalized_force_error(&response_3, &response_4)?;
    let error_38 = generalized_force_error(&response_3, &response_8)?;
    let error_3_12 = generalized_force_error(&response_3, &response_12)?;
    let error_48 = generalized_force_error(&response_4, &response_8)?;
    let error_4_12 = generalized_force_error(&response_4, &response_12)?;
    let error_8_12 = generalized_force_error(&response_8, &response_12)?;
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
    Ok(selected)
}

fn generalized_force_error(
    candidate: &Frame2dSectionResponse,
    reference: &Frame2dSectionResponse,
) -> Result<f64, String> {
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
    let point_count = candidate.fiber_point_count.max(reference.fiber_point_count);
    let roundoff = 8.0 * f64::EPSILON * (point_count as f64 + 1.0);
    let mut error = 0.0_f64;
    for axis in 0..3 {
        let (left, right) = (candidate_forces[axis], reference_forces[axis]);
        let (left_sum, right_sum) = (
            candidate.absolute_force_sums[axis],
            reference.absolute_force_sums[axis],
        );
        if [left, right, left_sum, right_sum]
            .iter()
            .any(|value| !value.is_finite())
            || left_sum < 0.0
            || right_sum < 0.0
        {
            return Err(format!(
                "non-finite or invalid longitudinal integration component {axis}"
            ));
        }
        // Each force/moment uses its own accumulation scale, never a different component's units.
        let scale = left.abs().max(right.abs()).max(left_sum).max(right_sum);
        if scale == 0.0 {
            continue;
        }
        let left = left / scale;
        let right = right / scale;
        let difference = (left - right).abs();
        // Suppress only estimated accumulation roundoff, not a user-specified error tolerance.
        if difference <= roundoff {
            continue;
        }
        let denominator = left.abs().max(right.abs()).max(1e-12);
        error = error.max(difference / denominator);
    }
    Ok(error)
}

#[cfg(test)]
#[path = "frame_2d_fiber_section_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "frame_2d_fiber_adaptive_tests.rs"]
mod adaptive_tests;

#[cfg(test)]
#[path = "frame_2d_fiber_section_cyclic_reference.rs"]
mod cyclic_reference;
