use kyuubiki_protocol::{
    Frame2dNodeInput, Frame3dNodeInput, ModalFrame2dElementInput, ModalFrame3dElementInput,
    SolveModalFrame2dRequest, SolveModalFrame3dRequest,
};
use kyuubiki_solver::{solve_modal_frame_2d, solve_modal_frame_3d};

#[test]
fn solves_modal_frame_2d_cantilever_modes() {
    let result = solve_modal_frame_2d(&SolveModalFrame2dRequest {
        nodes: vec![
            node("n0", 0.0, 0.0, true, true, true),
            node("n1", 2.0, 0.0, false, false, false),
        ],
        elements: vec![ModalFrame2dElementInput {
            id: "e0".to_string(),
            node_i: 0,
            node_j: 1,
            area: 0.01,
            youngs_modulus: 210.0e9,
            moment_of_inertia: 8.333e-6,
            section_modulus: 1.667e-4,
            density: 7850.0,
        }],
        mode_count: Some(3),
    })
    .expect("modal frame should solve");

    assert_eq!(result.modes.len(), 3);
    assert_eq!(result.free_dofs, vec![3, 4, 5]);
    assert!(result.total_mass > 0.0);
    assert!(result.min_frequency_hz > 0.0);
    assert!(result.max_frequency_hz >= result.min_frequency_hz);
    for mode in &result.modes {
        assert!(mode.natural_frequency_hz.is_finite());
        assert!(mode.natural_frequency_hz > 0.0);
        assert_eq!(mode.shape.len(), 6);
        assert!(mode.participation_norm > 0.0);
    }
}

#[test]
fn solves_modal_frame_3d_cantilever_modes() {
    let result = solve_modal_frame_3d(&SolveModalFrame3dRequest {
        nodes: vec![node_3d("n0", 0.0, true), node_3d("n1", 2.0, false)],
        elements: vec![ModalFrame3dElementInput {
            id: "e0".to_string(),
            node_i: 0,
            node_j: 1,
            area: 0.01,
            youngs_modulus: 210.0e9,
            shear_modulus: 80.0e9,
            torsion_constant: 1.0e-5,
            moment_of_inertia_y: 8.333e-6,
            moment_of_inertia_z: 8.333e-6,
            density: 7850.0,
        }],
        mode_count: Some(3),
    })
    .expect("3d modal frame should solve");

    assert_eq!(result.modes.len(), 3);
    assert_eq!(result.free_dofs, vec![6, 7, 8, 9, 10, 11]);
    assert!(result.total_mass > 0.0);
    assert!(result.min_frequency_hz > 0.0);
    assert!(result.max_frequency_hz >= result.min_frequency_hz);
    for mode in &result.modes {
        assert!(mode.natural_frequency_hz.is_finite());
        assert!(mode.natural_frequency_hz > 0.0);
        assert_eq!(mode.shape.len(), 12);
        assert!(mode.participation_norm > 0.0);
    }
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

fn node_3d(id: &str, x: f64, fixed: bool) -> Frame3dNodeInput {
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
