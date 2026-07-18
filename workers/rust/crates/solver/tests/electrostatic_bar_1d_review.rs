use kyuubiki_protocol::{
    ElectrostaticBar1dElementInput, ElectrostaticBar1dNodeInput, SolveElectrostaticBar1dRequest,
};
use kyuubiki_solver::solve_electrostatic_bar_1d;

const TOL: f64 = 1.0e-10;

#[test]
fn electrostatic_bar_1d_review_bundle_checks_dirichlet_gradient_flux_and_energy() {
    let result = solve_electrostatic_bar_1d(&SolveElectrostaticBar1dRequest {
        nodes: vec![
            node("left", 0.0, true, 12.0),
            node("mid", 1.0, false, 0.0),
            node("right", 2.0, true, 4.0),
        ],
        elements: vec![
            element("left-span", 0, 1, 0.01, 3.0),
            element("right-span", 1, 2, 0.01, 3.0),
        ],
    })
    .expect("review electrostatic bar should solve");

    assert_eq!(result.nodes.len(), 3);
    assert_eq!(result.elements.len(), 2);
    assert_close(result.nodes[0].potential, 12.0);
    assert_close(result.nodes[1].potential, 8.0);
    assert_close(result.nodes[2].potential, 4.0);
    assert_close(result.max_potential, 12.0);
    assert_close(result.max_electric_field, 4.0);
    assert_close(result.max_flux_density, 12.0);
    assert_close(result.total_stored_energy, 0.48);

    for (index, element) in result.elements.iter().enumerate() {
        assert_eq!(element.index, index);
        assert_close(element.length, 1.0);
        assert_close(element.potential_gradient, -4.0);
        assert_close(element.electric_field, 4.0);
        assert_close(element.electric_flux_density, 12.0);
        assert_close(element.stored_energy, 0.24);
    }
}

fn node(id: &str, x: f64, fix_potential: bool, potential: f64) -> ElectrostaticBar1dNodeInput {
    ElectrostaticBar1dNodeInput {
        id: id.to_string(),
        x,
        fix_potential,
        potential,
        charge_density: 0.0,
    }
}

fn element(
    id: &str,
    node_i: usize,
    node_j: usize,
    area: f64,
    permittivity: f64,
) -> ElectrostaticBar1dElementInput {
    ElectrostaticBar1dElementInput {
        id: id.to_string(),
        node_i,
        node_j,
        area,
        permittivity,
    }
}

fn assert_close(actual: f64, expected: f64) {
    let scale = expected.abs().max(1.0);
    assert!(
        (actual - expected).abs() <= TOL * scale,
        "expected {actual} to be close to {expected}",
    );
}
