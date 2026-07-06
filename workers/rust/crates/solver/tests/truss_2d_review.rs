use kyuubiki_protocol::{SolveTruss2dRequest, TrussElementInput, TrussNodeInput};
use kyuubiki_solver::solve_truss_2d;

const TOL: f64 = 1.0e-7;

#[test]
fn truss_2d_review_bundle_checks_supports_member_forces_and_loaded_node_balance() {
    let request = SolveTruss2dRequest {
        nodes: vec![
            node("left_support", 0.0, 0.0, true, true, 0.0, 0.0),
            node("right_roller", 1.0, 0.0, false, true, 0.0, 0.0),
            node("loaded_apex", 0.5, 0.75, false, false, 0.0, -1000.0),
        ],
        elements: vec![
            element("left_web", 0, 2),
            element("right_web", 1, 2),
            element("bottom_chord", 0, 1),
        ],
    };

    let result = solve_truss_2d(&request).expect("review truss should solve");

    assert_eq!(result.nodes.len(), 3);
    assert_eq!(result.elements.len(), 3);
    assert_close(result.nodes[0].ux, 0.0, 1.0e-12);
    assert_close(result.nodes[0].uy, 0.0, 1.0e-12);
    assert_close(result.nodes[1].uy, 0.0, 1.0e-12);
    assert!(result.nodes[2].uy < 0.0, "loaded apex should move downward");
    assert!(result.max_displacement > 0.0);
    assert!(result.max_stress > 0.0);

    for element in &result.elements {
        assert!(element.length > 0.0);
        assert!(element.strain.is_finite());
        assert!(element.stress.is_finite());
        assert!(element.axial_force.is_finite());
    }

    let (internal_x, internal_y) = loaded_node_internal_force(&request, &result.elements, 2);
    assert_close(internal_x + request.nodes[2].load_x, 0.0, TOL);
    assert_close(internal_y + request.nodes[2].load_y, 0.0, TOL);
}

fn loaded_node_internal_force(
    request: &SolveTruss2dRequest,
    elements: &[kyuubiki_protocol::TrussElementResult],
    node_index: usize,
) -> (f64, f64) {
    let mut force_x = 0.0;
    let mut force_y = 0.0;
    for element in elements {
        if element.node_i != node_index && element.node_j != node_index {
            continue;
        }
        let node_i = &request.nodes[element.node_i];
        let node_j = &request.nodes[element.node_j];
        let dx = node_j.x - node_i.x;
        let dy = node_j.y - node_i.y;
        let length = (dx * dx + dy * dy).sqrt();
        let c = dx / length;
        let s = dy / length;
        let sign = if element.node_i == node_index { 1.0 } else { -1.0 };
        force_x += sign * element.axial_force * c;
        force_y += sign * element.axial_force * s;
    }
    (force_x, force_y)
}

fn node(
    id: &str,
    x: f64,
    y: f64,
    fix_x: bool,
    fix_y: bool,
    load_x: f64,
    load_y: f64,
) -> TrussNodeInput {
    TrussNodeInput {
        id: id.to_string(),
        x,
        y,
        fix_x,
        fix_y,
        load_x,
        load_y,
    }
}

fn element(id: &str, node_i: usize, node_j: usize) -> TrussElementInput {
    TrussElementInput {
        id: id.to_string(),
        node_i,
        node_j,
        area: 0.01,
        youngs_modulus: 70.0e9,
    }
}

fn assert_close(actual: f64, expected: f64, tolerance: f64) {
    let scale = expected.abs().max(1.0);
    assert!(
        (actual - expected).abs() <= tolerance * scale,
        "expected {actual} to be close to {expected}",
    );
}
