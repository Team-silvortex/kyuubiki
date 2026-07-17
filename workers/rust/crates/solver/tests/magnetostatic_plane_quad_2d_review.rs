use kyuubiki_protocol::{
    MagnetostaticPlaneNodeInput, MagnetostaticPlaneQuadElementInput,
    SolveMagnetostaticPlaneQuad2dRequest,
};
use kyuubiki_solver::solve_magnetostatic_plane_quad_2d;

const TOL: f64 = 1.0e-10;

#[test]
fn magnetostatic_plane_quad_2d_review_bundle_checks_source_patch_flux_and_energy() {
    let result = solve_magnetostatic_plane_quad_2d(&SolveMagnetostaticPlaneQuad2dRequest {
        nodes: vec![
            node("ground-left", 0.0, 0.0, true, 0.0, 0.0),
            node("ground-right", 1.0, 0.0, true, 0.0, 0.0),
            node("source-right", 1.0, 1.0, false, 0.0, 5.0),
            node("source-left", 0.0, 1.0, false, 0.0, 5.0),
        ],
        elements: vec![MagnetostaticPlaneQuadElementInput {
            id: "quad".to_string(),
            node_i: 0,
            node_j: 1,
            node_k: 2,
            node_l: 3,
            thickness: 0.1,
            permeability: permeability(),
        }],
    })
    .expect("review magnetostatic quad should solve");

    let expected_vector_potential = 0.000_125_663_706_143_591_7;
    let expected_energy_density = 0.006_283_185_307_179_585;
    let expected_energy = 0.000_628_318_530_717_958_5;

    assert_eq!(result.nodes.len(), 4);
    assert_eq!(result.elements.len(), 1);
    assert_node_passthrough(&result.input.nodes, &result.nodes);
    assert_close(result.nodes[0].vector_potential, 0.0);
    assert_close(result.nodes[1].vector_potential, 0.0);
    assert_close(result.nodes[2].vector_potential, expected_vector_potential);
    assert_close(result.nodes[3].vector_potential, expected_vector_potential);
    assert_close(result.max_vector_potential, expected_vector_potential);
    assert_close(result.max_magnetic_field_strength, 100.0);
    assert_close(result.max_flux_density, expected_vector_potential);
    assert_close(result.max_magnetic_energy_density, expected_energy_density);
    assert_close(result.total_stored_energy, expected_energy);

    let element = &result.elements[0];
    assert_close(
        element.area,
        quad_area(
            &result.nodes[element.node_i],
            &result.nodes[element.node_j],
            &result.nodes[element.node_k],
            &result.nodes[element.node_l],
        ),
    );
    assert_close(
        element.average_vector_potential,
        expected_vector_potential / 2.0,
    );
    assert_close(element.vector_potential_gradient_x, 0.0);
    assert_close(
        element.vector_potential_gradient_y,
        expected_vector_potential,
    );
    assert_close(element.magnetic_flux_density_x, expected_vector_potential);
    assert_close(element.magnetic_flux_density_y, 0.0);
    assert_close(element.magnetic_field_strength_x, 100.0);
    assert_close(element.magnetic_field_strength_y, 0.0);
    assert_close(
        element.magnetic_flux_density_magnitude,
        magnitude(
            element.magnetic_flux_density_x,
            element.magnetic_flux_density_y,
        ),
    );
    assert_close(
        element.magnetic_field_strength_magnitude,
        magnitude(
            element.magnetic_field_strength_x,
            element.magnetic_field_strength_y,
        ),
    );
    assert_close(element.magnetic_energy_density, expected_energy_density);
    assert_close(
        element.stored_energy,
        element.magnetic_energy_density * element.area * result.input.elements[0].thickness,
    );
}

fn quad_area(
    a: &kyuubiki_protocol::MagnetostaticPlaneNodeResult,
    b: &kyuubiki_protocol::MagnetostaticPlaneNodeResult,
    c: &kyuubiki_protocol::MagnetostaticPlaneNodeResult,
    d: &kyuubiki_protocol::MagnetostaticPlaneNodeResult,
) -> f64 {
    triangle_area(a, b, c) + triangle_area(a, c, d)
}

fn triangle_area(
    a: &kyuubiki_protocol::MagnetostaticPlaneNodeResult,
    b: &kyuubiki_protocol::MagnetostaticPlaneNodeResult,
    c: &kyuubiki_protocol::MagnetostaticPlaneNodeResult,
) -> f64 {
    0.5 * ((b.x - a.x) * (c.y - a.y) - (c.x - a.x) * (b.y - a.y)).abs()
}

fn magnitude(x: f64, y: f64) -> f64 {
    (x * x + y * y).sqrt()
}

fn assert_node_passthrough(
    inputs: &[MagnetostaticPlaneNodeInput],
    nodes: &[kyuubiki_protocol::MagnetostaticPlaneNodeResult],
) {
    for node in nodes {
        let input = &inputs[node.index];
        assert_eq!(node.id, input.id);
        assert_close(node.x, input.x);
        assert_close(node.y, input.y);
        assert_close(node.current_density, input.current_density);
    }
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
