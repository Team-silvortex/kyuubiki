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
    assert_penalty_contact_law(&result);
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
    assert_penalty_contact_law(&result);
}

#[test]
fn contact_gap_1d_preserves_active_force_split_under_load_gap_scaling() {
    let spring_stiffness = 1200.0;
    let contact_stiffness = 9000.0;
    let gap = 0.04;
    let load = 88.0;
    let baseline = solve_contact_gap_1d(&request(load, spring_stiffness, gap, contact_stiffness))
        .expect("baseline active contact fixture should solve");

    let scale = 1.75;
    let scaled = solve_contact_gap_1d(&request(
        load * scale,
        spring_stiffness,
        gap * scale,
        contact_stiffness,
    ))
    .expect("scaled active contact fixture should solve");

    assert!(baseline.converged);
    assert!(scaled.converged);
    assert_eq!(baseline.active_contact_count, 1);
    assert_eq!(scaled.active_contact_count, 1);
    assert!(baseline.contacts[0].active);
    assert!(scaled.contacts[0].active);
    assert_close(scaled.nodes[1].ux / baseline.nodes[1].ux, scale);
    assert_close(
        scaled.contacts[0].penetration / baseline.contacts[0].penetration,
        scale,
    );
    assert_close(scaled.elements[0].force / baseline.elements[0].force, scale);
    assert_close(scaled.contacts[0].force / baseline.contacts[0].force, scale);
    assert_close(
        scaled.elements[0].force + scaled.contacts[0].force,
        load * scale,
    );
    assert_penalty_contact_law(&baseline);
    assert_penalty_contact_law(&scaled);
}

#[test]
fn contact_gap_1d_tracks_contact_stiffness_force_split() {
    let spring_stiffness = 1300.0;
    let contact_stiffness = 8000.0;
    let gap = 0.035;
    let load = 95.0;
    let baseline = solve_contact_gap_1d(&request(load, spring_stiffness, gap, contact_stiffness))
        .expect("baseline active contact fixture should solve");

    let stiffness_scale = 2.25;
    let stiffer = solve_contact_gap_1d(&request(
        load,
        spring_stiffness,
        gap,
        contact_stiffness * stiffness_scale,
    ))
    .expect("contact-stiffness-scaled fixture should solve");

    let expected_tip = (load + contact_stiffness * stiffness_scale * gap)
        / (spring_stiffness + contact_stiffness * stiffness_scale);
    let expected_penetration = expected_tip - gap;
    let expected_spring_force = spring_stiffness * expected_tip;
    let expected_contact_force = contact_stiffness * stiffness_scale * expected_penetration;

    assert!(baseline.converged);
    assert!(stiffer.converged);
    assert_eq!(baseline.active_contact_count, 1);
    assert_eq!(stiffer.active_contact_count, 1);
    assert!(stiffer.contacts[0].active);
    assert!(stiffer.contacts[0].penetration < baseline.contacts[0].penetration);
    assert!(stiffer.contacts[0].force > baseline.contacts[0].force);
    assert_close(stiffer.nodes[1].ux, expected_tip);
    assert_close(stiffer.contacts[0].penetration, expected_penetration);
    assert_close(stiffer.elements[0].force, expected_spring_force);
    assert_close(stiffer.contacts[0].force, expected_contact_force);
    assert_close(expected_spring_force + expected_contact_force, load);
    assert_penalty_contact_law(&baseline);
    assert_penalty_contact_law(&stiffer);

    let length_scale = 2.0;
    let longer = solve_contact_gap_1d(&request_with_length(
        length_scale,
        load,
        spring_stiffness,
        gap,
        contact_stiffness,
    ))
    .expect("geometry-scaled contact fixture should solve");
    assert!(longer.converged);
    assert_eq!(longer.active_contact_count, 1);
    assert_close(longer.elements[0].length, length_scale);
    assert_close(longer.nodes[1].ux, baseline.nodes[1].ux);
    assert_close(
        longer.contacts[0].penetration,
        baseline.contacts[0].penetration,
    );
    assert_close(longer.elements[0].force, baseline.elements[0].force);
    assert_close(longer.contacts[0].force, baseline.contacts[0].force);
    assert_penalty_contact_law(&longer);
}

fn request(
    load: f64,
    spring_stiffness: f64,
    gap: f64,
    contact_stiffness: f64,
) -> SolveContactGap1dRequest {
    request_with_length(1.0, load, spring_stiffness, gap, contact_stiffness)
}

fn request_with_length(
    length: f64,
    load: f64,
    spring_stiffness: f64,
    gap: f64,
    contact_stiffness: f64,
) -> SolveContactGap1dRequest {
    SolveContactGap1dRequest {
        nodes: vec![
            node("fixed", 0.0, true, 0.0),
            node("tip", length, false, load),
        ],
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

fn assert_penalty_contact_law(result: &kyuubiki_protocol::SolveContactGap1dResult) {
    let active_count = result
        .contacts
        .iter()
        .filter(|contact| contact.active)
        .count();
    assert_eq!(result.active_contact_count, active_count);
    assert_close(
        result.max_contact_force,
        result
            .contacts
            .iter()
            .map(|contact| contact.force.abs())
            .fold(0.0_f64, f64::max),
    );

    for contact in &result.contacts {
        let input = &result.input.contacts[contact.index];
        let node = &result.nodes[contact.node];
        let expected_penetration = (node.ux - input.gap).max(0.0);
        assert_close(contact.gap, input.gap);
        assert_close(contact.penetration, expected_penetration);
        assert_close(contact.force, input.normal_stiffness * expected_penetration);
        assert_eq!(contact.active, expected_penetration > 0.0);
        assert!(contact.penetration >= 0.0);
        assert!(contact.force >= 0.0);
    }

    let external_load = result
        .input
        .nodes
        .iter()
        .map(|node| node.load_x)
        .sum::<f64>();
    let spring_force = result
        .elements
        .iter()
        .map(|element| element.force)
        .sum::<f64>();
    let contact_force = result
        .contacts
        .iter()
        .map(|contact| contact.force)
        .sum::<f64>();
    assert_close(spring_force + contact_force, external_load);
}

fn assert_close(actual: f64, expected: f64) {
    let scale = expected.abs().max(1.0);
    assert!(
        (actual - expected).abs() <= TOL * scale,
        "expected {actual} to be close to {expected}",
    );
}
