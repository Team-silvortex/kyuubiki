use kyuubiki_protocol::{HeatBar1dElementInput, HeatBar1dNodeInput, SolveHeatBar1dRequest};
use kyuubiki_solver::solve_heat_bar_1d;

const TOL: f64 = 1.0e-10;

#[test]
fn heat_bar_1d_review_bundle_checks_boundaries_flux_and_gradient_diagnostics() {
    let request = SolveHeatBar1dRequest {
        nodes: vec![
            node("hot", 0.0, true, 100.0),
            node("mid", 0.5, false, 0.0),
            node("cold", 1.0, true, 20.0),
        ],
        elements: vec![element("left", 0, 1), element("right", 1, 2)],
    };

    let result = solve_heat_bar_1d(&request).expect("review heat bar should solve");

    let expected_gradient = -80.0;
    let expected_flux = 4_000.0;

    assert_eq!(result.nodes.len(), 3);
    assert_eq!(result.elements.len(), 2);
    assert_close(result.nodes[0].temperature, 100.0);
    assert_close(result.nodes[1].temperature, 60.0);
    assert_close(result.nodes[2].temperature, 20.0);
    assert_close(result.max_temperature, 100.0);
    assert_close(result.max_heat_flux, expected_flux);

    for element in &result.elements {
        assert_close(element.length, 0.5);
        assert_close(element.temperature_gradient, expected_gradient);
        assert_close(element.heat_flux, expected_flux);
    }
    assert_close(result.elements[0].average_temperature, 80.0);
    assert_close(result.elements[1].average_temperature, 40.0);
    assert_close(result.elements[0].heat_flux, result.elements[1].heat_flux);
}

fn node(id: &str, x: f64, fix_temperature: bool, temperature: f64) -> HeatBar1dNodeInput {
    HeatBar1dNodeInput {
        id: id.to_string(),
        x,
        fix_temperature,
        temperature,
        heat_load: 0.0,
    }
}

fn element(id: &str, node_i: usize, node_j: usize) -> HeatBar1dElementInput {
    HeatBar1dElementInput {
        id: id.to_string(),
        node_i,
        node_j,
        area: 0.02,
        conductivity: 50.0,
    }
}

fn assert_close(actual: f64, expected: f64) {
    let scale = expected.abs().max(1.0);
    assert!(
        (actual - expected).abs() <= TOL * scale,
        "expected {actual} to be close to {expected}",
    );
}
