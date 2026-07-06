use kyuubiki_protocol::{
    SolveThermalPlaneTriangle2dRequest, ThermalPlaneNodeInput, ThermalPlaneTriangleElementInput,
};
use kyuubiki_solver::solve_thermal_plane_triangle_2d;

const TOL: f64 = 1.0e-10;

#[test]
fn thermal_plane_triangle_2d_review_bundle_checks_restrained_thermal_stress_diagnostics() {
    let temperature_delta = 40.0;
    let thermal_expansion = 12.0e-6;
    let result = solve_thermal_plane_triangle_2d(&SolveThermalPlaneTriangle2dRequest {
        nodes: vec![
            node("n0", 0.0, 0.0, temperature_delta),
            node("n1", 1.0, 0.0, temperature_delta),
            node("n2", 1.0, 1.0, temperature_delta),
            node("n3", 0.0, 1.0, temperature_delta),
        ],
        elements: vec![
            element("tri_lower", 0, 1, 2, thermal_expansion),
            element("tri_upper", 0, 2, 3, thermal_expansion),
        ],
    })
    .expect("review thermal plane triangle should solve");

    let expected_thermal_strain = thermal_expansion * temperature_delta;
    let expected_stress = -50_149_253.731_343_284;

    assert_eq!(result.nodes.len(), 4);
    assert_eq!(result.elements.len(), 2);
    for node in &result.nodes {
        assert_close(node.ux, 0.0);
        assert_close(node.uy, 0.0);
        assert_close(node.displacement_magnitude, 0.0);
        assert_close(node.temperature_delta, temperature_delta);
    }

    for element in &result.elements {
        assert_close(element.area, 0.5);
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
        assert!(element.principal_stress_1 >= element.principal_stress_2);
        assert!(element.max_in_plane_shear >= 0.0);
        assert!(element.von_mises >= 0.0);
    }
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

fn element(
    id: &str,
    node_i: usize,
    node_j: usize,
    node_k: usize,
    thermal_expansion: f64,
) -> ThermalPlaneTriangleElementInput {
    ThermalPlaneTriangleElementInput {
        id: id.to_string(),
        node_i,
        node_j,
        node_k,
        thickness: 0.02,
        youngs_modulus: 70.0e9,
        poisson_ratio: 0.33,
        thermal_expansion,
    }
}

fn assert_close(actual: f64, expected: f64) {
    let scale = expected.abs().max(1.0);
    assert!(
        (actual - expected).abs() <= TOL * scale,
        "expected {actual} to be close to {expected}",
    );
}
