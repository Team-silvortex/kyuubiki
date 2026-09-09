use crate::plane_2d_math::{
    derive_planar_stress_metrics, multiply_matrix_vector_3x3, multiply_matrix_vector_3x6,
    strain_energy_density, subtract_vector_3,
};
use crate::solver_control::SolverStage;
use crate::solver_postprocess::{collect_results, fold_results, max_results};
use crate::thermal_plane_2d::ThermalPlaneTriangleComputed;
use kyuubiki_protocol::{
    SolveThermalPlaneQuad2dRequest, SolveThermalPlaneTriangle2dRequest, ThermalPlaneNodeInput,
    ThermalPlaneNodeResult, ThermalPlaneQuadElementResult, ThermalPlaneTriangleElementResult,
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
    nodes: &[ThermalPlaneNodeInput],
    displacements: &[f64],
) -> Result<Vec<ThermalPlaneNodeResult>, String> {
    collect_results(
        SolverStage::ResultNodes,
        nodes.iter().enumerate().map(|(index, node)| {
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
        }),
    )
}

pub(crate) fn max_thermal_plane_displacement(
    nodes: &[ThermalPlaneNodeResult],
) -> Result<f64, String> {
    max_results(SolverStage::ResultNodeSummary, nodes, |node| {
        node.displacement_magnitude
    })
}

pub(crate) fn max_temperature_delta(nodes: &[ThermalPlaneNodeResult]) -> Result<f64, String> {
    max_results(SolverStage::ResultNodeSummary, nodes, |node| {
        node.temperature_delta.abs()
    })
}

pub(crate) fn max_thermal_triangle_stress(
    elements: &[ThermalPlaneTriangleElementResult],
) -> Result<f64, String> {
    max_results(SolverStage::ResultElementSummary, elements, |element| {
        element.von_mises.abs()
    })
}

pub(crate) fn max_thermal_quad_stress(
    elements: &[ThermalPlaneQuadElementResult],
) -> Result<f64, String> {
    max_results(SolverStage::ResultElementSummary, elements, |element| {
        element.von_mises.abs()
    })
}

pub(crate) fn thermal_triangle_total_strain_energy(
    request: &SolveThermalPlaneTriangle2dRequest,
    elements: &[ThermalPlaneTriangleElementResult],
) -> Result<f64, String> {
    fold_results(
        SolverStage::ResultTotals,
        elements
            .iter()
            .zip(request.elements.iter())
            .map(|(element, input)| element.strain_energy_density * element.area * input.thickness),
        -0.0_f64,
        |sum, value| sum + value,
    )
}

pub(crate) fn thermal_quad_total_strain_energy(
    request: &SolveThermalPlaneQuad2dRequest,
    elements: &[ThermalPlaneQuadElementResult],
) -> Result<f64, String> {
    fold_results(
        SolverStage::ResultTotals,
        elements
            .iter()
            .zip(request.elements.iter())
            .map(|(element, input)| element.strain_energy_density * element.area * input.thickness),
        -0.0_f64,
        |sum, value| sum + value,
    )
}

pub(crate) fn max_thermal_triangle_strain_energy_density(
    elements: &[ThermalPlaneTriangleElementResult],
) -> Result<f64, String> {
    max_results(SolverStage::ResultElementSummary, elements, |element| {
        element.strain_energy_density.abs()
    })
}

pub(crate) fn max_thermal_quad_strain_energy_density(
    elements: &[ThermalPlaneQuadElementResult],
) -> Result<f64, String> {
    max_results(SolverStage::ResultElementSummary, elements, |element| {
        element.strain_energy_density.abs()
    })
}
