use kyuubiki_protocol::{
    Frame2dNodeInput, Frame3dNodeInput, ModalFrame2dElementInput, ModalFrame3dElementInput,
    SolveModalFrame2dRequest, SolveModalFrame3dRequest,
};
use kyuubiki_solver::{solve_modal_frame_2d, solve_modal_frame_3d};

#[test]
fn modal_frame_2d_rejects_non_finite_node_coordinates_and_load_fields() {
    let mut request = modal_frame_2d_request();
    request.nodes[1].x = f64::NAN;
    assert!(solve_modal_frame_2d(&request).is_err());

    let mut request = modal_frame_2d_request();
    request.nodes[1].load_y = f64::INFINITY;
    assert!(solve_modal_frame_2d(&request).is_err());

    let mut request = modal_frame_2d_request();
    request.nodes[1].moment_z = f64::NEG_INFINITY;
    assert!(solve_modal_frame_2d(&request).is_err());
}

#[test]
fn modal_frame_3d_rejects_non_finite_node_coordinates_loads_and_moments() {
    let mut request = modal_frame_3d_request();
    request.nodes[1].z = f64::NAN;
    assert!(solve_modal_frame_3d(&request).is_err());

    let mut request = modal_frame_3d_request();
    request.nodes[1].load_y = f64::INFINITY;
    assert!(solve_modal_frame_3d(&request).is_err());

    let mut request = modal_frame_3d_request();
    request.nodes[1].moment_z = f64::NEG_INFINITY;
    assert!(solve_modal_frame_3d(&request).is_err());
}

fn modal_frame_2d_request() -> SolveModalFrame2dRequest {
    SolveModalFrame2dRequest {
        nodes: vec![
            frame_2d_node("fixed", 0.0, 0.0, true),
            frame_2d_node("tip", 2.0, 0.0, false),
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
        mode_count: Some(3),
    }
}

fn modal_frame_3d_request() -> SolveModalFrame3dRequest {
    SolveModalFrame3dRequest {
        nodes: vec![
            frame_3d_node("fixed", 0.0, true),
            frame_3d_node("tip", 2.0, false),
        ],
        elements: vec![ModalFrame3dElementInput {
            id: "beam".to_string(),
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
    }
}

fn frame_2d_node(id: &str, x: f64, y: f64, fixed: bool) -> Frame2dNodeInput {
    Frame2dNodeInput {
        id: id.to_string(),
        x,
        y,
        fix_x: fixed,
        fix_y: fixed,
        fix_rz: fixed,
        load_x: 0.0,
        load_y: 0.0,
        moment_z: 0.0,
    }
}

fn frame_3d_node(id: &str, x: f64, fixed: bool) -> Frame3dNodeInput {
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
