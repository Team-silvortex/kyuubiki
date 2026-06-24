use super::prelude::*;

#[test]
fn serializes_torsion_1d_rpc_round_trip() {
    let request = RpcRequest {
        rpc_version: RPC_VERSION,
        id: "rpc-torsion-1d".to_string(),
        method: RpcMethod::SolveTorsion1d,
        params: serde_json::to_value(SolveTorsion1dRequest {
            nodes: vec![
                Torsion1dNodeInput {
                    id: "n0".to_string(),
                    x: 0.0,
                    fix_rz: true,
                    torque_z: 0.0,
                },
                Torsion1dNodeInput {
                    id: "n1".to_string(),
                    x: 1.5,
                    fix_rz: false,
                    torque_z: 500.0,
                },
            ],
            elements: vec![Torsion1dElementInput {
                id: "t0".to_string(),
                node_i: 0,
                node_j: 1,
                shear_modulus: 80.0e9,
                polar_moment: 3.0e-6,
                section_modulus: 2.0e-4,
            }],
        })
        .expect("request params should serialize"),
    };

    let json = serde_json::to_string(&request).expect("request should serialize");
    let decoded: RpcRequest = serde_json::from_str(&json).expect("request should decode");

    assert_eq!(decoded.method, RpcMethod::SolveTorsion1d);
    assert_eq!(decoded.id, "rpc-torsion-1d");
}

#[test]
fn serializes_spring_1d_rpc_round_trip() {
    let request = RpcRequest {
        rpc_version: RPC_VERSION,
        id: "rpc-spring-1d".to_string(),
        method: RpcMethod::SolveSpring1d,
        params: serde_json::to_value(SolveSpring1dRequest {
            nodes: vec![
                Spring1dNodeInput {
                    id: "n0".to_string(),
                    x: 0.0,
                    fix_x: true,
                    load_x: 0.0,
                },
                Spring1dNodeInput {
                    id: "n1".to_string(),
                    x: 1.0,
                    fix_x: false,
                    load_x: 1000.0,
                },
            ],
            elements: vec![Spring1dElementInput {
                id: "s0".to_string(),
                node_i: 0,
                node_j: 1,
                stiffness: 25_000.0,
            }],
        })
        .expect("request params should serialize"),
    };

    let json = serde_json::to_string(&request).expect("request should serialize");
    let decoded: RpcRequest = serde_json::from_str(&json).expect("request should decode");

    assert_eq!(decoded.method, RpcMethod::SolveSpring1d);
    assert_eq!(decoded.id, "rpc-spring-1d");
}

#[test]
fn serializes_spring_2d_rpc_round_trip() {
    let request = RpcRequest {
        rpc_version: RPC_VERSION,
        id: "rpc-spring-2d".to_string(),
        method: RpcMethod::SolveSpring2d,
        params: serde_json::to_value(SolveSpring2dRequest {
            nodes: vec![
                Spring2dNodeInput {
                    id: "n0".to_string(),
                    x: 0.0,
                    y: 0.0,
                    fix_x: true,
                    fix_y: true,
                    load_x: 0.0,
                    load_y: 0.0,
                },
                Spring2dNodeInput {
                    id: "n1".to_string(),
                    x: 1.0,
                    y: 1.0,
                    fix_x: false,
                    fix_y: false,
                    load_x: 1000.0,
                    load_y: -500.0,
                },
            ],
            elements: vec![Spring2dElementInput {
                id: "s0".to_string(),
                node_i: 0,
                node_j: 1,
                stiffness: 25_000.0,
            }],
        })
        .expect("request params should serialize"),
    };

    let json = serde_json::to_string(&request).expect("request should serialize");
    let decoded: RpcRequest = serde_json::from_str(&json).expect("request should decode");

    assert_eq!(decoded.method, RpcMethod::SolveSpring2d);
    assert_eq!(decoded.id, "rpc-spring-2d");
}

#[test]
fn serializes_spring_3d_rpc_round_trip() {
    let request = RpcRequest {
        rpc_version: RPC_VERSION,
        id: "rpc-spring-3d".to_string(),
        method: RpcMethod::SolveSpring3d,
        params: serde_json::to_value(SolveSpring3dRequest {
            nodes: vec![
                Spring3dNodeInput {
                    id: "n0".to_string(),
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                    fix_x: true,
                    fix_y: true,
                    fix_z: true,
                    load_x: 0.0,
                    load_y: 0.0,
                    load_z: 0.0,
                },
                Spring3dNodeInput {
                    id: "n1".to_string(),
                    x: 1.0,
                    y: 1.0,
                    z: 1.0,
                    fix_x: false,
                    fix_y: false,
                    fix_z: false,
                    load_x: 1000.0,
                    load_y: -500.0,
                    load_z: 250.0,
                },
            ],
            elements: vec![Spring3dElementInput {
                id: "s0".to_string(),
                node_i: 0,
                node_j: 1,
                stiffness: 25_000.0,
            }],
        })
        .expect("request params should serialize"),
    };

    let json = serde_json::to_string(&request).expect("request should serialize");
    let decoded: RpcRequest = serde_json::from_str(&json).expect("request should decode");

    assert_eq!(decoded.method, RpcMethod::SolveSpring3d);
    assert_eq!(decoded.id, "rpc-spring-3d");
}

#[test]
fn serializes_truss_3d_rpc_round_trip() {
    let request = RpcRequest {
        rpc_version: RPC_VERSION,
        id: "rpc-truss-3d".to_string(),
        method: RpcMethod::SolveTruss3d,
        params: serde_json::to_value(SolveTruss3dRequest {
            nodes: vec![],
            elements: vec![],
        })
        .expect("request params should serialize"),
    };

    let json = serde_json::to_string(&request).expect("request should serialize");
    let decoded: RpcRequest = serde_json::from_str(&json).expect("request should decode");

    assert_eq!(decoded.method, RpcMethod::SolveTruss3d);
    assert_eq!(decoded.id, "rpc-truss-3d");
}

#[test]
fn builds_error_responses() {
    let response = RpcResponse::error("rpc-1", "invalid_request", "unsupported method");

    assert!(!response.ok);
    assert!(response.result.is_none());
    assert_eq!(response.rpc_version, 1);
    assert_eq!(response.id, "rpc-1");
    assert_eq!(
        response.error.expect("error payload").code,
        "invalid_request"
    );
}

#[test]
fn serializes_agent_descriptor_round_trip() {
    let descriptor = AgentDescriptor::solver_agent_default();

    let json = serde_json::to_string(&descriptor).expect("descriptor should serialize");
    let decoded: AgentDescriptor = serde_json::from_str(&json).expect("descriptor should decode");

    assert_eq!(decoded.program, "kyuubiki-rust-agent");
    assert_eq!(decoded.protocol.rpc_version, RPC_VERSION);
    assert!(decoded.protocol.methods.contains(&RpcMethod::DescribeAgent));
    assert_eq!(decoded.authority.control_mode, "standalone");
    assert_eq!(decoded.authority.authority_mode, "self_directed");
}

#[test]
fn serializes_progress_frames() {
    let progress = RpcProgress::new(
        "rpc-1",
        ProgressEvent::new("job-1", JobStatus::Solving, 0.5),
    );

    let json = serde_json::to_string(&progress).expect("progress should serialize");
    let decoded: RpcProgress = serde_json::from_str(&json).expect("progress should decode");

    assert_eq!(decoded.id, "rpc-1");
    assert_eq!(decoded.event, "progress");
    assert_eq!(decoded.progress.job_id, "job-1");
}
