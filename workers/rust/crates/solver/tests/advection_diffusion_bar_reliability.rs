use kyuubiki_protocol::{
    AdvectionDiffusionBar1dElementInput, AdvectionDiffusionBar1dNodeInput,
    SolveAdvectionDiffusionBar1dRequest,
};
use kyuubiki_solver::solve_advection_diffusion_bar_1d;

const TOL: f64 = 1.0e-10;

#[test]
fn advection_diffusion_bar_1d_matches_single_element_flux_baseline() {
    let left_concentration = 1.0;
    let right_concentration = 0.2;
    let length = 1.0;
    let diffusivity = 0.05;
    let velocity = 0.1;

    let result = solve_advection_diffusion_bar_1d(&SolveAdvectionDiffusionBar1dRequest {
        nodes: vec![
            AdvectionDiffusionBar1dNodeInput {
                id: "inlet".to_string(),
                x: 0.0,
                fix_concentration: true,
                concentration: left_concentration,
                source: 0.0,
            },
            AdvectionDiffusionBar1dNodeInput {
                id: "outlet".to_string(),
                x: length,
                fix_concentration: true,
                concentration: right_concentration,
                source: 0.0,
            },
        ],
        elements: vec![AdvectionDiffusionBar1dElementInput {
            id: "transport".to_string(),
            node_i: 0,
            node_j: 1,
            area: 0.01,
            diffusivity,
            velocity,
        }],
    })
    .expect("advection-diffusion baseline should solve");

    let expected_average = (left_concentration + right_concentration) / 2.0;
    let expected_gradient = (right_concentration - left_concentration) / length;
    let expected_diffusive_flux = -diffusivity * expected_gradient;
    let expected_advective_flux = velocity * expected_average;
    let expected_total_flux = expected_diffusive_flux + expected_advective_flux;
    let expected_peclet = velocity.abs() * length / (2.0 * diffusivity);

    assert_close(result.nodes[0].concentration, left_concentration);
    assert_close(result.nodes[1].concentration, right_concentration);
    assert_close(result.max_concentration, left_concentration);
    assert_close(result.elements[0].average_concentration, expected_average);
    assert_close(result.elements[0].concentration_gradient, expected_gradient);
    assert_close(result.elements[0].diffusive_flux, expected_diffusive_flux);
    assert_close(result.elements[0].advective_flux, expected_advective_flux);
    assert_close(result.elements[0].total_flux, expected_total_flux);
    assert_close(result.elements[0].peclet_number, expected_peclet);
    assert_close(result.max_total_flux, expected_total_flux.abs());
    assert_close(result.max_peclet_number, expected_peclet);
}

#[test]
fn advection_diffusion_bar_1d_rejects_non_finite_node_values() {
    let mut request = transport_request();
    request.nodes[1].source = f64::NAN;

    let error = solve_advection_diffusion_bar_1d(&request)
        .expect_err("non-finite source should be rejected");
    assert!(
        error.contains("node values must be finite"),
        "unexpected non-finite node error: {error}"
    );
}

#[test]
fn advection_diffusion_bar_1d_rejects_missing_concentration_support() {
    let mut request = transport_request();
    for node in &mut request.nodes {
        node.fix_concentration = false;
    }

    let error = solve_advection_diffusion_bar_1d(&request)
        .expect_err("missing concentration support should be rejected");
    assert!(
        error.contains("at least one concentration support"),
        "unexpected missing-support error: {error}"
    );
}

#[test]
fn advection_diffusion_bar_1d_rejects_invalid_element_topology() {
    let mut request = transport_request();
    request.elements[0].node_j = 7;

    let error = solve_advection_diffusion_bar_1d(&request)
        .expect_err("out-of-range element node should be rejected");
    assert!(
        error.contains("out-of-range node"),
        "unexpected out-of-range node error: {error}"
    );

    let mut request = transport_request();
    request.nodes[1].x = request.nodes[0].x;
    let error =
        solve_advection_diffusion_bar_1d(&request).expect_err("zero-length element should fail");
    assert!(
        error.contains("length must be positive"),
        "unexpected zero-length error: {error}"
    );
}

#[test]
fn advection_diffusion_bar_1d_rejects_invalid_transport_materials() {
    let mut request = transport_request();
    request.elements[0].diffusivity = 0.0;

    let error = solve_advection_diffusion_bar_1d(&request)
        .expect_err("zero diffusivity should be rejected");
    assert!(
        error.contains("diffusivity must be positive"),
        "unexpected diffusivity error: {error}"
    );

    let mut request = transport_request();
    request.elements[0].area = f64::NAN;
    let error = solve_advection_diffusion_bar_1d(&request).expect_err("NaN area should fail");
    assert!(
        error.contains("area must be positive"),
        "unexpected area error: {error}"
    );

    let mut request = transport_request();
    request.elements[0].velocity = f64::INFINITY;
    let error =
        solve_advection_diffusion_bar_1d(&request).expect_err("infinite velocity should fail");
    assert!(
        error.contains("velocity must be finite"),
        "unexpected velocity error: {error}"
    );
}

fn transport_request() -> SolveAdvectionDiffusionBar1dRequest {
    SolveAdvectionDiffusionBar1dRequest {
        nodes: vec![
            AdvectionDiffusionBar1dNodeInput {
                id: "inlet".to_string(),
                x: 0.0,
                fix_concentration: true,
                concentration: 1.0,
                source: 0.0,
            },
            AdvectionDiffusionBar1dNodeInput {
                id: "outlet".to_string(),
                x: 1.0,
                fix_concentration: true,
                concentration: 0.2,
                source: 0.0,
            },
        ],
        elements: vec![AdvectionDiffusionBar1dElementInput {
            id: "transport".to_string(),
            node_i: 0,
            node_j: 1,
            area: 0.01,
            diffusivity: 0.05,
            velocity: 0.1,
        }],
    }
}

fn assert_close(actual: f64, expected: f64) {
    let scale = expected.abs().max(1.0);
    assert!(
        (actual - expected).abs() <= TOL * scale,
        "expected {actual} to be close to {expected}",
    );
}
