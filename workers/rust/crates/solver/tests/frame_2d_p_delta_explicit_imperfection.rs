use kyuubiki_protocol::{
    Frame2dElementInput, Frame2dImperfectionSource, Frame2dNodeInput, Frame2dStabilityKinematics,
    SolveBucklingFrame2dRequest, SolveFrame2dPDeltaRequest, SolveFrame2dRequest,
};
use kyuubiki_solver::solve_frame_2d_p_delta;

const IMPERFECTION_AMPLITUDE: f64 = 0.012;

#[test]
fn explicit_portal_imperfection_is_objective_under_rigid_rotation() {
    let baseline = solve_frame_2d_p_delta(&portal_request(0.0))
        .expect("baseline imperfect portal should solve");
    let rotated = solve_frame_2d_p_delta(&portal_request(0.731))
        .expect("rotated imperfect portal should solve");

    assert_eq!(
        baseline.imperfection_source,
        Frame2dImperfectionSource::ExplicitShape
    );
    assert_eq!(
        rotated.imperfection_source,
        Frame2dImperfectionSource::ExplicitShape
    );
    assert_relative(
        max_translation(&baseline.initial_imperfection_shape),
        IMPERFECTION_AMPLITUDE,
        1.0e-12,
    );
    assert_relative(
        rotated.buckling_result.minimum_load_factor,
        baseline.buckling_result.minimum_load_factor,
        2.0e-8,
    );
    assert_eq!(rotated.steps.len(), baseline.steps.len());
    for (rotated_step, baseline_step) in rotated.steps.iter().zip(&baseline.steps) {
        assert_relative(
            rotated_step.imperfection_amplification,
            baseline_step.imperfection_amplification,
            2.0e-8,
        );
        assert_relative(
            max_translation(&rotated_step.displacements),
            max_translation(&baseline_step.displacements),
            2.0e-8,
        );
        assert!(rotated_step.residual_norm < 1.0e-8);
    }
}

#[test]
fn rejects_malformed_explicit_imperfection_fields() {
    let mut wrong_length = portal_request(0.0);
    wrong_length.imperfection_shape = Some(vec![0.0; 3]);
    let error = solve_frame_2d_p_delta(&wrong_length)
        .expect_err("wrong explicit imperfection length must fail");
    assert!(error.contains("must contain 12 values"));

    let mut non_finite = portal_request(0.0);
    non_finite.imperfection_shape.as_mut().unwrap()[6] = f64::NAN;
    let error = solve_frame_2d_p_delta(&non_finite)
        .expect_err("non-finite explicit imperfection must fail");
    assert!(error.contains("must contain finite values"));

    let mut zero_shape = portal_request(0.0);
    zero_shape.imperfection_shape = Some(vec![0.0; 12]);
    let error =
        solve_frame_2d_p_delta(&zero_shape).expect_err("zero explicit imperfection must fail");
    assert!(error.contains("has no translational imperfection"));
}

#[test]
fn corotational_portal_converges_and_is_objective() {
    let mut baseline_request = portal_request(0.0);
    baseline_request.kinematics = Frame2dStabilityKinematics::Corotational;
    let mut rotated_request = portal_request(0.731);
    rotated_request.kinematics = Frame2dStabilityKinematics::Corotational;

    let baseline = solve_frame_2d_p_delta(&baseline_request)
        .expect("baseline corotational portal should solve");
    let rotated =
        solve_frame_2d_p_delta(&rotated_request).expect("rotated corotational portal should solve");

    assert!(baseline.converged);
    assert!(rotated.converged);
    assert_eq!(
        baseline.kinematics,
        Frame2dStabilityKinematics::Corotational
    );
    assert!(baseline.steps.iter().all(|step| step.converged));
    assert!(baseline.steps.iter().any(|step| step.iterations > 1));
    for (rotated_step, baseline_step) in rotated.steps.iter().zip(&baseline.steps) {
        assert_relative(
            rotated_step.imperfection_amplification,
            baseline_step.imperfection_amplification,
            2.0e-6,
        );
        assert_relative(
            max_translation(&rotated_step.displacements),
            max_translation(&baseline_step.displacements),
            2.0e-6,
        );
    }
}

#[test]
fn corotational_portal_matches_linearized_response_at_low_load() {
    let mut probe = portal_request(0.0);
    probe.load_steps = Some(1);
    let critical_factor = solve_frame_2d_p_delta(&probe)
        .expect("critical-factor probe should solve")
        .buckling_result
        .minimum_load_factor;

    let mut linearized_request = portal_request(0.0);
    linearized_request.load_steps = Some(1);
    linearized_request.maximum_load_factor = Some(0.005 * critical_factor);
    let mut corotational_request = linearized_request.clone();
    corotational_request.kinematics = Frame2dStabilityKinematics::Corotational;

    let linearized = solve_frame_2d_p_delta(&linearized_request)
        .expect("low-load linearized portal should solve");
    let corotational = solve_frame_2d_p_delta(&corotational_request)
        .expect("low-load corotational portal should solve");

    assert!(corotational.converged);
    assert_relative(
        corotational.steps[0].imperfection_amplification,
        linearized.steps[0].imperfection_amplification,
        2.0e-3,
    );
}

fn portal_request(angle: f64) -> SolveFrame2dPDeltaRequest {
    let points = [(0.0, 0.0), (4.0, 0.0), (0.0, 3.0), (4.0, 3.0)];
    let nodes = points
        .into_iter()
        .enumerate()
        .map(|(index, (x, y))| {
            let (x, y) = rotate(x, y, angle);
            let (load_x, load_y) = if index >= 2 {
                rotate(0.0, -80_000.0, angle)
            } else {
                (0.0, 0.0)
            };
            Frame2dNodeInput {
                id: format!("n{index}"),
                x,
                y,
                fix_x: index < 2,
                fix_y: index < 2,
                fix_rz: index < 2,
                load_x,
                load_y,
                moment_z: 0.0,
            }
        })
        .collect();
    let elements = vec![
        element("left-column", 0, 2),
        element("beam", 2, 3),
        element("right-column", 1, 3),
    ];
    let (shape_x, shape_y) = rotate(1.0, 0.0, angle);
    let imperfection_shape = vec![
        0.0, 0.0, 0.0, 0.0, 0.0, 0.0, shape_x, shape_y, 0.0, shape_x, shape_y, 0.0,
    ];
    SolveFrame2dPDeltaRequest {
        buckling: SolveBucklingFrame2dRequest {
            frame: SolveFrame2dRequest { nodes, elements },
            mode_count: Some(3),
        },
        imperfection_amplitude: IMPERFECTION_AMPLITUDE,
        kinematics: Default::default(),
        imperfection_shape: Some(imperfection_shape),
        imperfection_mode_index: None,
        maximum_load_factor: None,
        load_steps: Some(6),
    }
}

fn element(id: &str, node_i: usize, node_j: usize) -> Frame2dElementInput {
    Frame2dElementInput {
        id: id.to_string(),
        node_i,
        node_j,
        area: 0.012,
        youngs_modulus: 210.0e9,
        moment_of_inertia: 9.0e-5,
        section_modulus: 6.0e-4,
    }
}

fn rotate(x: f64, y: f64, angle: f64) -> (f64, f64) {
    let (sine, cosine) = angle.sin_cos();
    (cosine * x - sine * y, sine * x + cosine * y)
}

fn max_translation(values: &[f64]) -> f64 {
    (0..values.len() / 3)
        .map(|node| (values[node * 3].powi(2) + values[node * 3 + 1].powi(2)).sqrt())
        .fold(0.0_f64, f64::max)
}

fn assert_relative(actual: f64, expected: f64, tolerance: f64) {
    let relative = (actual - expected).abs() / expected.abs().max(1.0);
    assert!(
        relative <= tolerance,
        "actual={actual}, expected={expected}, relative={relative}"
    );
}
