use kyuubiki_protocol::{
    ContactGap1dContactInput, NonlinearSpring1dElementInput, NonlinearSpring1dNodeInput,
    SolveContactGap1dRequest,
};
use kyuubiki_solver::solve_contact_gap_1d;

const TOL: f64 = 1.0e-10;

#[test]
fn contact_gap_1d_matches_inactive_gap_closed_form() {
    let spring_stiffness = 1000.0;
    let gap = 0.05;
    let load = 25.0;
    let result = solve_contact_gap_1d(&request(load, spring_stiffness, gap, 10_000.0))
        .expect("inactive contact fixture should solve");

    let expected_tip = load / spring_stiffness;
    assert!(result.converged);
    assert_eq!(result.active_contact_count, 0);
    assert_close(result.nodes[1].ux, expected_tip);
    assert_close(result.elements[0].force, load);
    assert_close(result.contacts[0].penetration, 0.0);
    assert_close(result.contacts[0].force, 0.0);
    assert!(!result.contacts[0].active);
    assert!(expected_tip < gap);
}

#[test]
fn contact_gap_1d_matches_active_penalty_stop_closed_form() {
    let spring_stiffness = 1000.0;
    let contact_stiffness = 10_000.0;
    let gap = 0.05;
    let load = 100.0;
    let result = solve_contact_gap_1d(&request(load, spring_stiffness, gap, contact_stiffness))
        .expect("active contact fixture should solve");

    let expected_tip = (load + contact_stiffness * gap) / (spring_stiffness + contact_stiffness);
    let expected_penetration = expected_tip - gap;
    let expected_spring_force = spring_stiffness * expected_tip;
    let expected_contact_force = contact_stiffness * expected_penetration;
    assert!(result.converged);
    assert_eq!(result.active_contact_count, 1);
    assert_close(result.nodes[1].ux, expected_tip);
    assert_close(result.elements[0].force, expected_spring_force);
    assert_close(result.contacts[0].penetration, expected_penetration);
    assert_close(result.contacts[0].force, expected_contact_force);
    assert_close(expected_spring_force + expected_contact_force, load);
    assert!(result.contacts[0].active);
}

fn request(
    load: f64,
    spring_stiffness: f64,
    gap: f64,
    contact_stiffness: f64,
) -> SolveContactGap1dRequest {
    SolveContactGap1dRequest {
        nodes: vec![node("fixed", 0.0, true, 0.0), node("tip", 1.0, false, load)],
        elements: vec![NonlinearSpring1dElementInput {
            id: "spring".to_string(),
            node_i: 0,
            node_j: 1,
            stiffness: spring_stiffness,
            cubic_stiffness: 0.0,
        }],
        contacts: vec![ContactGap1dContactInput {
            id: "stop".to_string(),
            node: 1,
            gap,
            normal_stiffness: contact_stiffness,
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

fn assert_close(actual: f64, expected: f64) {
    let scale = expected.abs().max(1.0);
    assert!(
        (actual - expected).abs() <= TOL * scale,
        "expected {actual} to be close to {expected}",
    );
}
