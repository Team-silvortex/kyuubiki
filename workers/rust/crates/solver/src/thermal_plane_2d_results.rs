use crate::plane_2d_math::{
    derive_planar_stress_metrics, multiply_matrix_vector_3x3, multiply_matrix_vector_3x6,
    subtract_vector_3,
};
use crate::thermal_plane_2d::ThermalPlaneTriangleComputed;
use kyuubiki_protocol::{SolveThermalPlaneTriangle2dRequest, ThermalPlaneNodeResult};

#[derive(Debug, Clone)]
pub(crate) struct ThermalPlaneTriangleState {
    pub(crate) total_strain: [f64; 3],
    pub(crate) mechanical_strain: [f64; 3],
    pub(crate) thermal_strain: f64,
    pub(crate) stress: [f64; 3],
    pub(crate) principal_stress_1: f64,
    pub(crate) principal_stress_2: f64,
    pub(crate) max_in_plane_shear: f64,
    pub(crate) von_mises: f64,
}

pub(crate) fn thermal_plane_triangle_state(
    computed: &ThermalPlaneTriangleComputed,
    element_displacements: &[f64; 6],
    thermal_expansion: f64,
) -> ThermalPlaneTriangleState {
    let total_strain = multiply_matrix_vector_3x6(&computed.b_matrix, element_displacements);
    let thermal_strain = thermal_expansion * computed.average_temperature_delta;
    let thermal_vector = [thermal_strain, thermal_strain, 0.0];
    let mechanical_strain = subtract_vector_3(&total_strain, &thermal_vector);
    let stress = multiply_matrix_vector_3x3(&computed.d_matrix, &mechanical_strain);
    let derived = derive_planar_stress_metrics(stress[0], stress[1], stress[2]);

    ThermalPlaneTriangleState {
        total_strain,
        mechanical_strain,
        thermal_strain,
        stress,
        principal_stress_1: derived.principal_stress_1,
        principal_stress_2: derived.principal_stress_2,
        max_in_plane_shear: derived.max_in_plane_shear,
        von_mises: derived.von_mises,
    }
}

pub(crate) fn build_thermal_plane_nodes(
    request: &SolveThermalPlaneTriangle2dRequest,
    displacements: &[f64],
) -> Vec<ThermalPlaneNodeResult> {
    request
        .nodes
        .iter()
        .enumerate()
        .map(|(index, node)| {
            let ux = displacements[index * 2];
            let uy = displacements[index * 2 + 1];
            ThermalPlaneNodeResult {
                index,
                id: node.id.clone(),
                x: node.x,
                y: node.y,
                ux,
                uy,
                displacement_magnitude: (ux * ux + uy * uy).sqrt(),
                temperature_delta: node.temperature_delta,
            }
        })
        .collect()
}

pub(crate) fn max_thermal_plane_displacement(nodes: &[ThermalPlaneNodeResult]) -> f64 {
    nodes
        .iter()
        .map(|node| node.displacement_magnitude)
        .fold(0.0_f64, f64::max)
}

pub(crate) fn max_temperature_delta(nodes: &[ThermalPlaneNodeResult]) -> f64 {
    nodes
        .iter()
        .map(|node| node.temperature_delta.abs())
        .fold(0.0_f64, f64::max)
}
