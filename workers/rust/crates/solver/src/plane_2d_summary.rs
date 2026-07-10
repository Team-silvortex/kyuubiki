use kyuubiki_protocol::{
    PlaneNodeResult, PlaneQuadElementInput, PlaneQuadElementResult, PlaneTriangleElementInput,
    PlaneTriangleElementResult,
};

pub(super) fn max_plane_displacement(nodes: &[PlaneNodeResult]) -> f64 {
    nodes
        .iter()
        .map(|node| node.displacement_magnitude)
        .fold(0.0_f64, f64::max)
}

pub(super) fn max_triangle_stress(elements: &[PlaneTriangleElementResult]) -> f64 {
    elements
        .iter()
        .map(|element| element.von_mises.abs())
        .fold(0.0_f64, f64::max)
}

pub(super) fn max_quad_stress(elements: &[PlaneQuadElementResult]) -> f64 {
    elements
        .iter()
        .map(|element| element.von_mises.abs())
        .fold(0.0_f64, f64::max)
}

pub(super) fn triangle_total_strain_energy(
    elements: &[PlaneTriangleElementResult],
    inputs: &[PlaneTriangleElementInput],
) -> f64 {
    elements
        .iter()
        .zip(inputs.iter())
        .map(|(element, input)| element.strain_energy_density * element.area * input.thickness)
        .sum()
}

pub(super) fn quad_total_strain_energy(
    elements: &[PlaneQuadElementResult],
    inputs: &[PlaneQuadElementInput],
) -> f64 {
    elements
        .iter()
        .zip(inputs.iter())
        .map(|(element, input)| element.strain_energy_density * element.area * input.thickness)
        .sum()
}

pub(super) fn max_triangle_strain_energy_density(elements: &[PlaneTriangleElementResult]) -> f64 {
    elements
        .iter()
        .map(|element| element.strain_energy_density.abs())
        .fold(0.0_f64, f64::max)
}

pub(super) fn max_quad_strain_energy_density(elements: &[PlaneQuadElementResult]) -> f64 {
    elements
        .iter()
        .map(|element| element.strain_energy_density.abs())
        .fold(0.0_f64, f64::max)
}
