use kyuubiki_protocol::{
    SolveHarmonicSpring1dRequest, TransientSpring1dElementInput, TransientSpring1dNodeInput,
};
use kyuubiki_solver::solve_harmonic_spring_1d;

#[test]
fn harmonic_spring_1d_rejects_non_finite_frequency_and_node_state() {
    let mut request = harmonic_spring_request();
    request.frequencies_hz[0] = f64::NAN;
    let error =
        solve_harmonic_spring_1d(&request).expect_err("non-finite frequency should be rejected");
    assert!(
        error.contains("frequency 0 must be non-negative and finite"),
        "unexpected frequency error: {error}"
    );

    let mut request = harmonic_spring_request();
    request.nodes[1].initial_displacement = f64::INFINITY;
    let error =
        solve_harmonic_spring_1d(&request).expect_err("non-finite node state should be rejected");
    assert!(
        error.contains("finite coordinates, load, initial state, and positive mass"),
        "unexpected node-state error: {error}"
    );
}

#[test]
fn harmonic_spring_1d_rejects_invalid_element_and_degenerate_length() {
    let mut request = harmonic_spring_request();
    request.elements[0].node_j = 9;
    let error = solve_harmonic_spring_1d(&request).expect_err("missing node should be rejected");
    assert!(
        error.contains("references missing node 9"),
        "unexpected missing-node error: {error}"
    );

    let mut request = harmonic_spring_request();
    request.elements[0].damping = -1.0;
    let error =
        solve_harmonic_spring_1d(&request).expect_err("negative damping should be rejected");
    assert!(
        error.contains("valid connectivity, stiffness, and damping"),
        "unexpected damping error: {error}"
    );

    let mut request = harmonic_spring_request();
    request.nodes[1].x = request.nodes[0].x;
    let error =
        solve_harmonic_spring_1d(&request).expect_err("zero-length element should be rejected");
    assert!(
        error.contains("length must be positive"),
        "unexpected zero-length error: {error}"
    );
}

fn harmonic_spring_request() -> SolveHarmonicSpring1dRequest {
    SolveHarmonicSpring1dRequest {
        nodes: vec![
            node("fixed", 0.0, true, 0.0, 1.0, 0.0, 0.0),
            node("tip", 1.0, false, 10.0, 2.0, 0.0, 0.0),
        ],
        elements: vec![TransientSpring1dElementInput {
            id: "s0".to_string(),
            node_i: 0,
            node_j: 1,
            stiffness: 100.0,
            damping: 0.5,
        }],
        frequencies_hz: vec![2.0],
    }
}

fn node(
    id: &str,
    x: f64,
    fix_x: bool,
    load_x: f64,
    mass: f64,
    initial_displacement: f64,
    initial_velocity: f64,
) -> TransientSpring1dNodeInput {
    TransientSpring1dNodeInput {
        id: id.to_string(),
        x,
        fix_x,
        load_x,
        mass,
        initial_displacement,
        initial_velocity,
    }
}
