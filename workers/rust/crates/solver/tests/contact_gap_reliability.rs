use kyuubiki_protocol::{
    ContactGap1dContactInput, NonlinearSpring1dElementInput, NonlinearSpring1dNodeInput,
    SolveContactGap1dRequest,
};
use kyuubiki_solver::solve_contact_gap_1d;

#[test]
fn solves_active_gap_contact_response() {
    let result = solve_contact_gap_1d(&SolveContactGap1dRequest {
        nodes: vec![
            node("fixed", 0.0, true, 0.0),
            node("tip", 1.0, false, 100.0),
        ],
        elements: vec![NonlinearSpring1dElementInput {
            id: "spring".to_string(),
            node_i: 0,
            node_j: 1,
            stiffness: 1000.0,
            cubic_stiffness: 0.0,
        }],
        contacts: vec![ContactGap1dContactInput {
            id: "stop".to_string(),
            node: 1,
            gap: 0.05,
            normal_stiffness: 10_000.0,
        }],
        load_steps: Some(6),
        max_iterations: Some(32),
        tolerance: Some(1.0e-9),
    })
    .expect("contact solve should converge");

    assert!(result.converged);
    assert_eq!(result.active_contact_count, 1);
    assert!(result.contacts[0].active);
    assert!(result.contacts[0].force > 0.0);
    assert!(result.max_displacement > result.contacts[0].gap);
}

#[test]
fn contact_gap_1d_rejects_non_finite_node_and_contact_inputs() {
    let mut request = valid_contact_request();
    request.nodes[1].load_x = f64::INFINITY;
    let error =
        solve_contact_gap_1d(&request).expect_err("non-finite contact load should be rejected");
    assert!(
        error.contains("coordinates and loads must be finite"),
        "unexpected non-finite node error: {error}"
    );

    let mut request = valid_contact_request();
    request.contacts[0].gap = f64::NAN;
    let error =
        solve_contact_gap_1d(&request).expect_err("non-finite contact gap should be rejected");
    assert!(
        error.contains("gap must be finite"),
        "unexpected non-finite gap error: {error}"
    );

    let mut request = valid_contact_request();
    request.contacts[0].normal_stiffness = 0.0;
    let error =
        solve_contact_gap_1d(&request).expect_err("zero contact stiffness should be rejected");
    assert!(
        error.contains("normal stiffness must be positive"),
        "unexpected zero stiffness error: {error}"
    );
}

fn valid_contact_request() -> SolveContactGap1dRequest {
    SolveContactGap1dRequest {
        nodes: vec![
            node("fixed", 0.0, true, 0.0),
            node("tip", 1.0, false, 100.0),
        ],
        elements: vec![NonlinearSpring1dElementInput {
            id: "spring".to_string(),
            node_i: 0,
            node_j: 1,
            stiffness: 1000.0,
            cubic_stiffness: 0.0,
        }],
        contacts: vec![ContactGap1dContactInput {
            id: "stop".to_string(),
            node: 1,
            gap: 0.05,
            normal_stiffness: 10_000.0,
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
