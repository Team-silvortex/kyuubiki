use kyuubiki_protocol::{Frame2dElementInput, Frame2dNodeInput, SolveFrame2dRequest};
use kyuubiki_solver::solve_frame_2d;

const TOL: f64 = 1.0e-10;

#[test]
fn frame_2d_review_bundle_checks_cantilever_boundaries_moment_and_stress_diagnostics() {
    let result = solve_frame_2d(&SolveFrame2dRequest {
        nodes: vec![
            node("fixed", 0.0, 0.0, true, true, true, 0.0, 0.0, 0.0),
            node("tip", 2.0, 0.0, false, false, false, 0.0, -1000.0, 0.0),
        ],
        elements: vec![Frame2dElementInput {
            id: "beam".to_string(),
            node_i: 0,
            node_j: 1,
            area: 0.02,
            youngs_modulus: 210.0e9,
            moment_of_inertia: 8.0e-6,
            section_modulus: 1.6e-4,
        }],
    })
    .expect("review frame should solve");

    let expected_tip_uy = -0.001_587_301_587_301_587_3;
    let expected_tip_rz = -0.001_190_476_190_476_190_6;
    let expected_moment = 2000.0;
    let expected_stress = 12_500_000.0;

    assert_eq!(result.nodes.len(), 2);
    assert_eq!(result.elements.len(), 1);
    assert_close(result.nodes[0].ux, 0.0);
    assert_close(result.nodes[0].uy, 0.0);
    assert_close(result.nodes[0].rz, 0.0);
    assert_close(result.nodes[1].ux, 0.0);
    assert_close(result.nodes[1].uy, expected_tip_uy);
    assert_close(result.nodes[1].rz, expected_tip_rz);
    assert_close(result.max_displacement, expected_tip_uy.abs());
    assert_close(result.max_rotation, expected_tip_rz.abs());
    assert_close(result.max_moment, expected_moment);
    assert_close(result.max_stress, expected_stress);

    let element = &result.elements[0];
    assert_close(element.length, 2.0);
    assert_close(element.axial_force_i, 0.0);
    assert_close(element.axial_force_j, 0.0);
    assert_close(element.shear_force_i, 1000.0);
    assert_close(element.shear_force_j, -1000.0);
    assert_close(element.moment_i, 2000.0);
    assert_close(element.moment_j, 0.0);
    assert_close(element.axial_stress, 0.0);
    assert_close(element.max_bending_stress, expected_stress);
    assert_close(element.max_combined_stress, expected_stress);
}

#[allow(clippy::too_many_arguments)]
fn node(
    id: &str,
    x: f64,
    y: f64,
    fix_x: bool,
    fix_y: bool,
    fix_rz: bool,
    load_x: f64,
    load_y: f64,
    moment_z: f64,
) -> Frame2dNodeInput {
    Frame2dNodeInput {
        id: id.to_string(),
        x,
        y,
        fix_x,
        fix_y,
        fix_rz,
        load_x,
        load_y,
        moment_z,
    }
}

fn assert_close(actual: f64, expected: f64) {
    let scale = expected.abs().max(1.0);
    assert!(
        (actual - expected).abs() <= TOL * scale,
        "expected {actual} to be close to {expected}",
    );
}
