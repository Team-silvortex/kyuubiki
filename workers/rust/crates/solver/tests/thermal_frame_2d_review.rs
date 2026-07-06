use kyuubiki_protocol::{
    SolveThermalFrame2dRequest, ThermalFrame2dElementInput, ThermalFrame2dNodeInput,
};
use kyuubiki_solver::solve_thermal_frame_2d;

const TOL: f64 = 1.0e-10;

#[test]
fn thermal_frame_2d_review_bundle_checks_restrained_uniform_temperature_response() {
    let area = 0.02;
    let youngs_modulus = 210.0e9;
    let thermal_expansion = 12.0e-6;
    let temperature_delta = 40.0;
    let result = solve_thermal_frame_2d(&SolveThermalFrame2dRequest {
        nodes: vec![
            node("fixed", 0.0, 0.0, temperature_delta),
            node("restrained", 2.0, 0.0, temperature_delta),
        ],
        elements: vec![ThermalFrame2dElementInput {
            id: "beam".to_string(),
            node_i: 0,
            node_j: 1,
            area,
            youngs_modulus,
            moment_of_inertia: 8.0e-6,
            section_modulus: 1.6e-4,
            thermal_expansion,
            section_depth: 0.3,
            temperature_gradient_y: 0.0,
        }],
    })
    .expect("review thermal frame should solve");

    let expected_thermal_strain = thermal_expansion * temperature_delta;
    let expected_axial_force = youngs_modulus * area * expected_thermal_strain;
    let expected_axial_stress = youngs_modulus * expected_thermal_strain;

    assert_eq!(result.nodes.len(), 2);
    assert_eq!(result.elements.len(), 1);
    for node in &result.nodes {
        assert_close(node.ux, 0.0);
        assert_close(node.uy, 0.0);
        assert_close(node.rz, 0.0);
        assert_close(node.displacement_magnitude, 0.0);
        assert_close(node.temperature_delta, temperature_delta);
    }

    let element = &result.elements[0];
    assert_close(element.length, 2.0);
    assert_close(element.average_temperature_delta, temperature_delta);
    assert_close(element.thermal_strain, expected_thermal_strain);
    assert_close(element.mechanical_strain, -expected_thermal_strain);
    assert_close(element.total_strain, 0.0);
    assert_close(element.temperature_gradient_y, 0.0);
    assert_close(element.thermal_curvature, 0.0);
    assert_close(element.axial_force_i, expected_axial_force);
    assert_close(element.axial_force_j, -expected_axial_force);
    assert_close(element.shear_force_i, 0.0);
    assert_close(element.shear_force_j, 0.0);
    assert_close(element.moment_i, 0.0);
    assert_close(element.moment_j, 0.0);
    assert_close(element.axial_stress, expected_axial_stress);
    assert_close(element.max_bending_stress, 0.0);
    assert_close(element.max_combined_stress, expected_axial_stress);
    assert_close(result.max_displacement, 0.0);
    assert_close(result.max_rotation, 0.0);
    assert_close(result.max_moment, 0.0);
    assert_close(result.max_axial_force, expected_axial_force);
    assert_close(result.max_temperature_delta, temperature_delta);
    assert_close(result.max_temperature_gradient, 0.0);
}

fn node(id: &str, x: f64, y: f64, temperature_delta: f64) -> ThermalFrame2dNodeInput {
    ThermalFrame2dNodeInput {
        id: id.to_string(),
        x,
        y,
        fix_x: true,
        fix_y: true,
        fix_rz: true,
        load_x: 0.0,
        load_y: 0.0,
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
