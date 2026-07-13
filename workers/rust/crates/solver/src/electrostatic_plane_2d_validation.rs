use kyuubiki_protocol::{
    SolveElectrostaticPlaneQuad2dRequest, SolveElectrostaticPlaneTriangle2dRequest,
};

pub(super) fn validate_electrostatic_plane_triangle_request(
    request: &SolveElectrostaticPlaneTriangle2dRequest,
) -> Result<(), String> {
    if request.nodes.len() < 3 {
        return Err(
            "electrostatic plane triangle model must define at least three nodes".to_string(),
        );
    }
    if request.elements.is_empty() {
        return Err(
            "electrostatic plane triangle model must define at least one element".to_string(),
        );
    }
    if !request.nodes.iter().any(|node| node.fix_potential) {
        return Err(
            "electrostatic plane triangle model must include at least one potential support"
                .to_string(),
        );
    }

    for node in &request.nodes {
        if !(node.x.is_finite() && node.y.is_finite()) {
            return Err("electrostatic plane triangle node coordinates must be finite".to_string());
        }
        if !node.potential.is_finite() {
            return Err("electrostatic plane triangle node potential must be finite".to_string());
        }
        if !node.charge_density.is_finite() {
            return Err(
                "electrostatic plane triangle node charge_density must be finite".to_string(),
            );
        }
    }

    for element in &request.elements {
        if element.node_i >= request.nodes.len()
            || element.node_j >= request.nodes.len()
            || element.node_k >= request.nodes.len()
        {
            return Err(
                "electrostatic plane triangle element references an out-of-range node".to_string(),
            );
        }
        if has_duplicate(&[element.node_i, element.node_j, element.node_k]) {
            return Err(
                "electrostatic plane triangle element must reference three distinct nodes"
                    .to_string(),
            );
        }
        if !element.thickness.is_finite() || element.thickness <= 0.0 {
            return Err("electrostatic plane triangle thickness must be positive".to_string());
        }
        if !element.permittivity.is_finite() || element.permittivity <= 0.0 {
            return Err("electrostatic plane triangle permittivity must be positive".to_string());
        }
        let n = |index: usize| &request.nodes[index];
        let area = signed_triangle_area(
            n(element.node_i).x,
            n(element.node_i).y,
            n(element.node_j).x,
            n(element.node_j).y,
            n(element.node_k).x,
            n(element.node_k).y,
        )
        .abs();
        if !(area.is_finite() && area > 1.0e-12) {
            return Err("electrostatic plane triangle element area must be positive".to_string());
        }
    }

    Ok(())
}

pub(super) fn validate_electrostatic_plane_quad_request(
    request: &SolveElectrostaticPlaneQuad2dRequest,
) -> Result<(), String> {
    if request.nodes.len() < 4 {
        return Err("electrostatic plane quad model must define at least four nodes".to_string());
    }
    if request.elements.is_empty() {
        return Err("electrostatic plane quad model must define at least one element".to_string());
    }
    if !request.nodes.iter().any(|node| node.fix_potential) {
        return Err(
            "electrostatic plane quad model must include at least one potential support"
                .to_string(),
        );
    }

    for node in &request.nodes {
        if !(node.x.is_finite() && node.y.is_finite()) {
            return Err("electrostatic plane quad node coordinates must be finite".to_string());
        }
        if !node.potential.is_finite() {
            return Err("electrostatic plane quad node potential must be finite".to_string());
        }
        if !node.charge_density.is_finite() {
            return Err("electrostatic plane quad node charge_density must be finite".to_string());
        }
    }

    for element in &request.elements {
        if element.node_i >= request.nodes.len()
            || element.node_j >= request.nodes.len()
            || element.node_k >= request.nodes.len()
            || element.node_l >= request.nodes.len()
        {
            return Err(
                "electrostatic plane quad element references an out-of-range node".to_string(),
            );
        }
        if has_duplicate(&[
            element.node_i,
            element.node_j,
            element.node_k,
            element.node_l,
        ]) {
            return Err(
                "electrostatic plane quad element must reference four distinct nodes".to_string(),
            );
        }
        if !element.thickness.is_finite() || element.thickness <= 0.0 {
            return Err("electrostatic plane quad thickness must be positive".to_string());
        }
        if !element.permittivity.is_finite() || element.permittivity <= 0.0 {
            return Err("electrostatic plane quad permittivity must be positive".to_string());
        }

        let n = |index: usize| &request.nodes[index];
        let first_area = signed_triangle_area(
            n(element.node_i).x,
            n(element.node_i).y,
            n(element.node_j).x,
            n(element.node_j).y,
            n(element.node_k).x,
            n(element.node_k).y,
        )
        .abs();
        let second_area = signed_triangle_area(
            n(element.node_i).x,
            n(element.node_i).y,
            n(element.node_k).x,
            n(element.node_k).y,
            n(element.node_l).x,
            n(element.node_l).y,
        )
        .abs();
        if !(first_area.is_finite() && first_area > 1.0e-12)
            || !(second_area.is_finite() && second_area > 1.0e-12)
        {
            return Err("electrostatic plane quad triangles must have positive area".to_string());
        }
    }

    Ok(())
}

fn signed_triangle_area(ix: f64, iy: f64, jx: f64, jy: f64, kx: f64, ky: f64) -> f64 {
    0.5 * ((jx - ix) * (ky - iy) - (kx - ix) * (jy - iy))
}

fn has_duplicate(indices: &[usize]) -> bool {
    indices
        .iter()
        .enumerate()
        .any(|(left, value)| indices[(left + 1)..].contains(value))
}
