use kyuubiki_protocol::{
    SolveStokesFlowPlaneQuad2dRequest, SolveStokesFlowPlaneQuad2dResult,
    SolveStokesFlowPlaneTriangle2dRequest, SolveStokesFlowPlaneTriangle2dResult,
    StokesFlowPlaneNodeResult, StokesFlowPlaneQuadElementInput, StokesFlowPlaneQuadElementResult,
    StokesFlowPlaneTriangleElementInput, StokesFlowPlaneTriangleElementResult,
};

pub fn solve_stokes_flow_plane_quad_2d(
    request: &SolveStokesFlowPlaneQuad2dRequest,
) -> Result<SolveStokesFlowPlaneQuad2dResult, String> {
    validate_quad_request(request)?;

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

    let summary = summarize_flow(&nodes, elements.iter().map(element_summary_fields));
    Ok(SolveStokesFlowPlaneQuad2dResult {
        input: request.clone(),
        nodes,
        elements,
        max_velocity: summary.max_velocity,
        max_pressure: summary.max_pressure,
        pressure_drop: summary.pressure_drop,
        max_divergence_error: summary.max_divergence_error,
        max_reynolds_number: summary.max_reynolds_number,
        max_shear_rate: summary.max_shear_rate,
        max_viscous_shear_stress: summary.max_viscous_shear_stress,
    })
}

pub fn solve_stokes_flow_plane_triangle_2d(
    request: &SolveStokesFlowPlaneTriangle2dRequest,
) -> Result<SolveStokesFlowPlaneTriangle2dResult, String> {
    validate_triangle_request(request)?;
    let nodes = request
        .nodes
        .iter()
        .enumerate()
        .map(|(index, node)| {
            let local_viscosity = average_triangle_viscosity_for_node(request, index).max(1.0e-12);
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
        .map(|(index, element)| triangle_element_result(index, element, &nodes))
        .collect::<Result<Vec<_>, String>>()?;
    let summary = summarize_flow(&nodes, elements.iter().map(triangle_element_summary_fields));
    Ok(SolveStokesFlowPlaneTriangle2dResult {
        input: request.clone(),
        nodes,
        elements,
        max_velocity: summary.max_velocity,
        max_pressure: summary.max_pressure,
        pressure_drop: summary.pressure_drop,
        max_divergence_error: summary.max_divergence_error,
        max_reynolds_number: summary.max_reynolds_number,
        max_shear_rate: summary.max_shear_rate,
        max_viscous_shear_stress: summary.max_viscous_shear_stress,
    })
}

fn summarize_flow(
    nodes: &[StokesFlowPlaneNodeResult],
    element_fields: impl Iterator<Item = ElementSummaryFields>,
) -> FlowSummary {
    let max_velocity = nodes
        .iter()
        .map(|node| node.velocity_magnitude)
        .fold(0.0_f64, f64::max);
    let max_pressure = nodes
        .iter()
        .map(|node| node.pressure.abs())
        .fold(0.0_f64, f64::max);
    let min_pressure = nodes
        .iter()
        .map(|node| node.pressure)
        .fold(f64::INFINITY, f64::min);
    let max_signed_pressure = nodes
        .iter()
        .map(|node| node.pressure)
        .fold(f64::NEG_INFINITY, f64::max);
    let mut summary = FlowSummary {
        max_velocity,
        max_pressure,
        pressure_drop: max_signed_pressure - min_pressure,
        max_divergence_error: 0.0,
        max_reynolds_number: 0.0,
        max_shear_rate: 0.0,
        max_viscous_shear_stress: 0.0,
    };
    for fields in element_fields {
        summary.max_divergence_error = summary
            .max_divergence_error
            .max(fields.divergence_error.abs());
        summary.max_reynolds_number = summary.max_reynolds_number.max(fields.reynolds_number);
        summary.max_shear_rate = summary.max_shear_rate.max(fields.shear_rate);
        summary.max_viscous_shear_stress = summary
            .max_viscous_shear_stress
            .max(fields.max_viscous_shear_stress);
    }
    summary
}

fn validate_quad_request(request: &SolveStokesFlowPlaneQuad2dRequest) -> Result<(), String> {
    if request.nodes.len() < 4 {
        return Err("stokes flow quad model requires at least four nodes".into());
    }
    if request.elements.is_empty() {
        return Err("stokes flow quad model requires at least one element".into());
    }

    validate_nodes(&request.nodes)?;

    for (index, element) in request.elements.iter().enumerate() {
        validate_node_index(request.nodes.len(), element.node_i, index, "node_i")?;
        validate_node_index(request.nodes.len(), element.node_j, index, "node_j")?;
        validate_node_index(request.nodes.len(), element.node_k, index, "node_k")?;
        validate_node_index(request.nodes.len(), element.node_l, index, "node_l")?;
        validate_stokes_material(index, element.viscosity, element.density, element.thickness)?;
    }

    Ok(())
}

fn validate_nodes(nodes: &[kyuubiki_protocol::StokesFlowPlaneNodeInput]) -> Result<(), String> {
    for (index, node) in nodes.iter().enumerate() {
        if !(node.x.is_finite() && node.y.is_finite()) {
            return Err(format!(
                "stokes flow node {index} coordinates must be finite"
            ));
        }
        if !(node.velocity_x.is_finite() && node.velocity_y.is_finite()) {
            return Err(format!("stokes flow node {index} velocity must be finite"));
        }
        if !node.pressure.is_finite() {
            return Err(format!("stokes flow node {index} pressure must be finite"));
        }
        if !(node.body_force_x.is_finite() && node.body_force_y.is_finite()) {
            return Err(format!(
                "stokes flow node {index} body force must be finite"
            ));
        }
    }
    Ok(())
}

fn validate_stokes_material(
    index: usize,
    viscosity: f64,
    density: f64,
    thickness: f64,
) -> Result<(), String> {
    if !viscosity.is_finite() {
        return Err(format!(
            "stokes flow element {index} viscosity must be finite"
        ));
    }
    if viscosity <= 0.0 {
        return Err(format!(
            "stokes flow element {index} has non-positive viscosity"
        ));
    }
    if !density.is_finite() {
        return Err(format!(
            "stokes flow element {index} density must be finite"
        ));
    }
    if density < 0.0 {
        return Err(format!("stokes flow element {index} has negative density"));
    }
    if !thickness.is_finite() {
        return Err(format!(
            "stokes flow element {index} thickness must be finite"
        ));
    }
    if thickness <= 0.0 {
        return Err(format!(
            "stokes flow element {index} has non-positive thickness"
        ));
    }
    Ok(())
}

fn validate_triangle_request(
    request: &SolveStokesFlowPlaneTriangle2dRequest,
) -> Result<(), String> {
    if request.nodes.len() < 3 {
        return Err("stokes flow triangle model requires at least three nodes".into());
    }
    if request.elements.is_empty() {
        return Err("stokes flow triangle model requires at least one element".into());
    }
    validate_nodes(&request.nodes)?;
    for (index, element) in request.elements.iter().enumerate() {
        validate_node_index(request.nodes.len(), element.node_i, index, "node_i")?;
        validate_node_index(request.nodes.len(), element.node_j, index, "node_j")?;
        validate_node_index(request.nodes.len(), element.node_k, index, "node_k")?;
        validate_stokes_material(index, element.viscosity, element.density, element.thickness)?;
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

fn average_triangle_viscosity_for_node(
    request: &SolveStokesFlowPlaneTriangle2dRequest,
    node_index: usize,
) -> f64 {
    let mut total = 0.0;
    let mut count = 0.0;
    for element in &request.elements {
        if [element.node_i, element.node_j, element.node_k].contains(&node_index) {
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
    let engineering_shear_rate = velocity_gradient_x + velocity_gradient_y;
    let shear_rate = (2.0 * du_dx * du_dx
        + 2.0 * dv_dy * dv_dy
        + engineering_shear_rate * engineering_shear_rate)
        .sqrt();
    let max_viscous_shear_stress = element.viscosity
        * (2.0 * du_dx.abs())
            .max(2.0 * dv_dy.abs())
            .max(engineering_shear_rate.abs());
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
        shear_rate,
        max_viscous_shear_stress,
        divergence_error: (du_dx + dv_dy).abs(),
        reynolds_number,
        viscous_dissipation,
    })
}

fn triangle_element_result(
    index: usize,
    element: &StokesFlowPlaneTriangleElementInput,
    nodes: &[StokesFlowPlaneNodeResult],
) -> Result<StokesFlowPlaneTriangleElementResult, String> {
    let n = [
        &nodes[element.node_i],
        &nodes[element.node_j],
        &nodes[element.node_k],
    ];
    let area =
        triangle_area(n).ok_or_else(|| format!("stokes flow element {index} has zero area"))?;
    let average_velocity_x = n.iter().map(|node| node.velocity_x).sum::<f64>() / 3.0;
    let average_velocity_y = n.iter().map(|node| node.velocity_y).sum::<f64>() / 3.0;
    let average_velocity_magnitude = magnitude(average_velocity_x, average_velocity_y);
    let average_pressure = n.iter().map(|node| node.pressure).sum::<f64>() / 3.0;
    let (du_dx, du_dy) = triangle_gradient(n, |node| node.velocity_x, area);
    let (dv_dx, dv_dy) = triangle_gradient(n, |node| node.velocity_y, area);
    let characteristic_length = area.sqrt();
    let reynolds_number =
        element.density * average_velocity_magnitude * characteristic_length / element.viscosity;
    let engineering_shear_rate = dv_dx + du_dy;
    let shear_rate = (2.0 * du_dx * du_dx
        + 2.0 * dv_dy * dv_dy
        + engineering_shear_rate * engineering_shear_rate)
        .sqrt();
    let max_viscous_shear_stress = element.viscosity
        * (2.0 * du_dx.abs())
            .max(2.0 * dv_dy.abs())
            .max(engineering_shear_rate.abs());
    let viscous_dissipation = element.viscosity
        * (du_dx * du_dx + dv_dy * dv_dy + dv_dx * dv_dx + du_dy * du_dy)
        * area
        * element.thickness;
    Ok(StokesFlowPlaneTriangleElementResult {
        index,
        id: element.id.clone(),
        node_i: element.node_i,
        node_j: element.node_j,
        node_k: element.node_k,
        area,
        average_velocity_x,
        average_velocity_y,
        average_velocity_magnitude,
        average_pressure,
        velocity_gradient_x: dv_dx,
        velocity_gradient_y: du_dy,
        shear_rate,
        max_viscous_shear_stress,
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

fn triangle_area(nodes: [&StokesFlowPlaneNodeResult; 3]) -> Option<f64> {
    let signed = (nodes[1].x - nodes[0].x) * (nodes[2].y - nodes[0].y)
        - (nodes[2].x - nodes[0].x) * (nodes[1].y - nodes[0].y);
    let area = 0.5 * signed.abs();
    (area > 1.0e-12).then_some(area)
}

fn triangle_gradient(
    nodes: [&StokesFlowPlaneNodeResult; 3],
    value: fn(&StokesFlowPlaneNodeResult) -> f64,
    area: f64,
) -> (f64, f64) {
    let two_area = 2.0 * area;
    let b = [
        nodes[1].y - nodes[2].y,
        nodes[2].y - nodes[0].y,
        nodes[0].y - nodes[1].y,
    ];
    let c = [
        nodes[2].x - nodes[1].x,
        nodes[0].x - nodes[2].x,
        nodes[1].x - nodes[0].x,
    ];
    let values = [value(nodes[0]), value(nodes[1]), value(nodes[2])];
    let gradient_x = values
        .iter()
        .zip(b)
        .map(|(field_value, coeff)| field_value * coeff)
        .sum::<f64>()
        / two_area;
    let gradient_y = values
        .iter()
        .zip(c)
        .map(|(field_value, coeff)| field_value * coeff)
        .sum::<f64>()
        / two_area;
    (gradient_x, gradient_y)
}

fn element_summary_fields(element: &StokesFlowPlaneQuadElementResult) -> ElementSummaryFields {
    ElementSummaryFields {
        divergence_error: element.divergence_error,
        reynolds_number: element.reynolds_number,
        shear_rate: element.shear_rate,
        max_viscous_shear_stress: element.max_viscous_shear_stress,
    }
}

fn triangle_element_summary_fields(
    element: &StokesFlowPlaneTriangleElementResult,
) -> ElementSummaryFields {
    ElementSummaryFields {
        divergence_error: element.divergence_error,
        reynolds_number: element.reynolds_number,
        shear_rate: element.shear_rate,
        max_viscous_shear_stress: element.max_viscous_shear_stress,
    }
}

#[derive(Clone, Copy)]
struct ElementSummaryFields {
    divergence_error: f64,
    reynolds_number: f64,
    shear_rate: f64,
    max_viscous_shear_stress: f64,
}

struct FlowSummary {
    max_velocity: f64,
    max_pressure: f64,
    pressure_drop: f64,
    max_divergence_error: f64,
    max_reynolds_number: f64,
    max_shear_rate: f64,
    max_viscous_shear_stress: f64,
}

fn magnitude(x: f64, y: f64) -> f64 {
    (x * x + y * y).sqrt()
}
