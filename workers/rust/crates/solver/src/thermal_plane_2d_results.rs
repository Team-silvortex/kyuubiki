use crate::plane_2d_math::{
    derive_planar_stress_metrics, multiply_matrix_vector_3x3, multiply_matrix_vector_3x6,
    strain_energy_density, subtract_vector_3,
};
use crate::thermal_plane_2d::ThermalPlaneTriangleComputed;
use kyuubiki_protocol::{
    SolveThermalPlaneQuad2dRequest, SolveThermalPlaneTriangle2dRequest, ThermalPlaneNodeResult,
    ThermalPlaneQuadElementResult, ThermalPlaneTriangleElementResult,
};

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
    pub(crate) strain_energy_density: f64,
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
        strain_energy_density: strain_energy_density(&stress, &mechanical_strain),
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

pub(crate) fn max_thermal_triangle_stress(elements: &[ThermalPlaneTriangleElementResult]) -> f64 {
    elements
        .iter()
        .map(|element| element.von_mises.abs())
        .fold(0.0_f64, f64::max)
}

pub(crate) fn max_thermal_quad_stress(elements: &[ThermalPlaneQuadElementResult]) -> f64 {
    elements
        .iter()
        .map(|element| element.von_mises.abs())
        .fold(0.0_f64, f64::max)
}

pub(crate) fn thermal_triangle_total_strain_energy(
    request: &SolveThermalPlaneTriangle2dRequest,
    elements: &[ThermalPlaneTriangleElementResult],
) -> f64 {
    elements
        .iter()
        .zip(request.elements.iter())
        .map(|(element, input)| element.strain_energy_density * element.area * input.thickness)
        .sum()
}

pub(crate) fn thermal_quad_total_strain_energy(
    request: &SolveThermalPlaneQuad2dRequest,
    elements: &[ThermalPlaneQuadElementResult],
) -> f64 {
    elements
        .iter()
        .zip(request.elements.iter())
        .map(|(element, input)| element.strain_energy_density * element.area * input.thickness)
        .sum()
}

pub(crate) fn max_thermal_triangle_strain_energy_density(
    elements: &[ThermalPlaneTriangleElementResult],
) -> f64 {
    elements
        .iter()
        .map(|element| element.strain_energy_density.abs())
        .fold(0.0_f64, f64::max)
}

pub(crate) fn max_thermal_quad_strain_energy_density(
    elements: &[ThermalPlaneQuadElementResult],
) -> f64 {
    elements
        .iter()
        .map(|element| element.strain_energy_density.abs())
        .fold(0.0_f64, f64::max)
}
