use kyuubiki_protocol::{
    ContactGap1dContactInput, NonlinearSpring1dElementInput, NonlinearSpring1dNodeInput,
    SolveContactGap1dRequest, SolveNonlinearSpring1dRequest,
};
use kyuubiki_solver::{solve_contact_gap_1d, solve_nonlinear_spring_1d};

#[test]
fn nonlinear_spring_1d_matches_hardening_closed_form_under_perturbations() {
    for case in [
        NonlinearSpringCase {
            stiffness: 1000.0,
            cubic_stiffness: 50_000.0,
            load: 100.0,
        },
        NonlinearSpringCase {
            stiffness: 1000.0,
            cubic_stiffness: 50_000.0,
            load: 240.0,
        },
        NonlinearSpringCase {
            stiffness: 1800.0,
            cubic_stiffness: 50_000.0,
            load: 100.0,
        },
        NonlinearSpringCase {
            stiffness: 1000.0,
            cubic_stiffness: 125_000.0,
            load: 100.0,
        },
    ] {
        let result = solve_nonlinear_spring_1d(&nonlinear_spring_request(case))
            .expect("hardening nonlinear spring should solve");
        let expected = nonlinear_spring_closed_form(case);
        let tip = &result.nodes[1];
        let element = &result.elements[0];

        assert!(result.converged, "hardening spring should converge");
        assert_eq!(result.steps.len(), 8);
        assert_nonlinear_close(result.nodes[0].ux, 0.0, "nonlinear fixed ux");
        assert_nonlinear_close(tip.ux, expected.displacement, "nonlinear tip ux");
        assert_nonlinear_close(
            result.max_displacement,
            expected.displacement.abs(),
            "nonlinear max displacement",
        );
        assert_nonlinear_close(result.max_force, case.load.abs(), "nonlinear max force");
        assert_nonlinear_close(element.length, 1.0, "nonlinear element length");
        assert_nonlinear_close(
            element.extension,
            expected.displacement,
            "nonlinear extension",
        );
        assert_nonlinear_close(element.force, case.load, "nonlinear force");
        assert_nonlinear_close(
            element.tangent_stiffness,
            expected.tangent_stiffness,
            "nonlinear tangent",
        );
        assert!(element.tangent_stiffness >= case.stiffness);
        assert_load_steps(&result.steps, 8);
    }
}

#[test]
fn contact_gap_1d_matches_inactive_and_active_closed_form_under_perturbations() {
    for case in [
        ContactCase {
            load: 25.0,
            spring_stiffness: 1000.0,
            gap: 0.05,
            contact_stiffness: 10_000.0,
        },
        ContactCase {
            load: 75.0,
            spring_stiffness: 1000.0,
            gap: 0.05,
            contact_stiffness: 10_000.0,
        },
        ContactCase {
            load: 150.0,
            spring_stiffness: 1600.0,
            gap: 0.04,
            contact_stiffness: 18_000.0,
        },
    ] {
        let result =
            solve_contact_gap_1d(&contact_request(case)).expect("contact gap spring should solve");
        let expected = contact_closed_form(case);
        let tip = &result.nodes[1];
        let element = &result.elements[0];
        let contact = &result.contacts[0];

        assert!(result.converged, "contact gap should converge");
        assert_eq!(
            result.active_contact_count,
            usize::from(expected.active),
            "contact active count should match closed form",
        );
        assert_nonlinear_close(result.nodes[0].ux, 0.0, "contact fixed ux");
        assert_nonlinear_close(tip.ux, expected.displacement, "contact tip ux");
        assert_nonlinear_close(
            result.max_displacement,
            expected.displacement.abs(),
            "contact max displacement",
        );
        assert_nonlinear_close(element.force, expected.spring_force, "contact spring force");
        assert_nonlinear_close(
            contact.penetration,
            expected.penetration,
            "contact penetration",
        );
        assert_nonlinear_close(contact.force, expected.contact_force, "contact force");
        assert_nonlinear_close(
            result.max_contact_force,
            expected.contact_force.abs(),
            "contact max force",
        );
        assert_nonlinear_close(
            expected.spring_force + expected.contact_force,
            case.load,
            "contact force balance",
        );
        assert_eq!(contact.active, expected.active);
        assert_load_steps(&result.steps, 6);
    }
}

#[derive(Clone, Copy)]
struct NonlinearSpringCase {
    stiffness: f64,
    cubic_stiffness: f64,
    load: f64,
}

struct NonlinearSpringExpected {
    displacement: f64,
    tangent_stiffness: f64,
}

#[derive(Clone, Copy)]
struct ContactCase {
    load: f64,
    spring_stiffness: f64,
    gap: f64,
    contact_stiffness: f64,
}

struct ContactExpected {
    displacement: f64,
    spring_force: f64,
    penetration: f64,
    contact_force: f64,
    active: bool,
}

fn nonlinear_spring_closed_form(case: NonlinearSpringCase) -> NonlinearSpringExpected {
    let displacement = hardening_root(case.stiffness, case.cubic_stiffness, case.load);
    let tangent_stiffness = case.stiffness + 3.0 * case.cubic_stiffness * displacement.powi(2);

    NonlinearSpringExpected {
        displacement,
        tangent_stiffness,
    }
}

fn contact_closed_form(case: ContactCase) -> ContactExpected {
    let inactive_displacement = case.load / case.spring_stiffness;
    if inactive_displacement <= case.gap {
        return ContactExpected {
            displacement: inactive_displacement,
            spring_force: case.load,
            penetration: 0.0,
            contact_force: 0.0,
            active: false,
        };
    }

    let displacement = (case.load + case.contact_stiffness * case.gap)
        / (case.spring_stiffness + case.contact_stiffness);
    let penetration = displacement - case.gap;
    let spring_force = case.spring_stiffness * displacement;
    let contact_force = case.contact_stiffness * penetration;

    ContactExpected {
        displacement,
        spring_force,
        penetration,
        contact_force,
        active: true,
    }
}

fn nonlinear_spring_request(case: NonlinearSpringCase) -> SolveNonlinearSpring1dRequest {
    SolveNonlinearSpring1dRequest {
        nodes: vec![
            nonlinear_node("fixed", 0.0, true, 0.0),
            nonlinear_node("tip", 1.0, false, case.load),
        ],
        elements: vec![NonlinearSpring1dElementInput {
            id: "hardening".to_string(),
            node_i: 0,
            node_j: 1,
            stiffness: case.stiffness,
            cubic_stiffness: case.cubic_stiffness,
        }],
        load_steps: Some(8),
        max_iterations: Some(32),
        tolerance: Some(1.0e-12),
    }
}

fn contact_request(case: ContactCase) -> SolveContactGap1dRequest {
    SolveContactGap1dRequest {
        nodes: vec![
            nonlinear_node("fixed", 0.0, true, 0.0),
            nonlinear_node("tip", 1.0, false, case.load),
        ],
        elements: vec![NonlinearSpring1dElementInput {
            id: "spring".to_string(),
            node_i: 0,
            node_j: 1,
            stiffness: case.spring_stiffness,
            cubic_stiffness: 0.0,
        }],
        contacts: vec![ContactGap1dContactInput {
            id: "stop".to_string(),
            node: 1,
            gap: case.gap,
            normal_stiffness: case.contact_stiffness,
        }],
        load_steps: Some(6),
        max_iterations: Some(32),
        tolerance: Some(1.0e-10),
    }
}

fn nonlinear_node(id: &str, x: f64, fix_x: bool, load_x: f64) -> NonlinearSpring1dNodeInput {
    NonlinearSpring1dNodeInput {
        id: id.to_string(),
        x,
        fix_x,
        load_x,
    }
}

fn hardening_root(stiffness: f64, cubic_stiffness: f64, load: f64) -> f64 {
    let p = stiffness / cubic_stiffness;
    let q = -load / cubic_stiffness;
    let discriminant = (q * 0.5).powi(2) + (p / 3.0).powi(3);
    (-q * 0.5 + discriminant.sqrt()).cbrt() + (-q * 0.5 - discriminant.sqrt()).cbrt()
}

fn assert_load_steps(steps: &[kyuubiki_protocol::NonlinearSpring1dStepResult], count: usize) {
    assert_eq!(steps.len(), count);
    for (index, step) in steps.iter().enumerate() {
        assert_eq!(step.step, index + 1);
        assert_nonlinear_close(
            step.load_factor,
            (index + 1) as f64 / count as f64,
            "nonlinear load factor",
        );
        assert!(step.converged, "load step {} should converge", step.step);
        assert!(step.iterations <= 32);
        assert!(step.residual_norm <= 1.0e-9);
    }
}

fn assert_nonlinear_close(actual: f64, expected: f64, label: &str) {
    let scale = expected.abs().max(1.0);
    assert!(
        (actual - expected).abs() <= 1.0e-9 * scale,
        "{label}: expected {actual} to be close to {expected}",
    );
}
