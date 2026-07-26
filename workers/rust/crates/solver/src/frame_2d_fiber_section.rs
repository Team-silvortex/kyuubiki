use crate::frame_2d_material_p_delta::{
    CompiledFrame2dMaterial, Frame2dMaterialHistory, Frame2dMaterialPointHistory,
};

const GAUSS_STATIONS: [(f64, f64); 2] = [
    (0.211_324_865_405_187_13, 0.5),
    (0.788_675_134_594_812_9, 0.5),
];

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
    pub(crate) yielded_fiber_point_count: usize,
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
    if committed.fiber_points.len() != material.section_fibers.len() * GAUSS_STATIONS.len() {
        return youngs_modulus;
    }
    GAUSS_STATIONS
        .iter()
        .enumerate()
        .flat_map(|(station_index, &(_, weight))| {
            material
                .section_fibers
                .iter()
                .enumerate()
                .map(move |(fiber_index, fiber)| {
                    let point_index = station_index * material.section_fibers.len() + fiber_index;
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
        yielded_fiber_point_count: 0,
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
        },
        average_stress: response.stress,
        average_initial_stress: material.initial_axial_stress,
        average_plastic_strain: response.history.plastic_strain,
        average_backstress: response.history.backstress,
        max_equivalent_plastic_strain: response.history.equivalent_plastic_strain,
        fiber_point_count: 0,
        yielded_fiber_point_count: 0,
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
    let fiber_count = material.section_fibers.len();
    let point_count = fiber_count * GAUSS_STATIONS.len();
    let mut history = Vec::with_capacity(point_count);
    let mut forces = [0.0; 3];
    let mut tangent = [[0.0; 3]; 3];
    let mut average_initial_stress = 0.0;
    let mut average_plastic_strain = 0.0;
    let mut average_backstress = 0.0;
    let mut max_equivalent_plastic_strain = 0.0_f64;
    let mut yielded_fiber_point_count = 0;

    for (station_index, &(xi, weight)) in GAUSS_STATIONS.iter().enumerate() {
        let curvature_i = (-4.0 + 6.0 * xi) / length;
        let curvature_j = (-2.0 + 6.0 * xi) / length;
        for (fiber_index, fiber) in material.section_fibers.iter().enumerate() {
            let point_index = station_index * fiber_count + fiber_index;
            let committed_point = committed
                .fiber_points
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
        },
        average_stress: forces[0] / area,
        average_initial_stress,
        average_plastic_strain,
        average_backstress,
        max_equivalent_plastic_strain,
        fiber_point_count: point_count,
        yielded_fiber_point_count,
    }
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
        }
    }

    fn assert_close(actual: f64, expected: f64, tolerance: f64) {
        assert!(
            (actual - expected).abs() <= tolerance,
            "actual={actual:.12e}, expected={expected:.12e}"
        );
    }
}
