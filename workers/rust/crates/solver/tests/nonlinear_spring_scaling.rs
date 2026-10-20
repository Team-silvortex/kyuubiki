use kyuubiki_protocol::{
    ContactGap1dContactInput, NonlinearSpring1dElementInput, NonlinearSpring1dNodeInput,
    SolveContactGap1dRequest, SolveNonlinearSpring1dRequest,
};
use kyuubiki_solver::solver_control::{SolverControl, SolverStage, with_solver_observer};
use kyuubiki_solver::{
    solve_contact_gap_1d, solve_contact_gap_1d_owned, solve_nonlinear_spring_1d,
    solve_nonlinear_spring_1d_owned,
};
use std::cell::Cell;
use std::rc::Rc;

fn model(cubic: f64, expected: f64) -> SolveNonlinearSpring1dRequest {
    SolveNonlinearSpring1dRequest {
        nodes: vec![
            node(0, 0.0, true, 0.0),
            node(1, 1.0, false, 4.0 * expected + cubic * expected.powi(3)),
        ],
        elements: vec![NonlinearSpring1dElementInput {
            id: "spring".into(),
            node_i: 0,
            node_j: 1,
            stiffness: 4.0,
            cubic_stiffness: cubic,
        }],
        load_steps: Some(4),
        max_iterations: Some(32),
        tolerance: Some(1e-12),
    }
}

fn node(index: usize, x: f64, fix_x: bool, load_x: f64) -> NonlinearSpring1dNodeInput {
    NonlinearSpring1dNodeInput {
        id: format!("n{index}"),
        x,
        fix_x,
        load_x,
    }
}

fn scaled(
    mut input: SolveNonlinearSpring1dRequest,
    force: f64,
    length: f64,
) -> SolveNonlinearSpring1dRequest {
    for node in &mut input.nodes {
        node.x *= length;
        node.load_x *= force;
    }
    for element in &mut input.elements {
        element.stiffness *= force / length;
        element.cubic_stiffness *= (force / length) / (length * length);
    }
    input.tolerance = input.tolerance.map(|value| value * force);
    input
}

fn contact(
    mut input: SolveNonlinearSpring1dRequest,
    force: f64,
    length: f64,
    expected: f64,
) -> SolveContactGap1dRequest {
    let gap = expected / 2.0;
    input.nodes[1].load_x += force * (expected - gap);
    SolveContactGap1dRequest {
        nodes: input.nodes,
        elements: input.elements,
        load_steps: input.load_steps,
        max_iterations: input.max_iterations,
        tolerance: input.tolerance,
        contacts: vec![ContactGap1dContactInput {
            id: "stop".into(),
            node: 1,
            gap: gap * length,
            normal_stiffness: force / length,
        }],
    }
}

fn close(actual: f64, expected: f64) {
    assert!(actual.is_finite());
    assert!(
        (actual / expected - 1.0).abs() < 1e-10,
        "{actual} != {expected}"
    );
}

fn check_equivalent(force: f64, length: f64, cubic: f64, expected: f64) {
    let input = scaled(model(cubic, expected), force, length);
    let tolerance = input.tolerance.unwrap();
    for result in [
        solve_nonlinear_spring_1d(&input).unwrap(),
        solve_nonlinear_spring_1d_owned(input.clone()).unwrap(),
    ] {
        assert!(result.converged, "{:?}", result.steps);
        close(result.nodes[1].ux, expected * length);
        close(result.elements[0].force, input.nodes[1].load_x);
        close(result.elements[0].length, length);
        close(
            result.elements[0].tangent_stiffness,
            (4.0 + 3.0 * cubic * expected.powi(2)) * force / length,
        );
        assert!(result.residual_norm <= tolerance);
        assert!(
            result
                .steps
                .iter()
                .all(|step| step.residual_norm <= tolerance)
        );
    }
    // Only a positive target engages the supported unilateral stop.
    if expected > 0.0 {
        let input = contact(input, force, length, expected);
        for result in [
            solve_contact_gap_1d(&input).unwrap(),
            solve_contact_gap_1d_owned(input).unwrap(),
        ] {
            assert!(result.converged, "{:?}", result.steps);
            close(result.nodes[1].ux, expected * length);
            close(result.contacts[0].force, 0.5 * expected * force);
            close(
                result.elements[0].force + result.contacts[0].force,
                result.input.nodes[1].load_x,
            );
            assert!(result.contacts[0].active);
            assert!(result.residual_norm <= tolerance);
        }
    }
}

#[test]
fn positive_spring_laws_are_invariant_under_force_unit_scaling() {
    for exponent in [-400, -80, 0, 80, 400] {
        for cubic in [0.0, 8.0] {
            for expected in [-0.25, 0.25] {
                check_equivalent(2.0_f64.powi(exponent), 1.0, cubic, expected);
            }
        }
    }
}

#[test]
fn representable_nonzero_geometry_does_not_depend_on_the_length_unit() {
    for (force_exponent, length_exponent) in [
        (0, -200),
        (0, -60),
        (0, 0),
        (0, 200),
        (-400, -100),
        (400, 100),
    ] {
        for cubic in [0.0, 8.0] {
            check_equivalent(
                2.0_f64.powi(force_exponent),
                2.0_f64.powi(length_exponent),
                cubic,
                0.25,
            );
        }
    }
}

#[test]
fn a_stable_softening_branch_is_not_rejected_at_an_arbitrary_unit_extension() {
    for (force, length) in [(1.0, 1.0), (1e-20, 1.0), (1.0, 1e-20), (1.0, 1e20)] {
        for expected in [-0.125, 0.125] {
            check_equivalent(force, length, -16.0, expected);
        }
    }
}

#[test]
fn scaled_branched_networks_preserve_equilibrium_on_the_general_solver_path() {
    let expected = [0.0, 0.125, -0.125, 0.25, 0.375];
    let mut input = model(0.0, 0.25);
    input.nodes = (0..expected.len())
        .map(|index| node(index, index as f64, index == 0, 0.0))
        .collect();
    input.elements = [(0, 1), (1, 2), (2, 3), (1, 3), (3, 4), (0, 4)]
        .into_iter()
        .enumerate()
        .map(|(index, (i, j))| {
            let k = (index + 1) as f64;
            let extension: f64 = expected[j] - expected[i];
            let force = k * extension + 2.0 * k * extension.powi(3);
            input.nodes[i].load_x -= force;
            input.nodes[j].load_x += force;
            NonlinearSpring1dElementInput {
                id: format!("e{index}"),
                node_i: i,
                node_j: j,
                stiffness: k,
                cubic_stiffness: 2.0 * k,
            }
        })
        .collect();
    for (force, length) in [(1.0, 1.0), (1e-100, 1.0), (1.0, 1e-30), (1e100, 1e30)] {
        let request = scaled(input.clone(), force, length);
        let factorizations = Rc::new(Cell::new(0));
        let observed = factorizations.clone();
        let result = with_solver_observer(
            &SolverControl::default(),
            move |point| {
                if point.stage == SolverStage::DenseFactor && point.completed_steps == 0 {
                    observed.set(observed.get() + 1);
                }
            },
            || solve_nonlinear_spring_1d_owned(request),
        )
        .unwrap();
        assert!(factorizations.get() > 0, "exercise the non-path fallback");
        assert!(result.converged, "{:?}", result.steps);
        assert!(result.residual_norm <= result.input.tolerance.unwrap());
        for (node, expected) in result.nodes.iter().skip(1).zip(expected.iter().skip(1)) {
            close(node.ux, expected * length);
        }
        let mut residual = vec![0.0; expected.len()];
        for element in &result.elements {
            residual[element.node_i] -= element.force;
            residual[element.node_j] += element.force;
        }
        for (node, internal) in result.input.nodes.iter().zip(residual).skip(1) {
            assert!((node.load_x - internal).abs() <= result.input.tolerance.unwrap());
        }
        let request = contact(
            scaled(input.clone(), force, length),
            force,
            length,
            expected[1],
        );
        let result = solve_contact_gap_1d_owned(request).unwrap();
        assert!(result.converged, "{:?}", result.steps);
        assert!(result.residual_norm <= result.input.tolerance.unwrap());
        for (node, expected) in result.nodes.iter().skip(1).zip(expected.iter().skip(1)) {
            close(node.ux, expected * length);
        }
        close(result.contacts[0].force, expected[1] * force / 2.0);
    }
}

#[test]
fn invalid_stiffness_and_collapsed_geometry_remain_rejected() {
    let mut invalid = Vec::new();
    for stiffness in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        let mut input = model(0.0, 0.25);
        input.elements[0].stiffness = stiffness;
        invalid.push((input, "stiffness must be positive"));
    }
    for (first, second) in [(0.0, 0.0), (1e100, 1e100 + 1.0), (-1e308, 1e308)] {
        let mut input = model(0.0, 0.25);
        input.nodes[0].x = first;
        input.nodes[1].x = second;
        invalid.push((input, "length must be positive"));
    }
    let mut input = model(f64::NAN, 0.25);
    input.nodes[1].load_x = 1.0;
    invalid.push((input, "cubic stiffness must be finite"));
    for (input, message) in invalid {
        let errors = [
            solve_nonlinear_spring_1d(&input).unwrap_err(),
            solve_nonlinear_spring_1d_owned(input.clone()).unwrap_err(),
            solve_contact_gap_1d(&contact(input.clone(), 1.0, 1.0, 0.25)).unwrap_err(),
            solve_contact_gap_1d_owned(contact(input, 1.0, 1.0, 0.25)).unwrap_err(),
        ];
        for error in errors {
            assert!(error.contains(message), "{error}");
        }
    }
}
