use kyuubiki_protocol::{
    Frame2dStabilityKinematics, Frame2dStabilityPathControl, SolveFrame2dPDeltaRequest,
    SolveFrame2dPDeltaResult,
};
use kyuubiki_solver::solve_frame_2d_p_delta;
use serde_json::json;

fn cantilever() -> SolveFrame2dPDeltaRequest {
    serde_json::from_value(json!({
        "buckling": {"frame": {
            "nodes": (0..3).map(|i| json!({
                "id": format!("n{i}"), "x": 0.0, "y": 2.0 * i as f64,
                "fix_x": i == 0, "fix_y": i == 0, "fix_rz": i == 0,
                "load_x": if i == 2 { 2e5 } else { 0.0 },
                "load_y": if i == 2 { -2.5e6 } else { 0.0 }, "moment_z": 0.0
            })).collect::<Vec<_>>(),
            "elements": (0..2).map(|i| json!({
                "id": format!("e{i}"), "node_i": i, "node_j": i + 1,
                "area": 0.01, "youngs_modulus": 2e11,
                "moment_of_inertia": 5e-4, "section_modulus": 5e-4 / 0.3
            })).collect::<Vec<_>>()
        }, "mode_count": 1},
        "kinematics": "corotational", "imperfection_amplitude": 1e-3,
        "imperfection_shape": [0.0, 0.0, 0.0, 0.5, 0.0, 0.0, 1.0, 0.0, 0.0],
        "maximum_load_factor": 1.0, "load_steps": 4,
        "max_iterations": 64, "max_step_cutbacks": 12, "tolerance": 1e-9
    }))
    .unwrap()
}

fn add_support_load(request: &mut SolveFrame2dPDeltaRequest, scale: f64) {
    let support = &mut request.buckling.frame.nodes[0];
    support.load_x = scale;
    support.load_y = -scale;
    support.moment_z = 0.5 * scale;
}

fn assert_same_path(actual: &SolveFrame2dPDeltaResult, reference: &SolveFrame2dPDeltaResult) {
    assert!(actual.converged && reference.converged);
    assert_eq!(actual.steps.len(), reference.steps.len());
    for (step, expected) in actual.steps.iter().zip(&reference.steps) {
        assert!(step.converged && step.residual_norm.is_finite());
        assert!((step.load_factor - expected.load_factor).abs() < 1e-8);
        for (actual, expected) in step.displacements.iter().zip(&expected.displacements) {
            assert!(
                (actual - expected).abs() < 1e-8 * expected.abs().max(1e-6),
                "{actual} vs {expected}"
            );
        }
    }
}

#[test]
fn constrained_loads_cannot_falsely_converge_the_corotational_path() {
    let original = cantilever();
    let reference = solve_frame_2d_p_delta(&original).unwrap();
    assert!(reference.final_displacements[6].abs() > 0.01);
    for scale in [1e20, 1e100, -1e100] {
        let mut changed = original.clone();
        add_support_load(&mut changed, scale);
        assert_same_path(&solve_frame_2d_p_delta(&changed).unwrap(), &reference);
    }
}

#[test]
fn constrained_loads_do_not_change_arc_length_equilibrium_or_step_adaptation() {
    let mut original = cantilever();
    original.path_control = Frame2dStabilityPathControl::ArcLength;
    let reference = solve_frame_2d_p_delta(&original).unwrap();
    for scale in [1e20, 1e100] {
        let mut changed = original.clone();
        add_support_load(&mut changed, scale);
        let actual = solve_frame_2d_p_delta(&changed).unwrap();
        assert_same_path(&actual, &reference);
        for (actual, expected) in actual.steps.iter().zip(&reference.steps) {
            assert_eq!(actual.iterations, expected.iterations);
            assert_eq!(actual.cutbacks, expected.cutbacks);
            assert_eq!(actual.arc_length_radius, expected.arc_length_radius);
        }
    }
}

#[test]
fn support_loads_cannot_turn_an_exhausted_newton_budget_into_success() {
    let mut original = cantilever();
    original.max_iterations = Some(1);
    original.max_step_cutbacks = Some(0);
    let reference = solve_frame_2d_p_delta(&original).unwrap();
    assert!(!reference.converged);
    add_support_load(&mut original, 1e100);
    let actual = solve_frame_2d_p_delta(&original).unwrap();
    assert!(
        !actual.converged,
        "a large support load cannot certify an unequilibrated trial"
    );
    assert_eq!(
        actual.steps[0].failure_reason,
        reference.steps[0].failure_reason
    );
    assert_eq!(actual.steps[0].achieved_load_factor, Some(0.0));
    assert_eq!(actual.final_displacements, reference.final_displacements);
    assert!(solve_frame_2d_p_delta(&cantilever()).unwrap().converged);
}

#[test]
fn linearized_free_equations_remain_independent_of_constrained_loads() {
    let mut original = cantilever();
    original.kinematics = Frame2dStabilityKinematics::LinearizedPDelta;
    let reference = solve_frame_2d_p_delta(&original).unwrap();
    add_support_load(&mut original, 1e100);
    assert_same_path(&solve_frame_2d_p_delta(&original).unwrap(), &reference);
}

fn add_independent_axial_member(request: &mut SolveFrame2dPDeltaRequest) {
    let frame = &mut request.buckling.frame;
    let offset = frame.nodes.len();
    for end in 0..2 {
        frame.nodes.push(
            serde_json::from_value(json!({
                "id": format!("axial-{end}"), "x": 10.0 + end as f64, "y": 0.0,
                "fix_x": end == 0, "fix_y": true, "fix_rz": true,
                "load_x": if end == 1 { 1e18 } else { 0.0 },
                "load_y": 0.0, "moment_z": 0.0
            }))
            .unwrap(),
        );
    }
    frame.elements.push(
        serde_json::from_value(json!({
            "id": "independent-axial", "node_i": offset, "node_j": offset + 1,
            "area": 0.01, "youngs_modulus": 1e24,
            "moment_of_inertia": 0.01, "section_modulus": 0.01
        }))
        .unwrap(),
    );
    request
        .imperfection_shape
        .as_mut()
        .unwrap()
        .extend([0.0; 6]);
}

#[test]
fn unrelated_large_free_force_cannot_mask_cantilever_equilibrium() {
    let mut request = cantilever();
    let reference = solve_frame_2d_p_delta(&request).unwrap();
    add_independent_axial_member(&mut request);
    let actual = solve_frame_2d_p_delta(&request).unwrap();
    assert_eq!(
        actual.final_displacements.len(),
        reference.final_displacements.len() + 6
    );
    assert_same_path(&actual, &reference);
}

#[test]
fn unrelated_large_free_force_cannot_mask_free_moment_equilibrium() {
    let mut request = cantilever();
    request.buckling.frame.nodes[2].load_x = 0.0;
    request.buckling.frame.nodes[2].moment_z = 4e5;
    let reference = solve_frame_2d_p_delta(&request).unwrap();
    add_independent_axial_member(&mut request);
    let actual = solve_frame_2d_p_delta(&request).unwrap();
    assert_eq!(
        actual.final_displacements.len(),
        reference.final_displacements.len() + 6
    );
    assert_same_path(&actual, &reference);
}

#[test]
fn unrelated_free_load_cannot_certify_an_exhausted_iteration_budget() {
    let mut request = cantilever();
    request.max_iterations = Some(2);
    request.max_step_cutbacks = Some(0);
    let reference = solve_frame_2d_p_delta(&request).unwrap();
    assert!(!reference.converged);
    add_independent_axial_member(&mut request);
    let actual = solve_frame_2d_p_delta(&request).unwrap();
    assert!(
        !actual.converged,
        "unbalanced small-load branch was accepted"
    );
    assert_eq!(actual.steps[0].achieved_load_factor, Some(0.0));
    assert!(actual.final_displacements.iter().all(|value| *value == 0.0));
}

#[test]
fn arc_length_keeps_the_weak_branch_in_equilibrium_at_its_actual_load_factor() {
    let mut input = cantilever();
    input.path_control = Frame2dStabilityPathControl::ArcLength;
    add_independent_axial_member(&mut input);
    let actual = solve_frame_2d_p_delta(&input).unwrap();
    assert!(actual.converged);
    for step in &actual.steps {
        // The extra DOF changes arc-length parameterization, not equilibrium.
        let mut reference = cantilever();
        reference.load_steps = Some(1);
        reference.maximum_load_factor = Some(step.load_factor);
        let reference = solve_frame_2d_p_delta(&reference).unwrap();
        assert!(reference.converged);
        assert_eq!(
            step.displacements.len(),
            reference.final_displacements.len() + 6
        );
        for (&actual, &expected) in step
            .displacements
            .iter()
            .zip(&reference.final_displacements)
        {
            assert!(
                (actual - expected).abs() <= 1e-7 * expected.abs().max(1e-6),
                "{actual} vs {expected}"
            );
        }
    }
}
