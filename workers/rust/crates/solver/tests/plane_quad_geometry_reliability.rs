use kyuubiki_protocol::{
    PlaneNodeInput, PlaneQuadElementInput, SolvePlaneQuad2dRequest, SolveThermalPlaneQuad2dRequest,
    ThermalPlaneNodeInput, ThermalPlaneQuadElementInput,
};
use kyuubiki_solver::{solve_plane_quad_2d, solve_thermal_plane_quad_2d};

const POINTS: [[f64; 2]; 4] = [[0.0, 0.0], [2.0, 0.0], [2.25, 1.0], [0.0, 1.0]];

fn mechanical(offset: f64) -> SolvePlaneQuad2dRequest {
    SolvePlaneQuad2dRequest {
        nodes: POINTS
            .into_iter()
            .enumerate()
            .map(|(i, [x, y])| PlaneNodeInput {
                id: format!("n{i}"),
                x: x + offset,
                y: y - offset,
                fix_x: i == 0 || i == 3,
                fix_y: i == 0,
                load_x: if i == 1 || i == 2 { 1.0 } else { 0.0 },
                load_y: 0.0,
            })
            .collect(),
        elements: vec![PlaneQuadElementInput {
            id: "quad-patch".into(),
            node_i: 0,
            node_j: 1,
            node_k: 2,
            node_l: 3,
            thickness: 0.5,
            youngs_modulus: 1200.0,
            poisson_ratio: 0.25,
        }],
    }
}

fn thermal(offset: f64, restrained: bool) -> SolveThermalPlaneQuad2dRequest {
    SolveThermalPlaneQuad2dRequest {
        nodes: POINTS
            .into_iter()
            .enumerate()
            .map(|(i, [x, y])| ThermalPlaneNodeInput {
                id: format!("n{i}"),
                x: x + offset,
                y: y - offset,
                fix_x: restrained || i == 0,
                fix_y: restrained || i == 0 || i == 1,
                load_x: 0.0,
                load_y: 0.0,
                temperature_delta: if restrained { 30.0 + 8.0 * x } else { 50.0 },
            })
            .collect(),
        elements: vec![ThermalPlaneQuadElementInput {
            id: "thermal-quad-patch".into(),
            node_i: 0,
            node_j: 1,
            node_k: 2,
            node_l: 3,
            thickness: 0.5,
            youngs_modulus: 1200.0,
            poisson_ratio: 0.25,
            thermal_expansion: 1e-5,
        }],
    }
}

fn close(actual: f64, expected: f64, scale: f64) {
    assert!(
        actual.is_finite() && (actual - expected).abs() < 1e-11 * scale,
        "actual={actual:e} expected={expected:e} scale={scale:e}"
    );
}

#[test]
fn translated_q4_operator_keeps_analytic_displacement_stress_and_energy() {
    let reference = solve_plane_quad_2d(&mechanical(0.0)).unwrap();
    for offset in [2f64.powi(20), -2f64.powi(20), 2f64.powi(40), -2f64.powi(40)] {
        let result = solve_plane_quad_2d(&mechanical(offset)).unwrap();
        for (node, [x, y]) in result.nodes.iter().zip(POINTS) {
            close(node.ux, 4.0 / 1200.0 * x, 0.01);
            close(node.uy, -0.25 * 4.0 / 1200.0 * y, 0.01);
        }
        let element = &result.elements[0];
        close(element.area, 2.125, 3.0);
        close(element.stress_x, 4.0, 4.0);
        close(element.stress_y, 0.0, 4.0);
        close(element.tau_xy, 0.0, 4.0);
        close(
            result.total_strain_energy,
            reference.total_strain_energy,
            0.01,
        );
        close(
            result.total_strain_energy,
            0.5 * 4.0 * (4.0 / 1200.0) * 2.125 * 0.5,
            0.01,
        );
    }
}

#[test]
fn translated_thermal_q4_keeps_free_expansion_and_restrained_temperature_integration() {
    for restrained in [false, true] {
        let reference = solve_thermal_plane_quad_2d(&thermal(0.0, restrained)).unwrap();
        for offset in [2f64.powi(40), -2f64.powi(40)] {
            let result = solve_thermal_plane_quad_2d(&thermal(offset, restrained)).unwrap();
            for ((node, original), [x, y]) in result.nodes.iter().zip(&reference.nodes).zip(POINTS)
            {
                close(node.ux, original.ux, 0.01);
                close(node.uy, original.uy, 0.01);
                if !restrained {
                    close(node.ux, 0.0005 * x, 0.01);
                    close(node.uy, 0.0005 * y, 0.01);
                }
            }
            let actual = &result.elements[0];
            let expected = &reference.elements[0];
            for (a, b, scale) in [
                (actual.area, expected.area, 3.0),
                (
                    actual.average_temperature_delta,
                    expected.average_temperature_delta,
                    50.0,
                ),
                (actual.stress_x, expected.stress_x, 1.0),
                (actual.stress_y, expected.stress_y, 1.0),
                (actual.tau_xy, expected.tau_xy, 1.0),
                (
                    actual.strain_energy_density,
                    expected.strain_energy_density,
                    0.01,
                ),
            ] {
                close(a, b, scale);
            }
        }
    }
}

#[test]
fn q4_invalid_coefficients_fail_before_assembly_and_valid_models_replay() {
    let mut invalid = mechanical(0.0);
    invalid.elements[0].youngs_modulus = f64::MAX;
    invalid.elements[0].poisson_ratio = 0.49;
    let error = solve_plane_quad_2d(&invalid).unwrap_err();
    assert!(
        error.contains("quad-patch") && error.contains("coefficients are not representable"),
        "{error}"
    );
    assert!(solve_plane_quad_2d(&mechanical(0.0)).is_ok());
    let mut invalid = thermal(0.0, false);
    invalid.elements[0].youngs_modulus = f64::MAX;
    invalid.elements[0].poisson_ratio = 0.49;
    let error = solve_thermal_plane_quad_2d(&invalid).unwrap_err();
    assert!(
        error.contains("thermal-quad-patch")
            && error.contains("coefficients are not representable"),
        "{error}"
    );
    assert!(solve_thermal_plane_quad_2d(&thermal(0.0, false)).is_ok());
}
