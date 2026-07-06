use kyuubiki_protocol::{
    ContactGap1dContactInput, NonlinearSpring1dElementInput, NonlinearSpring1dNodeInput,
    SolveContactGap1dRequest,
};
use kyuubiki_solver::solve_contact_gap_1d;

const TOL: f64 = 1.0e-10;

#[test]
fn contact_gap_1d_review_bundle_checks_active_stop_penetration_and_force_split() {
    let result = solve_contact_gap_1d(&SolveContactGap1dRequest {
        nodes: vec![node("fixed", 0.0, true, 0.0), node("tip", 1.0, false, 100.0)],
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
    .expect("review contact gap should solve");

    let expected_tip_displacement = 600.0 / 11_000.0;
    let expected_spring_force = 1000.0 * expected_tip_displacement;
    let expected_penetration = expected_tip_displacement - 0.05;
    let expected_contact_force = 10_000.0 * expected_penetration;

    assert!(result.converged);
    assert_eq!(result.steps.len(), 6);
    assert_eq!(result.active_contact_count, 1);
    assert_eq!(result.nodes.len(), 2);
    assert_eq!(result.elements.len(), 1);
    assert_eq!(result.contacts.len(), 1);
    assert_close(result.nodes[0].ux, 0.0);
    assert_close(result.nodes[1].ux, expected_tip_displacement);
    assert_close(result.max_displacement, expected_tip_displacement);
    assert_close(result.max_force, expected_spring_force);
    assert_close(result.max_contact_force, expected_contact_force);
    assert!(result.residual_norm <= 1.0e-9);

    let element = &result.elements[0];
    assert_close(element.length, 1.0);
    assert_close(element.extension, expected_tip_displacement);
    assert_close(element.force, expected_spring_force);
    assert_close(element.tangent_stiffness, 1000.0);

    let contact = &result.contacts[0];
    assert!(contact.active);
    assert_close(contact.gap, 0.05);
    assert_close(contact.penetration, expected_penetration);
    assert_close(contact.force, expected_contact_force);
    assert_close(element.force + contact.force, 100.0);
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
