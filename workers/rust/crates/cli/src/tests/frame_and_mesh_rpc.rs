use super::*;

#[test]
fn handles_frame_2d_rpc_requests() {
    let request = RpcRequest {
        rpc_version: RPC_VERSION,
        id: "rpc-frame-2d".to_string(),
        method: RpcMethod::SolveFrame2d,
        params: serde_json::to_value(SolveFrame2dRequest {
            nodes: vec![
                kyuubiki_protocol::Frame2dNodeInput {
                    id: "n0".to_string(),
                    x: 0.0,
                    y: 0.0,
                    fix_x: true,
                    fix_y: true,
                    fix_rz: true,
                    load_x: 0.0,
                    load_y: 0.0,
                    moment_z: 0.0,
                },
                kyuubiki_protocol::Frame2dNodeInput {
                    id: "n1".to_string(),
                    x: 2.0,
                    y: 0.0,
                    fix_x: false,
                    fix_y: false,
                    fix_rz: false,
                    load_x: 0.0,
                    load_y: -1000.0,
                    moment_z: 0.0,
                },
            ],
            elements: vec![kyuubiki_protocol::Frame2dElementInput {
                id: "f0".to_string(),
                node_i: 0,
                node_j: 1,
                area: 0.02,
                youngs_modulus: 210.0e9,
                moment_of_inertia: 8.0e-6,
                section_modulus: 1.6e-4,
            }],
        })
        .expect("params"),
    };

    let response =
        handle_request_bytes(&serde_json::to_vec(&request).expect("request should serialize"));

    let AgentReply::Stream(progress_frames, final_response) = response;

    assert_eq!(progress_frames.len(), 4);
    assert!(final_response.ok);
    let result: kyuubiki_protocol::SolveFrame2dResult =
        serde_json::from_value(final_response.result.expect("solver result"))
            .expect("frame result");
    assert_eq!(result.nodes.len(), 2);
    assert_eq!(result.elements.len(), 1);
    assert!(result.max_moment > 0.0);
}

#[test]
fn handles_thermal_frame_2d_rpc_requests() {
    let request = RpcRequest {
        rpc_version: RPC_VERSION,
        id: "rpc-thermal-frame-2d".to_string(),
        method: RpcMethod::SolveThermalFrame2d,
        params: serde_json::to_value(SolveThermalFrame2dRequest {
            nodes: vec![
                kyuubiki_protocol::ThermalFrame2dNodeInput {
                    id: "n0".to_string(),
                    x: 0.0,
                    y: 0.0,
                    fix_x: true,
                    fix_y: true,
                    fix_rz: true,
                    load_x: 0.0,
                    load_y: 0.0,
                    moment_z: 0.0,
                    temperature_delta: 35.0,
                },
                kyuubiki_protocol::ThermalFrame2dNodeInput {
                    id: "n1".to_string(),
                    x: 2.0,
                    y: 0.0,
                    fix_x: true,
                    fix_y: true,
                    fix_rz: true,
                    load_x: 0.0,
                    load_y: 0.0,
                    moment_z: 0.0,
                    temperature_delta: 35.0,
                },
            ],
            elements: vec![kyuubiki_protocol::ThermalFrame2dElementInput {
                id: "tf0".to_string(),
                node_i: 0,
                node_j: 1,
                area: 0.02,
                youngs_modulus: 210.0e9,
                moment_of_inertia: 8.0e-6,
                section_modulus: 1.6e-4,
                thermal_expansion: 12.0e-6,
                section_depth: 0.2,
                temperature_gradient_y: 30.0,
            }],
        })
        .expect("params"),
    };

    let response =
        handle_request_bytes(&serde_json::to_vec(&request).expect("request should serialize"));

    let AgentReply::Stream(progress_frames, final_response) = response;

    assert_eq!(progress_frames.len(), 4);
    assert!(final_response.ok);
    let result: kyuubiki_protocol::SolveThermalFrame2dResult =
        serde_json::from_value(final_response.result.expect("solver result"))
            .expect("thermal frame result");
    assert_eq!(result.nodes.len(), 2);
    assert_eq!(result.elements.len(), 1);
    assert!(result.max_axial_force > 0.0);
    assert_eq!(result.max_temperature_delta, 35.0);
    assert_eq!(result.max_temperature_gradient, 30.0);
}

#[test]
fn handles_frame_3d_rpc_requests() {
    let request = RpcRequest {
        rpc_version: RPC_VERSION,
        id: "rpc-frame-3d".to_string(),
        method: RpcMethod::SolveFrame3d,
        params: serde_json::to_value(SolveFrame3dRequest {
            nodes: vec![
                kyuubiki_protocol::Frame3dNodeInput {
                    id: "n0".to_string(),
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                    fix_x: true,
                    fix_y: true,
                    fix_z: true,
                    fix_rx: true,
                    fix_ry: true,
                    fix_rz: true,
                    load_x: 0.0,
                    load_y: 0.0,
                    load_z: 0.0,
                    moment_x: 0.0,
                    moment_y: 0.0,
                    moment_z: 0.0,
                },
                kyuubiki_protocol::Frame3dNodeInput {
                    id: "n1".to_string(),
                    x: 2.0,
                    y: 0.0,
                    z: 0.0,
                    fix_x: false,
                    fix_y: false,
                    fix_z: false,
                    fix_rx: false,
                    fix_ry: false,
                    fix_rz: false,
                    load_x: 0.0,
                    load_y: -1000.0,
                    load_z: 0.0,
                    moment_x: 0.0,
                    moment_y: 0.0,
                    moment_z: 0.0,
                },
            ],
            elements: vec![kyuubiki_protocol::Frame3dElementInput {
                id: "f0".to_string(),
                node_i: 0,
                node_j: 1,
                area: 0.02,
                youngs_modulus: 210.0e9,
                shear_modulus: 80.0e9,
                torsion_constant: 5.0e-6,
                moment_of_inertia_y: 8.0e-6,
                moment_of_inertia_z: 8.0e-6,
                section_modulus_y: 1.6e-4,
                section_modulus_z: 1.6e-4,
            }],
        })
        .expect("params"),
    };

    let response =
        handle_request_bytes(&serde_json::to_vec(&request).expect("request should serialize"));

    let AgentReply::Stream(progress_frames, final_response) = response;

    assert_eq!(progress_frames.len(), 4);
    assert!(final_response.ok);
    let result: kyuubiki_protocol::SolveFrame3dResult =
        serde_json::from_value(final_response.result.expect("solver result"))
            .expect("frame 3d result");
    assert_eq!(result.nodes.len(), 2);
    assert_eq!(result.elements.len(), 1);
    assert!(result.max_displacement > 0.0);
    assert!(result.max_rotation > 0.0);
    assert!(result.max_moment > 0.0);
    assert!(result.max_stress > 0.0);
}

#[test]
fn handles_thermal_frame_3d_rpc_requests() {
    let request = RpcRequest {
        rpc_version: RPC_VERSION,
        id: "rpc-thermal-frame-3d".to_string(),
        method: RpcMethod::SolveThermalFrame3d,
        params: serde_json::to_value(SolveThermalFrame3dRequest {
            nodes: vec![
                kyuubiki_protocol::ThermalFrame3dNodeInput {
                    id: "n0".to_string(),
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                    fix_x: true,
                    fix_y: true,
                    fix_z: true,
                    fix_rx: true,
                    fix_ry: true,
                    fix_rz: true,
                    load_x: 0.0,
                    load_y: 0.0,
                    load_z: 0.0,
                    moment_x: 0.0,
                    moment_y: 0.0,
                    moment_z: 0.0,
                    temperature_delta: 35.0,
                },
                kyuubiki_protocol::ThermalFrame3dNodeInput {
                    id: "n1".to_string(),
                    x: 2.0,
                    y: 0.0,
                    z: 0.0,
                    fix_x: true,
                    fix_y: true,
                    fix_z: true,
                    fix_rx: true,
                    fix_ry: true,
                    fix_rz: true,
                    load_x: 0.0,
                    load_y: 0.0,
                    load_z: 0.0,
                    moment_x: 0.0,
                    moment_y: 0.0,
                    moment_z: 0.0,
                    temperature_delta: 35.0,
                },
            ],
            elements: vec![kyuubiki_protocol::ThermalFrame3dElementInput {
                id: "tf3-0".to_string(),
                node_i: 0,
                node_j: 1,
                area: 0.02,
                youngs_modulus: 210.0e9,
                shear_modulus: 80.0e9,
                torsion_constant: 5.0e-6,
                moment_of_inertia_y: 8.0e-6,
                moment_of_inertia_z: 6.0e-6,
                section_modulus_y: 1.6e-4,
                section_modulus_z: 1.2e-4,
                thermal_expansion: 12.0e-6,
                section_depth_y: 0.2,
                section_depth_z: 0.15,
                temperature_gradient_y: 30.0,
                temperature_gradient_z: 20.0,
            }],
        })
        .expect("params"),
    };

    let response =
        handle_request_bytes(&serde_json::to_vec(&request).expect("request should serialize"));

    let AgentReply::Stream(progress_frames, final_response) = response;

    assert_eq!(progress_frames.len(), 4);
    assert!(final_response.ok);
    let result: kyuubiki_protocol::SolveThermalFrame3dResult =
        serde_json::from_value(final_response.result.expect("solver result"))
            .expect("thermal frame 3d result");
    assert_eq!(result.nodes.len(), 2);
    assert_eq!(result.elements.len(), 1);
    assert!(result.max_axial_force > 0.0);
    assert!(result.max_moment > 0.0);
    assert_eq!(result.max_temperature_delta, 35.0);
    assert_eq!(result.max_temperature_gradient, 30.0);
}

#[test]
fn handles_truss_3d_rpc_requests() {
    let request = RpcRequest {
        rpc_version: RPC_VERSION,
        id: "rpc-truss-3d".to_string(),
        method: RpcMethod::SolveTruss3d,
        params: serde_json::to_value(SolveTruss3dRequest {
            nodes: vec![
                Truss3dNodeInput {
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
                Truss3dNodeInput {
                    id: "n1".to_string(),
                    x: 1.0,
                    y: 0.0,
                    z: 0.0,
                    fix_x: true,
                    fix_y: true,
                    fix_z: true,
                    load_x: 0.0,
                    load_y: 0.0,
                    load_z: 0.0,
                },
                Truss3dNodeInput {
                    id: "n2".to_string(),
                    x: 0.0,
                    y: 1.0,
                    z: 0.0,
                    fix_x: true,
                    fix_y: true,
                    fix_z: true,
                    load_x: 0.0,
                    load_y: 0.0,
                    load_z: 0.0,
                },
                Truss3dNodeInput {
                    id: "n3".to_string(),
                    x: 0.2,
                    y: 0.2,
                    z: 1.0,
                    fix_x: false,
                    fix_y: false,
                    fix_z: false,
                    load_x: 0.0,
                    load_y: 0.0,
                    load_z: -1000.0,
                },
            ],
            elements: vec![
                Truss3dElementInput {
                    id: "e0".to_string(),
                    node_i: 0,
                    node_j: 3,
                    area: 0.01,
                    youngs_modulus: 70.0e9,
                },
                Truss3dElementInput {
                    id: "e1".to_string(),
                    node_i: 1,
                    node_j: 3,
                    area: 0.01,
                    youngs_modulus: 70.0e9,
                },
                Truss3dElementInput {
                    id: "e2".to_string(),
                    node_i: 2,
                    node_j: 3,
                    area: 0.01,
                    youngs_modulus: 70.0e9,
                },
                Truss3dElementInput {
                    id: "e3".to_string(),
                    node_i: 0,
                    node_j: 1,
                    area: 0.01,
                    youngs_modulus: 70.0e9,
                },
                Truss3dElementInput {
                    id: "e4".to_string(),
                    node_i: 1,
                    node_j: 2,
                    area: 0.01,
                    youngs_modulus: 70.0e9,
                },
                Truss3dElementInput {
                    id: "e5".to_string(),
                    node_i: 2,
                    node_j: 0,
                    area: 0.01,
                    youngs_modulus: 70.0e9,
                },
            ],
        })
        .expect("params"),
    };

    let response =
        handle_request_bytes(&serde_json::to_vec(&request).expect("request should serialize"));

    let AgentReply::Stream(progress_frames, final_response) = response;

    assert_eq!(progress_frames.len(), 4);
    assert!(final_response.ok);
    let result: kyuubiki_protocol::SolveTruss3dResult =
        serde_json::from_value(final_response.result.expect("solver result"))
            .expect("3d truss result");
    assert_eq!(result.nodes.len(), 4);
    assert_eq!(result.elements.len(), 6);
}

#[test]
fn handles_ping_rpc_requests() {
    let request = RpcRequest {
        rpc_version: RPC_VERSION,
        id: "rpc-ping".to_string(),
        method: RpcMethod::Ping,
        params: serde_json::json!({}),
    };

    let AgentReply::Stream(progress_frames, final_response) =
        handle_request_bytes(&serde_json::to_vec(&request).expect("request should serialize"));

    assert!(progress_frames.is_empty());
    assert!(final_response.ok);
    assert_eq!(
        final_response.result.expect("ping result"),
        serde_json::json!({ "pong": true })
    );
}

#[test]
fn handles_describe_agent_rpc_requests() {
    let request = RpcRequest {
        rpc_version: RPC_VERSION,
        id: "rpc-describe".to_string(),
        method: RpcMethod::DescribeAgent,
        params: serde_json::json!({}),
    };

    let AgentReply::Stream(progress_frames, final_response) =
        handle_request_bytes(&serde_json::to_vec(&request).expect("request should serialize"));

    assert!(progress_frames.is_empty());
    assert!(final_response.ok);

    let descriptor: AgentDescriptor =
        serde_json::from_value(final_response.result.expect("descriptor result"))
            .expect("agent descriptor");

    assert_eq!(descriptor.program, "kyuubiki-rust-agent");
    assert_eq!(descriptor.protocol.rpc_version, RPC_VERSION);
    assert!(
        descriptor
            .protocol
            .methods
            .contains(&RpcMethod::SolveTruss3d)
    );
    assert_eq!(descriptor.runtime.runtime_mode, "standalone");
    assert_eq!(descriptor.authority.control_mode, "standalone");
    assert_eq!(descriptor.authority.authority_mode, "self_directed");
}

#[test]
fn builds_peer_mesh_runtime_descriptor() {
    let descriptor = build_agent_descriptor(&AgentConfig {
        host: "127.0.0.1".to_string(),
        port: 5001,
        agent_id: Some("solver-a".to_string()),
        advertise_host: Some("10.0.0.20".to_string()),
        orchestrator_url: None,
        cluster_api_token: None,
        agent_fingerprint: None,
        certificate_id: None,
        cert_path: None,
        key_path: None,
        ca_cert_path: None,
        register_interval_ms: 5_000,
        cluster_id: Some("lan-a".to_string()),
        peers: vec!["10.0.0.11:5001".to_string(), "10.0.0.12:5001".to_string()],
    });

    assert_eq!(descriptor.runtime.runtime_mode, "peer_mesh");
    assert_eq!(descriptor.runtime.cluster_id.as_deref(), Some("lan-a"));
    assert!(descriptor.runtime.headless);
    assert_eq!(descriptor.runtime.cluster_size, 3);
    assert_eq!(descriptor.runtime.health_score, 100);
    assert_eq!(descriptor.runtime.peers.len(), 2);
    assert_eq!(descriptor.runtime.peers[0].status, "seed");
    assert_eq!(descriptor.authority.control_mode, "offline_mesh");
    assert_eq!(descriptor.authority.authority_mode, "offline_mesh");
}

#[test]
fn computes_cluster_health_score_from_peer_states() {
    let peers = vec![
        ClusterPeerDescriptor {
            address: "10.0.0.10:5001".to_string(),
            status: "healthy".to_string(),
            failure_count: 0,
            last_seen_unix_s: Some(1),
        },
        ClusterPeerDescriptor {
            address: "10.0.0.11:5001".to_string(),
            status: "degraded".to_string(),
            failure_count: 2,
            last_seen_unix_s: Some(1),
        },
        ClusterPeerDescriptor {
            address: "10.0.0.12:5001".to_string(),
            status: "unreachable".to_string(),
            failure_count: 4,
            last_seen_unix_s: None,
        },
    ];

    assert_eq!(compute_cluster_health_score(&peers), 50);
}

#[test]
fn builds_peer_descriptors_from_failures_and_last_seen() {
    let peers = vec!["10.0.0.10:5001".to_string(), "10.0.0.11:5001".to_string()];
    let failures = HashMap::from([
        ("10.0.0.10:5001".to_string(), 0_u32),
        ("10.0.0.11:5001".to_string(), 2_u32),
    ]);
    let last_seen = HashMap::from([("10.0.0.10:5001".to_string(), 123_u64)]);

    let descriptors = build_peer_descriptors(&peers, &failures, &last_seen);

    assert_eq!(descriptors[0].status, "healthy");
    assert_eq!(descriptors[1].status, "unreachable");
    assert_eq!(descriptors[1].failure_count, 2);
}
