use kyuubiki_protocol::{
    MagnetostaticPlaneNodeInput, MagnetostaticPlaneQuadElementInput,
    MagnetostaticPlaneTriangleElementInput, SolveMagnetostaticPlaneQuad2dRequest,
    SolveMagnetostaticPlaneTriangle2dRequest,
};
use kyuubiki_solver::{solve_magnetostatic_plane_quad_2d, solve_magnetostatic_plane_triangle_2d};

#[test]
fn solves_magnetostatic_plane_triangle_field() {
    let result = solve_magnetostatic_plane_triangle_2d(&request())
        .expect("magnetostatic triangle should solve");

    assert_eq!(result.nodes.len(), 3);
    assert_eq!(result.elements.len(), 1);
    assert!(result.max_vector_potential > 0.0);
    assert!(result.max_magnetic_field_strength > 0.0);
    assert!(result.max_flux_density > 0.0);
    assert!(result.total_stored_energy > 0.0);
    let element = &result.elements[0];
    assert!(element.area > 0.0);
    assert!(element.magnetic_flux_density_magnitude.is_finite());
    assert!(element.magnetic_field_strength_magnitude.is_finite());
}

#[test]
fn solves_magnetostatic_plane_quad_field() {
    let result = solve_magnetostatic_plane_quad_2d(&quad_request())
        .expect("magnetostatic quad should solve");

    assert_eq!(result.nodes.len(), 4);
    assert_eq!(result.elements.len(), 1);
    assert!(result.max_vector_potential > 0.0);
    assert!(result.max_magnetic_field_strength > 0.0);
    assert!(result.max_flux_density > 0.0);
    assert!(result.total_stored_energy > 0.0);
    assert!(result.elements[0].area > 0.0);
}

fn request() -> SolveMagnetostaticPlaneTriangle2dRequest {
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
