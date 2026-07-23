use super::*;
use kyuubiki_protocol::{
    ContactGap1dContactInput, Frame2dElementInput, Frame2dNodeInput, Frame3dNodeInput,
    ModalFrame2dElementInput, ModalFrame3dElementInput, NonlinearSpring1dElementInput,
    NonlinearSpring1dNodeInput, SolveBucklingFrame2dRequest, SolveContactGap1dRequest,
    SolveFrame2dPDeltaRequest, SolveFrame2dRequest, SolveModalFrame2dRequest,
    SolveModalFrame3dRequest, SolveNonlinearSpring1dRequest, SolveStokesFlowPlaneQuad2dRequest,
    SolveStokesFlowPlaneTriangle2dRequest, StokesFlowPlaneNodeInput,
    StokesFlowPlaneQuadElementInput, StokesFlowPlaneTriangleElementInput,
};

#[test]
fn handles_stokes_flow_plane_quad_2d_rpc_requests() {
    let final_response = execute(RpcMethod::SolveStokesFlowPlaneQuad2d, stokes_request());

    assert!(final_response.ok);
    let result: kyuubiki_protocol::SolveStokesFlowPlaneQuad2dResult =
        serde_json::from_value(final_response.result.expect("solver result"))
            .expect("stokes result");
    assert_eq!(result.nodes.len(), 4);
    assert_eq!(result.elements.len(), 1);
    assert!(result.max_velocity > 0.0);
    assert!(result.max_reynolds_number >= 0.0);
}

#[test]
fn handles_stokes_flow_plane_triangle_2d_rpc_requests() {
    let final_response = execute(
        RpcMethod::SolveStokesFlowPlaneTriangle2d,
        stokes_triangle_request(),
    );
    assert!(final_response.ok);
    let result: kyuubiki_protocol::SolveStokesFlowPlaneTriangle2dResult =
        serde_json::from_value(final_response.result.expect("solver result"))
            .expect("stokes triangle result");
    assert_eq!(result.nodes.len(), 3);
    assert_eq!(result.elements.len(), 1);
    assert!(result.max_velocity > 0.0);
    assert!(result.max_reynolds_number >= 0.0);
}

#[test]
fn handles_nonlinear_spring_1d_rpc_requests() {
    let final_response = execute(RpcMethod::SolveNonlinearSpring1d, nonlinear_request());

    assert!(final_response.ok);
    let result: kyuubiki_protocol::SolveNonlinearSpring1dResult =
        serde_json::from_value(final_response.result.expect("solver result"))
            .expect("nonlinear spring result");
    assert!(result.converged);
    assert_eq!(result.nodes.len(), 2);
    assert_eq!(result.elements.len(), 1);
    assert!(result.max_displacement > 0.0);
    assert!(result.max_force > 0.0);
}

#[test]
fn handles_contact_gap_1d_rpc_requests() {
    let final_response = execute(RpcMethod::SolveContactGap1d, contact_request());

    assert!(final_response.ok);
    let result: kyuubiki_protocol::SolveContactGap1dResult =
        serde_json::from_value(final_response.result.expect("solver result"))
            .expect("contact gap result");
    assert!(result.converged);
    assert_eq!(result.contacts.len(), 1);
    assert_eq!(result.active_contact_count, 1);
    assert!(result.max_contact_force > 0.0);
}

#[test]
fn handles_modal_frame_rpc_requests() {
    let modal_2d = execute(RpcMethod::SolveModalFrame2d, modal_frame_2d_request());
    let modal_3d = execute(RpcMethod::SolveModalFrame3d, modal_frame_3d_request());

    assert!(modal_2d.ok);
    assert!(modal_3d.ok);

    let result_2d: kyuubiki_protocol::SolveModalFrame2dResult =
        serde_json::from_value(modal_2d.result.expect("2d modal result"))
            .expect("2d modal frame result");
    let result_3d: kyuubiki_protocol::SolveModalFrame3dResult =
        serde_json::from_value(modal_3d.result.expect("3d modal result"))
            .expect("3d modal frame result");

    assert!(!result_2d.modes.is_empty());
    assert!(!result_3d.modes.is_empty());
    assert!(result_2d.min_frequency_hz > 0.0);
    assert!(result_3d.min_frequency_hz > 0.0);
}

#[test]
fn handles_frame_2d_p_delta_rpc_requests() {
    let response = execute(RpcMethod::SolveFrame2dPDelta, frame_2d_p_delta_request());

    assert!(response.ok);
    let result: kyuubiki_protocol::SolveFrame2dPDeltaResult =
        serde_json::from_value(response.result.expect("p-delta solver result"))
            .expect("p-delta result");
    assert_eq!(result.steps.len(), 4);
    assert!(result.converged);
    assert_eq!(
        result.kinematics,
        kyuubiki_protocol::Frame2dStabilityKinematics::LinearizedPDelta
    );
    assert!(result.max_imperfection_amplification > 1.0);
    assert_eq!(
        result.imperfection_source,
        kyuubiki_protocol::Frame2dImperfectionSource::BucklingMode
    );
    assert_eq!(result.critical_factor_limit_ratio, 0.95);
}

fn execute(method: RpcMethod, params: impl serde::Serialize) -> kyuubiki_protocol::RpcResponse {
    let request = RpcRequest {
        rpc_version: RPC_VERSION,
        id: "advanced-rpc".to_string(),
        method,
        params: serde_json::to_value(params).expect("params should serialize"),
    };
    let response =
        handle_request_bytes(&serde_json::to_vec(&request).expect("request should serialize"));
    let AgentReply::Stream(progress_frames, final_response) = response;

    assert_eq!(progress_frames.len(), 4);
    final_response
}

fn nonlinear_request() -> SolveNonlinearSpring1dRequest {
    SolveNonlinearSpring1dRequest {
        nodes: spring_nodes(),
        elements: vec![spring_element()],
        load_steps: Some(8),
        max_iterations: Some(32),
        tolerance: Some(1.0e-10),
    }
}

fn contact_request() -> SolveContactGap1dRequest {
    SolveContactGap1dRequest {
        nodes: spring_nodes(),
        elements: vec![spring_element()],
        contacts: vec![ContactGap1dContactInput {
            id: "c0".to_string(),
            node: 1,
            gap: 0.01,
            normal_stiffness: 100_000.0,
        }],
        load_steps: Some(8),
        max_iterations: Some(32),
        tolerance: Some(1.0e-10),
    }
}

fn spring_nodes() -> Vec<NonlinearSpring1dNodeInput> {
    vec![
        NonlinearSpring1dNodeInput {
            id: "n0".to_string(),
            x: 0.0,
            fix_x: true,
            load_x: 0.0,
        },
        NonlinearSpring1dNodeInput {
            id: "n1".to_string(),
            x: 1.0,
            fix_x: false,
            load_x: 1000.0,
        },
    ]
}

fn spring_element() -> NonlinearSpring1dElementInput {
    NonlinearSpring1dElementInput {
        id: "s0".to_string(),
        node_i: 0,
        node_j: 1,
        stiffness: 25_000.0,
        cubic_stiffness: 10_000.0,
    }
}

fn stokes_request() -> SolveStokesFlowPlaneQuad2dRequest {
    SolveStokesFlowPlaneQuad2dRequest {
        nodes: vec![
            stokes_node("n0", 0.0, 0.0),
            stokes_node("n1", 1.0, 0.0),
            stokes_node("n2", 1.0, 1.0),
            stokes_node("n3", 0.0, 1.0),
        ],
        elements: vec![StokesFlowPlaneQuadElementInput {
            id: "q0".to_string(),
            node_i: 0,
            node_j: 1,
            node_k: 2,
            node_l: 3,
            thickness: 0.1,
            viscosity: 1.0,
            density: 1000.0,
        }],
    }
}

fn stokes_triangle_request() -> SolveStokesFlowPlaneTriangle2dRequest {
    SolveStokesFlowPlaneTriangle2dRequest {
        nodes: vec![
            stokes_node("n0", 0.0, 0.0),
            stokes_node("n1", 1.0, 0.0),
            stokes_node("n2", 0.0, 1.0),
        ],
        elements: vec![StokesFlowPlaneTriangleElementInput {
            id: "t0".to_string(),
            node_i: 0,
            node_j: 1,
            node_k: 2,
            thickness: 0.1,
            viscosity: 1.0,
            density: 1000.0,
        }],
    }
}

fn stokes_node(id: &str, x: f64, y: f64) -> StokesFlowPlaneNodeInput {
    StokesFlowPlaneNodeInput {
        id: id.to_string(),
        x,
        y,
        fix_velocity_x: false,
        velocity_x: 0.0,
        fix_velocity_y: false,
        velocity_y: 0.0,
        fix_pressure: false,
        pressure: 0.0,
        body_force_x: 1.0,
        body_force_y: 0.0,
    }
}

fn modal_frame_2d_request() -> SolveModalFrame2dRequest {
    SolveModalFrame2dRequest {
        nodes: vec![
            frame_2d_node("n0", 0.0, true),
            frame_2d_node("n1", 2.0, false),
        ],
        elements: vec![ModalFrame2dElementInput {
            id: "m0".to_string(),
            node_i: 0,
            node_j: 1,
            area: 0.02,
            youngs_modulus: 210.0e9,
            moment_of_inertia: 8.0e-6,
            section_modulus: 1.6e-4,
            density: 7850.0,
        }],
        mode_count: Some(2),
    }
}

fn frame_2d_node(id: &str, x: f64, fixed: bool) -> Frame2dNodeInput {
    Frame2dNodeInput {
        id: id.to_string(),
        x,
        y: 0.0,
        fix_x: fixed,
        fix_y: fixed,
        fix_rz: fixed,
        load_x: 0.0,
        load_y: 0.0,
        moment_z: 0.0,
    }
}

fn frame_2d_p_delta_request() -> SolveFrame2dPDeltaRequest {
    let nodes = vec![
        stability_node("n0", 0.0, true, true, 0.0),
        stability_node("n1", 1.0, false, false, 0.0),
        stability_node("n2", 2.0, true, false, -100_000.0),
    ];
    let elements = (0..2)
        .map(|index| Frame2dElementInput {
            id: format!("e{index}"),
            node_i: index,
            node_j: index + 1,
            area: 0.01,
            youngs_modulus: 210.0e9,
            moment_of_inertia: 8.0e-6,
            section_modulus: 1.6e-4,
        })
        .collect();
    SolveFrame2dPDeltaRequest {
        buckling: SolveBucklingFrame2dRequest {
            frame: SolveFrame2dRequest { nodes, elements },
            mode_count: Some(1),
        },
        imperfection_amplitude: 0.001,
        kinematics: Default::default(),
        path_control: Default::default(),
        imperfection_shape: None,
        imperfection_mode_index: Some(0),
        maximum_load_factor: None,
        load_steps: Some(4),
        max_iterations: None,
        tolerance: None,
        max_step_cutbacks: None,
        arc_length_radius: None,
        arc_length_load_scale: None,
        arc_length_target_iterations: None,
        tangent_transition_refinement_steps: None,
        branch_switch: Default::default(),
        branch_switch_amplitude: None,
        branch_continuation_steps: None,
    }
}

fn stability_node(id: &str, y: f64, fix_x: bool, fix_y: bool, load_y: f64) -> Frame2dNodeInput {
    Frame2dNodeInput {
        id: id.to_string(),
        x: 0.0,
        y,
        fix_x,
        fix_y,
        fix_rz: false,
        load_x: 0.0,
        load_y,
        moment_z: 0.0,
    }
}

fn modal_frame_3d_request() -> SolveModalFrame3dRequest {
    SolveModalFrame3dRequest {
        nodes: vec![
            frame_3d_node("n0", 0.0, true),
            frame_3d_node("n1", 2.0, false),
        ],
        elements: vec![ModalFrame3dElementInput {
            id: "m0".to_string(),
            node_i: 0,
            node_j: 1,
            area: 0.02,
            youngs_modulus: 210.0e9,
            shear_modulus: 80.0e9,
            torsion_constant: 5.0e-6,
            moment_of_inertia_y: 8.0e-6,
            moment_of_inertia_z: 6.0e-6,
            density: 7850.0,
        }],
        mode_count: Some(2),
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
