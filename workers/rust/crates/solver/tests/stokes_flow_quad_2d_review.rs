use kyuubiki_protocol::{
    SolveStokesFlowPlaneQuad2dRequest, StokesFlowPlaneNodeInput, StokesFlowPlaneQuadElementInput,
};
use kyuubiki_solver::solve_stokes_flow_plane_quad_2d;

const TOL: f64 = 1.0e-10;

#[test]
fn stokes_flow_quad_2d_review_bundle_checks_velocity_pressure_divergence_reynolds_and_dissipation()
{
    let viscosity = 2.0;
    let density = 1.0;
    let thickness = 0.1;

    let result = solve_stokes_flow_plane_quad_2d(&SolveStokesFlowPlaneQuad2dRequest {
        nodes: vec![
            node("n0", 0.0, 0.0, true, 0.0, true, 0.0, true, 1.0, 0.0, 0.0),
            node("n1", 1.0, 0.0, false, 0.0, true, 0.0, false, 0.0, 2.0, 0.0),
            node("n2", 1.0, 1.0, false, 0.0, false, 0.0, false, 0.0, 2.0, 0.5),
            node("n3", 0.0, 1.0, true, 0.0, true, 0.0, false, 0.0, 0.0, 0.0),
        ],
        elements: vec![StokesFlowPlaneQuadElementInput {
            id: "cell".to_string(),
            node_i: 0,
            node_j: 1,
            node_k: 2,
            node_l: 3,
            thickness,
            viscosity,
            density,
        }],
    })
    .expect("review stokes flow quad should solve");

    let expected_velocity_x = [0.0, 1.0, 1.0, 0.0];
    let expected_velocity_y = [0.0, 0.0, 0.25, 0.0];
    let expected_pressure = [1.0, -2.0, -2.5, 0.0];
    for index in 0..4 {
        assert_close(result.nodes[index].velocity_x, expected_velocity_x[index]);
        assert_close(result.nodes[index].velocity_y, expected_velocity_y[index]);
        assert_close(result.nodes[index].pressure, expected_pressure[index]);
    }

    let average_velocity_x = 0.5;
    let average_velocity_y = 0.0625;
    let average_velocity_magnitude = f64::sqrt(
        average_velocity_x * average_velocity_x + average_velocity_y * average_velocity_y,
    );
    let average_pressure = -0.875;
    let du_dx = 1.0;
    let dv_dy = 0.125;
    let velocity_gradient_x = 0.125;
    let velocity_gradient_y = 0.0;
    let divergence_error = du_dx + dv_dy;
    let reynolds_number = density * average_velocity_magnitude / viscosity;
    let viscous_dissipation = viscosity
        * (du_dx * du_dx
            + dv_dy * dv_dy
            + velocity_gradient_x * velocity_gradient_x
            + velocity_gradient_y * velocity_gradient_y)
        * thickness;

    let element = &result.elements[0];
    assert_close(element.area, 1.0);
    assert_close(element.average_velocity_x, average_velocity_x);
    assert_close(element.average_velocity_y, average_velocity_y);
    assert_close(
        element.average_velocity_magnitude,
        average_velocity_magnitude,
    );
    assert_close(element.average_pressure, average_pressure);
    assert_close(element.velocity_gradient_x, velocity_gradient_x);
    assert_close(element.velocity_gradient_y, velocity_gradient_y);
    assert_close(element.divergence_error, divergence_error);
    assert_close(element.reynolds_number, reynolds_number);
    assert_close(element.viscous_dissipation, viscous_dissipation);
    assert_close(result.max_velocity, f64::sqrt(1.0 * 1.0 + 0.25 * 0.25));
    assert_close(result.max_pressure, 2.5);
    assert_close(result.max_divergence_error, divergence_error);
    assert_close(result.max_reynolds_number, reynolds_number);
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

fn assert_close(actual: f64, expected: f64) {
    let scale = expected.abs().max(1.0);
    assert!(
        (actual - expected).abs() <= TOL * scale,
        "expected {actual} to be close to {expected}",
    );
}
