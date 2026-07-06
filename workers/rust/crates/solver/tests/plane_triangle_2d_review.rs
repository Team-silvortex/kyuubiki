use kyuubiki_protocol::{
    PlaneNodeInput, PlaneTriangleElementInput, PlaneTriangleElementResult,
    SolvePlaneTriangle2dRequest,
};
use kyuubiki_solver::solve_plane_triangle_2d;

#[test]
fn plane_triangle_2d_review_bundle_checks_panel_boundaries_stress_and_strain_diagnostics() {
    let result = solve_plane_triangle_2d(&request()).expect("review plane triangle should solve");

    assert_eq!(result.nodes.len(), 4);
    assert_eq!(result.elements.len(), 2);
    assert_close(result.nodes[0].ux, 0.0, 1.0e-12);
    assert_close(result.nodes[0].uy, 0.0, 1.0e-12);
    assert_close(result.nodes[1].uy, 0.0, 1.0e-12);
    assert_close(result.nodes[3].ux, 0.0, 1.0e-12);
    assert_close(result.nodes[2].ux, 4.714_285_714_285_715e-7, 1.0e-12);
    assert_close(result.nodes[2].uy, -1.428_571_428_571_429e-6, 1.0e-12);
    assert!(result.nodes[2].uy < 0.0);
    assert!(result.nodes[3].uy < 0.0);
    assert_close(result.max_displacement, 1.504_347_441_414_315e-6, 1.0e-12);
    assert_close(result.max_stress, 100_000.0, 1.0e-10);

    for element in &result.elements {
        assert_close(element.area, 0.5, 1.0e-12);
        assert_finite_plane_stress_state(element);
        assert!(element.von_mises >= 0.0);
        assert!(element.max_in_plane_shear >= 0.0);
        assert!(
            element.principal_stress_1 >= element.principal_stress_2,
            "principal stress ordering should be descending",
        );
    }
    assert_close(result.elements[0].von_mises, 100_000.0, 1.0e-10);
}

fn request() -> SolvePlaneTriangle2dRequest {
    SolvePlaneTriangle2dRequest {
        nodes: vec![
            node("bottom_left", 0.0, 0.0, true, true, 0.0, 0.0),
            node("bottom_right", 1.0, 0.0, false, true, 0.0, 0.0),
            node("top_right", 1.0, 1.0, false, false, 0.0, -1000.0),
            node("top_left", 0.0, 1.0, true, false, 0.0, -1000.0),
        ],
        elements: vec![element("tri_lower", 0, 1, 2), element("tri_upper", 0, 2, 3)],
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

fn element(id: &str, node_i: usize, node_j: usize, node_k: usize) -> PlaneTriangleElementInput {
    PlaneTriangleElementInput {
        id: id.to_string(),
        node_i,
        node_j,
        node_k,
        thickness: 0.02,
        youngs_modulus: 70.0e9,
        poisson_ratio: 0.33,
    }
}

fn assert_finite_plane_stress_state(element: &PlaneTriangleElementResult) {
    for value in [
        element.strain_x,
        element.strain_y,
        element.gamma_xy,
        element.stress_x,
        element.stress_y,
        element.tau_xy,
        element.principal_stress_1,
        element.principal_stress_2,
        element.max_in_plane_shear,
        element.von_mises,
    ] {
        assert!(value.is_finite());
    }
}

fn assert_close(actual: f64, expected: f64, tolerance: f64) {
    let scale = expected.abs().max(1.0);
    assert!(
        (actual - expected).abs() <= tolerance * scale,
        "expected {actual} to be close to {expected}",
    );
}
