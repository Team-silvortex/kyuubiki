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

    for subdivisions in [1_usize, 2, 4] {
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

    for subdivisions in [1_usize, 2, 4] {
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
            shear_rate: element.shear_rate,
            max_viscous_shear_stress: element.max_viscous_shear_stress,
            divergence_error: element.divergence_error,
            reynolds_number: element.reynolds_number,
            viscous_dissipation: element.viscous_dissipation,
            viscosity: result.input.elements[element.index].viscosity,
            density: result.input.elements[element.index].density,
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
            shear_rate: element.shear_rate,
            max_viscous_shear_stress: element.max_viscous_shear_stress,
            divergence_error: element.divergence_error,
            reynolds_number: element.reynolds_number,
            viscous_dissipation: element.viscous_dissipation,
            viscosity: result.input.elements[element.index].viscosity,
            density: result.input.elements[element.index].density,
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
    shear_rate: f64,
    max_viscous_shear_stress: f64,
    divergence_error: f64,
    reynolds_number: f64,
    viscous_dissipation: f64,
    viscosity: f64,
    density: f64,
    node_indices: Vec<usize>,
}

#[allow(clippy::too_many_arguments)]
fn assert_flow_summary(
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
        assert_close(
            element.reynolds_number,
            element.density * element.average_velocity_magnitude * element.area.sqrt()
                / element.viscosity,
        );
        assert!(element.area > 0.0);
        assert!(element.shear_rate >= 0.0);
        assert!(element.max_viscous_shear_stress >= 0.0);
        assert!(element.divergence_error >= 0.0);
        assert!(element.reynolds_number >= 0.0);
        assert!(element.viscous_dissipation >= 0.0);
    }
}

fn assert_close(actual: f64, expected: f64) {
    let scale = expected.abs().max(1.0);
    assert!(
        (actual - expected).abs() <= TOL * scale,
        "expected {actual} to be close to {expected}",
    );
}
