use kyuubiki_protocol::{Frame3dElementInput, Frame3dNodeInput, SolveFrame3dRequest};
use kyuubiki_solver::solve_frame_3d;

const TOL: f64 = 1.0e-10;

#[test]
fn frame_3d_review_bundle_checks_cantilever_boundaries_moment_and_stress_diagnostics() {
    let result = solve_frame_3d(&SolveFrame3dRequest {
        nodes: vec![
            node(
                "fixed", 0.0, 0.0, 0.0, true, true, true, true, true, true, 0.0, 0.0, 0.0, 0.0,
                0.0, 0.0,
            ),
            node(
                "tip", 2.0, 0.0, 0.0, false, false, false, false, false, false, 0.0, -1000.0, 0.0,
                0.0, 0.0, 0.0,
            ),
        ],
        elements: vec![Frame3dElementInput {
            id: "beam".to_string(),
            node_i: 0,
            node_j: 1,
            area: 0.02,
            youngs_modulus: 210.0e9,
            shear_modulus: 80.0e9,
            torsion_constant: 5.0e-6,
            moment_of_inertia_y: 8.0e-6,
            moment_of_inertia_z: 8.0e-6,
            section_modulus_y: 1.6e-4,
            section_modulus_z: 1.6e-4,
        }],
    })
    .expect("review 3d frame should solve");

    let expected_tip_uy = -0.001_587_301_587_301_587_3;
    let expected_tip_rz = -0.001_190_476_190_476_190_6;
    let expected_moment = 2000.0;
    let expected_stress = 12_500_000.0;

    assert_eq!(result.nodes.len(), 2);
    assert_eq!(result.elements.len(), 1);
    assert_close(result.nodes[0].ux, 0.0);
    assert_close(result.nodes[0].uy, 0.0);
    assert_close(result.nodes[0].uz, 0.0);
    assert_close(result.nodes[0].rx, 0.0);
    assert_close(result.nodes[0].ry, 0.0);
    assert_close(result.nodes[0].rz, 0.0);
    assert_close(result.nodes[1].ux, 0.0);
    assert_close(result.nodes[1].uy, expected_tip_uy);
    assert_close(result.nodes[1].uz, 0.0);
    assert_close(result.nodes[1].rx, 0.0);
    assert_close(result.nodes[1].ry, 0.0);
    assert_close(result.nodes[1].rz, expected_tip_rz);
    assert_close(result.max_displacement, expected_tip_uy.abs());
    assert_close(result.max_rotation, expected_tip_rz.abs());
    assert_close(result.max_moment, expected_moment);
    assert_close(result.max_stress, expected_stress);

    let element = &result.elements[0];
    assert_close(element.length, 2.0);
    assert_close(element.axial_force_i, 0.0);
    assert_close(element.axial_force_j, 0.0);
    assert_close(element.shear_force_y_i, 1000.0);
    assert_close(element.shear_force_y_j, -1000.0);
    assert_close(element.shear_force_z_i, 0.0);
    assert_close(element.shear_force_z_j, 0.0);
    assert_close(element.torsion_i, 0.0);
    assert_close(element.torsion_j, 0.0);
    assert_close(element.moment_y_i, 0.0);
    assert_close(element.moment_y_j, 0.0);
    assert_close(element.moment_z_i, 2000.0);
    assert_close(element.moment_z_j, 0.0);
    assert_close(element.axial_stress, 0.0);
    assert_close(element.max_bending_stress, expected_stress);
    assert_close(element.max_combined_stress, expected_stress);
}

#[allow(clippy::too_many_arguments)]
fn node(
    id: &str,
    x: f64,
    y: f64,
    z: f64,
    fix_x: bool,
    fix_y: bool,
    fix_z: bool,
    fix_rx: bool,
    fix_ry: bool,
    fix_rz: bool,
    load_x: f64,
    load_y: f64,
    load_z: f64,
    moment_x: f64,
    moment_y: f64,
    moment_z: f64,
) -> Frame3dNodeInput {
    Frame3dNodeInput {
        id: id.to_string(),
        x,
        y,
        z,
        fix_x,
        fix_y,
        fix_z,
        fix_rx,
        fix_ry,
        fix_rz,
        load_x,
        load_y,
        load_z,
        moment_x,
        moment_y,
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
