use kyuubiki_protocol::{
    SolveThermalFrame3dRequest, SolveThermalFrame3dResult, ThermalFrame3dElementInput,
    ThermalFrame3dNodeInput,
};
use kyuubiki_solver::solve_thermal_frame_3d;

const TOL: f64 = 1.0e-10;

#[test]
fn thermal_frame_3d_matches_restrained_temperature_gradient_closed_form() {
    let length = 1.6;
    let area = 0.018;
    let youngs_modulus = 210.0e9;
    let thermal_expansion = 11.5e-6;
    let temperature_delta = 28.0;
    let gradient_y = 24.0;
    let gradient_z = 18.0;
    let inertia_y = 7.0e-6;
    let inertia_z = 5.0e-6;
    let section_modulus_y = 1.4e-4;
    let section_modulus_z = 1.1e-4;
    let section_depth_y = 0.22;
    let section_depth_z = 0.16;

    let result = solve_thermal_frame_3d(&restrained_member(
        length,
        area,
        youngs_modulus,
        thermal_expansion,
        temperature_delta,
        gradient_y,
        gradient_z,
        inertia_y,
        inertia_z,
        section_modulus_y,
        section_modulus_z,
        section_depth_y,
        section_depth_z,
    ))
    .expect("restrained thermal frame 3d should solve");

    let thermal_strain = thermal_expansion * temperature_delta;
    let curvature_y = thermal_expansion * gradient_y / section_depth_y;
    let curvature_z = thermal_expansion * gradient_z / section_depth_z;
    let axial_force = youngs_modulus * area * thermal_strain;
    let moment_y = youngs_modulus * inertia_y * curvature_z;
    let moment_z = youngs_modulus * inertia_z * curvature_y;
    let axial_stress = axial_force / area;
    let bending_stress = moment_y / section_modulus_y + moment_z / section_modulus_z;
    let combined_stress = axial_stress + bending_stress;
    let strain_energy = 0.5
        * youngs_modulus
        * (area * thermal_strain.powi(2)
            + inertia_z * curvature_y.powi(2)
            + inertia_y * curvature_z.powi(2))
        * length;

    for (index, node) in result.nodes.iter().enumerate() {
        assert_eq!(node.index, index);
        assert_close(node.ux, 0.0);
        assert_close(node.uy, 0.0);
        assert_close(node.uz, 0.0);
        assert_close(node.rx, 0.0);
        assert_close(node.ry, 0.0);
        assert_close(node.rz, 0.0);
        assert_close(node.temperature_delta, temperature_delta);
    }

    let element = &result.elements[0];
    assert_close(element.length, length);
    assert_close(element.average_temperature_delta, temperature_delta);
    assert_close(element.thermal_strain, thermal_strain);
    assert_close(element.mechanical_strain, -thermal_strain);
    assert_close(element.total_strain, 0.0);
    assert_close(element.thermal_curvature_y, curvature_y);
    assert_close(element.thermal_curvature_z, curvature_z);
    assert_close(element.axial_force_i, axial_force);
    assert_close(element.axial_force_j, -axial_force);
    assert_close(element.moment_y_i, moment_y);
    assert_close(element.moment_y_j, -moment_y);
    assert_close(element.moment_z_i, moment_z);
    assert_close(element.moment_z_j, -moment_z);
    assert_close(element.axial_stress, axial_stress);
    assert_close(element.max_bending_stress, bending_stress);
    assert_close(element.max_combined_stress, combined_stress);
    assert_close(element.strain_energy, strain_energy);

    assert_close(result.max_displacement, 0.0);
    assert_close(result.max_rotation, 0.0);
    assert_close(result.max_axial_force, axial_force);
    assert_close(result.max_moment, moment_y.max(moment_z));
    assert_close(result.max_stress, combined_stress);
    assert_close(result.max_temperature_delta, temperature_delta);
    assert_close(result.max_temperature_gradient, gradient_y);
    assert_close(result.total_strain_energy, strain_energy);
    assert_member_energy_balance(&result.elements, result.total_strain_energy);
    assert_thermal_frame_summary(&result);
}

#[test]
fn thermal_frame_3d_tracks_temperature_gradient_and_inertia_scaling() {
    let length = 1.5;
    let area = 0.016;
    let youngs_modulus = 205.0e9;
    let thermal_expansion = 10.5e-6;
    let temperature_delta = 25.0;
    let gradient_y = 20.0;
    let gradient_z = 16.0;
    let inertia_y = 6.0e-6;
    let inertia_z = 4.5e-6;
    let section_modulus_y = 1.2e-4;
    let section_modulus_z = 9.5e-5;
    let section_depth_y = 0.2;
    let section_depth_z = 0.15;
    let baseline = solve_thermal_frame_3d(&restrained_member(
        length,
        area,
        youngs_modulus,
        thermal_expansion,
        temperature_delta,
        gradient_y,
        gradient_z,
        inertia_y,
        inertia_z,
        section_modulus_y,
        section_modulus_z,
        section_depth_y,
        section_depth_z,
    ))
    .expect("baseline thermal frame 3d should solve");
    assert_member_energy_balance(&baseline.elements, baseline.total_strain_energy);
    assert_thermal_frame_summary(&baseline);

    let thermal_scale = 1.4;
    let hotter = solve_thermal_frame_3d(&restrained_member(
        length,
        area,
        youngs_modulus,
        thermal_expansion,
        temperature_delta * thermal_scale,
        gradient_y * thermal_scale,
        gradient_z * thermal_scale,
        inertia_y,
        inertia_z,
        section_modulus_y,
        section_modulus_z,
        section_depth_y,
        section_depth_z,
    ))
    .expect("thermal-scaled frame 3d should solve");
    assert_close(
        hotter.elements[0].thermal_strain / baseline.elements[0].thermal_strain,
        thermal_scale,
    );
    assert_close(
        hotter.elements[0].thermal_curvature_y / baseline.elements[0].thermal_curvature_y,
        thermal_scale,
    );
    assert_close(
        hotter.elements[0].thermal_curvature_z / baseline.elements[0].thermal_curvature_z,
        thermal_scale,
    );
    assert_close(
        hotter.elements[0].axial_force_i / baseline.elements[0].axial_force_i,
        thermal_scale,
    );
    assert_close(
        hotter.elements[0].moment_y_i / baseline.elements[0].moment_y_i,
        thermal_scale,
    );
    assert_close(
        hotter.elements[0].moment_z_i / baseline.elements[0].moment_z_i,
        thermal_scale,
    );
    assert_close(
        hotter.elements[0].strain_energy / baseline.elements[0].strain_energy,
        thermal_scale * thermal_scale,
    );
    assert_member_energy_balance(&hotter.elements, hotter.total_strain_energy);
    assert_thermal_frame_summary(&hotter);

    let expansion_scale = 1.25;
    let expanded = solve_thermal_frame_3d(&restrained_member(
        length,
        area,
        youngs_modulus,
        thermal_expansion * expansion_scale,
        temperature_delta,
        gradient_y,
        gradient_z,
        inertia_y,
        inertia_z,
        section_modulus_y,
        section_modulus_z,
        section_depth_y,
        section_depth_z,
    ))
    .expect("thermal-expansion-scaled frame 3d should solve");
    assert_close(
        expanded.elements[0].thermal_strain / baseline.elements[0].thermal_strain,
        expansion_scale,
    );
    assert_close(
        expanded.elements[0].thermal_curvature_y / baseline.elements[0].thermal_curvature_y,
        expansion_scale,
    );
    assert_close(
        expanded.elements[0].thermal_curvature_z / baseline.elements[0].thermal_curvature_z,
        expansion_scale,
    );
    assert_close(
        expanded.elements[0].axial_force_i / baseline.elements[0].axial_force_i,
        expansion_scale,
    );
    assert_close(
        expanded.elements[0].moment_y_i / baseline.elements[0].moment_y_i,
        expansion_scale,
    );
    assert_close(
        expanded.elements[0].moment_z_i / baseline.elements[0].moment_z_i,
        expansion_scale,
    );
    assert_close(
        expanded.elements[0].strain_energy / baseline.elements[0].strain_energy,
        expansion_scale * expansion_scale,
    );
    assert_member_energy_balance(&expanded.elements, expanded.total_strain_energy);
    assert_thermal_frame_summary(&expanded);

    let modulus_scale = 1.2;
    let stiffer = solve_thermal_frame_3d(&restrained_member(
        length,
        area,
        youngs_modulus * modulus_scale,
        thermal_expansion,
        temperature_delta,
        gradient_y,
        gradient_z,
        inertia_y,
        inertia_z,
        section_modulus_y,
        section_modulus_z,
        section_depth_y,
        section_depth_z,
    ))
    .expect("modulus-scaled frame 3d should solve");
    assert_close(
        stiffer.elements[0].thermal_strain,
        baseline.elements[0].thermal_strain,
    );
    assert_close(
        stiffer.elements[0].thermal_curvature_y,
        baseline.elements[0].thermal_curvature_y,
    );
    assert_close(
        stiffer.elements[0].thermal_curvature_z,
        baseline.elements[0].thermal_curvature_z,
    );
    assert_close(
        stiffer.elements[0].axial_force_i / baseline.elements[0].axial_force_i,
        modulus_scale,
    );
    assert_close(
        stiffer.elements[0].moment_y_i / baseline.elements[0].moment_y_i,
        modulus_scale,
    );
    assert_close(
        stiffer.elements[0].moment_z_i / baseline.elements[0].moment_z_i,
        modulus_scale,
    );
    assert_close(
        stiffer.elements[0].strain_energy / baseline.elements[0].strain_energy,
        modulus_scale,
    );
    assert_member_energy_balance(&stiffer.elements, stiffer.total_strain_energy);
    assert_thermal_frame_summary(&stiffer);

    let inertia_scale = 1.6;
    let heavier = solve_thermal_frame_3d(&restrained_member(
        length,
        area,
        youngs_modulus,
        thermal_expansion,
        temperature_delta,
        gradient_y,
        gradient_z,
        inertia_y * inertia_scale,
        inertia_z * inertia_scale,
        section_modulus_y,
        section_modulus_z,
        section_depth_y,
        section_depth_z,
    ))
    .expect("inertia-scaled frame 3d should solve");
    assert_close(
        heavier.elements[0].thermal_curvature_y,
        baseline.elements[0].thermal_curvature_y,
    );
    assert_close(
        heavier.elements[0].thermal_curvature_z,
        baseline.elements[0].thermal_curvature_z,
    );
    assert_close(
        heavier.elements[0].moment_y_i / baseline.elements[0].moment_y_i,
        inertia_scale,
    );
    assert_close(
        heavier.elements[0].moment_z_i / baseline.elements[0].moment_z_i,
        inertia_scale,
    );
    assert_close(
        heavier.elements[0].axial_force_i,
        baseline.elements[0].axial_force_i,
    );
    assert!(heavier.elements[0].strain_energy > baseline.elements[0].strain_energy);
    assert_member_energy_balance(&heavier.elements, heavier.total_strain_energy);
    assert_thermal_frame_summary(&heavier);

    let length_scale = 1.4;
    let longer = solve_thermal_frame_3d(&restrained_member(
        length * length_scale,
        area,
        youngs_modulus,
        thermal_expansion,
        temperature_delta,
        gradient_y,
        gradient_z,
        inertia_y,
        inertia_z,
        section_modulus_y,
        section_modulus_z,
        section_depth_y,
        section_depth_z,
    ))
    .expect("length-scaled frame 3d should solve");
    assert_close(
        longer.elements[0].thermal_strain,
        baseline.elements[0].thermal_strain,
    );
    assert_close(
        longer.elements[0].thermal_curvature_y,
        baseline.elements[0].thermal_curvature_y,
    );
    assert_close(
        longer.elements[0].thermal_curvature_z,
        baseline.elements[0].thermal_curvature_z,
    );
    assert_close(
        longer.elements[0].axial_force_i,
        baseline.elements[0].axial_force_i,
    );
    assert_close(
        longer.elements[0].moment_y_i,
        baseline.elements[0].moment_y_i,
    );
    assert_close(
        longer.elements[0].moment_z_i,
        baseline.elements[0].moment_z_i,
    );
    assert_close(
        longer.elements[0].strain_energy / baseline.elements[0].strain_energy,
        length_scale,
    );
    assert_member_energy_balance(&longer.elements, longer.total_strain_energy);
    assert_thermal_frame_summary(&longer);
}

fn assert_thermal_frame_summary(result: &SolveThermalFrame3dResult) {
    let mut max_moment = 0.0_f64;
    let mut max_stress = 0.0_f64;
    let mut max_axial_force = 0.0_f64;
    let mut max_temperature_gradient = 0.0_f64;
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
        assert_close(node.temperature_delta, input.temperature_delta);
        assert_close(
            node.displacement_magnitude,
            (node.ux * node.ux + node.uy * node.uy + node.uz * node.uz).sqrt(),
        );
        assert_close(
            node.rotation_magnitude,
            (node.rx * node.rx + node.ry * node.ry + node.rz * node.rz).sqrt(),
        );
    }

    for (index, element) in result.elements.iter().enumerate() {
        let input = &result.input.elements[index];
        assert_eq!(element.index, index);
        let average_temperature_delta = (result.nodes[element.node_i].temperature_delta
            + result.nodes[element.node_j].temperature_delta)
            / 2.0;
        assert_close(element.average_temperature_delta, average_temperature_delta);
        assert_close(
            element.thermal_strain,
            input.thermal_expansion * average_temperature_delta,
        );
        assert_close(
            element.mechanical_strain,
            element.total_strain - element.thermal_strain,
        );
        assert_close(
            element.thermal_curvature_y,
            input.thermal_expansion * element.temperature_gradient_y / input.section_depth_y,
        );
        assert_close(
            element.thermal_curvature_z,
            input.thermal_expansion * element.temperature_gradient_z / input.section_depth_z,
        );
        assert_close(
            element.axial_stress,
            -input.youngs_modulus * element.mechanical_strain,
        );
        assert_close(element.axial_force_i, element.axial_stress * input.area);
        assert_close(element.axial_force_j, -element.axial_force_i);
        assert_close(
            element.moment_y_i,
            input.youngs_modulus * input.moment_of_inertia_y * element.thermal_curvature_z,
        );
        assert_close(element.moment_y_j, -element.moment_y_i);
        assert_close(
            element.moment_z_i,
            input.youngs_modulus * input.moment_of_inertia_z * element.thermal_curvature_y,
        );
        assert_close(element.moment_z_j, -element.moment_z_i);
        assert_close(
            element.max_bending_stress,
            element.moment_y_i.abs() / input.section_modulus_y
                + element.moment_z_i.abs() / input.section_modulus_z,
        );
        assert_close(
            element.max_combined_stress,
            element.axial_stress.abs() + element.max_bending_stress.abs(),
        );
        assert_close(
            element.strain_energy,
            0.5 * input.youngs_modulus
                * (input.area * element.thermal_strain.powi(2)
                    + input.moment_of_inertia_z * element.thermal_curvature_y.powi(2)
                    + input.moment_of_inertia_y * element.thermal_curvature_z.powi(2))
                * element.length,
        );
        max_moment = max_moment.max(element.moment_y_i.abs());
        max_moment = max_moment.max(element.moment_y_j.abs());
        max_moment = max_moment.max(element.moment_z_i.abs());
        max_moment = max_moment.max(element.moment_z_j.abs());
        max_stress = max_stress.max(element.max_combined_stress.abs());
        max_axial_force = max_axial_force.max(element.axial_force_i.abs());
        max_axial_force = max_axial_force.max(element.axial_force_j.abs());
        max_temperature_gradient =
            max_temperature_gradient.max(element.temperature_gradient_y.abs());
        max_temperature_gradient =
            max_temperature_gradient.max(element.temperature_gradient_z.abs());
        total_strain_energy += element.strain_energy;
    }

    assert_close(
        result.max_displacement,
        result
            .nodes
            .iter()
            .map(|node| node.displacement_magnitude)
            .fold(0.0_f64, f64::max),
    );
    assert_close(
        result.max_rotation,
        result
            .nodes
            .iter()
            .map(|node| node.rotation_magnitude)
            .fold(0.0_f64, f64::max),
    );
    assert_close(
        result.max_temperature_delta,
        result
            .nodes
            .iter()
            .map(|node| node.temperature_delta.abs())
            .fold(0.0_f64, f64::max),
    );
    assert_close(result.max_moment, max_moment);
    assert_close(result.max_stress, max_stress);
    assert_close(result.max_axial_force, max_axial_force);
    assert_close(result.max_temperature_gradient, max_temperature_gradient);
    assert_close(result.total_strain_energy, total_strain_energy);
}

#[allow(clippy::too_many_arguments)]
fn restrained_member(
    length: f64,
    area: f64,
    youngs_modulus: f64,
    thermal_expansion: f64,
    temperature_delta: f64,
    gradient_y: f64,
    gradient_z: f64,
    inertia_y: f64,
    inertia_z: f64,
    section_modulus_y: f64,
    section_modulus_z: f64,
    section_depth_y: f64,
    section_depth_z: f64,
) -> SolveThermalFrame3dRequest {
    SolveThermalFrame3dRequest {
        nodes: vec![
            node("left", 0.0, 0.0, 0.0, temperature_delta),
            node("right", length, 0.0, 0.0, temperature_delta),
        ],
        elements: vec![ThermalFrame3dElementInput {
            id: "member".to_string(),
            node_i: 0,
            node_j: 1,
            local_y_axis: None,
            area,
            youngs_modulus,
            shear_modulus: 80.0e9,
            torsion_constant: 5.0e-6,
            moment_of_inertia_y: inertia_y,
            moment_of_inertia_z: inertia_z,
            section_modulus_y,
            section_modulus_z,
            thermal_expansion,
            section_depth_y,
            section_depth_z,
            temperature_gradient_y: gradient_y,
            temperature_gradient_z: gradient_z,
        }],
        directional_springs: Vec::new(),
    }
}

fn node(id: &str, x: f64, y: f64, z: f64, temperature_delta: f64) -> ThermalFrame3dNodeInput {
    ThermalFrame3dNodeInput {
        id: id.to_string(),
        x,
        y,
        z,
        fix_x: true,
        fix_y: true,
        fix_z: true,
        fix_rx: true,
        fix_ry: true,
        fix_rz: true,
        load_x: 0.0,
        load_y: 0.0,
        load_z: 0.0,
        moment_x: 0.0,
        moment_y: 0.0,
        moment_z: 0.0,
        temperature_delta,
    }
}

fn assert_member_energy_balance(
    elements: &[kyuubiki_protocol::ThermalFrame3dElementResult],
    total_strain_energy: f64,
) {
    assert_close(
        total_strain_energy,
        elements.iter().map(|element| element.strain_energy).sum(),
    );
}

fn assert_close(actual: f64, expected: f64) {
    let scale = expected.abs().max(1.0);
    assert!(
        (actual - expected).abs() <= TOL * scale,
        "expected {actual} to be close to {expected}",
    );
}
