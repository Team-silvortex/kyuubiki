use kyuubiki_protocol::{SolveTorsion1dRequest, Torsion1dElementInput, Torsion1dNodeInput};
use kyuubiki_solver::solve_torsion_1d;

const TOL: f64 = 1.0e-10;

#[test]
fn torsion_1d_review_bundle_checks_fixed_root_tip_torque_and_shear_stress_diagnostics() {
    let result = solve_torsion_1d(&SolveTorsion1dRequest {
        nodes: vec![
            node("fixed", 0.0, true, 0.0),
            node("tip", 1.5, false, 2500.0),
        ],
        elements: vec![Torsion1dElementInput {
            id: "shaft".to_string(),
            node_i: 0,
            node_j: 1,
            shear_modulus: 79.0e9,
            polar_moment: 1.8e-6,
            section_modulus: 1.2e-4,
        }],
    })
    .expect("review torsion shaft should solve");

    let expected_tip_rotation = 0.026_371_308_016_877_638;
    let expected_torque = 2500.0;
    let expected_shear_stress = 20_833_333.333_333_332;

    assert_eq!(result.nodes.len(), 2);
    assert_eq!(result.elements.len(), 1);
    assert_close(result.nodes[0].rz, 0.0);
    assert_close(result.nodes[1].rz, expected_tip_rotation);
    assert_close(result.max_rotation, expected_tip_rotation);
    assert_close(result.max_torque, expected_torque);
    assert_close(result.max_stress, expected_shear_stress);

    let element = &result.elements[0];
    assert_close(element.length, 1.5);
    assert_close(element.twist, expected_tip_rotation);
    assert_close(element.torque, expected_torque);
    assert_close(element.shear_stress, expected_shear_stress);

    let twist_rate = element.twist / element.length;
    assert_close(twist_rate, expected_tip_rotation / 1.5);
}

fn node(id: &str, x: f64, fix_rz: bool, torque_z: f64) -> Torsion1dNodeInput {
    Torsion1dNodeInput {
        id: id.to_string(),
        x,
        fix_rz,
        torque_z,
    }
}

fn assert_close(actual: f64, expected: f64) {
    let scale = expected.abs().max(1.0);
    assert!(
        (actual - expected).abs() <= TOL * scale,
        "expected {actual} to be close to {expected}",
    );
}
