use kyuubiki_protocol::{
    SolveThermalBeam1dRequest, ThermalBeam1dElementInput, ThermalBeam1dNodeInput,
};
use kyuubiki_solver::solve_thermal_beam_1d;

const TOL: f64 = 1.0e-10;

#[test]
fn thermal_beam_1d_review_bundle_checks_free_thermal_curvature_response() {
    let thermal_expansion = 12.0e-6;
    let temperature_gradient_y = 45.0;
    let section_depth = 0.3;
    let result = solve_thermal_beam_1d(&SolveThermalBeam1dRequest {
        nodes: vec![
            node("fixed", 0.0, true, true),
            node("free_tip", 2.4, false, false),
        ],
        elements: vec![ThermalBeam1dElementInput {
            id: "beam".to_string(),
            node_i: 0,
            node_j: 1,
            youngs_modulus: 210.0e9,
            moment_of_inertia: 0.00012,
            section_modulus: 0.0011,
            thermal_expansion,
            section_depth,
            distributed_load_y: 0.0,
            temperature_gradient_y,
        }],
    })
    .expect("review thermal beam should solve");

    let expected_curvature = thermal_expansion * temperature_gradient_y / section_depth;
    let expected_tip_rotation = 0.004_320_000_000_000_001;
    let expected_tip_uy = 0.005_184_000_000_000_001;

    assert_eq!(result.nodes.len(), 2);
    assert_eq!(result.elements.len(), 1);
    assert_close(result.nodes[0].uy, 0.0);
    assert_close(result.nodes[0].rz, 0.0);
    assert_close(result.nodes[1].uy, expected_tip_uy);
    assert_close(result.nodes[1].rz, expected_tip_rotation);
    assert_close(result.max_displacement, expected_tip_uy);
    assert_close(result.max_rotation, expected_tip_rotation);
    assert_close(result.max_temperature_gradient, temperature_gradient_y);
    assert_close(result.max_moment, 7.275_957_614_183_426e-12);
    assert_close(result.max_stress, 6.614_506_921_984_932e-9);
    assert!(result.total_strain_energy.abs() < 1.0e-12);

    let element = &result.elements[0];
    assert_close(element.length, 2.4);
    assert_close(element.temperature_gradient_y, temperature_gradient_y);
    assert_close(element.thermal_curvature, expected_curvature);
    assert_close(element.shear_force_i, 0.0);
    assert_close(element.shear_force_j, 0.0);
    assert_close(element.moment_i, 0.0);
    assert_close(element.moment_j, 7.275_957_614_183_426e-12);
    assert_close(element.max_bending_stress, 6.614_506_921_984_932e-9);
    assert!(element.strain_energy.abs() < 1.0e-12);
}

fn node(id: &str, x: f64, fix_y: bool, fix_rz: bool) -> ThermalBeam1dNodeInput {
    ThermalBeam1dNodeInput {
        id: id.to_string(),
        x,
        fix_y,
        fix_rz,
        load_y: 0.0,
        moment_z: 0.0,
    }
}

fn assert_close(actual: f64, expected: f64) {
    let scale = expected.abs().max(1.0);
    assert!(
        (actual - expected).abs() <= TOL * scale,
        "expected {actual} to be close to {expected}",
    );
}
