use crate::plane_2d_math::{
    derive_planar_stress_metrics, multiply_matrix_vector_3x3, multiply_matrix_vector_3x6,
    strain_energy_density, subtract_vector_3,
};
use crate::plane_2d_summary::{total_energy, validate_plane_result};
use crate::solver_control::SolverStage;
use crate::solver_postprocess::{max_results, try_collect_results};
use crate::thermal_plane_2d::ThermalPlaneTriangleComputed;
use kyuubiki_protocol::{
    SolveThermalPlaneQuad2dRequest, SolveThermalPlaneTriangle2dRequest, ThermalPlaneNodeInput,
    ThermalPlaneNodeResult, ThermalPlaneQuadElementResult, ThermalPlaneTriangleElementResult,
};

#[derive(Debug, Clone)]
pub(crate) struct ThermalPlaneState {
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

impl ThermalPlaneState {
    pub(crate) fn validate(&self, id: &str) -> Result<(), String> {
        validate_plane_result(
            id,
            &[
                self.total_strain[0],
                self.total_strain[1],
                self.total_strain[2],
                self.mechanical_strain[0],
                self.mechanical_strain[1],
                self.mechanical_strain[2],
                self.thermal_strain,
                self.stress[0],
                self.stress[1],
                self.stress[2],
                self.principal_stress_1,
                self.principal_stress_2,
                self.max_in_plane_shear,
                self.von_mises,
                self.strain_energy_density,
            ],
        )
        .map_err(|error| format!("thermal {error}"))
    }
}

pub(crate) fn thermal_plane_triangle_state(
    computed: &ThermalPlaneTriangleComputed,
    element_displacements: &[f64; 6],
    thermal_expansion: f64,
) -> ThermalPlaneState {
    let total_strain = multiply_matrix_vector_3x6(&computed.b_matrix, element_displacements);
    let thermal_strain = thermal_expansion * computed.average_temperature_delta;
    let thermal_vector = [thermal_strain, thermal_strain, 0.0];
    let mechanical_strain = subtract_vector_3(&total_strain, &thermal_vector);
    let stress = multiply_matrix_vector_3x3(&computed.d_matrix, &mechanical_strain);
    let derived = derive_planar_stress_metrics(stress[0], stress[1], stress[2]);

    ThermalPlaneState {
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
    try_collect_results(
        SolverStage::ResultNodes,
        nodes.iter().enumerate().map(|(index, node)| {
            let ux = displacements[index * 2];
            let uy = displacements[index * 2 + 1];
            let displacement_magnitude = ux.hypot(uy);
            if !displacement_magnitude.is_finite() {
                return Err(format!(
                    "thermal plane node {}: displacement is not representable",
                    node.id
                ));
            }
            Ok(ThermalPlaneNodeResult {
                index,
                id: node.id.clone(),
                x: node.x,
                y: node.y,
                ux,
                uy,
                displacement_magnitude,
                temperature_delta: node.temperature_delta,
            })
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
    total_energy(
        elements
            .iter()
            .zip(&request.elements)
            .map(|(element, input)| {
                (
                    element.id.as_str(),
                    element.strain_energy_density,
                    element.area,
                    input.thickness,
                )
            }),
    )
    .map_err(|error| format!("thermal {error}"))
}

pub(crate) fn thermal_quad_total_strain_energy(
    request: &SolveThermalPlaneQuad2dRequest,
    elements: &[ThermalPlaneQuadElementResult],
) -> Result<f64, String> {
    total_energy(
        elements
            .iter()
            .zip(&request.elements)
            .map(|(element, input)| {
                (
                    element.id.as_str(),
                    element.strain_energy_density,
                    element.area,
                    input.thickness,
                )
            }),
    )
    .map_err(|error| format!("thermal {error}"))
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

pub(crate) fn weighted_average<const N: usize>(values: [f64; N], weights: [f64; N]) -> f64 {
    if values.iter().any(|value| !value.is_finite())
        || weights
            .iter()
            .any(|weight| !weight.is_finite() || *weight < 0.0)
    {
        return f64::NAN;
    }
    let weight_sum: f64 = weights.iter().sum();
    if !weight_sum.is_finite() || weight_sum <= 0.0 {
        return f64::NAN;
    }
    let scale = values.iter().map(|value| value.abs()).fold(0.0, f64::max);
    if scale == 0.0 {
        return 0.0;
    }
    // Normalize before integration and compensate cancellation. A convex mean
    // cannot exceed the value scale; clamp only rounding drift at that bound.
    let mut sum = 0.0_f64;
    let mut correction = 0.0;
    for (value, weight) in values.into_iter().zip(weights) {
        let term = (value / scale) * weight;
        let next = sum + term;
        correction += if sum.abs() >= term.abs() {
            (sum - next) + term
        } else {
            (term - next) + sum
        };
        sum = next;
    }
    ((sum + correction) / weight_sum).clamp(-1.0, 1.0) * scale
}

#[cfg(test)]
#[path = "thermal_plane_output_kernel_tests.rs"]
mod tests;
