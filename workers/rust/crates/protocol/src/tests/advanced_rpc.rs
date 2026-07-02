use crate::{
    ContactGap1dContactInput, Frame2dNodeInput, Frame3dNodeInput, ModalFrame2dElementInput,
    ModalFrame3dElementInput, NonlinearSpring1dElementInput, NonlinearSpring1dNodeInput,
    RPC_VERSION, RpcMethod, RpcRequest, SolidTetra3dElementInput, SolidTetra3dNodeInput,
    SolveContactGap1dRequest, SolveModalFrame2dRequest, SolveModalFrame3dRequest,
    SolveNonlinearSpring1dRequest, SolveSolidTetra3dRequest, SolveStokesFlowPlaneQuad2dRequest,
    StokesFlowPlaneNodeInput, StokesFlowPlaneQuadElementInput,
};

#[test]
fn serializes_stokes_flow_quad_2d_rpc_round_trip() {
    let decoded = round_trip(RpcMethod::SolveStokesFlowPlaneQuad2d, stokes_request());

    assert_eq!(decoded.method, RpcMethod::SolveStokesFlowPlaneQuad2d);
    assert_eq!(decoded.id, "advanced-rpc");
}

#[test]
fn serializes_nonlinear_and_contact_rpc_round_trips() {
    let nonlinear = round_trip(RpcMethod::SolveNonlinearSpring1d, nonlinear_request());
    let contact = round_trip(RpcMethod::SolveContactGap1d, contact_request());

    assert_eq!(nonlinear.method, RpcMethod::SolveNonlinearSpring1d);
    assert_eq!(contact.method, RpcMethod::SolveContactGap1d);
}

#[test]
fn serializes_modal_frame_rpc_round_trips() {
    let modal_2d = round_trip(RpcMethod::SolveModalFrame2d, modal_frame_2d_request());
    let modal_3d = round_trip(RpcMethod::SolveModalFrame3d, modal_frame_3d_request());

    assert_eq!(modal_2d.method, RpcMethod::SolveModalFrame2d);
    assert_eq!(modal_3d.method, RpcMethod::SolveModalFrame3d);
}

#[test]
fn serializes_solid_tetra_rpc_round_trip() {
    let decoded = round_trip(RpcMethod::SolveSolidTetra3d, solid_tetra_request());

    assert_eq!(decoded.method, RpcMethod::SolveSolidTetra3d);
}

fn round_trip(method: RpcMethod, params: impl serde::Serialize) -> RpcRequest {
    let request = RpcRequest {
        rpc_version: RPC_VERSION,
        id: "advanced-rpc".to_string(),
        method,
        params: serde_json::to_value(params).expect("params should serialize"),
    };
    let json = serde_json::to_string(&request).expect("request should serialize");
    serde_json::from_str(&json).expect("request should decode")
}

fn nonlinear_request() -> SolveNonlinearSpring1dRequest {
    SolveNonlinearSpring1dRequest {
        nodes: spring_nodes(),
        elements: vec![spring_element()],
        load_steps: Some(4),
        max_iterations: Some(16),
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
        load_steps: Some(4),
        max_iterations: Some(16),
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

fn solid_tetra_request() -> SolveSolidTetra3dRequest {
    SolveSolidTetra3dRequest {
        nodes: vec![
            solid_tetra_node("n0", 0.0, 0.0, 0.0, true, 0.0),
            solid_tetra_node("n1", 1.0, 0.0, 0.0, true, 0.0),
            solid_tetra_node("n2", 0.0, 1.0, 0.0, true, 0.0),
            solid_tetra_node("n3", 0.0, 0.0, 1.0, false, -1000.0),
        ],
        elements: vec![SolidTetra3dElementInput {
            id: "t0".to_string(),
            node_a: 0,
            node_b: 1,
            node_c: 2,
            node_d: 3,
            youngs_modulus: 70.0e9,
            poisson_ratio: 0.33,
        }],
    }
}

fn solid_tetra_node(
    id: &str,
    x: f64,
    y: f64,
    z: f64,
    fixed: bool,
    load_z: f64,
) -> SolidTetra3dNodeInput {
    SolidTetra3dNodeInput {
        id: id.to_string(),
        x,
        y,
        z,
        fix_x: fixed,
        fix_y: fixed,
        fix_z: fixed,
        load_x: 0.0,
        load_y: 0.0,
        load_z,
    }
}
