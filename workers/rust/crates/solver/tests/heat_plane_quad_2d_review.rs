use kyuubiki_protocol::{
    HeatPlaneNodeInput, HeatPlaneQuadElementInput, SolveHeatPlaneQuad2dRequest,
};
use kyuubiki_solver::solve_heat_plane_quad_2d;

const TOL: f64 = 1.0e-10;

#[test]
fn heat_plane_quad_2d_review_bundle_checks_boundary_field_and_flux_diagnostics() {
    let result = solve_heat_plane_quad_2d(&SolveHeatPlaneQuad2dRequest {
        nodes: vec![
            node("hot_lower", 0.0, 0.0, true, 100.0),
            node("free_lower", 1.0, 0.0, false, 0.0),
            node("cold_upper", 1.0, 1.0, true, 20.0),
            node("cool_upper", 0.0, 1.0, true, 20.0),
        ],
        elements: vec![HeatPlaneQuadElementInput {
            id: "quad".to_string(),
            node_i: 0,
            node_j: 1,
            node_k: 2,
            node_l: 3,
            thickness: 0.02,
            conductivity: 45.0,
        }],
    })
    .expect("review heat plane quad should solve");

    assert_eq!(result.nodes.len(), 4);
    assert_eq!(result.elements.len(), 1);
    assert_close(result.nodes[0].temperature, 100.0);
    assert_close(result.nodes[1].temperature, 60.0);
    assert_close(result.nodes[2].temperature, 20.0);
    assert_close(result.nodes[3].temperature, 20.0);
    assert_close(result.max_temperature, 100.0);

    let element = &result.elements[0];
    assert_close(element.area, 1.0);
    assert_close(element.average_temperature, 50.0);
    assert_close(element.temperature_gradient_x, -20.0);
    assert_close(element.temperature_gradient_y, -60.0);
    assert_close(element.heat_flux_x, 900.0);
    assert_close(element.heat_flux_y, 2700.0);
    assert_close(
        element.heat_flux_magnitude,
        f64::sqrt(900.0 * 900.0 + 2700.0 * 2700.0),
    );
    assert_close(result.max_heat_flux, element.heat_flux_magnitude);
}

fn node(id: &str, x: f64, y: f64, fix_temperature: bool, temperature: f64) -> HeatPlaneNodeInput {
    HeatPlaneNodeInput {
        id: id.to_string(),
        x,
        y,
        fix_temperature,
        temperature,
        heat_load: 0.0,
    }
}

fn assert_close(actual: f64, expected: f64) {
    let scale = expected.abs().max(1.0);
    assert!(
        (actual - expected).abs() <= TOL * scale,
        "expected {actual} to be close to {expected}",
    );
}
