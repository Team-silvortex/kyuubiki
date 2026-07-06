use kyuubiki_protocol::{Frame3dNodeInput, ModalFrame3dElementInput, SolveModalFrame3dRequest};
use kyuubiki_solver::solve_modal_frame_3d;

const TOL: f64 = 1.0e-10;

#[test]
fn modal_frame_3d_review_bundle_checks_cantilever_modes_and_shape_contract() {
    let density = 7850.0;
    let area = 0.01;
    let length = 2.0;
    let result = solve_modal_frame_3d(&SolveModalFrame3dRequest {
        nodes: vec![node("fixed", 0.0, true), node("tip", length, false)],
        elements: vec![ModalFrame3dElementInput {
            id: "beam".to_string(),
            node_i: 0,
            node_j: 1,
            area,
            youngs_modulus: 210.0e9,
            shear_modulus: 80.0e9,
            torsion_constant: 1.0e-5,
            moment_of_inertia_y: 8.333e-6,
            moment_of_inertia_z: 8.333e-6,
            density,
        }],
        mode_count: Some(3),
    })
    .expect("review 3d modal frame should solve");

    assert_eq!(result.free_dofs, vec![6, 7, 8, 9, 10, 11]);
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
        assert!(mode.natural_frequency_hz >= previous_frequency - TOL);
        assert_close(
            mode.natural_frequency_rad_s,
            mode.eigenvalue_rad_s_squared.sqrt(),
        );
        assert_close(
            mode.natural_frequency_hz,
            mode.natural_frequency_rad_s / std::f64::consts::TAU,
        );
        assert_close(mode.period_s, 1.0 / mode.natural_frequency_hz);
        assert_eq!(mode.shape.len(), 12);
        for restrained_dof in 0..6 {
            assert_close(mode.shape[restrained_dof], 0.0);
        }
        assert_close(mode.participation_norm, 1.0);
        assert!(mode.shape[6..].iter().all(|value| value.is_finite()));
        assert!(mode.shape[6..].iter().any(|value| value.abs() > 1.0e-6));
        previous_frequency = mode.natural_frequency_hz;
    }
}

fn node(id: &str, x: f64, fixed: bool) -> Frame3dNodeInput {
    Frame3dNodeInput {
        id: id.to_string(),
        x,
        y: 0.0,
        z: 0.0,
        fix_x: fixed,
        fix_y: fixed,
        fix_z: fixed,
        fix_rx: fixed,
        fix_ry: fixed,
        fix_rz: fixed,
        load_x: 0.0,
        load_y: 0.0,
        load_z: 0.0,
        moment_x: 0.0,
        moment_y: 0.0,
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
