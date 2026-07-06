use kyuubiki_protocol::{
    MagnetostaticBar1dElementInput, MagnetostaticBar1dNodeInput, SolveMagnetostaticBar1dRequest,
};
use kyuubiki_solver::solve_magnetostatic_bar_1d;

const TOL: f64 = 1.0e-10;

#[test]
fn magnetostatic_bar_1d_review_bundle_checks_permeance_field_flux_and_energy() {
    let length = 2.0;
    let area = 0.25;
    let permeability = 4.0e-7 * std::f64::consts::PI;
    let source = 3.0e-6;

    let result = solve_magnetostatic_bar_1d(&SolveMagnetostaticBar1dRequest {
        nodes: vec![
            node("ground", 0.0, true, 0.0, 0.0),
            node("source", length, false, 0.0, source),
        ],
        elements: vec![MagnetostaticBar1dElementInput {
            id: "core".to_string(),
            node_i: 0,
            node_j: 1,
            area,
            permeability,
        }],
    })
    .expect("review magnetostatic bar should solve");

    let permeance = permeability * area / length;
    let expected_potential = source / permeance;
    let expected_gradient = expected_potential / length;
    let expected_field = -expected_gradient;
    let expected_flux_density = permeability * expected_field;
    let expected_energy = 0.5 * permeability * expected_field * expected_field * area * length;

    assert_eq!(result.nodes.len(), 2);
    assert_eq!(result.elements.len(), 1);
    assert_close(result.nodes[0].magnetic_potential, 0.0);
    assert_close(result.nodes[1].magnetic_potential, expected_potential);
    assert_close(result.max_magnetic_potential, expected_potential.abs());
    assert_close(result.max_magnetic_field_strength, expected_field.abs());
    assert_close(result.max_flux_density, expected_flux_density.abs());
    assert_close(result.total_stored_energy, expected_energy);

    let element = &result.elements[0];
    assert_close(element.length, length);
    assert_close(element.average_magnetic_potential, expected_potential / 2.0);
    assert_close(element.magnetic_potential_gradient, expected_gradient);
    assert_close(element.magnetic_field_strength, expected_field);
    assert_close(element.magnetic_flux_density, expected_flux_density);
    assert_close(element.stored_energy, expected_energy);
}

fn node(
    id: &str,
    x: f64,
    fix_magnetic_potential: bool,
    magnetic_potential: f64,
    magnetomotive_source: f64,
) -> MagnetostaticBar1dNodeInput {
    MagnetostaticBar1dNodeInput {
        id: id.to_string(),
        x,
        fix_magnetic_potential,
        magnetic_potential,
        magnetomotive_source,
    }
}

fn assert_close(actual: f64, expected: f64) {
    let scale = expected.abs().max(1.0);
    assert!(
        (actual - expected).abs() <= TOL * scale,
        "expected {actual} to be close to {expected}",
    );
}
