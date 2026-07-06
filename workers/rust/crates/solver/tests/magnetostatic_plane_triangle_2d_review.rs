use kyuubiki_protocol::{
    MagnetostaticPlaneNodeInput, MagnetostaticPlaneTriangleElementInput,
    SolveMagnetostaticPlaneTriangle2dRequest,
};
use kyuubiki_solver::solve_magnetostatic_plane_triangle_2d;

const TOL: f64 = 1.0e-10;

#[test]
fn magnetostatic_plane_triangle_2d_review_bundle_checks_source_patch_flux_and_energy() {
    let result = solve_magnetostatic_plane_triangle_2d(&SolveMagnetostaticPlaneTriangle2dRequest {
        nodes: vec![
            node("ground-left", 0.0, 0.0, true, 0.0, 0.0),
            node("ground-right", 1.0, 0.0, true, 0.0, 0.0),
            node("source-top", 0.0, 1.0, false, 0.0, 5.0),
        ],
        elements: vec![MagnetostaticPlaneTriangleElementInput {
            id: "tri".to_string(),
            node_i: 0,
            node_j: 1,
            node_k: 2,
            thickness: 0.1,
            permeability: permeability(),
        }],
    })
    .expect("review magnetostatic triangle should solve");

    let expected_vector_potential = 0.000_125_663_706_143_591_7;
    let expected_energy = 0.000_314_159_265_358_979_25;

    assert_eq!(result.nodes.len(), 3);
    assert_eq!(result.elements.len(), 1);
    assert_close(result.nodes[0].vector_potential, 0.0);
    assert_close(result.nodes[1].vector_potential, 0.0);
    assert_close(result.nodes[2].vector_potential, expected_vector_potential);
    assert_close(result.max_vector_potential, expected_vector_potential);
    assert_close(result.max_magnetic_field_strength, 100.0);
    assert_close(result.max_flux_density, expected_vector_potential);
    assert_close(result.total_stored_energy, expected_energy);

    let element = &result.elements[0];
    assert_close(element.area, 0.5);
    assert_close(element.average_vector_potential, expected_vector_potential / 3.0);
    assert_close(element.vector_potential_gradient_x, 0.0);
    assert_close(element.vector_potential_gradient_y, expected_vector_potential);
    assert_close(element.magnetic_flux_density_x, expected_vector_potential);
    assert_close(element.magnetic_flux_density_y, 0.0);
    assert_close(element.magnetic_field_strength_x, 100.0);
    assert_close(element.magnetic_field_strength_y, 0.0);
    assert_close(element.stored_energy, expected_energy);
}

fn node(
    id: &str,
    x: f64,
    y: f64,
    fix_vector_potential: bool,
    vector_potential: f64,
    current_density: f64,
) -> MagnetostaticPlaneNodeInput {
    MagnetostaticPlaneNodeInput {
        id: id.to_string(),
        x,
        y,
        fix_vector_potential,
        vector_potential,
        current_density,
    }
}

fn permeability() -> f64 {
    4.0e-7 * std::f64::consts::PI
}

fn assert_close(actual: f64, expected: f64) {
    let scale = expected.abs().max(1.0);
    assert!(
        (actual - expected).abs() <= TOL * scale,
        "expected {actual} to be close to {expected}",
    );
}
