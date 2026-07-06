use kyuubiki_protocol::{SolveSpring3dRequest, Spring3dElementInput, Spring3dNodeInput};
use kyuubiki_solver::solve_spring_3d;

const TOL: f64 = 1.0e-10;

#[test]
fn spring_3d_review_bundle_checks_cage_supports_top_displacement_and_member_forces() {
    let result = solve_spring_3d(&SolveSpring3dRequest {
        nodes: vec![
            node("base-a", 0.0, 0.0, 0.0, true, true, true, 0.0, 0.0, 0.0),
            node("base-b", 1.2, 0.0, 0.0, true, true, true, 0.0, 0.0, 0.0),
            node("base-c", 0.0, 1.0, 0.0, true, true, true, 0.0, 0.0, 0.0),
            node(
                "top", 0.45, 0.35, 1.1, false, false, false, 250.0, 0.0, -1100.0,
            ),
        ],
        elements: vec![
            element("leg-a", 0, 3, 18_000.0),
            element("leg-b", 1, 3, 22_000.0),
            element("leg-c", 2, 3, 16_000.0),
            element("base-ab", 0, 1, 9_000.0),
            element("base-bc", 1, 2, 9_000.0),
            element("base-ca", 2, 0, 9_000.0),
        ],
    })
    .expect("review 3d spring cage should solve");

    let top = &result.nodes[3];
    assert_eq!(result.nodes.len(), 4);
    assert_eq!(result.elements.len(), 6);
    for fixed in &result.nodes[0..3] {
        assert_close(fixed.ux, 0.0);
        assert_close(fixed.uy, 0.0);
        assert_close(fixed.uz, 0.0);
    }
    assert_close(top.ux, 0.037_134_189_113_355_795);
    assert_close(top.uy, 0.034_455_439_814_814_82);
    assert_close(top.uz, -0.031_322_703_837_618_61);
    assert_close(result.max_displacement, 0.059_558_686_265_212_11);
    assert_close(result.max_force, 803.010_827_379_611_9);

    let leg_a = &result.elements[0];
    let leg_b = &result.elements[1];
    let leg_c = &result.elements[2];
    assert_close(leg_a.force, -82.596_744_622_425_67);
    assert_close(leg_b.force, -803.010_827_379_611_9);
    assert_close(leg_c.force, -474.117_601_445_042_34);
    assert_close(leg_a.extension, leg_a.force / 18_000.0);
    assert_close(leg_b.extension, leg_b.force / 22_000.0);
    assert_close(leg_c.extension, leg_c.force / 16_000.0);
    assert!(leg_a.length > 1.0);
    assert!(leg_b.length > 1.0);
    assert!(leg_c.length > 1.0);
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
    load_x: f64,
    load_y: f64,
    load_z: f64,
) -> Spring3dNodeInput {
    Spring3dNodeInput {
        id: id.to_string(),
        x,
        y,
        z,
        fix_x,
        fix_y,
        fix_z,
        load_x,
        load_y,
        load_z,
    }
}

fn element(id: &str, node_i: usize, node_j: usize, stiffness: f64) -> Spring3dElementInput {
    Spring3dElementInput {
        id: id.to_string(),
        node_i,
        node_j,
        stiffness,
    }
}

fn assert_close(actual: f64, expected: f64) {
    let scale = expected.abs().max(1.0);
    assert!(
        (actual - expected).abs() <= TOL * scale,
        "expected {actual} to be close to {expected}",
    );
}
