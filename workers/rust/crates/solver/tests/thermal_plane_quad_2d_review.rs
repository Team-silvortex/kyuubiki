use kyuubiki_protocol::{
    SolveThermalPlaneQuad2dRequest, ThermalPlaneNodeInput, ThermalPlaneQuadElementInput,
};
use kyuubiki_solver::solve_thermal_plane_quad_2d;

const TOL: f64 = 1.0e-10;

#[test]
fn thermal_plane_quad_2d_review_bundle_checks_restrained_thermal_stress_diagnostics() {
    let temperature_delta = 30.0;
    let thermal_expansion = 11.0e-6;
    let result = solve_thermal_plane_quad_2d(&SolveThermalPlaneQuad2dRequest {
        nodes: vec![
            node("n0", 0.0, 0.0, temperature_delta),
            node("n1", 1.0, 0.0, temperature_delta),
            node("n2", 1.0, 1.0, temperature_delta),
            node("n3", 0.0, 1.0, temperature_delta),
        ],
        elements: vec![ThermalPlaneQuadElementInput {
            id: "quad".to_string(),
            node_i: 0,
            node_j: 1,
            node_k: 2,
            node_l: 3,
            thickness: 0.02,
            youngs_modulus: 70.0e9,
            poisson_ratio: 0.33,
            thermal_expansion,
        }],
    })
    .expect("review thermal plane quad should solve");

    let expected_thermal_strain = thermal_expansion * temperature_delta;
    let expected_stress = -34_477_611.940_298_505;

    assert_eq!(result.nodes.len(), 4);
    assert_eq!(result.elements.len(), 1);
    for node in &result.nodes {
        assert_close(node.ux, 0.0);
        assert_close(node.uy, 0.0);
        assert_close(node.displacement_magnitude, 0.0);
        assert_close(node.temperature_delta, temperature_delta);
    }

    let element = &result.elements[0];
    assert_close(element.area, 1.0);
    assert_close(element.average_temperature_delta, temperature_delta);
    assert_close(element.thermal_strain, expected_thermal_strain);
    assert_close(element.mechanical_strain_x, -expected_thermal_strain);
    assert_close(element.mechanical_strain_y, -expected_thermal_strain);
    assert_close(element.total_strain_x, 0.0);
    assert_close(element.total_strain_y, 0.0);
    assert_close(element.gamma_xy, 0.0);
    assert_close(element.stress_x, expected_stress);
    assert_close(element.stress_y, expected_stress);
    assert_close(element.tau_xy, 0.0);
    assert!(element.von_mises >= 0.0);
    assert_close(result.max_displacement, 0.0);
    assert_close(result.max_temperature_delta, temperature_delta);
    assert_close(result.max_stress, expected_stress.abs());
}

fn node(id: &str, x: f64, y: f64, temperature_delta: f64) -> ThermalPlaneNodeInput {
    ThermalPlaneNodeInput {
        id: id.to_string(),
        x,
        y,
        fix_x: true,
        fix_y: true,
        load_x: 0.0,
        load_y: 0.0,
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
