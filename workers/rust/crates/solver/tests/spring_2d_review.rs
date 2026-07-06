use kyuubiki_protocol::{SolveSpring2dRequest, Spring2dElementInput, Spring2dNodeInput};
use kyuubiki_solver::solve_spring_2d;

const TOL: f64 = 1.0e-10;

#[test]
fn spring_2d_review_bundle_checks_grid_supports_member_extensions_and_diagonal_force() {
    let result = solve_spring_2d(&SolveSpring2dRequest {
        nodes: vec![
            node("fixed-left-bottom", 0.0, 0.0, true, true, 0.0, 0.0),
            node("roller-right-bottom", 1.0, 0.0, false, true, 0.0, 0.0),
            node("loaded-right-top", 1.0, 1.0, false, false, 1200.0, -600.0),
            node("roller-left-top", 0.0, 1.0, true, false, 0.0, 0.0),
        ],
        elements: vec![
            element("bottom", 0, 1, 25_000.0),
            element("right", 1, 2, 18_000.0),
            element("top", 2, 3, 22_000.0),
            element("left", 3, 0, 18_000.0),
            element("diagonal", 0, 2, 12_000.0),
        ],
    })
    .expect("review 2d spring grid should solve");

    let loaded = &result.nodes[2];
    assert_eq!(result.nodes.len(), 4);
    assert_eq!(result.elements.len(), 5);
    assert_close(result.nodes[0].ux, 0.0);
    assert_close(result.nodes[0].uy, 0.0);
    assert_close(result.nodes[1].uy, 0.0);
    assert_close(result.nodes[3].ux, 0.0);
    assert_close(loaded.ux, 0.050_943_396_226_415_1);
    assert_close(loaded.uy, -0.037_735_849_056_603_77);
    assert_close(result.max_displacement, 0.063_397_349_495_892_24);
    assert_close(result.max_force, 1120.754_716_981_132);

    let right = &result.elements[1];
    let top = &result.elements[2];
    let diagonal = &result.elements[4];
    assert_close(right.length, 1.0);
    assert_close(top.length, 1.0);
    assert_close(diagonal.length, 2.0_f64.sqrt());
    assert_close(right.force, -679.245_283_018_867_9);
    assert_close(top.force, 1120.754_716_981_132);
    assert_close(diagonal.force, 112.069_753_999_377_37);
    assert_close(right.extension, right.force / 18_000.0);
    assert_close(top.extension, top.force / 22_000.0);
    assert_close(diagonal.extension, diagonal.force / 12_000.0);
}

fn node(
    id: &str,
    x: f64,
    y: f64,
    fix_x: bool,
    fix_y: bool,
    load_x: f64,
    load_y: f64,
) -> Spring2dNodeInput {
    Spring2dNodeInput {
        id: id.to_string(),
        x,
        y,
        fix_x,
        fix_y,
        load_x,
        load_y,
    }
}

fn element(id: &str, node_i: usize, node_j: usize, stiffness: f64) -> Spring2dElementInput {
    Spring2dElementInput {
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
