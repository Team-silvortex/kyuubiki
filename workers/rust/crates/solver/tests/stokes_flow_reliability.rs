use kyuubiki_protocol::{
    SolveStokesFlowPlaneQuad2dRequest, StokesFlowPlaneNodeInput, StokesFlowPlaneQuadElementInput,
};
use kyuubiki_solver::solve_stokes_flow_plane_quad_2d;

const TOL: f64 = 1.0e-10;
const DIVERGENCE_TOLERANCE: f64 = 1.0e-10;

#[test]
fn stokes_flow_quad_2d_matches_rectangular_screening_baseline() {
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
    .expect("stokes screening baseline should solve");

    let expected_velocity_x = [0.0, 1.0, 1.0, 0.0];
    let expected_velocity_y = [0.0, 0.0, 0.25, 0.0];
    let expected_pressure = [1.0, -2.0, -2.5, -0.0];
    for index in 0..4 {
        assert_close(result.nodes[index].velocity_x, expected_velocity_x[index]);
        assert_close(result.nodes[index].velocity_y, expected_velocity_y[index]);
        assert_close(result.nodes[index].pressure, expected_pressure[index]);
    }

    let element = &result.elements[0];
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
    let shear_rate = f64::sqrt(2.0 * du_dx * du_dx + 2.0 * dv_dy * dv_dy + 0.125 * 0.125);
    let max_viscous_shear_stress = viscosity * 2.0;
    let divergence_error = du_dx + dv_dy;
    let reynolds_number = density * average_velocity_magnitude / viscosity;
    let viscous_dissipation = viscosity
        * (du_dx * du_dx
            + dv_dy * dv_dy
            + velocity_gradient_x * velocity_gradient_x
            + velocity_gradient_y * velocity_gradient_y)
        * thickness;

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
    assert_close(element.shear_rate, shear_rate);
    assert_close(element.max_viscous_shear_stress, max_viscous_shear_stress);
    assert_close(element.divergence_error, divergence_error);
    assert_close(element.reynolds_number, reynolds_number);
    assert_close(element.viscous_dissipation, viscous_dissipation);
    assert_close(result.max_velocity, f64::sqrt(1.0 * 1.0 + 0.25 * 0.25));
    assert_close(result.max_pressure, 2.5);
    assert_close(result.pressure_drop, 3.5);
    assert_close(result.max_divergence_error, divergence_error);
    assert_close(result.max_reynolds_number, reynolds_number);
    assert_close(result.max_shear_rate, shear_rate);
    assert_close(result.max_viscous_shear_stress, max_viscous_shear_stress);
}

#[test]
fn stokes_flow_quad_2d_captures_lid_driven_shear_boundary_response() {
    let viscosity = 3.0;
    let density = 1.2;
    let thickness = 0.2;

    let result = solve_stokes_flow_plane_quad_2d(&SolveStokesFlowPlaneQuad2dRequest {
        nodes: vec![
            node("n0", 0.0, 0.0, true, 0.0, true, 0.0, true, 0.0, 0.0, 0.0),
            node("n1", 2.0, 0.0, true, 0.0, true, 0.0, true, 0.0, 0.0, 0.0),
            node("n2", 2.0, 1.0, true, 1.0, true, 0.0, true, 0.0, 0.0, 0.0),
            node("n3", 0.0, 1.0, true, 1.0, true, 0.0, true, 0.0, 0.0, 0.0),
        ],
        elements: vec![StokesFlowPlaneQuadElementInput {
            id: "lid-cell".to_string(),
            node_i: 0,
            node_j: 1,
            node_k: 2,
            node_l: 3,
            thickness,
            viscosity,
            density,
        }],
    })
    .expect("lid-driven stokes screening case should solve");

    let element = &result.elements[0];
    let average_velocity_magnitude = 0.5;
    let characteristic_length = f64::sqrt(2.0);
    let reynolds_number = density * average_velocity_magnitude * characteristic_length / viscosity;
    let viscous_dissipation = viscosity * 1.0 * 2.0 * thickness;

    assert_close(element.area, 2.0);
    assert_close(element.average_velocity_x, 0.5);
    assert_close(element.average_velocity_y, 0.0);
    assert_close(
        element.average_velocity_magnitude,
        average_velocity_magnitude,
    );
    assert_close(element.average_pressure, 0.0);
    assert_close(element.velocity_gradient_x, 0.0);
    assert_close(element.velocity_gradient_y, 1.0);
    assert_close(element.shear_rate, 1.0);
    assert_close(element.max_viscous_shear_stress, viscosity);
    assert_close(element.divergence_error, 0.0);
    assert_screening_divergence(result.max_divergence_error);
    assert_close(element.reynolds_number, reynolds_number);
    assert_close(element.viscous_dissipation, viscous_dissipation);
    assert_close(result.max_velocity, 1.0);
    assert_close(result.max_pressure, 0.0);
    assert_close(result.pressure_drop, 0.0);
    assert_close(result.max_reynolds_number, reynolds_number);
    assert_close(result.max_shear_rate, 1.0);
    assert_close(result.max_viscous_shear_stress, viscosity);
}

#[test]
fn stokes_flow_quad_2d_rejects_non_finite_node_input() {
    let mut request = SolveStokesFlowPlaneQuad2dRequest {
        nodes: vec![
            node("n0", 0.0, 0.0, true, 0.0, true, 0.0, true, 0.0, 0.0, 0.0),
            node("n1", 1.0, 0.0, true, 0.0, true, 0.0, true, 0.0, 0.0, 0.0),
            node("n2", 1.0, 1.0, true, 0.0, true, 0.0, true, 0.0, 0.0, 0.0),
            node("n3", 0.0, 1.0, true, 0.0, true, 0.0, true, 0.0, 0.0, 0.0),
        ],
        elements: vec![StokesFlowPlaneQuadElementInput {
            id: "cell".to_string(),
            node_i: 0,
            node_j: 1,
            node_k: 2,
            node_l: 3,
            thickness: 0.1,
            viscosity: 1.0,
            density: 1.0,
        }],
    };
    request.nodes[2].body_force_y = f64::NAN;

    let error = solve_stokes_flow_plane_quad_2d(&request)
        .expect_err("non-finite stokes body force should be rejected");
    assert!(
        error.contains("body force must be finite"),
        "unexpected error: {error}"
    );
}

#[test]
fn stokes_flow_quad_2d_rejects_non_finite_element_input() {
    let mut request = SolveStokesFlowPlaneQuad2dRequest {
        nodes: vec![
            node("n0", 0.0, 0.0, true, 0.0, true, 0.0, true, 0.0, 0.0, 0.0),
            node("n1", 1.0, 0.0, true, 0.0, true, 0.0, true, 0.0, 0.0, 0.0),
            node("n2", 1.0, 1.0, true, 0.0, true, 0.0, true, 0.0, 0.0, 0.0),
            node("n3", 0.0, 1.0, true, 0.0, true, 0.0, true, 0.0, 0.0, 0.0),
        ],
        elements: vec![StokesFlowPlaneQuadElementInput {
            id: "cell".to_string(),
            node_i: 0,
            node_j: 1,
            node_k: 2,
            node_l: 3,
            thickness: 0.1,
            viscosity: f64::INFINITY,
            density: 1.0,
        }],
    };

    let error = solve_stokes_flow_plane_quad_2d(&request)
        .expect_err("non-finite stokes viscosity should be rejected");
    assert!(
        error.contains("viscosity must be finite"),
        "unexpected error: {error}"
    );

    request.elements[0].viscosity = 1.0;
    request.elements[0].density = f64::NAN;
    let error = solve_stokes_flow_plane_quad_2d(&request)
        .expect_err("non-finite stokes density should be rejected");
    assert!(
        error.contains("density must be finite"),
        "unexpected error: {error}"
    );
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

fn assert_screening_divergence(actual: f64) {
    assert!(
        actual <= DIVERGENCE_TOLERANCE,
        "expected Stokes screening divergence {actual} <= {DIVERGENCE_TOLERANCE}",
    );
}
