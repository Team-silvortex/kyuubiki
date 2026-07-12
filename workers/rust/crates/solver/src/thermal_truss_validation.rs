use kyuubiki_protocol::{SolveThermalTruss2dRequest, SolveThermalTruss3dRequest};

pub(crate) fn validate_thermal_truss_2d_request(
    request: &SolveThermalTruss2dRequest,
) -> Result<(), String> {
    if request.nodes.len() < 2 {
        return Err("thermal truss must define at least two nodes".to_string());
    }

    if request.elements.is_empty() {
        return Err("thermal truss must define at least one element".to_string());
    }

    if !request.nodes.iter().any(|node| node.fix_x || node.fix_y) {
        return Err("thermal truss must include at least one support".to_string());
    }

    for (index, node) in request.nodes.iter().enumerate() {
        if !(node.x.is_finite() && node.y.is_finite()) {
            return Err(format!(
                "thermal truss node {index} coordinates must be finite"
            ));
        }
        if !(node.load_x.is_finite() && node.load_y.is_finite()) {
            return Err(format!("thermal truss node {index} load must be finite"));
        }
        if !node.temperature_delta.is_finite() {
            return Err(format!(
                "thermal truss node {index} temperature_delta must be finite"
            ));
        }
    }

    for element in &request.elements {
        if element.node_i >= request.nodes.len() || element.node_j >= request.nodes.len() {
            return Err("thermal truss element references an out-of-range node".to_string());
        }
        if element.node_i == element.node_j {
            return Err("thermal truss element must connect two distinct nodes".to_string());
        }

        if !(element.area.is_finite() && element.area > 0.0) {
            return Err("thermal truss element area must be positive".to_string());
        }

        if !(element.youngs_modulus.is_finite() && element.youngs_modulus > 0.0) {
            return Err("thermal truss element youngs_modulus must be positive".to_string());
        }

        if !(element.thermal_expansion.is_finite() && element.thermal_expansion >= 0.0) {
            return Err("thermal truss element thermal_expansion must be non-negative".to_string());
        }

        let node_i = &request.nodes[element.node_i];
        let node_j = &request.nodes[element.node_j];
        let dx = node_j.x - node_i.x;
        let dy = node_j.y - node_i.y;
        let length = (dx * dx + dy * dy).sqrt();
        if !(length.is_finite() && length > 1.0e-12) {
            return Err("thermal truss element length must be positive".to_string());
        }
    }

    Ok(())
}

pub(crate) fn validate_thermal_truss_3d_request(
    request: &SolveThermalTruss3dRequest,
) -> Result<(), String> {
    if request.nodes.len() < 3 {
        return Err("3d thermal truss must define at least three nodes".to_string());
    }

    if request.elements.is_empty() {
        return Err("3d thermal truss must define at least one element".to_string());
    }

    let constrained_dofs = request.nodes.iter().fold(0, |sum, node| {
        sum + usize::from(node.fix_x) + usize::from(node.fix_y) + usize::from(node.fix_z)
    });
    if constrained_dofs < 6 {
        return Err("3d thermal truss must restrain at least six degrees of freedom".to_string());
    }

    for (index, node) in request.nodes.iter().enumerate() {
        if !(node.x.is_finite() && node.y.is_finite() && node.z.is_finite()) {
            return Err(format!(
                "3d thermal truss node {index} coordinates must be finite"
            ));
        }
        if !(node.load_x.is_finite() && node.load_y.is_finite() && node.load_z.is_finite()) {
            return Err(format!("3d thermal truss node {index} load must be finite"));
        }
        if !node.temperature_delta.is_finite() {
            return Err(format!(
                "3d thermal truss node {index} temperature_delta must be finite"
            ));
        }
    }

    for element in &request.elements {
        if element.node_i >= request.nodes.len() || element.node_j >= request.nodes.len() {
            return Err("3d thermal truss element references an out-of-range node".to_string());
        }
        if element.node_i == element.node_j {
            return Err("3d thermal truss element must connect two distinct nodes".to_string());
        }
        if !(element.area.is_finite() && element.area > 0.0) {
            return Err("3d thermal truss element area must be positive".to_string());
        }
        if !(element.youngs_modulus.is_finite() && element.youngs_modulus > 0.0) {
            return Err("3d thermal truss element youngs_modulus must be positive".to_string());
        }
        if !(element.thermal_expansion.is_finite() && element.thermal_expansion >= 0.0) {
            return Err(
                "3d thermal truss element thermal_expansion must be non-negative".to_string(),
            );
        }
        let node_i = &request.nodes[element.node_i];
        let node_j = &request.nodes[element.node_j];
        let length = ((node_j.x - node_i.x).powi(2)
            + (node_j.y - node_i.y).powi(2)
            + (node_j.z - node_i.z).powi(2))
        .sqrt();
        if !(length.is_finite() && length > 1.0e-12) {
            return Err("3d thermal truss element length must be positive".to_string());
        }
    }

    Ok(())
}

pub(crate) fn validate_small_displacement_thermal_truss_2d(
    request: &SolveThermalTruss2dRequest,
    max_displacement: f64,
) -> Result<(), String> {
    let bounds = get_planar_bounds(
        &request
            .nodes
            .iter()
            .map(|node| (node.x, node.y))
            .collect::<Vec<_>>(),
    );
    let characteristic_length = bounds.0.max(bounds.1).max(1.0e-9);

    if max_displacement > characteristic_length * 0.25 {
        return Err(
            "thermal truss response exceeds the small-deformation limit; check supports or connectivity"
                .to_string(),
        );
    }

    Ok(())
}

pub(crate) fn validate_small_displacement_thermal_truss_3d(
    request: &SolveThermalTruss3dRequest,
    max_displacement: f64,
) -> Result<(), String> {
    let characteristic_length = get_spatial_bounds(
        &request
            .nodes
            .iter()
            .map(|node| (node.x, node.y, node.z))
            .collect::<Vec<_>>(),
    );

    if max_displacement > characteristic_length * 0.25 {
        return Err(
            "3d thermal truss response exceeds the small-deformation limit; check supports or connectivity"
                .to_string(),
        );
    }

    Ok(())
}

fn get_planar_bounds(points: &[(f64, f64)]) -> (f64, f64) {
    let min_x = points.iter().map(|point| point.0).fold(0.0_f64, f64::min);
    let max_x = points.iter().map(|point| point.0).fold(1.0_f64, f64::max);
    let min_y = points.iter().map(|point| point.1).fold(0.0_f64, f64::min);
    let max_y = points.iter().map(|point| point.1).fold(1.0_f64, f64::max);

    (max_x - min_x, max_y - min_y)
}

fn get_spatial_bounds(points: &[(f64, f64, f64)]) -> f64 {
    let min_x = points.iter().map(|point| point.0).fold(0.0_f64, f64::min);
    let max_x = points.iter().map(|point| point.0).fold(1.0_f64, f64::max);
    let min_y = points.iter().map(|point| point.1).fold(0.0_f64, f64::min);
    let max_y = points.iter().map(|point| point.1).fold(1.0_f64, f64::max);
    let min_z = points.iter().map(|point| point.2).fold(0.0_f64, f64::min);
    let max_z = points.iter().map(|point| point.2).fold(1.0_f64, f64::max);

    (max_x - min_x).max(max_y - min_y).max(max_z - min_z)
}
