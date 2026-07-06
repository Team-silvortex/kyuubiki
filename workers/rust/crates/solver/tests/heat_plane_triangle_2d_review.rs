use kyuubiki_protocol::{
    HeatPlaneNodeInput, HeatPlaneTriangleElementInput, SolveHeatPlaneTriangle2dRequest,
};
use kyuubiki_solver::solve_heat_plane_triangle_2d;

const TOL: f64 = 1.0e-10;

#[test]
fn heat_plane_triangle_2d_review_bundle_checks_boundary_field_and_flux_diagnostics() {
    let result = solve_heat_plane_triangle_2d(&SolveHeatPlaneTriangle2dRequest {
        nodes: vec![
            node("hot_lower", 0.0, 0.0, true, 100.0),
            node("free_lower", 1.0, 0.0, false, 0.0),
            node("cold_upper", 1.0, 1.0, true, 20.0),
            node("cool_upper", 0.0, 1.0, true, 20.0),
        ],
        elements: vec![
            element("tri_lower", 0, 1, 2),
            element("tri_upper", 0, 2, 3),
        ],
    })
    .expect("review heat plane triangle should solve");

    assert_eq!(result.nodes.len(), 4);
    assert_eq!(result.elements.len(), 2);
    assert_close(result.nodes[0].temperature, 100.0);
    assert_close(result.nodes[1].temperature, 60.0);
    assert_close(result.nodes[2].temperature, 20.0);
    assert_close(result.nodes[3].temperature, 20.0);
    assert_close(result.max_temperature, 100.0);

    let lower = &result.elements[0];
    assert_close(lower.area, 0.5);
    assert_close(lower.average_temperature, 60.0);
    assert_close(lower.temperature_gradient_x, -40.0);
    assert_close(lower.temperature_gradient_y, -40.0);
    assert_close(lower.heat_flux_x, 1800.0);
    assert_close(lower.heat_flux_y, 1800.0);
    assert_close(
        lower.heat_flux_magnitude,
        f64::sqrt(1800.0 * 1800.0 + 1800.0 * 1800.0),
    );

    let upper = &result.elements[1];
    assert_close(upper.area, 0.5);
    assert_close(upper.average_temperature, 140.0 / 3.0);
    assert_close(upper.temperature_gradient_x, 0.0);
    assert_close(upper.temperature_gradient_y, -80.0);
    assert_close(upper.heat_flux_x, 0.0);
    assert_close(upper.heat_flux_y, 3600.0);
    assert_close(upper.heat_flux_magnitude, 3600.0);
    assert_close(result.max_heat_flux, upper.heat_flux_magnitude);
}

fn node(
    id: &str,
    x: f64,
    y: f64,
    fix_temperature: bool,
    temperature: f64,
) -> HeatPlaneNodeInput {
    HeatPlaneNodeInput {
        id: id.to_string(),
        x,
        y,
        fix_temperature,
        temperature,
        heat_load: 0.0,
    }
}

fn element(id: &str, node_i: usize, node_j: usize, node_k: usize) -> HeatPlaneTriangleElementInput {
    HeatPlaneTriangleElementInput {
        id: id.to_string(),
        node_i,
        node_j,
        node_k,
        thickness: 0.02,
        conductivity: 45.0,
    }
}

fn assert_close(actual: f64, expected: f64) {
    let scale = expected.abs().max(1.0);
    assert!(
        (actual - expected).abs() <= TOL * scale,
        "expected {actual} to be close to {expected}",
    );
}
