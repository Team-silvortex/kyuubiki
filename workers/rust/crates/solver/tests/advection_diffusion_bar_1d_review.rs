use kyuubiki_protocol::{
    AdvectionDiffusionBar1dElementInput, AdvectionDiffusionBar1dNodeInput,
    SolveAdvectionDiffusionBar1dRequest,
};
use kyuubiki_solver::solve_advection_diffusion_bar_1d;

const TOL: f64 = 1.0e-10;

#[test]
fn advection_diffusion_bar_1d_review_bundle_checks_boundary_concentrations_fluxes_and_peclet() {
    let left_concentration = 1.0;
    let right_concentration = 0.2;
    let length = 1.0;
    let diffusivity = 0.05;
    let velocity = 0.1;

    let result = solve_advection_diffusion_bar_1d(&SolveAdvectionDiffusionBar1dRequest {
        nodes: vec![
            node("inlet", 0.0, true, left_concentration),
            node("outlet", length, true, right_concentration),
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
    .expect("review advection-diffusion bar should solve");

    let expected_average = (left_concentration + right_concentration) / 2.0;
    let expected_gradient = (right_concentration - left_concentration) / length;
    let expected_diffusive_flux = -diffusivity * expected_gradient;
    let expected_advective_flux = velocity * expected_average;
    let expected_total_flux = expected_diffusive_flux + expected_advective_flux;
    let expected_peclet = velocity.abs() * length / (2.0 * diffusivity);

    assert_eq!(result.nodes.len(), 2);
    assert_eq!(result.elements.len(), 1);
    assert_close(result.nodes[0].concentration, left_concentration);
    assert_close(result.nodes[1].concentration, right_concentration);
    assert_close(result.max_concentration, left_concentration);
    assert_close(result.max_total_flux, expected_total_flux.abs());
    assert_close(result.max_peclet_number, expected_peclet);

    let element = &result.elements[0];
    assert_close(element.length, length);
    assert_close(element.average_concentration, expected_average);
    assert_close(element.concentration_gradient, expected_gradient);
    assert_close(element.diffusive_flux, expected_diffusive_flux);
    assert_close(element.advective_flux, expected_advective_flux);
    assert_close(element.total_flux, expected_total_flux);
    assert_close(element.peclet_number, expected_peclet);
}

fn node(
    id: &str,
    x: f64,
    fix_concentration: bool,
    concentration: f64,
) -> AdvectionDiffusionBar1dNodeInput {
    AdvectionDiffusionBar1dNodeInput {
        id: id.to_string(),
        x,
        fix_concentration,
        concentration,
        source: 0.0,
    }
}

fn assert_close(actual: f64, expected: f64) {
    let scale = expected.abs().max(1.0);
    assert!(
        (actual - expected).abs() <= TOL * scale,
        "expected {actual} to be close to {expected}",
    );
}
