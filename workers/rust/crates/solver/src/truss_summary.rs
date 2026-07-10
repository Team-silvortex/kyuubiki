use kyuubiki_protocol::{
    SolveTruss2dRequest, SolveTruss3dRequest, Truss3dElementResult, Truss3dNodeResult,
    TrussElementResult, TrussNodeResult,
};

pub(super) fn max_truss_2d_displacement(nodes: &[TrussNodeResult]) -> f64 {
    nodes
        .iter()
        .map(|node| (node.ux * node.ux + node.uy * node.uy).sqrt())
        .fold(0.0_f64, f64::max)
}

pub(super) fn max_truss_3d_displacement(nodes: &[Truss3dNodeResult]) -> f64 {
    nodes
        .iter()
        .map(|node| (node.ux * node.ux + node.uy * node.uy + node.uz * node.uz).sqrt())
        .fold(0.0_f64, f64::max)
}

pub(super) fn max_truss_stress(elements: &[TrussElementResult]) -> f64 {
    elements
        .iter()
        .map(|element| element.stress.abs())
        .fold(0.0_f64, f64::max)
}

pub(super) fn max_truss_3d_stress(elements: &[Truss3dElementResult]) -> f64 {
    elements
        .iter()
        .map(|element| element.stress.abs())
        .fold(0.0_f64, f64::max)
}

pub(super) fn total_truss_2d_strain_energy(
    request: &SolveTruss2dRequest,
    elements: &[TrussElementResult],
) -> f64 {
    elements
        .iter()
        .zip(request.elements.iter())
        .map(|(element, input)| element.strain_energy_density * input.area * element.length)
        .sum()
}

pub(super) fn total_truss_3d_strain_energy(
    request: &SolveTruss3dRequest,
    elements: &[Truss3dElementResult],
) -> f64 {
    elements
        .iter()
        .zip(request.elements.iter())
        .map(|(element, input)| element.strain_energy_density * input.area * element.length)
        .sum()
}

pub(super) fn max_truss_strain_energy_density(elements: &[TrussElementResult]) -> f64 {
    elements
        .iter()
        .map(|element| element.strain_energy_density.abs())
        .fold(0.0_f64, f64::max)
}

pub(super) fn max_truss_3d_strain_energy_density(elements: &[Truss3dElementResult]) -> f64 {
    elements
        .iter()
        .map(|element| element.strain_energy_density.abs())
        .fold(0.0_f64, f64::max)
}

pub(super) fn validate_small_displacement_truss(
    request: &SolveTruss2dRequest,
    max_displacement: f64,
) -> Result<(), String> {
    let bounds = planar_bounds(
        &request
            .nodes
            .iter()
            .map(|node| (node.x, node.y))
            .collect::<Vec<_>>(),
    );
    let characteristic_length = bounds.0.max(bounds.1).max(1.0e-9);

    if max_displacement > characteristic_length * 0.25 {
        return Err(
            "truss response exceeds the small-deformation limit; check supports or connectivity"
                .to_string(),
        );
    }

    Ok(())
}

pub(super) fn validate_small_displacement_truss_3d(
    request: &SolveTruss3dRequest,
    max_displacement: f64,
) -> Result<(), String> {
    let characteristic_length = spatial_bounds(
        &request
            .nodes
            .iter()
            .map(|node| (node.x, node.y, node.z))
            .collect::<Vec<_>>(),
    );

    if max_displacement > characteristic_length * 0.25 {
        return Err(
            "3d truss response exceeds the small-deformation limit; check supports or connectivity"
                .to_string(),
        );
    }

    Ok(())
}

fn planar_bounds(points: &[(f64, f64)]) -> (f64, f64) {
    let min_x = points.iter().map(|point| point.0).fold(0.0_f64, f64::min);
    let max_x = points.iter().map(|point| point.0).fold(1.0_f64, f64::max);
    let min_y = points.iter().map(|point| point.1).fold(0.0_f64, f64::min);
    let max_y = points.iter().map(|point| point.1).fold(1.0_f64, f64::max);

    (max_x - min_x, max_y - min_y)
}

fn spatial_bounds(points: &[(f64, f64, f64)]) -> f64 {
    let min_x = points.iter().map(|point| point.0).fold(0.0_f64, f64::min);
    let max_x = points.iter().map(|point| point.0).fold(1.0_f64, f64::max);
    let min_y = points.iter().map(|point| point.1).fold(0.0_f64, f64::min);
    let max_y = points.iter().map(|point| point.1).fold(1.0_f64, f64::max);
    let min_z = points.iter().map(|point| point.2).fold(0.0_f64, f64::min);
    let max_z = points.iter().map(|point| point.2).fold(1.0_f64, f64::max);

    (max_x - min_x).max(max_y - min_y).max(max_z - min_z)
}
