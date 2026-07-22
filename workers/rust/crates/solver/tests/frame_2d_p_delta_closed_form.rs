use kyuubiki_protocol::{
    Frame2dElementInput, Frame2dImperfectionSource, Frame2dNodeInput, SolveBucklingFrame2dRequest,
    SolveFrame2dPDeltaRequest, SolveFrame2dRequest,
};
use kyuubiki_solver::solve_frame_2d_p_delta;

const ELEMENT_COUNT: usize = 16;
const LENGTH: f64 = 3.2;
const YOUNGS_MODULUS: f64 = 205.0e9;
const INERTIA: f64 = 7.4e-6;
const REFERENCE_FORCE: f64 = 100_000.0;

#[test]
fn modal_imperfection_matches_precritical_secant_amplification() {
    let result = solve_frame_2d_p_delta(&request(None, 8))
        .expect("precritical imperfect column should solve");

    assert_eq!(
        result.imperfection_source,
        Frame2dImperfectionSource::BucklingMode
    );
    assert_eq!(result.steps.len(), 8);
    assert!(
        result
            .steps
            .windows(2)
            .all(|pair| pair[1].imperfection_amplification > pair[0].imperfection_amplification)
    );
    for step in &result.steps {
        let expected = 1.0 / (1.0 - step.critical_factor_ratio);
        assert_relative(step.imperfection_amplification, expected, 2.0e-7);
        assert!(step.residual_norm < 1.0e-8);
    }
    let final_step = result.steps.last().unwrap();
    assert_relative(
        result.max_imperfection_amplification,
        final_step.imperfection_amplification,
        1.0e-12,
    );
    assert_eq!(
        result.final_displacements,
        result.steps.last().unwrap().displacements
    );
}

#[test]
fn rejects_a_path_that_enters_the_critical_limit_band() {
    let baseline =
        solve_frame_2d_p_delta(&request(None, 2)).expect("default precritical path should solve");
    let excessive = baseline.buckling_result.minimum_load_factor * 0.96;
    let error = solve_frame_2d_p_delta(&request(Some(excessive), 2))
        .expect_err("critical limit band must be rejected");
    assert!(error.contains("below 0.950 of the first critical factor"));
}

fn request(maximum_load_factor: Option<f64>, load_steps: usize) -> SolveFrame2dPDeltaRequest {
    let segment = LENGTH / ELEMENT_COUNT as f64;
    let nodes = (0..=ELEMENT_COUNT)
        .map(|index| Frame2dNodeInput {
            id: format!("n{index}"),
            x: 0.0,
            y: index as f64 * segment,
            fix_x: index == 0 || index == ELEMENT_COUNT,
            fix_y: index == 0,
            fix_rz: false,
            load_x: 0.0,
            load_y: if index == ELEMENT_COUNT {
                -REFERENCE_FORCE
            } else {
                0.0
            },
            moment_z: 0.0,
        })
        .collect();
    let elements = (0..ELEMENT_COUNT)
        .map(|index| Frame2dElementInput {
            id: format!("e{index}"),
            node_i: index,
            node_j: index + 1,
            area: 0.01,
            youngs_modulus: YOUNGS_MODULUS,
            moment_of_inertia: INERTIA,
            section_modulus: 1.0e-4,
        })
        .collect();
    SolveFrame2dPDeltaRequest {
        buckling: SolveBucklingFrame2dRequest {
            frame: SolveFrame2dRequest { nodes, elements },
            mode_count: Some(1),
        },
        imperfection_amplitude: 0.0032,
        kinematics: Default::default(),
        imperfection_shape: None,
        imperfection_mode_index: Some(0),
        maximum_load_factor,
        load_steps: Some(load_steps),
    }
}

fn assert_relative(actual: f64, expected: f64, tolerance: f64) {
    let relative = (actual - expected).abs() / expected.abs().max(1.0);
    assert!(
        relative <= tolerance,
        "actual={actual}, expected={expected}, relative={relative}"
    );
}
