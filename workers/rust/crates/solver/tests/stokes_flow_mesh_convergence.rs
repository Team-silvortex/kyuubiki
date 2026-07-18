use kyuubiki_protocol::{
    SolveStokesFlowPlaneQuad2dRequest, SolveStokesFlowPlaneTriangle2dRequest,
    StokesFlowPlaneNodeInput, StokesFlowPlaneQuadElementInput, StokesFlowPlaneTriangleElementInput,
};
use kyuubiki_solver::{solve_stokes_flow_plane_quad_2d, solve_stokes_flow_plane_triangle_2d};

const TOL: f64 = 1.0e-10;

#[test]
fn stokes_flow_quad_2d_preserves_linear_screening_field_under_refinement() {
    let baseline = solve_stokes_flow_plane_quad_2d(&quad_mesh(1))
        .expect("baseline quad Stokes refinement fixture should solve");
    let baseline_dissipation = total_quad_dissipation(&baseline);

    for subdivisions in [1_usize, 2, 4, 8] {
        let result = solve_stokes_flow_plane_quad_2d(&quad_mesh(subdivisions))
            .expect("refined quad Stokes refinement fixture should solve");

        assert_quad_summary(&result);
        assert_eq!(result.elements.len(), subdivisions * subdivisions);
        assert_close(
            result.elements.iter().map(|element| element.area).sum(),
            1.0,
        );
        assert_close(result.max_velocity, baseline.max_velocity);
        assert_close(result.pressure_drop, baseline.pressure_drop);
        assert_close(result.max_divergence_error, baseline.max_divergence_error);
        assert_close(result.max_shear_rate, baseline.max_shear_rate);
        assert_close(
            result.max_viscous_shear_stress,
            baseline.max_viscous_shear_stress,
        );
        assert_close(total_quad_dissipation(&result), baseline_dissipation);
        assert_close(result.max_divergence_error, 0.0);
        assert_close(result.max_shear_rate, 1.0);
    }
}

#[test]
fn stokes_flow_triangle_2d_preserves_linear_screening_field_under_refinement() {
    let baseline = solve_stokes_flow_plane_triangle_2d(&triangle_mesh(1))
        .expect("baseline triangle Stokes refinement fixture should solve");
    let baseline_dissipation = total_triangle_dissipation(&baseline);

    for subdivisions in [1_usize, 2, 4, 8] {
        let result = solve_stokes_flow_plane_triangle_2d(&triangle_mesh(subdivisions))
            .expect("refined triangle Stokes refinement fixture should solve");

        assert_triangle_summary(&result);
        assert_eq!(result.elements.len(), subdivisions * subdivisions * 2);
        assert_close(
            result.elements.iter().map(|element| element.area).sum(),
            1.0,
        );
        assert_close(result.max_velocity, baseline.max_velocity);
        assert_close(result.pressure_drop, baseline.pressure_drop);
        assert_close(result.max_divergence_error, baseline.max_divergence_error);
        assert_close(result.max_shear_rate, baseline.max_shear_rate);
        assert_close(
            result.max_viscous_shear_stress,
            baseline.max_viscous_shear_stress,
        );
        assert_close(total_triangle_dissipation(&result), baseline_dissipation);
        assert_close(result.max_divergence_error, 0.0);
        assert_close(result.max_shear_rate, 1.0);
    }
}

#[test]
fn stokes_flow_linear_field_scales_viscosity_and_density_diagnostics() {
    let baseline = solve_stokes_flow_plane_quad_2d(&quad_mesh_with_material(1, 2.0, 1.0, 1.0))
        .expect("baseline Stokes material scaling fixture should solve");
    let viscous = solve_stokes_flow_plane_quad_2d(&quad_mesh_with_material(1, 2.4, 1.0, 1.0))
        .expect("viscosity-perturbed Stokes fixture should solve");
    let dense = solve_stokes_flow_plane_quad_2d(&quad_mesh_with_material(1, 2.0, 1.25, 1.0))
        .expect("density-perturbed Stokes fixture should solve");
    assert_quad_summary(&baseline);
    assert_quad_summary(&viscous);
    assert_quad_summary(&dense);

    assert_close(viscous.max_velocity, baseline.max_velocity);
    assert_close(viscous.max_shear_rate, baseline.max_shear_rate);
    assert_close(
        viscous.max_viscous_shear_stress / baseline.max_viscous_shear_stress,
        1.2,
    );
    assert_close(
        viscous.elements[0].viscous_dissipation / baseline.elements[0].viscous_dissipation,
        1.2,
    );
    assert_close(
        baseline.max_reynolds_number / viscous.max_reynolds_number,
        1.2,
    );

    assert_close(dense.max_velocity, baseline.max_velocity);
    assert_close(dense.max_shear_rate, baseline.max_shear_rate);
    assert_close(
        dense.max_reynolds_number / baseline.max_reynolds_number,
        1.25,
    );
    assert_close(
        dense.max_viscous_shear_stress,
        baseline.max_viscous_shear_stress,
    );

    let geometry_scale = 1.5;
    let larger =
        solve_stokes_flow_plane_quad_2d(&quad_mesh_scaled(1, 2.0, 1.0, 1.0, geometry_scale))
            .expect("geometry-scaled Stokes quad fixture should solve");
    assert_quad_summary(&larger);
    assert_close(larger.max_velocity, baseline.max_velocity);
    assert_close(
        larger.max_shear_rate / baseline.max_shear_rate,
        1.0 / geometry_scale,
    );
    assert_close(
        larger.max_viscous_shear_stress / baseline.max_viscous_shear_stress,
        1.0 / geometry_scale,
    );
    assert_close(
        larger.elements[0].area / baseline.elements[0].area,
        geometry_scale * geometry_scale,
    );
    assert_close(
        larger.elements[0].viscous_dissipation,
        baseline.elements[0].viscous_dissipation,
    );
}

#[test]
fn stokes_flow_triangle_linear_field_scales_material_diagnostics() {
    let baseline =
        solve_stokes_flow_plane_triangle_2d(&triangle_mesh_with_material(1, 2.0, 1.0, 1.0))
            .expect("baseline triangle Stokes material scaling fixture should solve");
    let viscous =
        solve_stokes_flow_plane_triangle_2d(&triangle_mesh_with_material(1, 2.4, 1.0, 1.0))
            .expect("viscosity-perturbed triangle Stokes fixture should solve");
    let dense =
        solve_stokes_flow_plane_triangle_2d(&triangle_mesh_with_material(1, 2.0, 1.25, 1.0))
            .expect("density-perturbed triangle Stokes fixture should solve");
    let thick = solve_stokes_flow_plane_triangle_2d(&triangle_mesh_with_material(1, 2.0, 1.0, 1.5))
        .expect("thickness-perturbed triangle Stokes fixture should solve");
    assert_triangle_summary(&baseline);
    assert_triangle_summary(&viscous);
    assert_triangle_summary(&dense);
    assert_triangle_summary(&thick);

    assert_close(viscous.max_velocity, baseline.max_velocity);
    assert_close(viscous.max_shear_rate, baseline.max_shear_rate);
    assert_close(
        viscous.max_viscous_shear_stress / baseline.max_viscous_shear_stress,
        1.2,
    );
    assert_close(
        viscous.elements[0].viscous_dissipation / baseline.elements[0].viscous_dissipation,
        1.2,
    );
    assert_close(
        baseline.max_reynolds_number / viscous.max_reynolds_number,
        1.2,
    );

    assert_close(dense.max_velocity, baseline.max_velocity);
    assert_close(dense.max_shear_rate, baseline.max_shear_rate);
    assert_close(
        dense.max_reynolds_number / baseline.max_reynolds_number,
        1.25,
    );
    assert_close(
        dense.max_viscous_shear_stress,
        baseline.max_viscous_shear_stress,
    );

    assert_close(thick.max_velocity, baseline.max_velocity);
    assert_close(thick.max_shear_rate, baseline.max_shear_rate);
    assert_close(
        thick.max_viscous_shear_stress,
        baseline.max_viscous_shear_stress,
    );
    assert_close(
        thick.elements[0].viscous_dissipation / baseline.elements[0].viscous_dissipation,
        1.5,
    );

    let geometry_scale = 1.4;
    let larger = solve_stokes_flow_plane_triangle_2d(&triangle_mesh_scaled(
        1,
        2.0,
        1.0,
        1.0,
        geometry_scale,
    ))
    .expect("geometry-scaled triangle Stokes fixture should solve");
    assert_triangle_summary(&larger);
    assert_close(larger.max_velocity, baseline.max_velocity);
    assert_close(
        larger.max_shear_rate / baseline.max_shear_rate,
        1.0 / geometry_scale,
    );
    assert_close(
        larger.max_viscous_shear_stress / baseline.max_viscous_shear_stress,
        1.0 / geometry_scale,
    );
    assert_close(
        larger.elements[0].area / baseline.elements[0].area,
        geometry_scale * geometry_scale,
    );
    assert_close(
        larger.elements[0].viscous_dissipation,
        baseline.elements[0].viscous_dissipation,
    );
}

fn quad_mesh(subdivisions: usize) -> SolveStokesFlowPlaneQuad2dRequest {
    quad_mesh_with_material(subdivisions, 2.0, 1.0, 1.0)
}

fn quad_mesh_with_material(
    subdivisions: usize,
    viscosity: f64,
    density: f64,
    thickness: f64,
) -> SolveStokesFlowPlaneQuad2dRequest {
    quad_mesh_scaled(subdivisions, viscosity, density, thickness, 1.0)
}

fn quad_mesh_scaled(
    subdivisions: usize,
    viscosity: f64,
    density: f64,
    thickness: f64,
    geometry_scale: f64,
) -> SolveStokesFlowPlaneQuad2dRequest {
    let mut nodes = Vec::new();
    for y_index in 0..=subdivisions {
        for x_index in 0..=subdivisions {
            nodes.push(linear_stokes_node_scaled(
                x_index,
                y_index,
                subdivisions,
                geometry_scale,
            ));
        }
    }

    let mut elements = Vec::new();
    for y_index in 0..subdivisions {
        for x_index in 0..subdivisions {
            let node_i = grid_index(x_index, y_index, subdivisions);
            let node_j = grid_index(x_index + 1, y_index, subdivisions);
            let node_k = grid_index(x_index + 1, y_index + 1, subdivisions);
            let node_l = grid_index(x_index, y_index + 1, subdivisions);
            elements.push(StokesFlowPlaneQuadElementInput {
                id: format!("quad-{x_index}-{y_index}"),
                node_i,
                node_j,
                node_k,
                node_l,
                thickness,
                viscosity,
                density,
            });
        }
    }

    SolveStokesFlowPlaneQuad2dRequest { nodes, elements }
}

fn triangle_mesh(subdivisions: usize) -> SolveStokesFlowPlaneTriangle2dRequest {
    triangle_mesh_with_material(subdivisions, 2.0, 1.0, 1.0)
}

fn triangle_mesh_with_material(
    subdivisions: usize,
    viscosity: f64,
    density: f64,
    thickness: f64,
) -> SolveStokesFlowPlaneTriangle2dRequest {
    triangle_mesh_scaled(subdivisions, viscosity, density, thickness, 1.0)
}

fn triangle_mesh_scaled(
    subdivisions: usize,
    viscosity: f64,
    density: f64,
    thickness: f64,
    geometry_scale: f64,
) -> SolveStokesFlowPlaneTriangle2dRequest {
    let mut nodes = Vec::new();
    for y_index in 0..=subdivisions {
        for x_index in 0..=subdivisions {
            nodes.push(linear_stokes_node_scaled(
                x_index,
                y_index,
                subdivisions,
                geometry_scale,
            ));
        }
    }

    let mut elements = Vec::new();
    for y_index in 0..subdivisions {
        for x_index in 0..subdivisions {
            let lower_left = grid_index(x_index, y_index, subdivisions);
            let lower_right = grid_index(x_index + 1, y_index, subdivisions);
            let upper_right = grid_index(x_index + 1, y_index + 1, subdivisions);
            let upper_left = grid_index(x_index, y_index + 1, subdivisions);
            elements.push(triangle(
                format!("tri-a-{x_index}-{y_index}"),
                lower_left,
                lower_right,
                upper_right,
                viscosity,
                density,
                thickness,
            ));
            elements.push(triangle(
                format!("tri-b-{x_index}-{y_index}"),
                lower_left,
                upper_right,
                upper_left,
                viscosity,
                density,
                thickness,
            ));
        }
    }

    SolveStokesFlowPlaneTriangle2dRequest { nodes, elements }
}

fn linear_stokes_node_scaled(
    x_index: usize,
    y_index: usize,
    subdivisions: usize,
    geometry_scale: f64,
) -> StokesFlowPlaneNodeInput {
    let x = x_index as f64 / subdivisions as f64 * geometry_scale;
    let y = y_index as f64 / subdivisions as f64 * geometry_scale;
    StokesFlowPlaneNodeInput {
        id: format!("n-{x_index}-{y_index}"),
        x,
        y,
        fix_velocity_x: true,
        velocity_x: y / geometry_scale,
        fix_velocity_y: true,
        velocity_y: 0.0,
        fix_pressure: true,
        pressure: 0.0,
        body_force_x: 0.0,
        body_force_y: 0.0,
    }
}

fn triangle(
    id: String,
    node_i: usize,
    node_j: usize,
    node_k: usize,
    viscosity: f64,
    density: f64,
    thickness: f64,
) -> StokesFlowPlaneTriangleElementInput {
    StokesFlowPlaneTriangleElementInput {
        id,
        node_i,
        node_j,
        node_k,
        thickness,
        viscosity,
        density,
    }
}

fn grid_index(x_index: usize, y_index: usize, subdivisions: usize) -> usize {
    y_index * (subdivisions + 1) + x_index
}

fn total_quad_dissipation(result: &kyuubiki_protocol::SolveStokesFlowPlaneQuad2dResult) -> f64 {
    result
        .elements
        .iter()
        .map(|element| element.viscous_dissipation)
        .sum()
}

fn total_triangle_dissipation(
    result: &kyuubiki_protocol::SolveStokesFlowPlaneTriangle2dResult,
) -> f64 {
    result
        .elements
        .iter()
        .map(|element| element.viscous_dissipation)
        .sum()
}

fn assert_quad_summary(result: &kyuubiki_protocol::SolveStokesFlowPlaneQuad2dResult) {
    assert_flow_summary(
        &result.input.nodes,
        &result.nodes,
        result.max_velocity,
        result.max_pressure,
        result.pressure_drop,
        result.max_divergence_error,
        result.max_reynolds_number,
        result.max_shear_rate,
        result.max_viscous_shear_stress,
        result.elements.iter().map(|element| FlowElementFields {
            area: element.area,
            average_velocity_x: element.average_velocity_x,
            average_velocity_y: element.average_velocity_y,
            average_velocity_magnitude: element.average_velocity_magnitude,
            average_pressure: element.average_pressure,
            velocity_gradient_x: element.velocity_gradient_x,
            velocity_gradient_y: element.velocity_gradient_y,
            shear_rate: element.shear_rate,
            max_viscous_shear_stress: element.max_viscous_shear_stress,
            divergence_error: element.divergence_error,
            reynolds_number: element.reynolds_number,
            viscous_dissipation: element.viscous_dissipation,
            viscosity: result.input.elements[element.index].viscosity,
            density: result.input.elements[element.index].density,
            thickness: result.input.elements[element.index].thickness,
            node_indices: vec![
                element.node_i,
                element.node_j,
                element.node_k,
                element.node_l,
            ],
        }),
    );
}

fn assert_triangle_summary(result: &kyuubiki_protocol::SolveStokesFlowPlaneTriangle2dResult) {
    assert_flow_summary(
        &result.input.nodes,
        &result.nodes,
        result.max_velocity,
        result.max_pressure,
        result.pressure_drop,
        result.max_divergence_error,
        result.max_reynolds_number,
        result.max_shear_rate,
        result.max_viscous_shear_stress,
        result.elements.iter().map(|element| FlowElementFields {
            area: element.area,
            average_velocity_x: element.average_velocity_x,
            average_velocity_y: element.average_velocity_y,
            average_velocity_magnitude: element.average_velocity_magnitude,
            average_pressure: element.average_pressure,
            velocity_gradient_x: element.velocity_gradient_x,
            velocity_gradient_y: element.velocity_gradient_y,
            shear_rate: element.shear_rate,
            max_viscous_shear_stress: element.max_viscous_shear_stress,
            divergence_error: element.divergence_error,
            reynolds_number: element.reynolds_number,
            viscous_dissipation: element.viscous_dissipation,
            viscosity: result.input.elements[element.index].viscosity,
            density: result.input.elements[element.index].density,
            thickness: result.input.elements[element.index].thickness,
            node_indices: vec![element.node_i, element.node_j, element.node_k],
        }),
    );
}

struct FlowElementFields {
    area: f64,
    average_velocity_x: f64,
    average_velocity_y: f64,
    average_velocity_magnitude: f64,
    average_pressure: f64,
    velocity_gradient_x: f64,
    velocity_gradient_y: f64,
    shear_rate: f64,
    max_viscous_shear_stress: f64,
    divergence_error: f64,
    reynolds_number: f64,
    viscous_dissipation: f64,
    viscosity: f64,
    density: f64,
    thickness: f64,
    node_indices: Vec<usize>,
}

#[allow(clippy::too_many_arguments)]
fn assert_flow_summary(
    inputs: &[kyuubiki_protocol::StokesFlowPlaneNodeInput],
    nodes: &[kyuubiki_protocol::StokesFlowPlaneNodeResult],
    max_velocity: f64,
    max_pressure: f64,
    pressure_drop: f64,
    max_divergence_error: f64,
    max_reynolds_number: f64,
    max_shear_rate: f64,
    max_viscous_shear_stress: f64,
    elements: impl Iterator<Item = FlowElementFields>,
) {
    assert_close(
        max_velocity,
        nodes
            .iter()
            .map(|node| {
                let input = &inputs[node.index];
                assert_eq!(node.id, input.id);
                assert_close(node.x, input.x);
                assert_close(node.y, input.y);
                assert_close(node.body_force_x, input.body_force_x);
                assert_close(node.body_force_y, input.body_force_y);
                assert_close(
                    node.velocity_magnitude,
                    (node.velocity_x * node.velocity_x + node.velocity_y * node.velocity_y).sqrt(),
                );
                node.velocity_magnitude
            })
            .fold(0.0_f64, f64::max),
    );
    assert_close(
        max_pressure,
        nodes
            .iter()
            .map(|node| node.pressure.abs())
            .fold(0.0_f64, f64::max),
    );
    let min_pressure = nodes
        .iter()
        .map(|node| node.pressure)
        .fold(f64::INFINITY, f64::min);
    let max_signed_pressure = nodes
        .iter()
        .map(|node| node.pressure)
        .fold(f64::NEG_INFINITY, f64::max);
    assert_close(pressure_drop, max_signed_pressure - min_pressure);

    let elements = elements.collect::<Vec<_>>();
    assert_close(
        max_divergence_error,
        elements
            .iter()
            .map(|element| element.divergence_error.abs())
            .fold(0.0_f64, f64::max),
    );
    assert_close(
        max_reynolds_number,
        elements
            .iter()
            .map(|element| element.reynolds_number)
            .fold(0.0_f64, f64::max),
    );
    assert_close(
        max_shear_rate,
        elements
            .iter()
            .map(|element| element.shear_rate)
            .fold(0.0_f64, f64::max),
    );
    assert_close(
        max_viscous_shear_stress,
        elements
            .iter()
            .map(|element| element.max_viscous_shear_stress)
            .fold(0.0_f64, f64::max),
    );

    for element in elements {
        let node_count = element.node_indices.len() as f64;
        let average_velocity_x = element
            .node_indices
            .iter()
            .map(|&index| nodes[index].velocity_x)
            .sum::<f64>()
            / node_count;
        let average_velocity_y = element
            .node_indices
            .iter()
            .map(|&index| nodes[index].velocity_y)
            .sum::<f64>()
            / node_count;
        let average_pressure = element
            .node_indices
            .iter()
            .map(|&index| nodes[index].pressure)
            .sum::<f64>()
            / node_count;

        assert_close(element.average_velocity_x, average_velocity_x);
        assert_close(element.average_velocity_y, average_velocity_y);
        assert_close(
            element.average_velocity_magnitude,
            (average_velocity_x * average_velocity_x + average_velocity_y * average_velocity_y)
                .sqrt(),
        );
        assert_close(element.average_pressure, average_pressure);
        let kinematics = flow_kinematics(nodes, &element.node_indices);
        assert_close(element.area, kinematics.area);
        assert_close(element.velocity_gradient_x, kinematics.velocity_gradient_x);
        assert_close(element.velocity_gradient_y, kinematics.velocity_gradient_y);
        assert_close(
            element.divergence_error,
            (kinematics.du_dx + kinematics.dv_dy).abs(),
        );
        let engineering_shear_rate =
            kinematics.velocity_gradient_x + kinematics.velocity_gradient_y;
        assert_close(
            element.shear_rate,
            (2.0 * kinematics.du_dx * kinematics.du_dx
                + 2.0 * kinematics.dv_dy * kinematics.dv_dy
                + engineering_shear_rate * engineering_shear_rate)
                .sqrt(),
        );
        assert_close(
            element.max_viscous_shear_stress,
            element.viscosity
                * (2.0 * kinematics.du_dx.abs())
                    .max(2.0 * kinematics.dv_dy.abs())
                    .max(engineering_shear_rate.abs()),
        );
        assert_close(
            element.reynolds_number,
            element.density * element.average_velocity_magnitude * element.area.sqrt()
                / element.viscosity,
        );
        assert_close(
            element.viscous_dissipation,
            element.viscosity
                * (kinematics.du_dx * kinematics.du_dx
                    + kinematics.dv_dy * kinematics.dv_dy
                    + kinematics.velocity_gradient_x * kinematics.velocity_gradient_x
                    + kinematics.velocity_gradient_y * kinematics.velocity_gradient_y)
                * element.area
                * element.thickness,
        );
        assert!(element.area > 0.0);
        assert!(element.shear_rate >= 0.0);
        assert!(element.max_viscous_shear_stress >= 0.0);
        assert!(element.divergence_error >= 0.0);
        assert!(element.reynolds_number >= 0.0);
        assert!(element.viscous_dissipation >= 0.0);
    }
}

struct FlowKinematics {
    area: f64,
    du_dx: f64,
    dv_dy: f64,
    velocity_gradient_x: f64,
    velocity_gradient_y: f64,
}

fn flow_kinematics(
    nodes: &[kyuubiki_protocol::StokesFlowPlaneNodeResult],
    node_indices: &[usize],
) -> FlowKinematics {
    match node_indices {
        [a, b, c] => triangle_flow_kinematics([&nodes[*a], &nodes[*b], &nodes[*c]]),
        [a, b, c, d] => quad_flow_kinematics([&nodes[*a], &nodes[*b], &nodes[*c], &nodes[*d]]),
        _ => panic!("unsupported Stokes element arity"),
    }
}

fn triangle_flow_kinematics(
    nodes: [&kyuubiki_protocol::StokesFlowPlaneNodeResult; 3],
) -> FlowKinematics {
    let area = polygon_area(&nodes);
    let (du_dx, velocity_gradient_y) = triangle_gradient(nodes, |node| node.velocity_x, area);
    let (velocity_gradient_x, dv_dy) = triangle_gradient(nodes, |node| node.velocity_y, area);
    FlowKinematics {
        area,
        du_dx,
        dv_dy,
        velocity_gradient_x,
        velocity_gradient_y,
    }
}

fn quad_flow_kinematics(
    nodes: [&kyuubiki_protocol::StokesFlowPlaneNodeResult; 4],
) -> FlowKinematics {
    let area = polygon_area(&nodes);
    let width = ((nodes[1].x - nodes[0].x).abs() + (nodes[2].x - nodes[3].x).abs()) / 2.0;
    let height = ((nodes[3].y - nodes[0].y).abs() + (nodes[2].y - nodes[1].y).abs()) / 2.0;
    let du_dx = ((nodes[1].velocity_x + nodes[2].velocity_x)
        - (nodes[0].velocity_x + nodes[3].velocity_x))
        / (2.0 * width);
    let dv_dy = ((nodes[2].velocity_y + nodes[3].velocity_y)
        - (nodes[0].velocity_y + nodes[1].velocity_y))
        / (2.0 * height);
    let velocity_gradient_x = ((nodes[1].velocity_y + nodes[2].velocity_y)
        - (nodes[0].velocity_y + nodes[3].velocity_y))
        / (2.0 * width);
    let velocity_gradient_y = ((nodes[2].velocity_x + nodes[3].velocity_x)
        - (nodes[0].velocity_x + nodes[1].velocity_x))
        / (2.0 * height);
    FlowKinematics {
        area,
        du_dx,
        dv_dy,
        velocity_gradient_x,
        velocity_gradient_y,
    }
}

fn triangle_gradient(
    nodes: [&kyuubiki_protocol::StokesFlowPlaneNodeResult; 3],
    value: impl Fn(&kyuubiki_protocol::StokesFlowPlaneNodeResult) -> f64,
    area: f64,
) -> (f64, f64) {
    let two_area = 2.0 * area;
    let gradient_x = (value(nodes[0]) * (nodes[1].y - nodes[2].y)
        + value(nodes[1]) * (nodes[2].y - nodes[0].y)
        + value(nodes[2]) * (nodes[0].y - nodes[1].y))
        / two_area;
    let gradient_y = (value(nodes[0]) * (nodes[2].x - nodes[1].x)
        + value(nodes[1]) * (nodes[0].x - nodes[2].x)
        + value(nodes[2]) * (nodes[1].x - nodes[0].x))
        / two_area;
    (gradient_x, gradient_y)
}

fn polygon_area(nodes: &[&kyuubiki_protocol::StokesFlowPlaneNodeResult]) -> f64 {
    let mut twice_area = 0.0_f64;
    for index in 0..nodes.len() {
        let next = (index + 1) % nodes.len();
        twice_area += nodes[index].x * nodes[next].y - nodes[next].x * nodes[index].y;
    }
    0.5 * twice_area.abs()
}

fn assert_close(actual: f64, expected: f64) {
    let scale = expected.abs().max(1.0);
    assert!(
        (actual - expected).abs() <= TOL * scale,
        "expected {actual} to be close to {expected}",
    );
}
