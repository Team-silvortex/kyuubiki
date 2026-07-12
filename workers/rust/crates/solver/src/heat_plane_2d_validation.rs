use kyuubiki_protocol::{SolveHeatPlaneQuad2dRequest, SolveHeatPlaneTriangle2dRequest};

pub(super) fn validate_heat_plane_triangle_request(
    request: &SolveHeatPlaneTriangle2dRequest,
) -> Result<(), String> {
    if request.nodes.len() < 3 {
        return Err("heat plane triangle model must define at least three nodes".to_string());
    }
    if request.elements.is_empty() {
        return Err("heat plane triangle model must define at least one element".to_string());
    }
    if !request.nodes.iter().any(|node| node.fix_temperature) {
        return Err(
            "heat plane triangle model must include at least one temperature support".to_string(),
        );
    }

    for node in &request.nodes {
        if !(node.x.is_finite() && node.y.is_finite()) {
            return Err("heat plane triangle node coordinates must be finite".to_string());
        }
        if !node.temperature.is_finite() {
            return Err("heat plane triangle node temperature must be finite".to_string());
        }
        if !node.heat_load.is_finite() {
            return Err("heat plane triangle node heat_load must be finite".to_string());
        }
    }

    for element in &request.elements {
        if element.node_i >= request.nodes.len()
            || element.node_j >= request.nodes.len()
            || element.node_k >= request.nodes.len()
        {
            return Err("heat plane triangle element references an out-of-range node".to_string());
        }
        if has_duplicate(&[element.node_i, element.node_j, element.node_k]) {
            return Err(
                "heat plane triangle element must reference three distinct nodes".to_string(),
            );
        }
        if !element.thickness.is_finite() || element.thickness <= 0.0 {
            return Err("heat plane triangle thickness must be positive".to_string());
        }
        if !element.conductivity.is_finite() || element.conductivity <= 0.0 {
            return Err("heat plane triangle conductivity must be positive".to_string());
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
        let area = signed_heat_triangle_area(&ni, &nj, &nk).abs();
        if !(area.is_finite() && area > 1.0e-12) {
            return Err("heat plane triangle element area must be positive".to_string());
        }
    }

    Ok(())
}

pub(super) fn validate_heat_plane_quad_request(
    request: &SolveHeatPlaneQuad2dRequest,
) -> Result<(), String> {
    if request.nodes.len() < 4 {
        return Err("heat plane quad model must define at least four nodes".to_string());
    }
    if request.elements.is_empty() {
        return Err("heat plane quad model must define at least one element".to_string());
    }
    if !request.nodes.iter().any(|node| node.fix_temperature) {
        return Err(
            "heat plane quad model must include at least one temperature support".to_string(),
        );
    }

    for node in &request.nodes {
        if !(node.x.is_finite() && node.y.is_finite()) {
            return Err("heat plane quad node coordinates must be finite".to_string());
        }
        if !node.temperature.is_finite() {
            return Err("heat plane quad node temperature must be finite".to_string());
        }
        if !node.heat_load.is_finite() {
            return Err("heat plane quad node heat_load must be finite".to_string());
        }
    }

    for element in &request.elements {
        if element.node_i >= request.nodes.len()
            || element.node_j >= request.nodes.len()
            || element.node_k >= request.nodes.len()
            || element.node_l >= request.nodes.len()
        {
            return Err("heat plane quad element references an out-of-range node".to_string());
        }
        if has_duplicate(&[
            element.node_i,
            element.node_j,
            element.node_k,
            element.node_l,
        ]) {
            return Err("heat plane quad element must reference four distinct nodes".to_string());
        }
        if !element.thickness.is_finite() || element.thickness <= 0.0 {
            return Err("heat plane quad thickness must be positive".to_string());
        }
        if !element.conductivity.is_finite() || element.conductivity <= 0.0 {
            return Err("heat plane quad conductivity must be positive".to_string());
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
        let first_area = signed_heat_triangle_area(&ni, &nj, &nk).abs();
        let second_area = signed_heat_triangle_area(&ni, &nk, &nl).abs();
        if !(first_area.is_finite() && first_area > 1.0e-12)
            || !(second_area.is_finite() && second_area > 1.0e-12)
        {
            return Err("heat plane quad triangles must have positive area".to_string());
        }
    }

    Ok(())
}

fn signed_heat_triangle_area(
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
