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

fn node(id: &str, x: f64, fix_x: bool, load_x: f64) -> NonlinearSpring1dNodeInput {
    NonlinearSpring1dNodeInput {
        id: id.to_string(),
        x,
        fix_x,
        load_x,
    }
}
