use kyuubiki_protocol::{SolveSpring1dRequest, Spring1dElementInput, Spring1dNodeInput};
use kyuubiki_solver::solve_spring_1d;

const TOL: f64 = 1.0e-10;

#[test]
fn spring_1d_review_bundle_checks_series_chain_boundaries_extensions_and_member_forces() {
    let result = solve_spring_1d(&SolveSpring1dRequest {
        nodes: vec![
            node("fixed", 0.0, true, 0.0),
            node("joint", 1.2, false, 0.0),
            node("loaded", 2.4, false, 1200.0),
        ],
        elements: vec![
            element("soft-left", 0, 1, 35_000.0),
            element("soft-right", 1, 2, 20_000.0),
        ],
    })
    .expect("review spring chain should solve");

    let expected_left_extension = 1200.0 / 35_000.0;
    let expected_right_extension = 1200.0 / 20_000.0;
    let expected_tip_displacement = expected_left_extension + expected_right_extension;

    assert_eq!(result.nodes.len(), 3);
    assert_eq!(result.elements.len(), 2);
    assert_close(result.nodes[0].ux, 0.0);
    assert_close(result.nodes[1].ux, expected_left_extension);
    assert_close(result.nodes[2].ux, expected_tip_displacement);
    assert_close(result.max_displacement, expected_tip_displacement);
    assert_close(result.max_force, 1200.0);

    let left = &result.elements[0];
    let right = &result.elements[1];
    assert_close(left.length, 1.2);
    assert_close(right.length, 1.2);
    assert_close(left.extension, expected_left_extension);
    assert_close(right.extension, expected_right_extension);
    assert_close(left.force, 1200.0);
    assert_close(right.force, 1200.0);

    let equivalent_stiffness = 1.0 / (1.0 / 35_000.0 + 1.0 / 20_000.0);
    assert_close(equivalent_stiffness * result.nodes[2].ux, 1200.0);
}

fn node(id: &str, x: f64, fix_x: bool, load_x: f64) -> Spring1dNodeInput {
    Spring1dNodeInput {
        id: id.to_string(),
        x,
        fix_x,
        load_x,
    }
}

fn element(id: &str, node_i: usize, node_j: usize, stiffness: f64) -> Spring1dElementInput {
    Spring1dElementInput {
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
