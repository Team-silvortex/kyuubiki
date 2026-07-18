use kyuubiki_protocol::{
    SolveThermalTruss3dRequest, ThermalTruss3dElementInput, ThermalTruss3dNodeInput,
};
use kyuubiki_solver::solve_thermal_truss_3d;

const TOL: f64 = 1.0e-10;

#[test]
fn thermal_truss_3d_review_bundle_checks_restrained_uniform_temperature_response() {
    let area = 0.01;
    let youngs_modulus = 210.0e9;
    let thermal_expansion = 12.0e-6;
    let temperature_delta = 40.0;
    let result = solve_thermal_truss_3d(&SolveThermalTruss3dRequest {
        nodes: vec![
            node("n0", 0.0, 0.0, 0.0, temperature_delta),
            node("n1", 1.0, 0.0, 0.0, temperature_delta),
            node("n2", 0.0, 1.0, 0.0, temperature_delta),
        ],
        elements: vec![
            element("edge_01", 0, 1, area, youngs_modulus, thermal_expansion),
            element("edge_12", 1, 2, area, youngs_modulus, thermal_expansion),
            element("edge_20", 2, 0, area, youngs_modulus, thermal_expansion),
        ],
    })
    .expect("review 3d thermal truss should solve");

    let expected_thermal_strain = thermal_expansion * temperature_delta;
    let expected_stress = -youngs_modulus * expected_thermal_strain;
    let expected_axial_force = expected_stress * area;
    let expected_energy_density = 0.5 * expected_stress * -expected_thermal_strain;

    assert_eq!(result.nodes.len(), 3);
    assert_eq!(result.elements.len(), 3);
    for (index, node) in result.nodes.iter().enumerate() {
        assert_eq!(node.index, index);
        assert_close(node.ux, 0.0);
        assert_close(node.uy, 0.0);
        assert_close(node.uz, 0.0);
        assert_close(node.temperature_delta, temperature_delta);
    }

    assert_close(result.elements[0].length, 1.0);
    assert_close(result.elements[1].length, 2.0_f64.sqrt());
    assert_close(result.elements[2].length, 1.0);
    for (index, element) in result.elements.iter().enumerate() {
        assert_eq!(element.index, index);
        assert_close(element.average_temperature_delta, temperature_delta);
        assert_close(element.thermal_strain, expected_thermal_strain);
        assert_close(element.mechanical_strain, -expected_thermal_strain);
        assert_close(element.total_strain, 0.0);
        assert_close(element.stress, expected_stress);
        assert_close(element.axial_force, expected_axial_force);
        assert_close(element.strain_energy_density, expected_energy_density);
    }

    assert_close(result.max_displacement, 0.0);
    assert_close(result.max_stress, expected_stress.abs());
    assert_close(result.max_axial_force, expected_axial_force.abs());
    assert_close(result.max_temperature_delta, temperature_delta);
    assert_close(result.max_strain_energy_density, expected_energy_density);
    assert_close(
        result.total_strain_energy,
        total_strain_energy(&result.elements, area),
    );
}

fn node(id: &str, x: f64, y: f64, z: f64, temperature_delta: f64) -> ThermalTruss3dNodeInput {
    ThermalTruss3dNodeInput {
        id: id.to_string(),
        x,
        y,
        z,
        fix_x: true,
        fix_y: true,
        fix_z: true,
        load_x: 0.0,
        load_y: 0.0,
        load_z: 0.0,
        temperature_delta,
    }
}

fn element(
    id: &str,
    node_i: usize,
    node_j: usize,
    area: f64,
    youngs_modulus: f64,
    thermal_expansion: f64,
) -> ThermalTruss3dElementInput {
    ThermalTruss3dElementInput {
        id: id.to_string(),
        node_i,
        node_j,
        area,
        youngs_modulus,
        thermal_expansion,
    }
}

fn assert_close(actual: f64, expected: f64) {
    let scale = expected.abs().max(1.0);
    assert!(
        (actual - expected).abs() <= TOL * scale,
        "expected {actual} to be close to {expected}",
    );
}

fn total_strain_energy(
    elements: &[kyuubiki_protocol::ThermalTruss3dElementResult],
    area: f64,
) -> f64 {
    elements
        .iter()
        .map(|element| element.strain_energy_density * area * element.length)
        .sum()
}
