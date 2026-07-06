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

fn assert_close(actual: f64, expected: f64) {
    let scale = expected.abs().max(1.0);
    assert!(
        (actual - expected).abs() <= TOL * scale,
        "expected {actual} to be close to {expected}",
    );
}
