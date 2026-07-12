use kyuubiki_protocol::{
    SolveThermalPlaneQuad2dRequest, SolveThermalPlaneTriangle2dRequest, ThermalPlaneNodeInput,
    ThermalPlaneQuadElementInput, ThermalPlaneTriangleElementInput,
};
use kyuubiki_solver::{solve_thermal_plane_quad_2d, solve_thermal_plane_triangle_2d};

#[test]
fn thermal_plane_triangle_rejects_non_finite_node_inputs_and_invalid_topology() {
    let mut request = triangle_request();
    request.nodes[2].x = f64::NAN;
    assert!(solve_thermal_plane_triangle_2d(&request).is_err());

    let mut request = triangle_request();
    request.nodes[2].load_y = f64::INFINITY;
    assert!(solve_thermal_plane_triangle_2d(&request).is_err());

    let mut request = triangle_request();
    request.elements[0].node_k = request.elements[0].node_i;
    assert!(solve_thermal_plane_triangle_2d(&request).is_err());

    let mut request = triangle_request();
    request.nodes[1].x = f64::MAX;
    request.nodes[2].y = f64::MAX;
    assert!(solve_thermal_plane_triangle_2d(&request).is_err());
}

#[test]
fn thermal_plane_quad_rejects_non_finite_temperature_and_invalid_expansion() {
    let mut request = quad_request();
    request.nodes[2].temperature_delta = f64::NEG_INFINITY;
    assert!(solve_thermal_plane_quad_2d(&request).is_err());

    let mut request = quad_request();
    request.elements[0].thermal_expansion = -1.0e-6;
    assert!(solve_thermal_plane_quad_2d(&request).is_err());
}

fn triangle_request() -> SolveThermalPlaneTriangle2dRequest {
    SolveThermalPlaneTriangle2dRequest {
        nodes: vec![
            node("n0", 0.0, 0.0, true, true, 40.0),
            node("n1", 1.0, 0.0, true, true, 40.0),
            node("n2", 1.0, 1.0, true, true, 40.0),
        ],
        elements: vec![ThermalPlaneTriangleElementInput {
            id: "tri".to_string(),
            node_i: 0,
            node_j: 1,
            node_k: 2,
            thickness: 0.02,
            youngs_modulus: 70.0e9,
            poisson_ratio: 0.33,
            thermal_expansion: 12.0e-6,
        }],
    }
}

fn quad_request() -> SolveThermalPlaneQuad2dRequest {
    SolveThermalPlaneQuad2dRequest {
        nodes: vec![
            node("n0", 0.0, 0.0, true, true, 30.0),
            node("n1", 1.0, 0.0, true, true, 30.0),
            node("n2", 1.0, 1.0, true, true, 30.0),
            node("n3", 0.0, 1.0, true, true, 30.0),
        ],
        elements: vec![ThermalPlaneQuadElementInput {
            id: "quad".to_string(),
            node_i: 0,
            node_j: 1,
            node_k: 2,
            node_l: 3,
            thickness: 0.02,
            youngs_modulus: 70.0e9,
            poisson_ratio: 0.33,
            thermal_expansion: 11.0e-6,
        }],
    }
}

fn node(
    id: &str,
    x: f64,
    y: f64,
    fix_x: bool,
    fix_y: bool,
    temperature_delta: f64,
) -> ThermalPlaneNodeInput {
    ThermalPlaneNodeInput {
        id: id.to_string(),
        x,
        y,
        fix_x,
        fix_y,
        load_x: 0.0,
        load_y: 0.0,
        temperature_delta,
    }
}
