use kyuubiki_protocol::{
    PlaneNodeInput, PlaneQuadElementInput, PlaneQuadElementResult, SolvePlaneQuad2dRequest,
};
use kyuubiki_solver::solve_plane_quad_2d;

#[test]
fn plane_quad_2d_review_bundle_checks_panel_boundaries_weighted_stress_and_strain_diagnostics() {
    let result = solve_plane_quad_2d(&request()).expect("review plane quad should solve");

    assert_eq!(result.nodes.len(), 4);
    assert_eq!(result.elements.len(), 1);
    assert_close(result.nodes[0].ux, 0.0, 1.0e-12);
    assert_close(result.nodes[0].uy, 0.0, 1.0e-12);
    assert_close(result.nodes[1].uy, 0.0, 1.0e-12);
    assert_close(result.nodes[3].ux, 0.0, 1.0e-12);
    assert_close(result.nodes[2].ux, 2.576_145_151_695_419e-7, 1.0e-12);
    assert_close(result.nodes[2].uy, -4.670_094_331_605_336_6e-7, 1.0e-12);
    assert!(result.nodes[2].ux > 0.0);
    assert!(result.nodes[2].uy < 0.0);
    assert!(result.nodes[3].uy < 0.0);
    assert_close(result.max_displacement, 5.333_507_749_004_975e-7, 1.0e-12);
    assert_close(result.max_stress, 126_981.385_278_360_32, 1.0e-10);

    let element = &result.elements[0];
    assert_close(element.area, 0.8, 1.0e-12);
    assert_close(element.stress_x, 12_500.0, 1.0e-10);
    assert_close(element.stress_y, -120_000.0, 1.0e-10);
    assert_close(element.tau_xy, 3_048.780_487_804_874_6, 1.0e-10);
    assert_finite_plane_stress_state(element);
    assert!(element.von_mises >= 0.0);
    assert!(element.max_in_plane_shear >= 0.0);
    assert!(
        element.principal_stress_1 >= element.principal_stress_2,
        "principal stress ordering should be descending",
    );
    assert_close(element.von_mises, result.max_stress, 1.0e-12);
}

fn request() -> SolvePlaneQuad2dRequest {
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

fn assert_finite_plane_stress_state(element: &PlaneQuadElementResult) {
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
