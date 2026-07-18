use kyuubiki_protocol::{
    Frame3dElementInput, Frame3dElementResult, Frame3dNodeInput, SolveFrame3dRequest,
};
use kyuubiki_solver::solve_frame_3d;

const TOL: f64 = 1.0e-10;

#[test]
fn frame_3d_closed_form_matches_tip_loaded_cantilever() {
    let length = 2.0;
    let load_y: f64 = -1000.0;
    let youngs_modulus = 210.0e9;
    let iz = 8.0e-6;
    let section_modulus_z = 1.6e-4;
    let result = solve_frame_3d(&cantilever(
        length,
        load_y,
        youngs_modulus,
        iz,
        section_modulus_z,
    ))
    .expect("3D frame closed-form cantilever should solve");
    assert_frame_summary(&result);
    assert_tip_work_energy(&result);

    let expected_uy = load_y * length.powi(3) / (3.0 * youngs_modulus * iz);
    let expected_rz = load_y * length.powi(2) / (2.0 * youngs_modulus * iz);
    let expected_moment = load_y.abs() * length;
    let expected_bending_stress = expected_moment / section_modulus_z;
    let expected_energy = 0.5 * load_y.abs() * expected_uy.abs();

    assert_close(result.nodes[0].ux, 0.0);
    assert_close(result.nodes[0].uy, 0.0);
    assert_close(result.nodes[0].uz, 0.0);
    assert_close(result.nodes[0].rx, 0.0);
    assert_close(result.nodes[0].ry, 0.0);
    assert_close(result.nodes[0].rz, 0.0);
    assert_close(result.nodes[1].ux, 0.0);
    assert_close(result.nodes[1].uy, expected_uy);
    assert_close(result.nodes[1].uz, 0.0);
    assert_close(result.nodes[1].rx, 0.0);
    assert_close(result.nodes[1].ry, 0.0);
    assert_close(result.nodes[1].rz, expected_rz);
    assert_close(result.max_displacement, expected_uy.abs());
    assert_close(result.max_rotation, expected_rz.abs());
    assert_close(result.max_moment, expected_moment);
    assert_close(result.max_stress, expected_bending_stress);
    assert_close(result.total_strain_energy, expected_energy);

    let element = &result.elements[0];
    assert_close(element.length, length);
    assert_close(element.axial_force_i, 0.0);
    assert_close(element.axial_force_j, 0.0);
    assert_close(element.shear_force_y_i, load_y.abs());
    assert_close(element.shear_force_y_j, load_y);
    assert_close(element.shear_force_z_i, 0.0);
    assert_close(element.shear_force_z_j, 0.0);
    assert_close(element.torsion_i, 0.0);
    assert_close(element.torsion_j, 0.0);
    assert_close(element.moment_y_i, 0.0);
    assert_close(element.moment_y_j, 0.0);
    assert_close(element.moment_z_i, expected_moment);
    assert_close(element.moment_z_j, 0.0);
    assert_close(element.axial_stress, 0.0);
    assert_close(element.max_bending_stress, expected_bending_stress);
    assert_close(element.max_combined_stress, expected_bending_stress);
    assert_close(element.strain_energy, expected_energy);
}

#[test]
fn frame_3d_tracks_tip_load_and_bending_inertia_scaling() {
    let length = 1.8;
    let load_y: f64 = -850.0;
    let youngs_modulus = 205.0e9;
    let iz = 7.5e-6;
    let section_modulus_z = 1.45e-4;
    let baseline = solve_frame_3d(&cantilever(
        length,
        load_y,
        youngs_modulus,
        iz,
        section_modulus_z,
    ))
    .expect("baseline 3D frame cantilever should solve");
    assert_frame_summary(&baseline);
    assert_tip_work_energy(&baseline);

    let load_scale = 1.45;
    let load_scaled = solve_frame_3d(&cantilever(
        length,
        load_y * load_scale,
        youngs_modulus,
        iz,
        section_modulus_z,
    ))
    .expect("load-scaled 3D frame cantilever should solve");
    assert_frame_summary(&load_scaled);
    assert_tip_work_energy(&load_scaled);
    assert_close(load_scaled.nodes[1].uy / baseline.nodes[1].uy, load_scale);
    assert_close(load_scaled.nodes[1].rz / baseline.nodes[1].rz, load_scale);
    assert_close(
        load_scaled.elements[0].moment_z_i / baseline.elements[0].moment_z_i,
        load_scale,
    );
    assert_close(load_scaled.max_stress / baseline.max_stress, load_scale);
    assert_close(
        load_scaled.total_strain_energy / baseline.total_strain_energy,
        load_scale * load_scale,
    );

    let inertia_scale = 1.6;
    let inertia_scaled = solve_frame_3d(&cantilever(
        length,
        load_y,
        youngs_modulus,
        iz * inertia_scale,
        section_modulus_z,
    ))
    .expect("inertia-scaled 3D frame cantilever should solve");
    assert_frame_summary(&inertia_scaled);
    assert_tip_work_energy(&inertia_scaled);
    assert_close(
        inertia_scaled.nodes[1].uy / baseline.nodes[1].uy,
        1.0 / inertia_scale,
    );
    assert_close(
        inertia_scaled.nodes[1].rz / baseline.nodes[1].rz,
        1.0 / inertia_scale,
    );
    assert_close(
        inertia_scaled.elements[0].moment_z_i,
        baseline.elements[0].moment_z_i,
    );
    assert_close(inertia_scaled.max_stress, baseline.max_stress);
    assert_close(
        inertia_scaled.total_strain_energy / baseline.total_strain_energy,
        1.0 / inertia_scale,
    );

    let length_scale: f64 = 1.25;
    let longer = solve_frame_3d(&cantilever(
        length * length_scale,
        load_y,
        youngs_modulus,
        iz,
        section_modulus_z,
    ))
    .expect("length-scaled 3D frame cantilever should solve");
    assert_frame_summary(&longer);
    assert_tip_work_energy(&longer);
    assert_close(
        longer.elements[0].length / baseline.elements[0].length,
        length_scale,
    );
    assert_close(
        longer.nodes[1].uy / baseline.nodes[1].uy,
        length_scale.powi(3),
    );
    assert_close(
        longer.nodes[1].rz / baseline.nodes[1].rz,
        length_scale.powi(2),
    );
    assert_close(
        longer.elements[0].moment_z_i / baseline.elements[0].moment_z_i,
        length_scale,
    );
    assert_close(longer.max_stress / baseline.max_stress, length_scale);
    assert_close(
        longer.total_strain_energy / baseline.total_strain_energy,
        length_scale.powi(3),
    );
}

fn cantilever(
    length: f64,
    load_y: f64,
    youngs_modulus: f64,
    iz: f64,
    section_modulus_z: f64,
) -> SolveFrame3dRequest {
    SolveFrame3dRequest {
        nodes: vec![
            node(
                "fixed", 0.0, true, true, true, true, true, true, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0,
            ),
            node(
                "tip", length, false, false, false, false, false, false, 0.0, load_y, 0.0, 0.0,
                0.0, 0.0,
            ),
        ],
        elements: vec![Frame3dElementInput {
            id: "beam".to_string(),
            node_i: 0,
            node_j: 1,
            area: 0.02,
            youngs_modulus,
            shear_modulus: 80.0e9,
            torsion_constant: 5.0e-6,
            moment_of_inertia_y: iz,
            moment_of_inertia_z: iz,
            section_modulus_y: section_modulus_z,
            section_modulus_z,
        }],
    }
}

#[allow(clippy::too_many_arguments)]
fn node(
    id: &str,
    x: f64,
    fix_x: bool,
    fix_y: bool,
    fix_z: bool,
    fix_rx: bool,
    fix_ry: bool,
    fix_rz: bool,
    load_x: f64,
    load_y: f64,
    load_z: f64,
    moment_x: f64,
    moment_y: f64,
    moment_z: f64,
) -> Frame3dNodeInput {
    Frame3dNodeInput {
        id: id.to_string(),
        x,
        y: 0.0,
        z: 0.0,
        fix_x,
        fix_y,
        fix_z,
        fix_rx,
        fix_ry,
        fix_rz,
        load_x,
        load_y,
        load_z,
        moment_x,
        moment_y,
        moment_z,
    }
}

fn assert_frame_summary(result: &kyuubiki_protocol::SolveFrame3dResult) {
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
            (node.ux * node.ux + node.uy * node.uy + node.uz * node.uz).sqrt(),
        );
    }
    assert_close(
        result.max_displacement,
        result
            .nodes
            .iter()
            .map(|node| {
                assert_close(
                    node.displacement_magnitude,
                    (node.ux * node.ux + node.uy * node.uy + node.uz * node.uz).sqrt(),
                );
                node.displacement_magnitude
            })
            .fold(0.0_f64, f64::max),
    );
    assert_close(
        result.max_rotation,
        result
            .nodes
            .iter()
            .map(|node| {
                assert_close(
                    node.rotation_magnitude,
                    (node.rx * node.rx + node.ry * node.ry + node.rz * node.rz).sqrt(),
                );
                node.rotation_magnitude
            })
            .fold(0.0_f64, f64::max),
    );
    assert_close(
        result.max_moment,
        result
            .elements
            .iter()
            .flat_map(|element| {
                [
                    element.moment_y_i.abs(),
                    element.moment_z_i.abs(),
                    element.moment_y_j.abs(),
                    element.moment_z_j.abs(),
                ]
            })
            .fold(0.0_f64, f64::max),
    );
    let mut max_stress = 0.0_f64;
    for (index, element) in result.elements.iter().enumerate() {
        assert_eq!(element.index, index);
        assert_frame_element_law(result, element);
        max_stress = max_stress.max(element.max_combined_stress);
    }
    assert_close(result.max_stress, max_stress);
    let mut total_strain_energy = 0.0_f64;
    for (index, element) in result.elements.iter().enumerate() {
        assert_eq!(element.index, index);
        assert_close(
            element.max_combined_stress,
            element.axial_stress + element.max_bending_stress,
        );
        total_strain_energy += element.strain_energy;
    }
    assert_close(result.total_strain_energy, total_strain_energy);
}

fn assert_frame_element_law(
    result: &kyuubiki_protocol::SolveFrame3dResult,
    element: &Frame3dElementResult,
) {
    let input = &result.input.elements[element.index];
    let node_i = &result.nodes[element.node_i];
    let node_j = &result.nodes[element.node_j];
    let dx = node_j.x - node_i.x;
    let dy = node_j.y - node_i.y;
    let dz = node_j.z - node_i.z;
    let expected_length = (dx * dx + dy * dy + dz * dz).sqrt();
    let expected_axial_stress =
        element.axial_force_i.abs().max(element.axial_force_j.abs()) / input.area;
    let bending_y =
        element.moment_y_i.abs().max(element.moment_y_j.abs()) / input.section_modulus_y;
    let bending_z =
        element.moment_z_i.abs().max(element.moment_z_j.abs()) / input.section_modulus_z;
    assert_close(element.length, expected_length);
    assert_close(element.axial_stress, expected_axial_stress);
    assert_close(element.max_bending_stress, bending_y + bending_z);
    assert_close(
        element.max_combined_stress,
        expected_axial_stress + element.max_bending_stress,
    );
}

fn assert_tip_work_energy(result: &kyuubiki_protocol::SolveFrame3dResult) {
    let external_work = result
        .nodes
        .iter()
        .zip(result.input.nodes.iter())
        .map(|(node, input)| {
            input.load_x * node.ux
                + input.load_y * node.uy
                + input.load_z * node.uz
                + input.moment_x * node.rx
                + input.moment_y * node.ry
                + input.moment_z * node.rz
        })
        .sum::<f64>();
    assert_close(result.total_strain_energy, 0.5 * external_work);
}

fn assert_close(actual: f64, expected: f64) {
    let scale = expected.abs().max(1.0);
    assert!(
        (actual - expected).abs() <= TOL * scale,
        "expected {actual} to be close to {expected}",
    );
}
