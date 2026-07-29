use kyuubiki_protocol::{
    MagnetostaticBar1dElementInput, MagnetostaticBar1dNodeInput, SolveMagnetostaticBar1dRequest,
};
use kyuubiki_solver::solve_magnetostatic_bar_1d;

const TOL: f64 = 1.0e-10;
const LENGTH: f64 = 2.5;
const LEFT_POTENTIAL: f64 = 0.0;
const RIGHT_POTENTIAL: f64 = 4.0e-4;
const AREA: f64 = 0.12;
const PERMEABILITY: f64 = 4.0e-7 * std::f64::consts::PI;

#[test]
fn fixed_magnetic_potential_linear_field_is_refinement_invariant() {
    let gradient = (RIGHT_POTENTIAL - LEFT_POTENTIAL) / LENGTH;
    let field_strength = -gradient;
    let flux_density = PERMEABILITY * field_strength;
    let stored_energy = 0.5 * PERMEABILITY * field_strength * field_strength * AREA * LENGTH;

    for elements in [1_usize, 2, 4, 8, 16] {
        let result = solve_magnetostatic_bar_1d(&mesh(elements))
            .expect("refined magnetostatic field should solve");
        assert_eq!(result.elements.len(), elements);
        assert_close(result.max_magnetic_potential, RIGHT_POTENTIAL.abs());
        assert_close(result.max_magnetic_field_strength, field_strength.abs());
        assert_close(result.max_flux_density, flux_density.abs());
        assert_close(result.total_stored_energy, stored_energy);
        for node in &result.nodes {
            assert_close(node.magnetic_potential, potential_at(node.x));
        }
        for element in &result.elements {
            assert_close(element.magnetic_potential_gradient, gradient);
            assert_close(element.magnetic_field_strength, field_strength);
            assert_close(element.magnetic_flux_density, flux_density);
        }
    }
}

fn mesh(count: usize) -> SolveMagnetostaticBar1dRequest {
    let nodes = (0..=count)
        .map(|index| {
            let x = LENGTH * index as f64 / count as f64;
            MagnetostaticBar1dNodeInput {
                id: format!("node-{index}"),
                x,
                fix_magnetic_potential: true,
                magnetic_potential: potential_at(x),
                magnetomotive_source: 0.0,
            }
        })
        .collect();
    let elements = (0..count)
        .map(|index| MagnetostaticBar1dElementInput {
            id: format!("element-{index}"),
            node_i: index,
            node_j: index + 1,
            area: AREA,
            permeability: PERMEABILITY,
        })
        .collect();
    SolveMagnetostaticBar1dRequest { nodes, elements }
}

fn potential_at(x: f64) -> f64 {
    LEFT_POTENTIAL + (RIGHT_POTENTIAL - LEFT_POTENTIAL) * x / LENGTH
}

fn assert_close(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() <= TOL * expected.abs().max(1.0),
        "expected {actual} to be close to {expected}",
    );
}
