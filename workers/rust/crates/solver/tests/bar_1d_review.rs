use kyuubiki_protocol::SolveBarRequest;
use kyuubiki_solver::solve_bar_1d;

const TOL: f64 = 1.0e-12;

#[test]
fn bar_1d_review_bundle_exposes_assumptions_boundaries_and_diagnostics() {
    let request = SolveBarRequest {
        length: 2.0,
        area: 0.02,
        youngs_modulus: 200.0e9,
        elements: 4,
        tip_force: 4_000.0,
    };

    let result = solve_bar_1d(&request).expect("review bar should solve");

    let element_length = request.length / request.elements as f64;
    let expected_stress = request.tip_force / request.area;
    let expected_tip_displacement =
        request.tip_force * request.length / (request.youngs_modulus * request.area);

    assert_eq!(result.nodes.len(), request.elements + 1);
    assert_eq!(result.elements.len(), request.elements);
    assert_close(result.nodes[0].displacement, 0.0);
    assert_close(result.tip_displacement, expected_tip_displacement);
    assert_close(result.reaction_force, -request.tip_force);
    assert_close(result.max_stress, expected_stress);
    assert_close(result.max_displacement, expected_tip_displacement);

    for (index, element) in result.elements.iter().enumerate() {
        assert_close(element.x2 - element.x1, element_length);
        assert_close(element.stress, expected_stress);
        assert_close(element.axial_force, request.tip_force);
        assert_close(element.strain, expected_stress / request.youngs_modulus);
        assert_eq!(element.index, index);
    }
}

fn assert_close(actual: f64, expected: f64) {
    let scale = expected.abs().max(1.0);
    assert!(
        (actual - expected).abs() <= TOL * scale,
        "expected {actual} to be close to {expected}",
    );
}
