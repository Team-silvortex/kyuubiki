use kyuubiki_protocol::{
    SolidTetra3dElementInput, SolidTetra3dElementResult, SolidTetra3dNodeInput,
    SolveSolidTetra3dRequest, SolveSolidTetra3dResult,
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
    assert_tip_force_energy(result.total_strain_energy, load_z, tip.uz);
    assert_solid_summary(&result);
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
    assert_tip_force_energy(baseline.total_strain_energy, load_z, baseline.nodes[3].uz);
    assert_solid_summary(&baseline);

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
    assert_tip_force_energy(
        load_scaled.total_strain_energy,
        load_z * load_scale,
        load_scaled.nodes[3].uz,
    );
    assert_solid_summary(&load_scaled);

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
    assert_tip_force_energy(stiffer.total_strain_energy, load_z, stiffer.nodes[3].uz);
    assert_solid_summary(&stiffer);

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
    assert_tip_force_energy(taller.total_strain_energy, load_z, taller.nodes[3].uz);
    assert_solid_summary(&taller);

    let base_scale = 1.6;
    let wider_base = solve_solid_tetra_3d(&restrained_tip_load_request_with_geometry(
        youngs_modulus,
        poisson_ratio,
        load_z,
        base_scale,
        1.0,
    ))
    .expect("base-area-scaled solid tetra fixture should solve");
    let inverse_base_area_scale = 1.0 / base_scale.powi(2);
    assert_close(
        wider_base.total_volume / baseline.total_volume,
        base_scale.powi(2),
    );
    assert_close(
        wider_base.nodes[3].uz / baseline.nodes[3].uz,
        inverse_base_area_scale,
    );
    assert_close(
        wider_base.elements[0].strain_z / baseline.elements[0].strain_z,
        inverse_base_area_scale,
    );
    assert_close(
        wider_base.elements[0].stress_z / baseline.elements[0].stress_z,
        inverse_base_area_scale,
    );
    assert_close(
        wider_base.max_von_mises_stress / baseline.max_von_mises_stress,
        inverse_base_area_scale,
    );
    assert_close(
        wider_base.total_strain_energy / baseline.total_strain_energy,
        inverse_base_area_scale,
    );
    assert_tip_force_energy(
        wider_base.total_strain_energy,
        load_z,
        wider_base.nodes[3].uz,
    );
    assert_solid_summary(&wider_base);
}

fn assert_solid_summary(result: &SolveSolidTetra3dResult) {
    let mut total_volume = 0.0_f64;
    let mut max_von_mises = 0.0_f64;
    let mut max_energy_density = 0.0_f64;
    let mut total_strain_energy = 0.0_f64;
    assert_eq!(result.nodes.len(), result.input.nodes.len());
    assert_eq!(result.elements.len(), result.input.elements.len());

    for (index, node) in result.nodes.iter().enumerate() {
        let input = &result.input.nodes[index];
        assert_eq!(node.index, index);
        assert_eq!(node.id, input.id);
        assert_close(node.x, input.x);
        assert_close(node.y, input.y);
        assert_close(node.z, input.z);
        assert_close(
            node.displacement_magnitude,
            magnitude3(node.ux, node.uy, node.uz),
        );
    }

    for (index, element) in result.elements.iter().enumerate() {
        assert_eq!(element.index, index);
        assert_solid_element_contract(result, element);
        total_volume += element.volume;
        max_von_mises = max_von_mises.max(element.von_mises_stress);
        max_energy_density = max_energy_density.max(element.strain_energy_density.abs());
        total_strain_energy += element.strain_energy_density * element.volume;
    }

    assert_close(
        result.max_displacement,
        result
            .nodes
            .iter()
            .map(|node| node.displacement_magnitude)
            .fold(0.0_f64, f64::max),
    );
    assert_close(result.total_volume, total_volume);
    assert_close(result.max_von_mises_stress, max_von_mises);
    assert_close(result.max_strain_energy_density, max_energy_density);
    assert_close(result.total_strain_energy, total_strain_energy);

    let external_work = result
        .input
        .nodes
        .iter()
        .zip(result.nodes.iter())
        .map(|(input, node)| {
            input.load_x * node.ux + input.load_y * node.uy + input.load_z * node.uz
        })
        .sum::<f64>();
    assert_close(result.total_strain_energy, 0.5 * external_work);
}

fn assert_solid_element_contract(
    result: &SolveSolidTetra3dResult,
    element: &SolidTetra3dElementResult,
) {
    assert_close(element.volume, tetra_volume(result, element));

    let expected_von_mises = (0.5
        * ((element.stress_x - element.stress_y).powi(2)
            + (element.stress_y - element.stress_z).powi(2)
            + (element.stress_z - element.stress_x).powi(2))
        + 3.0
            * (element.shear_xy * element.shear_xy
                + element.shear_yz * element.shear_yz
                + element.shear_zx * element.shear_zx))
        .sqrt();
    let expected_energy_density = 0.5
        * (element.stress_x * element.strain_x
            + element.stress_y * element.strain_y
            + element.stress_z * element.strain_z
            + element.shear_xy * element.gamma_xy
            + element.shear_yz * element.gamma_yz
            + element.shear_zx * element.gamma_zx);

    assert_close(element.von_mises_stress, expected_von_mises);
    assert_close(element.strain_energy_density, expected_energy_density);
}

fn tetra_volume(result: &SolveSolidTetra3dResult, element: &SolidTetra3dElementResult) -> f64 {
    let a = &result.nodes[element.node_a];
    let b = &result.nodes[element.node_b];
    let c = &result.nodes[element.node_c];
    let d = &result.nodes[element.node_d];
    let ab = [b.x - a.x, b.y - a.y, b.z - a.z];
    let ac = [c.x - a.x, c.y - a.y, c.z - a.z];
    let ad = [d.x - a.x, d.y - a.y, d.z - a.z];
    let cross = [
        ac[1] * ad[2] - ac[2] * ad[1],
        ac[2] * ad[0] - ac[0] * ad[2],
        ac[0] * ad[1] - ac[1] * ad[0],
    ];
    (ab[0] * cross[0] + ab[1] * cross[1] + ab[2] * cross[2]).abs() / 6.0
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
    restrained_tip_load_request_with_geometry(youngs_modulus, poisson_ratio, load_z, 1.0, height)
}

fn restrained_tip_load_request_with_geometry(
    youngs_modulus: f64,
    poisson_ratio: f64,
    load_z: f64,
    base_scale: f64,
    height: f64,
) -> SolveSolidTetra3dRequest {
    SolveSolidTetra3dRequest {
        nodes: vec![
            node("n0", 0.0, 0.0, 0.0, true, 0.0),
            node("n1", base_scale, 0.0, 0.0, true, 0.0),
            node("n2", 0.0, base_scale, 0.0, true, 0.0),
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

fn assert_tip_force_energy(total_strain_energy: f64, tip_load: f64, tip_displacement: f64) {
    assert_close(total_strain_energy, 0.5 * tip_load * tip_displacement);
}

fn magnitude3(x: f64, y: f64, z: f64) -> f64 {
    (x * x + y * y + z * z).sqrt()
}

fn assert_close(actual: f64, expected: f64) {
    let scale = expected.abs().max(1.0);
    assert!(
        (actual - expected).abs() <= TOL * scale,
        "expected {actual} to be close to {expected}",
    );
}
