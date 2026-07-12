use kyuubiki_protocol::{
    NonlinearSpring1dElementInput, NonlinearSpring1dNodeInput, SolveNonlinearSpring1dRequest,
};
use kyuubiki_solver::solve_nonlinear_spring_1d;

#[test]
fn solves_hardening_nonlinear_spring_chain() {
    let result = solve_nonlinear_spring_1d(&SolveNonlinearSpring1dRequest {
        nodes: vec![
            node("fixed", 0.0, true, 0.0),
            node("tip", 1.0, false, 100.0),
        ],
        elements: vec![NonlinearSpring1dElementInput {
            id: "nl0".to_string(),
            node_i: 0,
            node_j: 1,
            stiffness: 1000.0,
            cubic_stiffness: 50_000.0,
        }],
        load_steps: Some(6),
        max_iterations: Some(32),
        tolerance: Some(1.0e-9),
    })
    .expect("nonlinear spring solve should converge");

    assert!(result.converged);
    assert_eq!(result.steps.len(), 6);
    assert!(result.max_displacement > 0.0);
    assert!(result.max_displacement < 0.1);
    assert!(result.elements[0].tangent_stiffness > result.input.elements[0].stiffness);
}

#[test]
fn nonlinear_spring_1d_rejects_non_finite_node_and_degenerate_element_inputs() {
    let mut request = valid_request();
    request.nodes[1].load_x = f64::NAN;
    let error = solve_nonlinear_spring_1d(&request)
        .expect_err("non-finite nonlinear spring load should be rejected");
    assert!(
        error.contains("coordinates and loads must be finite"),
        "unexpected non-finite node error: {error}"
    );

    let mut request = valid_request();
    request.nodes[1].x = request.nodes[0].x;
    let error = solve_nonlinear_spring_1d(&request)
        .expect_err("zero-length nonlinear spring should be rejected");
    assert!(
        error.contains("length must be positive"),
        "unexpected zero-length error: {error}"
    );
}

fn valid_request() -> SolveNonlinearSpring1dRequest {
    SolveNonlinearSpring1dRequest {
        nodes: vec![
            node("fixed", 0.0, true, 0.0),
            node("tip", 1.0, false, 100.0),
        ],
        elements: vec![NonlinearSpring1dElementInput {
            id: "nl0".to_string(),
            node_i: 0,
            node_j: 1,
            stiffness: 1000.0,
            cubic_stiffness: 50_000.0,
        }],
        load_steps: Some(6),
        max_iterations: Some(32),
        tolerance: Some(1.0e-9),
    }
}

fn node(id: &str, x: f64, fix_x: bool, load_x: f64) -> NonlinearSpring1dNodeInput {
    NonlinearSpring1dNodeInput {
        id: id.to_string(),
        x,
        fix_x,
        load_x,
    }
}
