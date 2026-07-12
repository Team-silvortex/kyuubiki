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
        let ni = kyuubiki_protocol::PlaneNodeInput {
            id: request.nodes[element.node_i].id.clone(),
            x: request.nodes[element.node_i].x,
            y: request.nodes[element.node_i].y,
            fix_x: false,
            fix_y: false,
            load_x: 0.0,
            load_y: 0.0,
        };
        let nj = kyuubiki_protocol::PlaneNodeInput {
            id: request.nodes[element.node_j].id.clone(),
            x: request.nodes[element.node_j].x,
            y: request.nodes[element.node_j].y,
            fix_x: false,
            fix_y: false,
            load_x: 0.0,
            load_y: 0.0,
        };
        let nk = kyuubiki_protocol::PlaneNodeInput {
            id: request.nodes[element.node_k].id.clone(),
            x: request.nodes[element.node_k].x,
            y: request.nodes[element.node_k].y,
            fix_x: false,
            fix_y: false,
            load_x: 0.0,
            load_y: 0.0,
        };
        let area = signed_triangle_area(&ni, &nj, &nk).abs();
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

        let to_node = |index: usize| kyuubiki_protocol::PlaneNodeInput {
            id: request.nodes[index].id.clone(),
            x: request.nodes[index].x,
            y: request.nodes[index].y,
            fix_x: false,
            fix_y: false,
            load_x: 0.0,
            load_y: 0.0,
        };
        let ni = to_node(element.node_i);
        let nj = to_node(element.node_j);
        let nk = to_node(element.node_k);
        let nl = to_node(element.node_l);
        let first_area = signed_triangle_area(&ni, &nj, &nk).abs();
        let second_area = signed_triangle_area(&ni, &nk, &nl).abs();
        if !(first_area.is_finite() && first_area > 1.0e-12)
            || !(second_area.is_finite() && second_area > 1.0e-12)
        {
            return Err("electrostatic plane quad triangles must have positive area".to_string());
        }
    }

    Ok(())
}

fn signed_triangle_area(
    node_i: &kyuubiki_protocol::PlaneNodeInput,
    node_j: &kyuubiki_protocol::PlaneNodeInput,
    node_k: &kyuubiki_protocol::PlaneNodeInput,
) -> f64 {
    0.5 * ((node_j.x - node_i.x) * (node_k.y - node_i.y)
        - (node_k.x - node_i.x) * (node_j.y - node_i.y))
}

fn has_duplicate(indices: &[usize]) -> bool {
    indices
        .iter()
        .enumerate()
        .any(|(left, value)| indices[(left + 1)..].contains(value))
}
