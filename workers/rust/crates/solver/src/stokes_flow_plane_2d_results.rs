use kyuubiki_protocol::{
    StokesFlowPlaneNodeResult, StokesFlowPlaneQuadElementInput, StokesFlowPlaneQuadElementResult,
    StokesFlowPlaneTriangleElementInput, StokesFlowPlaneTriangleElementResult,
};

pub(super) struct FlowSummary {
    pub max_velocity: f64,
    pub max_pressure: f64,
    pub pressure_drop: f64,
    pub max_divergence_error: f64,
    pub max_reynolds_number: f64,
    pub max_shear_rate: f64,
    pub max_viscous_shear_stress: f64,
}

pub(super) fn summarize_quad_flow(
    nodes: &[StokesFlowPlaneNodeResult],
    elements: &[StokesFlowPlaneQuadElementResult],
) -> FlowSummary {
    summarize_flow(nodes, elements.iter().map(quad_summary_fields))
}

pub(super) fn summarize_triangle_flow(
    nodes: &[StokesFlowPlaneNodeResult],
    elements: &[StokesFlowPlaneTriangleElementResult],
) -> FlowSummary {
    summarize_flow(nodes, elements.iter().map(triangle_summary_fields))
}

pub(super) fn quad_element_result(
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

pub(super) fn triangle_element_result(
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

pub(super) fn magnitude(x: f64, y: f64) -> f64 {
    (x * x + y * y).sqrt()
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

fn quad_summary_fields(element: &StokesFlowPlaneQuadElementResult) -> ElementSummaryFields {
    ElementSummaryFields {
        divergence_error: element.divergence_error,
        reynolds_number: element.reynolds_number,
        shear_rate: element.shear_rate,
        max_viscous_shear_stress: element.max_viscous_shear_stress,
    }
}

fn triangle_summary_fields(element: &StokesFlowPlaneTriangleElementResult) -> ElementSummaryFields {
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
