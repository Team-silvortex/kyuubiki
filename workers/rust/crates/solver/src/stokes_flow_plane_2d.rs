use crate::stokes_flow_plane_2d_results::{
    magnitude, quad_element_result, summarize_quad_flow, summarize_triangle_flow,
    triangle_element_result,
};
use crate::stokes_flow_plane_2d_validation::{validate_quad_request, validate_triangle_request};
use kyuubiki_protocol::{
    SolveStokesFlowPlaneQuad2dRequest, SolveStokesFlowPlaneQuad2dResult,
    SolveStokesFlowPlaneTriangle2dRequest, SolveStokesFlowPlaneTriangle2dResult,
    StokesFlowPlaneNodeResult,
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
        .map(|(index, element)| quad_element_result(index, element, &nodes))
        .collect::<Result<Vec<_>, String>>()?;

    let summary = summarize_quad_flow(&nodes, &elements);
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
    let summary = summarize_triangle_flow(&nodes, &elements);
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
