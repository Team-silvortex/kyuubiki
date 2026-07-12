use kyuubiki_protocol::{
    MagnetostaticPlaneNodeInput, MagnetostaticPlaneQuadElementInput,
    MagnetostaticPlaneTriangleElementInput, SolveMagnetostaticPlaneQuad2dRequest,
    SolveMagnetostaticPlaneTriangle2dRequest,
};
use kyuubiki_solver::{solve_magnetostatic_plane_quad_2d, solve_magnetostatic_plane_triangle_2d};

#[test]
fn magnetostatic_plane_triangle_rejects_non_finite_node_inputs_and_duplicate_nodes() {
    let mut request = triangle_request();
    request.nodes[2].x = f64::NAN;
    assert!(solve_magnetostatic_plane_triangle_2d(&request).is_err());

    let mut request = triangle_request();
    request.nodes[2].current_density = f64::INFINITY;
    assert!(solve_magnetostatic_plane_triangle_2d(&request).is_err());

    let mut request = triangle_request();
    request.elements[0].node_k = request.elements[0].node_i;
    assert!(solve_magnetostatic_plane_triangle_2d(&request).is_err());
}

#[test]
fn magnetostatic_plane_quad_rejects_non_finite_potential_and_invalid_material() {
    let mut request = quad_request();
    request.nodes[2].vector_potential = f64::NEG_INFINITY;
    assert!(solve_magnetostatic_plane_quad_2d(&request).is_err());

    let mut request = quad_request();
    request.elements[0].permeability = f64::NAN;
    assert!(solve_magnetostatic_plane_quad_2d(&request).is_err());

    let mut request = quad_request();
    request.elements[0].node_l = request.elements[0].node_i;
    assert!(solve_magnetostatic_plane_quad_2d(&request).is_err());
}

fn triangle_request() -> SolveMagnetostaticPlaneTriangle2dRequest {
    SolveMagnetostaticPlaneTriangle2dRequest {
        nodes: vec![
            node("n0", 0.0, 0.0, true, 0.0, 0.0),
            node("n1", 1.0, 0.0, true, 0.0, 0.0),
            node("n2", 0.0, 1.0, false, 0.0, 5.0),
        ],
        elements: vec![MagnetostaticPlaneTriangleElementInput {
            id: "m0".to_string(),
            node_i: 0,
            node_j: 1,
            node_k: 2,
            thickness: 0.1,
            permeability: 4.0e-7 * std::f64::consts::PI,
        }],
    }
}

fn quad_request() -> SolveMagnetostaticPlaneQuad2dRequest {
    SolveMagnetostaticPlaneQuad2dRequest {
        nodes: vec![
            node("n0", 0.0, 0.0, true, 0.0, 0.0),
            node("n1", 1.0, 0.0, true, 0.0, 0.0),
            node("n2", 1.0, 1.0, false, 0.0, 5.0),
            node("n3", 0.0, 1.0, false, 0.0, 5.0),
        ],
        elements: vec![MagnetostaticPlaneQuadElementInput {
            id: "q0".to_string(),
            node_i: 0,
            node_j: 1,
            node_k: 2,
            node_l: 3,
            thickness: 0.1,
            permeability: 4.0e-7 * std::f64::consts::PI,
        }],
    }
}

fn node(
    id: &str,
    x: f64,
    y: f64,
    fixed: bool,
    vector_potential: f64,
    current_density: f64,
) -> MagnetostaticPlaneNodeInput {
    MagnetostaticPlaneNodeInput {
        id: id.to_string(),
        x,
        y,
        fix_vector_potential: fixed,
        vector_potential,
        current_density,
    }
}
