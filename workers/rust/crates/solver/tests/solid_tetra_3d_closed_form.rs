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

#[test]
fn solid_tetra_3d_tracks_load_and_modulus_scaling() {
    let youngs_modulus = 68.0e9;
    let poisson_ratio = 0.31;
    let load_z = -850.0;
    let baseline = solve_solid_tetra_3d(&restrained_tip_load_request(
        youngs_modulus,
        poisson_ratio,
        load_z,
    ))
    .expect("baseline solid tetra fixture should solve");

    let load_scale = 1.5;
    let load_scaled = solve_solid_tetra_3d(&restrained_tip_load_request(
        youngs_modulus,
        poisson_ratio,
        load_z * load_scale,
    ))
    .expect("load-scaled solid tetra fixture should solve");
    assert_close(load_scaled.nodes[3].uz / baseline.nodes[3].uz, load_scale);
    assert_close(
        load_scaled.elements[0].stress_z / baseline.elements[0].stress_z,
        load_scale,
    );
    assert_close(
        load_scaled.max_von_mises_stress / baseline.max_von_mises_stress,
        load_scale,
    );
    assert_close(
        load_scaled.total_strain_energy / baseline.total_strain_energy,
        load_scale * load_scale,
    );

    let modulus_scale = 1.7;
    let stiffer = solve_solid_tetra_3d(&restrained_tip_load_request(
        youngs_modulus * modulus_scale,
        poisson_ratio,
        load_z,
    ))
    .expect("modulus-scaled solid tetra fixture should solve");
    assert_close(
        stiffer.nodes[3].uz / baseline.nodes[3].uz,
        1.0 / modulus_scale,
    );
    assert_close(stiffer.elements[0].stress_z, baseline.elements[0].stress_z);
    assert_close(stiffer.max_von_mises_stress, baseline.max_von_mises_stress);
    assert_close(
        stiffer.total_strain_energy / baseline.total_strain_energy,
        1.0 / modulus_scale,
    );

    let height_scale = 1.4;
    let taller = solve_solid_tetra_3d(&restrained_tip_load_request_with_height(
        youngs_modulus,
        poisson_ratio,
        load_z,
        height_scale,
    ))
    .expect("height-scaled solid tetra fixture should solve");
    assert_close(taller.total_volume / baseline.total_volume, height_scale);
    assert_close(taller.nodes[3].uz / baseline.nodes[3].uz, height_scale);
    assert_close(taller.elements[0].strain_z, baseline.elements[0].strain_z);
    assert_close(taller.elements[0].stress_z, baseline.elements[0].stress_z);
    assert_close(taller.max_von_mises_stress, baseline.max_von_mises_stress);
    assert_close(
        taller.max_strain_energy_density,
        baseline.max_strain_energy_density,
    );
    assert_close(
        taller.total_strain_energy / baseline.total_strain_energy,
        height_scale,
    );
}

fn restrained_tip_load_request(
    youngs_modulus: f64,
    poisson_ratio: f64,
    load_z: f64,
) -> SolveSolidTetra3dRequest {
    restrained_tip_load_request_with_height(youngs_modulus, poisson_ratio, load_z, 1.0)
}

fn restrained_tip_load_request_with_height(
    youngs_modulus: f64,
    poisson_ratio: f64,
    load_z: f64,
    height: f64,
) -> SolveSolidTetra3dRequest {
    SolveSolidTetra3dRequest {
        nodes: vec![
            node("n0", 0.0, 0.0, 0.0, true, 0.0),
            node("n1", 1.0, 0.0, 0.0, true, 0.0),
            node("n2", 0.0, 1.0, 0.0, true, 0.0),
            node("n3", 0.0, 0.0, height, false, load_z),
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
