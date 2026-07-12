use kyuubiki_protocol::{
    SolveStokesFlowPlaneTriangle2dRequest, StokesFlowPlaneNodeInput,
    StokesFlowPlaneTriangleElementInput,
};
use kyuubiki_solver::solve_stokes_flow_plane_triangle_2d;

const TOL: f64 = 1.0e-10;

#[test]
fn stokes_flow_triangle_2d_uses_local_viscosity_average_for_shared_nodes() {
    let result = solve_stokes_flow_plane_triangle_2d(&SolveStokesFlowPlaneTriangle2dRequest {
        nodes: vec![
            node("n0", 0.0, 0.0, true, 0.0, true, 0.0, true, 0.0, 0.0, 0.0),
            node("n1", 1.0, 0.0, false, 0.0, true, 0.0, false, 0.0, 6.0, 0.0),
            node("n2", 1.0, 1.0, false, 0.0, true, 0.0, false, 0.0, 6.0, 0.0),
            node("n3", 0.0, 1.0, false, 0.0, true, 0.0, false, 0.0, 8.0, 8.0),
        ],
        elements: vec![
            element("tri-a", 0, 1, 2, 0.1, 2.0, 1.0),
            element("tri-b", 0, 2, 3, 0.1, 4.0, 1.0),
        ],
    })
    .expect("heterogeneous triangle Stokes model should solve");

    assert_close(result.nodes[1].velocity_x, 3.0);
    assert_close(result.nodes[2].velocity_x, 2.0);
    assert_close(result.nodes[3].velocity_x, 2.0);
    assert_close(result.elements[0].area, 0.5);
    assert_close(result.elements[1].area, 0.5);
    assert_close(result.max_velocity, 3.0);
    assert_close(result.max_pressure, 8.0);
    assert_close(result.pressure_drop, 8.0);
    assert!(result.max_reynolds_number > 0.0);
    assert!(result.max_viscous_shear_stress > 0.0);
}

#[test]
fn stokes_flow_triangle_2d_rejects_degenerate_triangle_geometry() {
    let request = SolveStokesFlowPlaneTriangle2dRequest {
        nodes: vec![
            node("n0", 0.0, 0.0, true, 0.0, true, 0.0, true, 0.0, 0.0, 0.0),
            node("n1", 1.0, 0.0, true, 0.0, true, 0.0, true, 0.0, 0.0, 0.0),
            node("n2", 2.0, 0.0, true, 0.0, true, 0.0, true, 0.0, 0.0, 0.0),
        ],
        elements: vec![element("flat", 0, 1, 2, 0.1, 1.0, 1.0)],
    };

    let error = solve_stokes_flow_plane_triangle_2d(&request)
        .expect_err("collinear triangle should be rejected");
    assert!(
        error.contains("zero area"),
        "unexpected degenerate triangle error: {error}"
    );
}

#[test]
fn stokes_flow_triangle_2d_rejects_missing_node_references() {
    let request = SolveStokesFlowPlaneTriangle2dRequest {
        nodes: vec![
            node("n0", 0.0, 0.0, true, 0.0, true, 0.0, true, 0.0, 0.0, 0.0),
            node("n1", 1.0, 0.0, true, 0.0, true, 0.0, true, 0.0, 0.0, 0.0),
            node("n2", 0.0, 1.0, true, 0.0, true, 0.0, true, 0.0, 0.0, 0.0),
        ],
        elements: vec![element("bad", 0, 1, 7, 0.1, 1.0, 1.0)],
    };

    let error = solve_stokes_flow_plane_triangle_2d(&request)
        .expect_err("missing node index should be rejected");
    assert!(
        error.contains("node_k references missing node 7"),
        "unexpected missing-node error: {error}"
    );
}

#[test]
fn stokes_flow_triangle_2d_rejects_duplicate_element_nodes() {
    let request = SolveStokesFlowPlaneTriangle2dRequest {
        nodes: vec![
            node("n0", 0.0, 0.0, true, 0.0, true, 0.0, true, 0.0, 0.0, 0.0),
            node("n1", 1.0, 0.0, true, 0.0, true, 0.0, true, 0.0, 0.0, 0.0),
            node("n2", 0.0, 1.0, true, 0.0, true, 0.0, true, 0.0, 0.0, 0.0),
        ],
        elements: vec![element("folded", 0, 1, 1, 0.1, 1.0, 1.0)],
    };

    let error = solve_stokes_flow_plane_triangle_2d(&request)
        .expect_err("duplicate stokes triangle node should be rejected");
    assert!(
        error.contains("three distinct nodes"),
        "unexpected duplicate-node error: {error}"
    );
}

#[test]
fn stokes_flow_triangle_2d_rejects_non_finite_inputs_and_invalid_material() {
    let mut request = valid_triangle_request();
    request.nodes[1].body_force_x = f64::NAN;
    let error = solve_stokes_flow_plane_triangle_2d(&request)
        .expect_err("non-finite body force should be rejected");
    assert!(
        error.contains("body force must be finite"),
        "unexpected non-finite node error: {error}"
    );

    let mut request = valid_triangle_request();
    request.elements[0].viscosity = f64::INFINITY;
    let error = solve_stokes_flow_plane_triangle_2d(&request)
        .expect_err("non-finite viscosity should be rejected");
    assert!(
        error.contains("viscosity must be finite"),
        "unexpected non-finite viscosity error: {error}"
    );

    let mut request = valid_triangle_request();
    request.elements[0].thickness = 0.0;
    let error = solve_stokes_flow_plane_triangle_2d(&request)
        .expect_err("zero thickness should be rejected");
    assert!(
        error.contains("non-positive thickness"),
        "unexpected zero-thickness error: {error}"
    );
}

fn valid_triangle_request() -> SolveStokesFlowPlaneTriangle2dRequest {
    SolveStokesFlowPlaneTriangle2dRequest {
        nodes: vec![
            node("n0", 0.0, 0.0, true, 0.0, true, 0.0, true, 0.0, 0.0, 0.0),
            node("n1", 1.0, 0.0, false, 0.0, true, 0.0, false, 0.0, 1.0, 0.0),
            node("n2", 0.0, 1.0, true, 0.0, false, 0.0, false, 0.0, 0.0, 1.0),
        ],
        elements: vec![element("tri", 0, 1, 2, 0.1, 1.0, 1.0)],
    }
}

#[allow(clippy::too_many_arguments)]
fn node(
    id: &str,
    x: f64,
    y: f64,
    fix_velocity_x: bool,
    velocity_x: f64,
    fix_velocity_y: bool,
    velocity_y: f64,
    fix_pressure: bool,
    pressure: f64,
    body_force_x: f64,
    body_force_y: f64,
) -> StokesFlowPlaneNodeInput {
    StokesFlowPlaneNodeInput {
        id: id.to_string(),
        x,
        y,
        fix_velocity_x,
        velocity_x,
        fix_velocity_y,
        velocity_y,
        fix_pressure,
        pressure,
        body_force_x,
        body_force_y,
    }
}

fn element(
    id: &str,
    node_i: usize,
    node_j: usize,
    node_k: usize,
    thickness: f64,
    viscosity: f64,
    density: f64,
) -> StokesFlowPlaneTriangleElementInput {
    StokesFlowPlaneTriangleElementInput {
        id: id.to_string(),
        node_i,
        node_j,
        node_k,
        thickness,
        viscosity,
        density,
    }
}

fn assert_close(actual: f64, expected: f64) {
    let scale = expected.abs().max(1.0);
    assert!(
        (actual - expected).abs() <= TOL * scale,
        "expected {actual} to be close to {expected}",
    );
}
