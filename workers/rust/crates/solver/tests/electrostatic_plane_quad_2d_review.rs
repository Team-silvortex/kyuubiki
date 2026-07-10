use kyuubiki_protocol::{
    ElectrostaticPlaneNodeInput, ElectrostaticPlaneQuadElementInput,
    SolveElectrostaticPlaneQuad2dRequest,
};
use kyuubiki_solver::solve_electrostatic_plane_quad_2d;

const TOL: f64 = 1.0e-10;

#[test]
fn electrostatic_plane_quad_2d_review_bundle_checks_patch_gradient_flux_and_energy() {
    let result = solve_electrostatic_plane_quad_2d(&SolveElectrostaticPlaneQuad2dRequest {
        nodes: vec![
            node("left-bottom", 0.0, 0.0, 12.0),
            node("right-bottom", 1.0, 0.0, 4.0),
            node("right-top", 1.0, 1.0, 4.0),
            node("left-top", 0.0, 1.0, 12.0),
        ],
        elements: vec![ElectrostaticPlaneQuadElementInput {
            id: "quad".to_string(),
            node_i: 0,
            node_j: 1,
            node_k: 2,
            node_l: 3,
            thickness: 0.1,
            permittivity: 3.0,
        }],
    })
    .expect("review electrostatic quad should solve");

    assert_eq!(result.nodes.len(), 4);
    assert_eq!(result.elements.len(), 1);
    assert_close(result.max_potential, 12.0);
    assert_close(result.max_electric_field, 8.0);
    assert_close(result.max_flux_density, 24.0);
    assert_close(result.max_electric_energy_density, 96.0);
    assert_close(result.total_stored_energy, 9.6);

    let element = &result.elements[0];
    assert_close(element.area, 1.0);
    assert_close(element.average_potential, 8.0);
    assert_close(element.potential_gradient_x, -8.0);
    assert_close(element.potential_gradient_y, 0.0);
    assert_close(element.electric_field_x, 8.0);
    assert_close(element.electric_field_y, 0.0);
    assert_close(element.electric_flux_density_x, 24.0);
    assert_close(element.electric_flux_density_y, 0.0);
    assert_close(element.electric_energy_density, 96.0);
    assert_close(element.stored_energy, 9.6);
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
