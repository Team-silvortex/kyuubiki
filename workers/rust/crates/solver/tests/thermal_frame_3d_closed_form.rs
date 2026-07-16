use kyuubiki_protocol::{
    SolveThermalFrame3dRequest, ThermalFrame3dElementInput, ThermalFrame3dNodeInput,
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

    for node in &result.nodes {
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

fn assert_close(actual: f64, expected: f64) {
    let scale = expected.abs().max(1.0);
    assert!(
        (actual - expected).abs() <= TOL * scale,
        "expected {actual} to be close to {expected}",
    );
}
