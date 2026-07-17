use kyuubiki_protocol::{
    ElectrostaticPlaneNodeInput, ElectrostaticPlaneTriangleElementInput,
    SolveElectrostaticPlaneTriangle2dRequest,
};
use kyuubiki_solver::solve_electrostatic_plane_triangle_2d;

const TOL: f64 = 1.0e-10;

#[test]
fn electrostatic_plane_triangle_2d_review_bundle_checks_patch_gradient_flux_and_energy() {
    let result = solve_electrostatic_plane_triangle_2d(&SolveElectrostaticPlaneTriangle2dRequest {
        nodes: vec![
            node("left-bottom", 0.0, 0.0, 12.0),
            node("right-bottom", 1.0, 0.0, 4.0),
            node("left-top", 0.0, 1.0, 12.0),
        ],
        elements: vec![ElectrostaticPlaneTriangleElementInput {
            id: "tri".to_string(),
            node_i: 0,
            node_j: 1,
            node_k: 2,
            thickness: 0.1,
            permittivity: 3.0,
        }],
    })
    .expect("review electrostatic triangle should solve");

    assert_eq!(result.nodes.len(), 3);
    assert_eq!(result.elements.len(), 1);
    assert_node_passthrough(&result.input.nodes, &result.nodes);
    assert_close(result.max_potential, 12.0);
    assert_close(result.max_electric_field, 8.0);
    assert_close(result.max_flux_density, 24.0);
    assert_close(result.max_electric_energy_density, 96.0);
    assert_close(result.total_stored_energy, 4.8);

    let element = &result.elements[0];
    assert_close(
        element.area,
        triangle_area(
            &result.nodes[element.node_i],
            &result.nodes[element.node_j],
            &result.nodes[element.node_k],
        ),
    );
    assert_close(element.average_potential, 28.0 / 3.0);
    assert_close(element.potential_gradient_x, -8.0);
    assert_close(element.potential_gradient_y, 0.0);
    assert_close(element.electric_field_x, 8.0);
    assert_close(element.electric_field_y, 0.0);
    assert_close(element.electric_flux_density_x, 24.0);
    assert_close(element.electric_flux_density_y, 0.0);
    assert_close(
        element.electric_field_magnitude,
        magnitude(element.electric_field_x, element.electric_field_y),
    );
    assert_close(
        element.electric_flux_density_magnitude,
        magnitude(
            element.electric_flux_density_x,
            element.electric_flux_density_y,
        ),
    );
    assert_close(element.electric_energy_density, 96.0);
    assert_close(
        element.stored_energy,
        element.electric_energy_density * element.area * result.input.elements[0].thickness,
    );
}

fn triangle_area(
    a: &kyuubiki_protocol::ElectrostaticPlaneNodeResult,
    b: &kyuubiki_protocol::ElectrostaticPlaneNodeResult,
    c: &kyuubiki_protocol::ElectrostaticPlaneNodeResult,
) -> f64 {
    0.5 * ((b.x - a.x) * (c.y - a.y) - (c.x - a.x) * (b.y - a.y)).abs()
}

fn magnitude(x: f64, y: f64) -> f64 {
    (x * x + y * y).sqrt()
}

fn assert_node_passthrough(
    inputs: &[ElectrostaticPlaneNodeInput],
    nodes: &[kyuubiki_protocol::ElectrostaticPlaneNodeResult],
) {
    for node in nodes {
        let input = &inputs[node.index];
        assert_eq!(node.id, input.id);
        assert_close(node.x, input.x);
        assert_close(node.y, input.y);
        assert_close(node.potential, input.potential);
        assert_close(node.charge_density, input.charge_density);
    }
}

fn node(id: &str, x: f64, y: f64, potential: f64) -> ElectrostaticPlaneNodeInput {
    ElectrostaticPlaneNodeInput {
        id: id.to_string(),
        x,
        y,
        fix_potential: true,
        potential,
        charge_density: 0.0,
    }
}

fn assert_close(actual: f64, expected: f64) {
    let scale = expected.abs().max(1.0);
    assert!(
        (actual - expected).abs() <= TOL * scale,
        "expected {actual} to be close to {expected}",
    );
}
