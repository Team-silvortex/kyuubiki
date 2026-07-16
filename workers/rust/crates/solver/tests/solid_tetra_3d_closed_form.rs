use kyuubiki_protocol::{
    SolidTetra3dElementInput, SolidTetra3dNodeInput, SolveSolidTetra3dRequest,
};
use kyuubiki_solver::solve_solid_tetra_3d;

const TOL: f64 = 1.0e-10;

#[test]
fn solid_tetra_3d_closed_form_matches_single_free_node_solution() {
    let youngs_modulus = 70.0e9;
    let poisson_ratio = 0.33;
    let load_z = -1000.0;
    let result = solve_solid_tetra_3d(&restrained_tip_load_request(
        youngs_modulus,
        poisson_ratio,
        load_z,
    ))
    .expect("closed-form solid tetra fixture should solve");

    let volume = 1.0 / 6.0;
    let dzz = youngs_modulus * (1.0 - poisson_ratio)
        / ((1.0 + poisson_ratio) * (1.0 - 2.0 * poisson_ratio));
    let lambda =
        youngs_modulus * poisson_ratio / ((1.0 + poisson_ratio) * (1.0 - 2.0 * poisson_ratio));
    let shear_modulus = youngs_modulus / (2.0 * (1.0 + poisson_ratio));
    let expected_uz = load_z / (dzz * volume);
    let expected_lateral_stress = lambda * expected_uz;
    let expected_axial_stress = load_z / volume;
    let expected_von_mises = (expected_axial_stress - expected_lateral_stress).abs();
    let expected_energy_density = 0.5 * expected_axial_stress * expected_uz;
    let expected_total_energy = expected_energy_density * volume;

    for index in 0..3 {
        assert_close(result.nodes[index].ux, 0.0);
        assert_close(result.nodes[index].uy, 0.0);
        assert_close(result.nodes[index].uz, 0.0);
    }

    let tip = &result.nodes[3];
    assert_close(tip.ux, 0.0);
    assert_close(tip.uy, 0.0);
    assert_close(tip.uz, expected_uz);
    assert_close(tip.displacement_magnitude, expected_uz.abs());
    assert_close(result.max_displacement, expected_uz.abs());

    let element = &result.elements[0];
    assert_close(element.volume, volume);
    assert_close(result.total_volume, volume);
    assert_close(element.strain_x, 0.0);
    assert_close(element.strain_y, 0.0);
    assert_close(element.strain_z, expected_uz);
    assert_close(element.gamma_xy, 0.0);
    assert_close(element.gamma_yz, 0.0);
    assert_close(element.gamma_zx, 0.0);
    assert_close(element.stress_x, expected_lateral_stress);
    assert_close(element.stress_y, expected_lateral_stress);
    assert_close(element.stress_z, expected_axial_stress);
    assert_close(element.shear_xy, 0.0);
    assert_close(element.shear_yz, 0.0);
    assert_close(element.shear_zx, 0.0);
    assert_close(element.von_mises_stress, expected_von_mises);
    assert_close(
        element.von_mises_stress,
        (2.0 * shear_modulus * expected_uz).abs(),
    );
    assert_close(element.strain_energy_density, expected_energy_density);
    assert_close(result.max_von_mises_stress, expected_von_mises);
    assert_close(result.max_strain_energy_density, expected_energy_density);
    assert_close(result.total_strain_energy, expected_total_energy);
}

fn restrained_tip_load_request(
    youngs_modulus: f64,
    poisson_ratio: f64,
    load_z: f64,
) -> SolveSolidTetra3dRequest {
    SolveSolidTetra3dRequest {
        nodes: vec![
            node("n0", 0.0, 0.0, 0.0, true, 0.0),
            node("n1", 1.0, 0.0, 0.0, true, 0.0),
            node("n2", 0.0, 1.0, 0.0, true, 0.0),
            node("n3", 0.0, 0.0, 1.0, false, load_z),
        ],
        elements: vec![SolidTetra3dElementInput {
            id: "t0".to_string(),
            node_a: 0,
            node_b: 1,
            node_c: 2,
            node_d: 3,
            youngs_modulus,
            poisson_ratio,
        }],
    }
}

fn node(id: &str, x: f64, y: f64, z: f64, fixed: bool, load_z: f64) -> SolidTetra3dNodeInput {
    SolidTetra3dNodeInput {
        id: id.to_string(),
        x,
        y,
        z,
        fix_x: fixed,
        fix_y: fixed,
        fix_z: fixed,
        load_x: 0.0,
        load_y: 0.0,
        load_z,
    }
}

fn assert_close(actual: f64, expected: f64) {
    let scale = expected.abs().max(1.0);
    assert!(
        (actual - expected).abs() <= TOL * scale,
        "expected {actual} to be close to {expected}",
    );
}
