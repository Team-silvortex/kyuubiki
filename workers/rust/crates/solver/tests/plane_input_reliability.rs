use kyuubiki_protocol::{
    PlaneNodeInput, PlaneQuadElementInput, PlaneTriangleElementInput, SolvePlaneQuad2dRequest,
    SolvePlaneTriangle2dRequest,
};
use kyuubiki_solver::{solve_plane_quad_2d, solve_plane_triangle_2d};

#[test]
fn plane_triangle_2d_rejects_non_finite_node_inputs_and_invalid_topology() {
    let mut request = triangle_request();
    request.nodes[2].x = f64::NAN;
    assert!(solve_plane_triangle_2d(&request).is_err());

    let mut request = triangle_request();
    request.nodes[2].load_y = f64::INFINITY;
    assert!(solve_plane_triangle_2d(&request).is_err());

    let mut request = triangle_request();
    request.elements[0].node_k = request.elements[0].node_i;
    assert!(solve_plane_triangle_2d(&request).is_err());
}

#[test]
fn plane_quad_2d_rejects_non_finite_node_inputs_and_invalid_material() {
    let mut request = quad_request();
    request.nodes[2].y = f64::NAN;
    assert!(solve_plane_quad_2d(&request).is_err());

    let mut request = quad_request();
    request.nodes[2].load_x = f64::NEG_INFINITY;
    assert!(solve_plane_quad_2d(&request).is_err());

    let mut request = quad_request();
    request.elements[0].poisson_ratio = 0.5;
    assert!(solve_plane_quad_2d(&request).is_err());
}

fn triangle_request() -> SolvePlaneTriangle2dRequest {
    SolvePlaneTriangle2dRequest {
        nodes: vec![
            node("bottom_left", 0.0, 0.0, true, true, 0.0, 0.0),
            node("bottom_right", 1.0, 0.0, false, true, 0.0, 0.0),
            node("top_right", 1.0, 1.0, false, false, 0.0, -1000.0),
        ],
        elements: vec![PlaneTriangleElementInput {
            id: "tri".to_string(),
            node_i: 0,
            node_j: 1,
            node_k: 2,
            thickness: 0.02,
            youngs_modulus: 70.0e9,
            poisson_ratio: 0.33,
        }],
    }
}

fn quad_request() -> SolvePlaneQuad2dRequest {
    SolvePlaneQuad2dRequest {
        nodes: vec![
            node("bottom_left", 0.0, 0.0, true, true, 0.0, 0.0),
            node("bottom_right", 1.0, 0.0, false, true, 0.0, 0.0),
            node("top_right", 1.0, 0.8, false, false, 200.0, -1200.0),
            node("top_left", 0.0, 0.8, true, false, 200.0, -1200.0),
        ],
        elements: vec![PlaneQuadElementInput {
            id: "quad_panel".to_string(),
            node_i: 0,
            node_j: 1,
            node_k: 2,
            node_l: 3,
            thickness: 0.02,
            youngs_modulus: 210.0e9,
            poisson_ratio: 0.3,
        }],
    }
}

fn node(
    id: &str,
    x: f64,
    y: f64,
    fix_x: bool,
    fix_y: bool,
    load_x: f64,
    load_y: f64,
) -> PlaneNodeInput {
    PlaneNodeInput {
        id: id.to_string(),
        x,
        y,
        fix_x,
        fix_y,
        load_x,
        load_y,
    }
}
