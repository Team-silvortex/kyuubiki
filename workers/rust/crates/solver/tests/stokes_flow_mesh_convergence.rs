use kyuubiki_protocol::{
    SolveStokesFlowPlaneQuad2dRequest, SolveStokesFlowPlaneTriangle2dRequest,
    StokesFlowPlaneNodeInput, StokesFlowPlaneQuadElementInput, StokesFlowPlaneTriangleElementInput,
};
use kyuubiki_solver::{solve_stokes_flow_plane_quad_2d, solve_stokes_flow_plane_triangle_2d};

const TOL: f64 = 1.0e-10;

#[test]
fn stokes_flow_quad_2d_preserves_linear_screening_field_under_refinement() {
    let coarse = solve_stokes_flow_plane_quad_2d(&quad_mesh(1))
        .expect("coarse quad Stokes refinement fixture should solve");
    let refined = solve_stokes_flow_plane_quad_2d(&quad_mesh(2))
        .expect("refined quad Stokes refinement fixture should solve");

    assert_close(coarse.elements.iter().map(|element| element.area).sum(), 1.0);
    assert_close(refined.elements.iter().map(|element| element.area).sum(), 1.0);
    assert_close(coarse.max_velocity, refined.max_velocity);
    assert_close(coarse.pressure_drop, refined.pressure_drop);
    assert_close(coarse.max_divergence_error, refined.max_divergence_error);
    assert_close(coarse.max_shear_rate, refined.max_shear_rate);
    assert_close(
        coarse.max_viscous_shear_stress,
        refined.max_viscous_shear_stress,
    );
    assert_close(refined.max_divergence_error, 0.0);
    assert_close(refined.max_shear_rate, 1.0);
}

#[test]
fn stokes_flow_triangle_2d_preserves_linear_screening_field_under_refinement() {
    let coarse = solve_stokes_flow_plane_triangle_2d(&triangle_mesh(1))
        .expect("coarse triangle Stokes refinement fixture should solve");
    let refined = solve_stokes_flow_plane_triangle_2d(&triangle_mesh(2))
        .expect("refined triangle Stokes refinement fixture should solve");

    assert_close(coarse.elements.iter().map(|element| element.area).sum(), 1.0);
    assert_close(refined.elements.iter().map(|element| element.area).sum(), 1.0);
    assert_close(coarse.max_velocity, refined.max_velocity);
    assert_close(coarse.pressure_drop, refined.pressure_drop);
    assert_close(coarse.max_divergence_error, refined.max_divergence_error);
    assert_close(coarse.max_shear_rate, refined.max_shear_rate);
    assert_close(
        coarse.max_viscous_shear_stress,
        refined.max_viscous_shear_stress,
    );
    assert_close(refined.max_divergence_error, 0.0);
    assert_close(refined.max_shear_rate, 1.0);
}

fn quad_mesh(subdivisions: usize) -> SolveStokesFlowPlaneQuad2dRequest {
    let mut nodes = Vec::new();
    for y_index in 0..=subdivisions {
        for x_index in 0..=subdivisions {
            nodes.push(linear_stokes_node(x_index, y_index, subdivisions));
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
                thickness: 1.0,
                viscosity: 2.0,
                density: 1.0,
            });
        }
    }

    SolveStokesFlowPlaneQuad2dRequest { nodes, elements }
}

fn triangle_mesh(subdivisions: usize) -> SolveStokesFlowPlaneTriangle2dRequest {
    let mut nodes = Vec::new();
    for y_index in 0..=subdivisions {
        for x_index in 0..=subdivisions {
            nodes.push(linear_stokes_node(x_index, y_index, subdivisions));
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
            ));
            elements.push(triangle(
                format!("tri-b-{x_index}-{y_index}"),
                lower_left,
                upper_right,
                upper_left,
            ));
        }
    }

    SolveStokesFlowPlaneTriangle2dRequest { nodes, elements }
}

fn linear_stokes_node(
    x_index: usize,
    y_index: usize,
    subdivisions: usize,
) -> StokesFlowPlaneNodeInput {
    let x = x_index as f64 / subdivisions as f64;
    let y = y_index as f64 / subdivisions as f64;
    StokesFlowPlaneNodeInput {
        id: format!("n-{x_index}-{y_index}"),
        x,
        y,
        fix_velocity_x: true,
        velocity_x: y,
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
) -> StokesFlowPlaneTriangleElementInput {
    StokesFlowPlaneTriangleElementInput {
        id,
        node_i,
        node_j,
        node_k,
        thickness: 1.0,
        viscosity: 2.0,
        density: 1.0,
    }
}

fn grid_index(x_index: usize, y_index: usize, subdivisions: usize) -> usize {
    y_index * (subdivisions + 1) + x_index
}

fn assert_close(actual: f64, expected: f64) {
    let scale = expected.abs().max(1.0);
    assert!(
        (actual - expected).abs() <= TOL * scale,
        "expected {actual} to be close to {expected}",
    );
}
