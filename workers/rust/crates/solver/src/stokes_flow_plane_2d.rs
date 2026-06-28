use kyuubiki_protocol::{
    SolveStokesFlowPlaneQuad2dRequest, SolveStokesFlowPlaneQuad2dResult, StokesFlowPlaneNodeResult,
    StokesFlowPlaneQuadElementInput, StokesFlowPlaneQuadElementResult,
};

pub fn solve_stokes_flow_plane_quad_2d(
    request: &SolveStokesFlowPlaneQuad2dRequest,
) -> Result<SolveStokesFlowPlaneQuad2dResult, String> {
    validate_request(request)?;

    let nodes = request
        .nodes
        .iter()
        .enumerate()
        .map(|(index, node)| {
            let local_viscosity = average_viscosity_for_node(request, index).max(1.0e-12);
            let velocity_x = if node.fix_velocity_x {
                node.velocity_x
            } else {
                node.body_force_x / local_viscosity
            };
            let velocity_y = if node.fix_velocity_y {
                node.velocity_y
            } else {
                node.body_force_y / local_viscosity
            };
            let pressure = if node.fix_pressure {
                node.pressure
            } else {
                -(node.body_force_x * node.x + node.body_force_y * node.y)
            };

            StokesFlowPlaneNodeResult {
                index,
                id: node.id.clone(),
                x: node.x,
                y: node.y,
                velocity_x,
                velocity_y,
                velocity_magnitude: magnitude(velocity_x, velocity_y),
                pressure,
                body_force_x: node.body_force_x,
                body_force_y: node.body_force_y,
            }
        })
        .collect::<Vec<_>>();

    let elements = request
        .elements
        .iter()
        .enumerate()
        .map(|(index, element)| element_result(index, element, &nodes))
        .collect::<Result<Vec<_>, String>>()?;

    let max_velocity = nodes
        .iter()
        .map(|node| node.velocity_magnitude)
        .fold(0.0_f64, f64::max);
    let max_pressure = nodes
        .iter()
        .map(|node| node.pressure.abs())
        .fold(0.0_f64, f64::max);
    let max_divergence_error = elements
        .iter()
        .map(|element| element.divergence_error.abs())
        .fold(0.0_f64, f64::max);
    let max_reynolds_number = elements
        .iter()
        .map(|element| element.reynolds_number)
        .fold(0.0_f64, f64::max);

    Ok(SolveStokesFlowPlaneQuad2dResult {
        input: request.clone(),
        nodes,
        elements,
        max_velocity,
        max_pressure,
        max_divergence_error,
        max_reynolds_number,
    })
}

fn validate_request(request: &SolveStokesFlowPlaneQuad2dRequest) -> Result<(), String> {
    if request.nodes.len() < 4 {
        return Err("stokes flow quad model requires at least four nodes".into());
    }
    if request.elements.is_empty() {
        return Err("stokes flow quad model requires at least one element".into());
    }

    for (index, element) in request.elements.iter().enumerate() {
        validate_node_index(request.nodes.len(), element.node_i, index, "node_i")?;
        validate_node_index(request.nodes.len(), element.node_j, index, "node_j")?;
        validate_node_index(request.nodes.len(), element.node_k, index, "node_k")?;
        validate_node_index(request.nodes.len(), element.node_l, index, "node_l")?;
        if element.viscosity <= 0.0 {
            return Err(format!(
                "stokes flow element {index} has non-positive viscosity"
            ));
        }
        if element.density < 0.0 {
            return Err(format!("stokes flow element {index} has negative density"));
        }
        if element.thickness <= 0.0 {
            return Err(format!(
                "stokes flow element {index} has non-positive thickness"
            ));
        }
    }

    Ok(())
}

fn validate_node_index(
    node_count: usize,
    node_index: usize,
    element_index: usize,
    field: &str,
) -> Result<(), String> {
    if node_index >= node_count {
        Err(format!(
            "stokes flow element {element_index} {field} references missing node {node_index}"
        ))
    } else {
        Ok(())
    }
}

fn average_viscosity_for_node(
    request: &SolveStokesFlowPlaneQuad2dRequest,
    node_index: usize,
) -> f64 {
    let mut total = 0.0;
    let mut count = 0.0;

    for element in &request.elements {
        if [
            element.node_i,
            element.node_j,
            element.node_k,
            element.node_l,
        ]
        .contains(&node_index)
        {
            total += element.viscosity;
            count += 1.0;
        }
    }

    if count == 0.0 { 1.0 } else { total / count }
}

fn element_result(
    index: usize,
    element: &StokesFlowPlaneQuadElementInput,
    nodes: &[StokesFlowPlaneNodeResult],
) -> Result<StokesFlowPlaneQuadElementResult, String> {
    let n = [
        &nodes[element.node_i],
        &nodes[element.node_j],
        &nodes[element.node_k],
        &nodes[element.node_l],
    ];
    let area =
        polygon_area(&n).ok_or_else(|| format!("stokes flow element {index} has zero area"))?;
    let average_velocity_x = n.iter().map(|node| node.velocity_x).sum::<f64>() / 4.0;
    let average_velocity_y = n.iter().map(|node| node.velocity_y).sum::<f64>() / 4.0;
    let average_velocity_magnitude = magnitude(average_velocity_x, average_velocity_y);
    let average_pressure = n.iter().map(|node| node.pressure).sum::<f64>() / 4.0;
    let width = ((n[1].x - n[0].x).abs() + (n[2].x - n[3].x).abs()).max(1.0e-12) / 2.0;
    let height = ((n[3].y - n[0].y).abs() + (n[2].y - n[1].y).abs()).max(1.0e-12) / 2.0;
    let du_dx =
        ((n[1].velocity_x + n[2].velocity_x) - (n[0].velocity_x + n[3].velocity_x)) / (2.0 * width);
    let dv_dy = ((n[2].velocity_y + n[3].velocity_y) - (n[0].velocity_y + n[1].velocity_y))
        / (2.0 * height);
    let velocity_gradient_x =
        ((n[1].velocity_y + n[2].velocity_y) - (n[0].velocity_y + n[3].velocity_y)) / (2.0 * width);
    let velocity_gradient_y = ((n[2].velocity_x + n[3].velocity_x)
        - (n[0].velocity_x + n[1].velocity_x))
        / (2.0 * height);
    let characteristic_length = area.sqrt();
    let reynolds_number =
        element.density * average_velocity_magnitude * characteristic_length / element.viscosity;
    let viscous_dissipation = element.viscosity
        * (du_dx * du_dx
            + dv_dy * dv_dy
            + velocity_gradient_x * velocity_gradient_x
            + velocity_gradient_y * velocity_gradient_y)
        * area
        * element.thickness;

    Ok(StokesFlowPlaneQuadElementResult {
        index,
        id: element.id.clone(),
        node_i: element.node_i,
        node_j: element.node_j,
        node_k: element.node_k,
        node_l: element.node_l,
        area,
        average_velocity_x,
        average_velocity_y,
        average_velocity_magnitude,
        average_pressure,
        velocity_gradient_x,
        velocity_gradient_y,
        divergence_error: (du_dx + dv_dy).abs(),
        reynolds_number,
        viscous_dissipation,
    })
}

fn polygon_area(nodes: &[&StokesFlowPlaneNodeResult; 4]) -> Option<f64> {
    let mut sum = 0.0;
    for index in 0..4 {
        let current = nodes[index];
        let next = nodes[(index + 1) % 4];
        sum += current.x * next.y - next.x * current.y;
    }
    let area = 0.5 * sum.abs();
    (area > 1.0e-12).then_some(area)
}

fn magnitude(x: f64, y: f64) -> f64 {
    (x * x + y * y).sqrt()
}
