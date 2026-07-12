use kyuubiki_protocol::{
    MagnetostaticBar1dElementInput, MagnetostaticBar1dNodeInput, SolveMagnetostaticBar1dRequest,
};
use kyuubiki_solver::solve_magnetostatic_bar_1d;

const TOL: f64 = 1.0e-10;

#[test]
fn magnetostatic_bar_1d_matches_single_element_permeance_baseline() {
    let length = 2.0;
    let area = 0.25;
    let permeability = 4.0e-7 * std::f64::consts::PI;
    let source = 3.0e-6;
    let fixed_potential = 0.0;

    let result = solve_magnetostatic_bar_1d(&SolveMagnetostaticBar1dRequest {
        nodes: vec![
            MagnetostaticBar1dNodeInput {
                id: "ground".to_string(),
                x: 0.0,
                fix_magnetic_potential: true,
                magnetic_potential: fixed_potential,
                magnetomotive_source: 0.0,
            },
            MagnetostaticBar1dNodeInput {
                id: "source".to_string(),
                x: length,
                fix_magnetic_potential: false,
                magnetic_potential: 0.0,
                magnetomotive_source: source,
            },
        ],
        elements: vec![MagnetostaticBar1dElementInput {
            id: "core".to_string(),
            node_i: 0,
            node_j: 1,
            area,
            permeability,
        }],
    })
    .expect("magnetostatic baseline should solve");

    let permeance = permeability * area / length;
    let expected_potential = source / permeance;
    let expected_gradient = (expected_potential - fixed_potential) / length;
    let expected_field = -expected_gradient;
    let expected_flux_density = permeability * expected_field;
    let expected_energy = 0.5 * permeability * expected_field * expected_field * area * length;

    assert_close(result.nodes[0].magnetic_potential, fixed_potential);
    assert_close(result.nodes[1].magnetic_potential, expected_potential);
    assert_close(result.max_magnetic_potential, expected_potential.abs());
    assert_close(
        result.elements[0].average_magnetic_potential,
        expected_potential / 2.0,
    );
    assert_close(
        result.elements[0].magnetic_potential_gradient,
        expected_gradient,
    );
    assert_close(result.elements[0].magnetic_field_strength, expected_field);
    assert_close(
        result.elements[0].magnetic_flux_density,
        expected_flux_density,
    );
    assert_close(result.elements[0].stored_energy, expected_energy);
    assert_close(result.total_stored_energy, expected_energy);
    assert_close(result.max_magnetic_field_strength, expected_field.abs());
    assert_close(result.max_flux_density, expected_flux_density.abs());
}

#[test]
fn magnetostatic_bar_1d_rejects_non_finite_node_values_and_missing_support() {
    let mut request = magnetostatic_request();
    request.nodes[1].magnetomotive_source = f64::INFINITY;
    let error = solve_magnetostatic_bar_1d(&request)
        .expect_err("infinite magnetomotive source should be rejected");
    assert!(
        error.contains("magnetomotive_source must be finite"),
        "unexpected source validation error: {error}"
    );

    let mut request = magnetostatic_request();
    for node in &mut request.nodes {
        node.fix_magnetic_potential = false;
    }
    let error =
        solve_magnetostatic_bar_1d(&request).expect_err("missing support should be rejected");
    assert!(
        error.contains("at least one magnetic potential support"),
        "unexpected missing-support error: {error}"
    );
}

#[test]
fn magnetostatic_bar_1d_rejects_invalid_element_topology_and_materials() {
    let mut request = magnetostatic_request();
    request.elements[0].node_j = 7;
    let error =
        solve_magnetostatic_bar_1d(&request).expect_err("out-of-range node should be rejected");
    assert!(
        error.contains("out-of-range node"),
        "unexpected out-of-range node error: {error}"
    );

    let mut request = magnetostatic_request();
    request.nodes[1].x = request.nodes[0].x;
    let error =
        solve_magnetostatic_bar_1d(&request).expect_err("zero-length element should be rejected");
    assert!(
        error.contains("length must be positive"),
        "unexpected length error: {error}"
    );

    let mut request = magnetostatic_request();
    request.elements[0].permeability = 0.0;
    let error =
        solve_magnetostatic_bar_1d(&request).expect_err("zero permeability should be rejected");
    assert!(
        error.contains("permeability must be positive"),
        "unexpected permeability error: {error}"
    );
}

fn magnetostatic_request() -> SolveMagnetostaticBar1dRequest {
    SolveMagnetostaticBar1dRequest {
        nodes: vec![
            MagnetostaticBar1dNodeInput {
                id: "ground".to_string(),
                x: 0.0,
                fix_magnetic_potential: true,
                magnetic_potential: 0.0,
                magnetomotive_source: 0.0,
            },
            MagnetostaticBar1dNodeInput {
                id: "source".to_string(),
                x: 2.0,
                fix_magnetic_potential: false,
                magnetic_potential: 0.0,
                magnetomotive_source: 3.0e-6,
            },
        ],
        elements: vec![MagnetostaticBar1dElementInput {
            id: "core".to_string(),
            node_i: 0,
            node_j: 1,
            area: 0.25,
            permeability: 4.0e-7 * std::f64::consts::PI,
        }],
    }
}

fn assert_close(actual: f64, expected: f64) {
    let scale = expected.abs().max(1.0);
    assert!(
        (actual - expected).abs() <= TOL * scale,
        "expected {actual} to be close to {expected}",
    );
}
