use kyuubiki_protocol::{Frame2dNodeInput, ModalFrame2dElementInput, SolveModalFrame2dRequest};
use kyuubiki_solver::solve_modal_frame_2d;

const TOL: f64 = 1.0e-10;

#[test]
fn modal_frame_2d_review_bundle_checks_cantilever_modes_and_shape_contract() {
    let density = 7850.0;
    let area = 0.01;
    let length = 2.0;
    let result = solve_modal_frame_2d(&SolveModalFrame2dRequest {
        nodes: vec![
            node("fixed", 0.0, 0.0, true, true, true),
            node("tip", length, 0.0, false, false, false),
        ],
        elements: vec![ModalFrame2dElementInput {
            id: "beam".to_string(),
            node_i: 0,
            node_j: 1,
            area,
            youngs_modulus: 210.0e9,
            moment_of_inertia: 8.333e-6,
            section_modulus: 1.667e-4,
            density,
        }],
        mode_count: Some(3),
    })
    .expect("review modal frame should solve");

    assert_eq!(result.free_dofs, vec![3, 4, 5]);
    assert_close(result.total_mass, density * area * length);
    assert_eq!(result.modes.len(), 3);
    assert_close(
        result.min_frequency_hz,
        result.modes[0].natural_frequency_hz,
    );
    assert_close(
        result.max_frequency_hz,
        result.modes[2].natural_frequency_hz,
    );

    let mut previous_frequency = 0.0;
    for (index, mode) in result.modes.iter().enumerate() {
        assert_eq!(mode.index, index);
        assert!(mode.eigenvalue_rad_s_squared.is_finite());
        assert!(mode.eigenvalue_rad_s_squared > 0.0);
        assert!(mode.natural_frequency_rad_s.is_finite());
        assert!(mode.natural_frequency_hz.is_finite());
        assert!(mode.natural_frequency_hz > previous_frequency);
        assert_close(
            mode.natural_frequency_rad_s,
            mode.eigenvalue_rad_s_squared.sqrt(),
        );
        assert_close(
            mode.natural_frequency_hz,
            mode.natural_frequency_rad_s / std::f64::consts::TAU,
        );
        assert_close(mode.period_s, 1.0 / mode.natural_frequency_hz);
        assert_eq!(mode.shape.len(), 6);
        assert_close(mode.shape[0], 0.0);
        assert_close(mode.shape[1], 0.0);
        assert_close(mode.shape[2], 0.0);
        assert_close(mode.participation_norm, 1.0);
        assert!(mode.shape[3..].iter().all(|value| value.is_finite()));
        assert!(mode.shape[3..].iter().any(|value| value.abs() > 1.0e-6));
        previous_frequency = mode.natural_frequency_hz;
    }
}

#[test]
fn single_mode_request_uses_the_sparse_inverse_iteration_path() {
    let result = solve_modal_frame_2d(&SolveModalFrame2dRequest {
        nodes: vec![
            node("fixed", 0.0, 0.0, true, true, true),
            node("tip", 2.0, 0.0, false, false, false),
        ],
        elements: vec![ModalFrame2dElementInput {
            id: "beam".to_string(),
            node_i: 0,
            node_j: 1,
            area: 0.01,
            youngs_modulus: 210.0e9,
            moment_of_inertia: 8.333e-6,
            section_modulus: 1.667e-4,
            density: 7850.0,
        }],
        mode_count: Some(1),
    })
    .expect("sparse single-mode modal frame should solve");

    assert_eq!(result.modes.len(), 1);
    assert!(result.modes[0].eigenvalue_rad_s_squared > 0.0);
    assert!(result.modes[0].participation_norm > 0.0);
}

fn node(id: &str, x: f64, y: f64, fix_x: bool, fix_y: bool, fix_rz: bool) -> Frame2dNodeInput {
    Frame2dNodeInput {
        id: id.to_string(),
        x,
        y,
        fix_x,
        fix_y,
        fix_rz,
        load_x: 0.0,
        load_y: 0.0,
        moment_z: 0.0,
    }
}

fn assert_close(actual: f64, expected: f64) {
    let scale = expected.abs().max(1.0);
    assert!(
        (actual - expected).abs() <= TOL * scale,
        "expected {actual} to be close to {expected}",
    );
}
