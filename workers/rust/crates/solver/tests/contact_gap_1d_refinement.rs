use kyuubiki_protocol::{
    ContactGap1dContactInput, NonlinearSpring1dElementInput, NonlinearSpring1dNodeInput,
    SolveContactGap1dRequest,
};
use kyuubiki_solver::solve_contact_gap_1d;

const TOL: f64 = 1.0e-8;
const LENGTH: f64 = 1.5;

#[test]
fn inactive_gap_chain_is_refinement_invariant() {
    let case = ContactCase {
        load: 25.0,
        spring_stiffness: 1_000.0,
        gap: 0.05,
        contact_stiffness: 10_000.0,
    };
    let expected_tip = case.load / case.spring_stiffness;
    assert!(expected_tip < case.gap);

    for elements in [1_usize, 2, 4, 8, 16] {
        let result = solve_contact_gap_1d(&mesh(elements, case))
            .expect("inactive refined contact chain should solve");
        assert!(result.converged);
        assert_eq!(result.active_contact_count, 0);
        assert_close(result.max_displacement, expected_tip);
        assert_close(result.max_force, case.load);
        assert_close(result.contacts[0].penetration, 0.0);
        assert_close(result.contacts[0].force, 0.0);
        assert!(!result.contacts[0].active);
        assert_refined_spring_chain(&result, elements, expected_tip, case.load);
    }
}

#[test]
fn active_penalty_stop_chain_is_refinement_invariant() {
    let case = ContactCase {
        load: 88.0,
        spring_stiffness: 1_200.0,
        gap: 0.04,
        contact_stiffness: 9_000.0,
    };
    let expected_tip = (case.load + case.contact_stiffness * case.gap)
        / (case.spring_stiffness + case.contact_stiffness);
    let expected_penetration = expected_tip - case.gap;
    let expected_spring_force = case.spring_stiffness * expected_tip;
    let expected_contact_force = case.contact_stiffness * expected_penetration;
    assert!(expected_penetration > 0.0);

    for elements in [1_usize, 2, 4, 8, 16] {
        let result = solve_contact_gap_1d(&mesh(elements, case))
            .expect("active refined contact chain should solve");
        assert!(result.converged);
        assert_eq!(result.active_contact_count, 1);
        assert_close(result.max_displacement, expected_tip);
        assert_close(result.max_force, expected_spring_force);
        assert_close(result.contacts[0].penetration, expected_penetration);
        assert_close(result.contacts[0].force, expected_contact_force);
        assert_close(expected_spring_force + expected_contact_force, case.load);
        assert!(result.contacts[0].active);
        assert_refined_spring_chain(&result, elements, expected_tip, expected_spring_force);
    }
}

#[derive(Clone, Copy)]
struct ContactCase {
    load: f64,
    spring_stiffness: f64,
    gap: f64,
    contact_stiffness: f64,
}

fn mesh(count: usize, case: ContactCase) -> SolveContactGap1dRequest {
    let nodes = (0..=count)
        .map(|index| NonlinearSpring1dNodeInput {
            id: format!("node-{index}"),
            x: LENGTH * index as f64 / count as f64,
            fix_x: index == 0,
            load_x: if index == count { case.load } else { 0.0 },
        })
        .collect();
    let elements = (0..count)
        .map(|index| NonlinearSpring1dElementInput {
            id: format!("spring-{index}"),
            node_i: index,
            node_j: index + 1,
            stiffness: case.spring_stiffness * count as f64,
            cubic_stiffness: 0.0,
        })
        .collect();
    SolveContactGap1dRequest {
        nodes,
        elements,
        contacts: vec![ContactGap1dContactInput {
            id: "stop".to_string(),
            node: count,
            gap: case.gap,
            normal_stiffness: case.contact_stiffness,
        }],
        load_steps: Some(8),
        max_iterations: Some(40),
        tolerance: Some(1.0e-9),
    }
}

fn assert_refined_spring_chain(
    result: &kyuubiki_protocol::SolveContactGap1dResult,
    elements: usize,
    tip_displacement: f64,
    spring_force: f64,
) {
    for node in &result.nodes {
        assert_close(node.ux, tip_displacement * node.x / LENGTH);
    }
    for element in &result.elements {
        assert_close(element.length, LENGTH / elements as f64);
        assert_close(element.extension, tip_displacement / elements as f64);
        assert_close(element.force, spring_force);
        assert_close(
            element.tangent_stiffness,
            result.input.elements[element.index].stiffness,
        );
    }
    let contact = &result.contacts[0];
    let input = &result.input.contacts[0];
    assert_close(contact.penetration, (tip_displacement - input.gap).max(0.0));
    assert_close(contact.force, input.normal_stiffness * contact.penetration);
}

fn assert_close(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() <= TOL * expected.abs().max(1.0),
        "expected {actual} to be close to {expected}",
    );
}
