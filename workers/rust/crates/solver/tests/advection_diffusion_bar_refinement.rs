use kyuubiki_protocol::{
    AdvectionDiffusionBar1dElementInput, AdvectionDiffusionBar1dNodeInput,
    SolveAdvectionDiffusionBar1dRequest,
};
use kyuubiki_solver::solve_advection_diffusion_bar_1d;

const TOL: f64 = 1.0e-10;
const LENGTH: f64 = 2.0;
const LEFT_CONCENTRATION: f64 = 1.0;
const RIGHT_CONCENTRATION: f64 = 0.2;
const DIFFUSIVITY: f64 = 0.05;

#[test]
fn pure_diffusion_linear_field_is_refinement_invariant_and_flux_conservative() {
    let expected_gradient = (RIGHT_CONCENTRATION - LEFT_CONCENTRATION) / LENGTH;
    let expected_flux = -DIFFUSIVITY * expected_gradient;
    for elements in [1_usize, 2, 4, 8, 16, 32] {
        let result = solve_advection_diffusion_bar_1d(&mesh(elements))
            .expect("refined pure-diffusion field should solve");
        assert_eq!(result.elements.len(), elements);
        assert_close(result.max_peclet_number, 0.0);
        assert_close(result.max_total_flux, expected_flux);
        for node in &result.nodes {
            let expected = LEFT_CONCENTRATION + expected_gradient * node.x;
            assert_close(node.concentration, expected);
        }
        for element in &result.elements {
            assert_close(element.concentration_gradient, expected_gradient);
            assert_close(element.diffusive_flux, expected_flux);
            assert_close(element.advective_flux, 0.0);
            assert_close(element.total_flux, expected_flux);
            assert_close(element.peclet_number, 0.0);
        }
    }
}

fn mesh(element_count: usize) -> SolveAdvectionDiffusionBar1dRequest {
    let nodes = (0..=element_count)
        .map(|index| {
            let x = LENGTH * index as f64 / element_count as f64;
            AdvectionDiffusionBar1dNodeInput {
                id: format!("node-{index}"),
                x,
                fix_concentration: index == 0 || index == element_count,
                concentration: if index == 0 {
                    LEFT_CONCENTRATION
                } else if index == element_count {
                    RIGHT_CONCENTRATION
                } else {
                    0.0
                },
                source: 0.0,
            }
        })
        .collect();
    let elements = (0..element_count)
        .map(|index| AdvectionDiffusionBar1dElementInput {
            id: format!("element-{index}"),
            node_i: index,
            node_j: index + 1,
            area: 0.01,
            diffusivity: DIFFUSIVITY,
            velocity: 0.0,
        })
        .collect();
    SolveAdvectionDiffusionBar1dRequest { nodes, elements }
}

fn assert_close(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() <= TOL * expected.abs().max(1.0),
        "expected {actual} to be close to {expected}"
    );
}
