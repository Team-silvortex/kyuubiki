use kyuubiki_protocol::{
    SolveTruss3dRequest, SolveTruss3dResult, Truss3dElementInput, Truss3dNodeInput,
};
use kyuubiki_solver::solve_truss_3d;

const TOL: f64 = 1.0e-8;

#[test]
fn truss_3d_matches_symmetric_tripod_closed_form() {
    let radius = 0.6;
    let height = 0.9;
    let load = -1500.0;
    let area = 0.012;
    let youngs_modulus = 70.0e9;
    let result = solve_truss_3d(&tripod_request(radius, height, load, area, youngs_modulus))
        .expect("symmetric tripod truss should solve");

    let length = (radius * radius + height * height).sqrt();
    let vertical_direction = height / length;
    let axial_force = load / (3.0 * vertical_direction);
    let expected_uz =
        load * length / (3.0 * youngs_modulus * area * vertical_direction * vertical_direction);
    let expected_stress = axial_force / area;
    let expected_strain = expected_stress / youngs_modulus;
    let expected_energy = 3.0 * 0.5 * expected_stress * expected_strain * area * length;

    for (index, support) in result.nodes[0..3].iter().enumerate() {
        assert_eq!(support.index, index);
        assert_close(support.ux, 0.0, 1.0e-12);
        assert_close(support.uy, 0.0, 1.0e-12);
        assert_close(support.uz, 0.0, 1.0e-12);
    }

    let apex = &result.nodes[3];
    assert_close(apex.ux, 0.0, TOL);
    assert_close(apex.uy, 0.0, TOL);
    assert_close(apex.uz, expected_uz, TOL);
    assert_close(result.max_displacement, expected_uz.abs(), TOL);
    assert_close(result.max_stress, expected_stress.abs(), TOL);
    assert_close(result.total_strain_energy, expected_energy, TOL);
    assert_apex_force_energy(result.total_strain_energy, load, apex.uz);

    for (index, element) in result.elements.iter().enumerate() {
        assert_eq!(element.index, index);
        assert_close(element.length, length, TOL);
        assert_close(element.axial_force, axial_force, TOL);
        assert_close(element.stress, expected_stress, TOL);
        assert_close(element.strain, expected_strain, TOL);
        assert_close(
            element.strain_energy_density,
            0.5 * element.stress * element.strain,
            TOL,
        );
    }
    assert_truss_summary(&result);
}

#[test]
fn truss_3d_tracks_load_and_area_scaling() {
    let radius = 0.55;
    let height = 0.95;
    let load = -1350.0;
    let area = 0.014;
    let youngs_modulus = 72.0e9;
    let baseline = solve_truss_3d(&tripod_request(radius, height, load, area, youngs_modulus))
        .expect("baseline tripod truss should solve");
    assert_apex_force_energy(baseline.total_strain_energy, load, baseline.nodes[3].uz);
    assert_truss_summary(&baseline);

    let load_scale = 1.35;
    let load_scaled = solve_truss_3d(&tripod_request(
        radius,
        height,
        load * load_scale,
        area,
        youngs_modulus,
    ))
    .expect("load-scaled tripod truss should solve");
    assert_close(
        load_scaled.nodes[3].uz / baseline.nodes[3].uz,
        load_scale,
        TOL,
    );
    assert_close(
        load_scaled.elements[0].axial_force / baseline.elements[0].axial_force,
        load_scale,
        TOL,
    );
    assert_close(
        load_scaled.max_stress / baseline.max_stress,
        load_scale,
        TOL,
    );
    assert_close(
        load_scaled.total_strain_energy / baseline.total_strain_energy,
        load_scale * load_scale,
        TOL,
    );
    assert_apex_force_energy(
        load_scaled.total_strain_energy,
        load * load_scale,
        load_scaled.nodes[3].uz,
    );
    assert_truss_summary(&load_scaled);

    let area_scale = 1.5;
    let area_scaled = solve_truss_3d(&tripod_request(
        radius,
        height,
        load,
        area * area_scale,
        youngs_modulus,
    ))
    .expect("area-scaled tripod truss should solve");
    assert_close(
        area_scaled.nodes[3].uz / baseline.nodes[3].uz,
        1.0 / area_scale,
        TOL,
    );
    assert_close(
        area_scaled.elements[0].axial_force,
        baseline.elements[0].axial_force,
        TOL,
    );
    assert_close(
        area_scaled.max_stress / baseline.max_stress,
        1.0 / area_scale,
        TOL,
    );
    assert_close(
        area_scaled.total_strain_energy / baseline.total_strain_energy,
        1.0 / area_scale,
        TOL,
    );
    assert_apex_force_energy(
        area_scaled.total_strain_energy,
        load,
        area_scaled.nodes[3].uz,
    );
    assert_truss_summary(&area_scaled);

    let modulus_scale = 1.2;
    let stiffened = solve_truss_3d(&tripod_request(
        radius,
        height,
        load,
        area,
        youngs_modulus * modulus_scale,
    ))
    .expect("modulus-scaled tripod truss should solve");
    assert_close(
        stiffened.nodes[3].uz / baseline.nodes[3].uz,
        1.0 / modulus_scale,
        TOL,
    );
    assert_close(
        stiffened.elements[0].axial_force,
        baseline.elements[0].axial_force,
        TOL,
    );
    assert_close(stiffened.max_stress, baseline.max_stress, TOL);
    assert_close(
        stiffened.total_strain_energy / baseline.total_strain_energy,
        1.0 / modulus_scale,
        TOL,
    );
    assert_apex_force_energy(stiffened.total_strain_energy, load, stiffened.nodes[3].uz);
    assert_truss_summary(&stiffened);

    let geometry_scale = 1.4;
    let longer = solve_truss_3d(&tripod_request(
        radius * geometry_scale,
        height * geometry_scale,
        load,
        area,
        youngs_modulus,
    ))
    .expect("geometry-scaled tripod truss should solve");
    assert_close(
        longer.elements[0].length / baseline.elements[0].length,
        geometry_scale,
        TOL,
    );
    assert_close(
        longer.nodes[3].uz / baseline.nodes[3].uz,
        geometry_scale,
        TOL,
    );
    assert_close(
        longer.elements[0].axial_force,
        baseline.elements[0].axial_force,
        TOL,
    );
    assert_close(longer.max_stress, baseline.max_stress, TOL);
    assert_close(
        longer.total_strain_energy / baseline.total_strain_energy,
        geometry_scale,
        TOL,
    );
    assert_apex_force_energy(longer.total_strain_energy, load, longer.nodes[3].uz);
    assert_truss_summary(&longer);
}

fn assert_truss_summary(result: &SolveTruss3dResult) {
    let mut max_stress = 0.0_f64;
    let mut max_energy_density = 0.0_f64;
    let mut total_strain_energy = 0.0_f64;
    assert_eq!(result.nodes.len(), result.input.nodes.len());
    assert_eq!(result.elements.len(), result.input.elements.len());
    for (index, node) in result.nodes.iter().enumerate() {
        let input = &result.input.nodes[index];
        assert_eq!(node.index, index);
        assert_eq!(node.id, input.id);
        assert_close(node.x, input.x, TOL);
        assert_close(node.y, input.y, TOL);
        assert_close(node.z, input.z, TOL);
    }

    for (index, element) in result.elements.iter().enumerate() {
        let input = &result.input.elements[index];
        assert_eq!(element.index, index);
        assert_close(element.stress, input.youngs_modulus * element.strain, TOL);
        assert_close(element.axial_force, element.stress * input.area, TOL);
        assert_close(
            element.strain_energy_density,
            0.5 * element.stress * element.strain,
            TOL,
        );
        max_stress = max_stress.max(element.stress.abs());
        max_energy_density = max_energy_density.max(element.strain_energy_density.abs());
        total_strain_energy += element.strain_energy_density * input.area * element.length;
    }

    assert_close(
        result.max_displacement,
        result
            .nodes
            .iter()
            .map(|node| {
                let input = &result.input.nodes[node.index];
                assert_eq!(node.id, input.id);
                assert_close(node.x, input.x, TOL);
                assert_close(node.y, input.y, TOL);
                assert_close(node.z, input.z, TOL);
                (node.ux * node.ux + node.uy * node.uy + node.uz * node.uz).sqrt()
            })
            .fold(0.0_f64, f64::max),
        TOL,
    );
    assert_close(result.max_stress, max_stress, TOL);
    assert_close(result.max_strain_energy_density, max_energy_density, TOL);
    assert_close(result.total_strain_energy, total_strain_energy, TOL);

    let external_work = result
        .input
        .nodes
        .iter()
        .zip(result.nodes.iter())
        .map(|(input, node)| {
            input.load_x * node.ux + input.load_y * node.uy + input.load_z * node.uz
        })
        .sum::<f64>();
    assert_close(result.total_strain_energy, 0.5 * external_work, TOL);
}

fn tripod_request(
    radius: f64,
    height: f64,
    load_z: f64,
    area: f64,
    youngs_modulus: f64,
) -> SolveTruss3dRequest {
    let root_three_over_two = 3.0_f64.sqrt() * 0.5;
    SolveTruss3dRequest {
        nodes: vec![
            node("base-a", radius, 0.0, 0.0, true, 0.0),
            node(
                "base-b",
                -0.5 * radius,
                root_three_over_two * radius,
                0.0,
                true,
                0.0,
            ),
            node(
                "base-c",
                -0.5 * radius,
                -root_three_over_two * radius,
                0.0,
                true,
                0.0,
            ),
            node("apex", 0.0, 0.0, height, false, load_z),
        ],
        elements: vec![
            element("leg-a", 0, 3, area, youngs_modulus),
            element("leg-b", 1, 3, area, youngs_modulus),
            element("leg-c", 2, 3, area, youngs_modulus),
        ],
    }
}

fn node(id: &str, x: f64, y: f64, z: f64, fixed: bool, load_z: f64) -> Truss3dNodeInput {
    Truss3dNodeInput {
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

fn element(
    id: &str,
    node_i: usize,
    node_j: usize,
    area: f64,
    youngs_modulus: f64,
) -> Truss3dElementInput {
    Truss3dElementInput {
        id: id.to_string(),
        node_i,
        node_j,
        area,
        youngs_modulus,
    }
}

fn assert_apex_force_energy(total_strain_energy: f64, apex_load: f64, apex_displacement: f64) {
    assert_close(
        total_strain_energy,
        0.5 * apex_load * apex_displacement,
        TOL,
    );
}

fn assert_close(actual: f64, expected: f64, tolerance: f64) {
    let scale = expected.abs().max(1.0);
    assert!(
        (actual - expected).abs() <= tolerance * scale,
        "expected {actual} to be close to {expected}",
    );
}
