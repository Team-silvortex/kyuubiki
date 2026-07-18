use kyuubiki_protocol::{
    SolveThermalFrame3dRequest, ThermalFrame3dElementInput, ThermalFrame3dNodeInput,
};
use kyuubiki_solver::solve_thermal_frame_3d;

const TOL: f64 = 1.0e-10;

#[test]
fn thermal_frame_3d_review_bundle_checks_restrained_temperature_and_gradient_response() {
    let area = 0.02;
    let youngs_modulus = 210.0e9;
    let thermal_expansion = 12.0e-6;
    let temperature_delta = 35.0;
    let gradient_y = 30.0;
    let gradient_z = 20.0;
    let section_depth_y = 0.2;
    let section_depth_z = 0.15;
    let inertia_y = 8.0e-6;
    let inertia_z = 6.0e-6;
    let section_modulus_y = 1.6e-4;
    let section_modulus_z = 1.2e-4;

    let result = solve_thermal_frame_3d(&SolveThermalFrame3dRequest {
        nodes: vec![
            node("fixed", 0.0, 0.0, 0.0, temperature_delta),
            node("restrained", 2.0, 0.0, 0.0, temperature_delta),
        ],
        elements: vec![ThermalFrame3dElementInput {
            id: "beam".to_string(),
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
    })
    .expect("review thermal 3d frame should solve");

    let expected_thermal_strain = thermal_expansion * temperature_delta;
    let expected_curvature_y = thermal_expansion * gradient_y / section_depth_y;
    let expected_curvature_z = thermal_expansion * gradient_z / section_depth_z;
    let expected_axial_force = youngs_modulus * area * expected_thermal_strain;
    let expected_moment_y = youngs_modulus * inertia_y * expected_curvature_z;
    let expected_moment_z = youngs_modulus * inertia_z * expected_curvature_y;
    let expected_axial_stress = expected_axial_force / area;
    let expected_bending_stress =
        expected_moment_y / section_modulus_y + expected_moment_z / section_modulus_z;
    let expected_combined_stress = expected_axial_stress + expected_bending_stress;
    let expected_strain_energy = 0.5
        * youngs_modulus
        * (area * expected_thermal_strain.powi(2)
            + inertia_z * expected_curvature_y.powi(2)
            + inertia_y * expected_curvature_z.powi(2))
        * 2.0;

    assert_eq!(result.nodes.len(), 2);
    assert_eq!(result.elements.len(), 1);
    for (index, node) in result.nodes.iter().enumerate() {
        assert_eq!(node.index, index);
        assert_close(node.ux, 0.0);
        assert_close(node.uy, 0.0);
        assert_close(node.uz, 0.0);
        assert_close(node.rx, 0.0);
        assert_close(node.ry, 0.0);
        assert_close(node.rz, 0.0);
        assert_close(node.displacement_magnitude, 0.0);
        assert_close(node.rotation_magnitude, 0.0);
        assert_close(node.temperature_delta, temperature_delta);
    }

    let element = &result.elements[0];
    assert_close(element.length, 2.0);
    assert_close(element.average_temperature_delta, temperature_delta);
    assert_close(element.thermal_strain, expected_thermal_strain);
    assert_close(element.mechanical_strain, -expected_thermal_strain);
    assert_close(element.total_strain, 0.0);
    assert_close(element.temperature_gradient_y, gradient_y);
    assert_close(element.temperature_gradient_z, gradient_z);
    assert_close(element.thermal_curvature_y, expected_curvature_y);
    assert_close(element.thermal_curvature_z, expected_curvature_z);
    assert_close(element.axial_force_i, expected_axial_force);
    assert_close(element.axial_force_j, -expected_axial_force);
    assert_close(element.shear_force_y_i, 0.0);
    assert_close(element.shear_force_y_j, 0.0);
    assert_close(element.shear_force_z_i, 0.0);
    assert_close(element.shear_force_z_j, 0.0);
    assert_close(element.torsion_i, 0.0);
    assert_close(element.torsion_j, 0.0);
    assert_close(element.moment_y_i, expected_moment_y);
    assert_close(element.moment_y_j, -expected_moment_y);
    assert_close(element.moment_z_i, expected_moment_z);
    assert_close(element.moment_z_j, -expected_moment_z);
    assert_close(element.axial_stress, expected_axial_stress);
    assert_close(element.max_bending_stress, expected_bending_stress);
    assert_close(element.max_combined_stress, expected_combined_stress);
    assert_close(element.strain_energy, expected_strain_energy);

    assert_close(result.max_displacement, 0.0);
    assert_close(result.max_rotation, 0.0);
    assert_close(result.max_axial_force, expected_axial_force);
    assert_close(result.max_moment, expected_moment_y);
    assert_close(result.max_stress, expected_combined_stress);
    assert_close(result.max_temperature_delta, temperature_delta);
    assert_close(result.max_temperature_gradient, gradient_y);
    assert_close(result.total_strain_energy, expected_strain_energy);
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
