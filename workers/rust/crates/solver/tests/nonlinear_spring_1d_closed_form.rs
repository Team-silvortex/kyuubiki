use kyuubiki_protocol::{
    NonlinearSpring1dElementInput, NonlinearSpring1dNodeInput, SolveNonlinearSpring1dRequest,
};
use kyuubiki_solver::solve_nonlinear_spring_1d;

const TOL: f64 = 1.0e-10;

#[test]
fn nonlinear_spring_1d_closed_form_matches_cardano_root() {
    let stiffness = 1000.0;
    let cubic_stiffness = 50_000.0;
    let load = 100.0;
    let result =
        solve_nonlinear_spring_1d(&single_hardening_spring(stiffness, cubic_stiffness, load))
            .expect("single hardening spring closed-form fixture should solve");

    let expected_u = hardening_root(stiffness, cubic_stiffness, load);
    let expected_force = stiffness * expected_u + cubic_stiffness * expected_u.powi(3);
    let expected_tangent = stiffness + 3.0 * cubic_stiffness * expected_u.powi(2);

    assert!(result.converged);
    assert_eq!(result.steps.len(), 8);
    assert!(result.residual_norm <= 1.0e-10);
    assert_close(result.nodes[0].ux, 0.0);
    assert_close(result.nodes[1].ux, expected_u);
    assert_close(result.max_displacement, expected_u.abs());
    assert_close(result.max_force, load);

    for (index, step) in result.steps.iter().enumerate() {
        assert_eq!(step.step, index + 1);
        assert_close(step.load_factor, (index + 1) as f64 / 8.0);
        assert!(step.converged);
        assert!(step.iterations <= 32);
        assert!(step.residual_norm <= 1.0e-10);
    }

    let element = &result.elements[0];
    assert_close(element.length, 1.0);
    assert_close(element.extension, expected_u);
    assert_close(element.force, expected_force);
    assert_close(element.force, load);
    assert_close(element.tangent_stiffness, expected_tangent);
    assert!(element.tangent_stiffness > stiffness);
}

#[test]
fn nonlinear_spring_1d_preserves_displacement_under_law_and_load_scaling() {
    let stiffness = 850.0;
    let cubic_stiffness = 42_000.0;
    let load = 95.0;
    let baseline =
        solve_nonlinear_spring_1d(&single_hardening_spring(stiffness, cubic_stiffness, load))
            .expect("baseline hardening spring should solve");

    let scale = 1.7;
    let scaled = solve_nonlinear_spring_1d(&single_hardening_spring(
        stiffness * scale,
        cubic_stiffness * scale,
        load * scale,
    ))
    .expect("scaled hardening spring should solve");

    assert!(baseline.converged);
    assert!(scaled.converged);
    assert_eq!(baseline.steps.len(), scaled.steps.len());
    assert_close(scaled.nodes[1].ux, baseline.nodes[1].ux);
    assert_close(scaled.elements[0].extension, baseline.elements[0].extension);
    assert_close(scaled.elements[0].force / baseline.elements[0].force, scale);
    assert_close(scaled.max_force / baseline.max_force, scale);
    assert_close(
        scaled.elements[0].tangent_stiffness / baseline.elements[0].tangent_stiffness,
        scale,
    );
    assert_close(
        scaled.elements[0].force,
        stiffness * scale * scaled.elements[0].extension
            + cubic_stiffness * scale * scaled.elements[0].extension.powi(3),
    );
    for (scaled_step, baseline_step) in scaled.steps.iter().zip(&baseline.steps) {
        assert_eq!(scaled_step.step, baseline_step.step);
        assert_close(scaled_step.load_factor, baseline_step.load_factor);
        assert!(scaled_step.converged);
    }
}

fn single_hardening_spring(
    stiffness: f64,
    cubic_stiffness: f64,
    load: f64,
) -> SolveNonlinearSpring1dRequest {
    SolveNonlinearSpring1dRequest {
        nodes: vec![node("fixed", 0.0, true, 0.0), node("tip", 1.0, false, load)],
        elements: vec![NonlinearSpring1dElementInput {
            id: "hardening".to_string(),
            node_i: 0,
            node_j: 1,
            stiffness,
            cubic_stiffness,
        }],
        load_steps: Some(8),
        max_iterations: Some(32),
        tolerance: Some(1.0e-12),
    }
}

fn hardening_root(stiffness: f64, cubic_stiffness: f64, load: f64) -> f64 {
    let p = stiffness / cubic_stiffness;
    let q = -load / cubic_stiffness;
    let discriminant = (q * 0.5).powi(2) + (p / 3.0).powi(3);
    (-q * 0.5 + discriminant.sqrt()).cbrt() + (-q * 0.5 - discriminant.sqrt()).cbrt()
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
