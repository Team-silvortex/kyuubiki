use kyuubiki_protocol::{
    ContactGap1dContactInput, NonlinearSpring1dElementInput, NonlinearSpring1dNodeInput,
    SolveContactGap1dRequest, SolveNonlinearSpring1dRequest,
};
use kyuubiki_solver::{
    solve_contact_gap_1d, solve_contact_gap_1d_owned, solve_nonlinear_spring_1d,
    solve_nonlinear_spring_1d_owned,
};

fn spring(unit: f64, steps: usize) -> SolveNonlinearSpring1dRequest {
    SolveNonlinearSpring1dRequest {
        nodes: vec![
            NonlinearSpring1dNodeInput {
                id: "base".into(),
                x: 0.0,
                fix_x: true,
                load_x: 1e100,
            },
            NonlinearSpring1dNodeInput {
                id: "tip".into(),
                x: 1.0,
                fix_x: false,
                load_x: 8.0 * unit,
            },
        ],
        elements: vec![NonlinearSpring1dElementInput {
            id: "spring".into(),
            node_i: 0,
            node_j: 1,
            stiffness: 1.0,
            cubic_stiffness: 0.0,
        }],
        load_steps: Some(steps),
        max_iterations: Some(32),
        tolerance: Some(unit * 2.0_f64.powi(-40)),
    }
}

fn contact(input: SolveNonlinearSpring1dRequest, gap: f64) -> SolveContactGap1dRequest {
    SolveContactGap1dRequest {
        nodes: input.nodes,
        elements: input.elements,
        load_steps: input.load_steps,
        max_iterations: input.max_iterations,
        tolerance: input.tolerance,
        contacts: vec![ContactGap1dContactInput {
            id: "stop".into(),
            node: 1,
            gap,
            normal_stiffness: 3.0,
        }],
    }
}

fn close(actual: f64, expected: f64) {
    assert!(actual.is_finite());
    assert!(
        (actual / expected - 1.0).abs() < 1e-11,
        "{actual} != {expected}"
    );
}

#[test]
fn explicit_subfloor_tolerance_cannot_skip_nonzero_spring_loads() {
    for exponent in [-50, -100, -500] {
        let unit = 2.0_f64.powi(exponent);
        for steps in [1, 2, 4, 8] {
            for sign in [-1.0, 1.0] {
                let mut input = spring(unit, steps);
                input.nodes[1].load_x *= sign;
                let tolerance = input.tolerance.unwrap();
                for result in [
                    solve_nonlinear_spring_1d(&input).unwrap(),
                    solve_nonlinear_spring_1d_owned(input).unwrap(),
                ] {
                    assert!(result.converged);
                    assert!(
                        result.residual_norm <= tolerance,
                        "requested={tolerance}, reported={}",
                        result.residual_norm
                    );
                    close(result.nodes[1].ux, sign * 8.0 * unit);
                    close(result.elements[0].force, sign * 8.0 * unit);
                    assert_eq!(result.achieved_load_factor, Some(1.0));
                    assert_eq!(result.steps.len(), steps);
                    for step in &result.steps {
                        assert!(step.converged && step.iterations > 0);
                        assert!(step.residual_norm <= tolerance);
                    }
                }
            }
        }
    }
}

#[test]
fn small_load_contact_activation_matches_the_piecewise_equilibrium_at_each_step() {
    for exponent in [-50, -100, -500] {
        let unit = 2.0_f64.powi(exponent);
        for steps in [1, 2, 4, 8] {
            let input = contact(spring(unit, steps), 2.0 * unit);
            let tolerance = input.tolerance.unwrap();
            for result in [
                solve_contact_gap_1d(&input).unwrap(),
                solve_contact_gap_1d_owned(input).unwrap(),
            ] {
                assert!(result.converged);
                assert!(result.residual_norm <= tolerance);
                close(result.nodes[1].ux, 3.5 * unit);
                close(result.elements[0].force, 3.5 * unit);
                close(result.contacts[0].force, 4.5 * unit);
                assert!(result.contacts[0].active);
                for step in &result.steps {
                    assert!(step.converged && step.residual_norm <= tolerance);
                    let load = step.load_factor * 8.0 * unit;
                    let mut prefix = contact(spring(unit, 1), 2.0 * unit);
                    prefix.nodes[1].load_x = load;
                    let prefix = solve_contact_gap_1d_owned(prefix).unwrap();
                    let expected = if load <= 2.0 * unit {
                        load
                    } else {
                        (load + 6.0 * unit) / 4.0
                    };
                    assert!(prefix.converged);
                    close(prefix.nodes[1].ux, expected);
                    assert_eq!(prefix.contacts[0].active, load > 2.0 * unit);
                    assert!(prefix.residual_norm <= tolerance);
                }
            }
        }
    }
}

#[test]
fn strict_small_scale_hardening_is_load_step_invariant() {
    for exponent in [-50, -100, -250] {
        let unit = 2.0_f64.powi(exponent);
        for steps in [1, 4, 16] {
            let mut input = spring(unit, steps);
            input.nodes[1].load_x = 2.0 * unit;
            input.elements[0].cubic_stiffness = 1.0 / (unit * unit);
            let tolerance = input.tolerance.unwrap();
            let result = solve_nonlinear_spring_1d(&input).unwrap();
            assert!(result.converged, "{:?}", result.steps);
            assert!(result.residual_norm <= tolerance);
            close(result.nodes[1].ux, unit);
            let mut input = contact(input, 0.5 * unit);
            input.nodes[1].load_x = 3.0 * unit;
            input.contacts[0].normal_stiffness = 2.0;
            let result = solve_contact_gap_1d_owned(input).unwrap();
            assert!(result.converged, "{:?}", result.steps);
            assert!(result.residual_norm <= tolerance);
            close(result.nodes[1].ux, unit);
            close(result.contacts[0].force, unit);
        }
    }
}

#[test]
fn unmet_strict_tolerance_cannot_commit_a_small_residual_as_success() {
    let unit = 2.0_f64.powi(-50);
    let mut input = spring(unit, 1);
    input.nodes[1].load_x = 3.0 * unit;
    input.elements[0].cubic_stiffness = 1.0 / (unit * unit);
    input.max_iterations = Some(1);
    let tolerance = input.tolerance.unwrap();
    let result = solve_nonlinear_spring_1d(&input).unwrap();
    assert!(!result.converged);
    assert_eq!(result.achieved_load_factor, Some(0.0));
    assert_eq!(result.nodes[1].ux, 0.0);
    assert_eq!(result.residual_norm, 0.0);
    close(result.steps[0].residual_norm, 1.875 * unit);
    assert!(result.steps[0].residual_norm > tolerance);
    assert!(result.steps[0].residual_norm < 1e-14);

    let result = solve_contact_gap_1d(&contact(input, 10.0 * unit)).unwrap();
    assert!(!result.converged);
    assert_eq!(result.achieved_load_factor, Some(0.0));
    assert_eq!(result.nodes[1].ux, 0.0);
    assert_eq!(result.residual_norm, 0.0);
    close(result.steps[0].residual_norm, 1.875 * unit);
    assert!(!result.contacts[0].active);
}

#[test]
fn strict_contact_failure_keeps_the_last_equilibrated_small_load() {
    let unit = 2.0_f64.powi(-50);
    let mut input = contact(spring(unit, 4), unit);
    input.nodes[1].load_x = 2.0 * unit;
    input.contacts[0].normal_stiffness = 10.0;
    input.max_iterations = Some(1);
    let tolerance = input.tolerance.unwrap();
    let result = solve_contact_gap_1d_owned(input).unwrap();
    assert!(!result.converged);
    assert_eq!(result.achieved_load_factor, Some(0.5));
    assert_eq!(result.steps.len(), 3);
    assert!(result.steps[..2].iter().all(|step| step.converged));
    assert!(!result.steps[2].converged);
    close(result.steps[2].residual_norm, 0.1875 * unit);
    assert!(result.steps[2].residual_norm > tolerance);
    close(result.nodes[1].ux, unit);
    close(result.elements[0].force, unit);
    assert_eq!(result.residual_norm, 0.0);
    assert!(!result.contacts[0].active);
}

#[test]
fn omitted_tolerance_retains_the_documented_default() {
    let mut input = spring(2.0_f64.powi(-50), 4);
    input.tolerance = None;
    let result = solve_nonlinear_spring_1d(&input).unwrap();
    assert!(result.converged);
    assert_eq!(result.nodes[1].ux, 0.0);
    assert!(result.residual_norm <= 1e-8);
    assert!(result.steps.iter().all(|step| step.iterations == 0));
    let result = solve_contact_gap_1d_owned(contact(input, 0.0)).unwrap();
    assert!(result.converged);
    assert_eq!(result.nodes[1].ux, 0.0);
    assert!(!result.contacts[0].active);
    assert!(result.residual_norm <= 1e-8);
}

#[test]
fn floating_point_precision_limits_never_relax_the_requested_tolerance() {
    let tolerance = f64::from_bits(1);
    let mut failures = 0;
    for cubic in [0.1, 0.7, 1.3, 7.0] {
        for load in [0.1, 0.2, 0.7, 1.0, 1.3] {
            let mut input = spring(1.0, 1);
            input.nodes[1].load_x = load;
            input.elements[0].cubic_stiffness = cubic;
            input.max_iterations = Some(16);
            input.tolerance = Some(tolerance);
            let result = solve_nonlinear_spring_1d_owned(input).unwrap();
            assert!(result.steps[0].iterations <= 16);
            if result.converged {
                assert!(result.residual_norm <= tolerance);
                // The tolerance applies to evaluated forces, not to a different
                // floating-point parenthesization of the same polynomial.
                let residual = (load - result.elements[0].force).abs();
                assert_eq!(result.residual_norm, residual);
                assert!(residual <= tolerance);
                let u = result.nodes[1].ux;
                let independent_force = u + cubic * u.powi(3);
                assert!(
                    (independent_force - result.elements[0].force).abs()
                        <= 8.0 * f64::EPSILON * load.abs()
                );
            } else {
                failures += 1;
                assert_eq!(result.achieved_load_factor, Some(0.0));
                assert_eq!(result.nodes[1].ux, 0.0);
                assert_eq!(result.residual_norm, 0.0);
                assert!(result.steps[0].residual_norm.is_finite());
                assert!(result.steps[0].residual_norm > tolerance);
            }
        }
    }
    assert!(
        failures > 0,
        "strict precision must exercise nonconvergence"
    );
}
