use kyuubiki_protocol::{
    NonlinearSpring1dElementInput, NonlinearSpring1dNodeInput, SolveNonlinearSpring1dRequest,
};
use kyuubiki_solver::solve_nonlinear_spring_1d;

const TOL: f64 = 1.0e-10;

#[test]
fn nonlinear_spring_1d_review_bundle_checks_load_steps_convergence_and_tangent_response() {
    let result = solve_nonlinear_spring_1d(&SolveNonlinearSpring1dRequest {
        nodes: vec![node("fixed", 0.0, true, 0.0), node("tip", 1.0, false, 100.0)],
        elements: vec![NonlinearSpring1dElementInput {
            id: "hardening".to_string(),
            node_i: 0,
            node_j: 1,
            stiffness: 1000.0,
            cubic_stiffness: 50_000.0,
        }],
        load_steps: Some(6),
        max_iterations: Some(32),
        tolerance: Some(1.0e-9),
    })
    .expect("review nonlinear spring should solve");

    let expected_tip_displacement = 0.077_091_699_705_924_8;
    let expected_tangent_stiffness = 1891.469_524_532_273;

    assert!(result.converged);
    assert_eq!(result.steps.len(), 6);
    assert_eq!(result.nodes.len(), 2);
    assert_eq!(result.elements.len(), 1);
    assert_close(result.nodes[0].ux, 0.0);
    assert_close(result.nodes[1].ux, expected_tip_displacement);
    assert_close(result.max_displacement, expected_tip_displacement);
    assert_close(result.max_force, 100.0);
    assert!(result.residual_norm <= 1.0e-9);

    for (index, step) in result.steps.iter().enumerate() {
        assert_eq!(step.step, index + 1);
        assert_close(step.load_factor, (index + 1) as f64 / 6.0);
        assert!(step.converged);
        assert!(step.iterations <= 32);
    }

    let element = &result.elements[0];
    assert_close(element.length, 1.0);
    assert_close(element.extension, expected_tip_displacement);
    assert_close(element.force, 100.0);
    assert_close(element.tangent_stiffness, expected_tangent_stiffness);
    assert!(element.tangent_stiffness > result.input.elements[0].stiffness);
}

fn node(id: &str, x: f64, fix_x: bool, load_x: f64) -> NonlinearSpring1dNodeInput {
    NonlinearSpring1dNodeInput {
        id: id.to_string(),
        x,
        fix_x,
        load_x,
    }
}

fn assert_close(actual: f64, expected: f64) {
    let scale = expected.abs().max(1.0);
    assert!(
        (actual - expected).abs() <= TOL * scale,
        "expected {actual} to be close to {expected}",
    );
}
