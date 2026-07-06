use kyuubiki_protocol::{
    SolveThermalBar1dRequest, ThermalBar1dElementInput, ThermalBar1dNodeInput,
};
use kyuubiki_solver::solve_thermal_bar_1d;

const TOL: f64 = 1.0e-10;

#[test]
fn thermal_bar_1d_review_bundle_checks_restrained_uniform_rise_strain_and_force_balance() {
    let result = solve_thermal_bar_1d(&SolveThermalBar1dRequest {
        nodes: vec![
            node("fixed-left", 0.0, true, 0.0, 40.0),
            node("fixed-right", 1.0, true, 0.0, 40.0),
        ],
        elements: vec![ThermalBar1dElementInput {
            id: "bar".to_string(),
            node_i: 0,
            node_j: 1,
            area: 0.01,
            youngs_modulus: 210.0e9,
            thermal_expansion: 12.0e-6,
        }],
    })
    .expect("review thermal bar should solve");

    let expected_thermal_strain: f64 = 12.0e-6 * 40.0;
    let expected_stress: f64 = -210.0e9 * expected_thermal_strain;
    let expected_axial_force: f64 = expected_stress * 0.01;

    assert_eq!(result.nodes.len(), 2);
    assert_eq!(result.elements.len(), 1);
    assert_close(result.nodes[0].ux, 0.0);
    assert_close(result.nodes[1].ux, 0.0);
    assert_close(result.max_displacement, 0.0);
    assert_close(result.max_temperature_delta, 40.0);
    assert_close(result.max_stress, expected_stress.abs());
    assert_close(result.max_axial_force, expected_axial_force.abs());

    let element = &result.elements[0];
    assert_close(element.length, 1.0);
    assert_close(element.average_temperature_delta, 40.0);
    assert_close(element.thermal_strain, expected_thermal_strain);
    assert_close(element.mechanical_strain, -expected_thermal_strain);
    assert_close(element.total_strain, 0.0);
    assert_close(element.stress, expected_stress);
    assert_close(element.axial_force, expected_axial_force);
    assert!(element.stress < 0.0);
}

fn node(
    id: &str,
    x: f64,
    fix_x: bool,
    load_x: f64,
    temperature_delta: f64,
) -> ThermalBar1dNodeInput {
    ThermalBar1dNodeInput {
        id: id.to_string(),
        x,
        fix_x,
        load_x,
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
