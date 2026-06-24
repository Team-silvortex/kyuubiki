use super::*;

#[test]
fn handles_beam_1d_rpc_requests() {
    let request = RpcRequest {
        rpc_version: RPC_VERSION,
        id: "rpc-beam".to_string(),
        method: RpcMethod::SolveBeam1d,
        params: serde_json::to_value(SolveBeam1dRequest {
            nodes: vec![
                Beam1dNodeInput {
                    id: "n0".to_string(),
                    x: 0.0,
                    fix_y: true,
                    fix_rz: true,
                    load_y: 0.0,
                    moment_z: 0.0,
                },
                Beam1dNodeInput {
                    id: "n1".to_string(),
                    x: 2.0,
                    fix_y: false,
                    fix_rz: false,
                    load_y: -1000.0,
                    moment_z: 0.0,
                },
            ],
            elements: vec![Beam1dElementInput {
                id: "b0".to_string(),
                node_i: 0,
                node_j: 1,
                youngs_modulus: 210.0e9,
                moment_of_inertia: 8.0e-6,
                section_modulus: 1.6e-4,
                distributed_load_y: 0.0,
            }],
        })
        .expect("params"),
    };

    let response =
        handle_request_bytes(&serde_json::to_vec(&request).expect("request should serialize"));

    let AgentReply::Stream(progress_frames, final_response) = response;

    assert_eq!(progress_frames.len(), 4);
    assert!(final_response.ok);
    let result: kyuubiki_protocol::SolveBeam1dResult =
        serde_json::from_value(final_response.result.expect("solver result")).expect("beam result");
    assert_eq!(result.nodes.len(), 2);
    assert_eq!(result.elements.len(), 1);
    assert!((result.max_moment - 2000.0).abs() < 1.0e-6);
}

#[test]
fn handles_thermal_beam_1d_rpc_requests() {
    let request = RpcRequest {
        rpc_version: RPC_VERSION,
        id: "rpc-thermal-beam".to_string(),
        method: RpcMethod::SolveThermalBeam1d,
        params: serde_json::to_value(SolveThermalBeam1dRequest {
            nodes: vec![
                ThermalBeam1dNodeInput {
                    id: "n0".to_string(),
                    x: 0.0,
                    fix_y: true,
                    fix_rz: true,
                    load_y: 0.0,
                    moment_z: 0.0,
                },
                ThermalBeam1dNodeInput {
                    id: "n1".to_string(),
                    x: 2.0,
                    fix_y: true,
                    fix_rz: true,
                    load_y: 0.0,
                    moment_z: 0.0,
                },
            ],
            elements: vec![ThermalBeam1dElementInput {
                id: "tb0".to_string(),
                node_i: 0,
                node_j: 1,
                youngs_modulus: 210.0e9,
                moment_of_inertia: 8.0e-6,
                section_modulus: 1.6e-4,
                thermal_expansion: 12.0e-6,
                section_depth: 0.2,
                distributed_load_y: 0.0,
                temperature_gradient_y: 40.0,
            }],
        })
        .expect("params"),
    };

    let response =
        handle_request_bytes(&serde_json::to_vec(&request).expect("request should serialize"));

    let AgentReply::Stream(progress_frames, final_response) = response;

    assert_eq!(progress_frames.len(), 4);
    assert!(final_response.ok);
    let result: kyuubiki_protocol::SolveThermalBeam1dResult =
        serde_json::from_value(final_response.result.expect("solver result"))
            .expect("thermal beam result");
    assert_eq!(result.nodes.len(), 2);
    assert_eq!(result.elements.len(), 1);
    assert!(result.max_moment > 0.0);
    assert_eq!(result.max_temperature_gradient, 40.0);
}

#[test]
fn handles_torsion_1d_rpc_requests() {
    let request = RpcRequest {
        rpc_version: RPC_VERSION,
        id: "rpc-torsion".to_string(),
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
                    x: 2.0,
                    fix_rz: false,
                    torque_z: 1200.0,
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
        .expect("params"),
    };

    let response =
        handle_request_bytes(&serde_json::to_vec(&request).expect("request should serialize"));

    let AgentReply::Stream(progress_frames, final_response) = response;

    assert_eq!(progress_frames.len(), 4);
    assert!(final_response.ok);
    let result: kyuubiki_protocol::SolveTorsion1dResult =
        serde_json::from_value(final_response.result.expect("solver result"))
            .expect("torsion result");
    assert!((result.max_torque - 1200.0).abs() < 1.0e-6);
}

#[test]
fn handles_spring_1d_rpc_requests() {
    let request = RpcRequest {
        rpc_version: RPC_VERSION,
        id: "rpc-spring".to_string(),
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
        .expect("params"),
    };

    let response =
        handle_request_bytes(&serde_json::to_vec(&request).expect("request should serialize"));

    let AgentReply::Stream(progress_frames, final_response) = response;

    assert_eq!(progress_frames.len(), 4);
    assert!(final_response.ok);
    let result: kyuubiki_protocol::SolveSpring1dResult =
        serde_json::from_value(final_response.result.expect("solver result"))
            .expect("spring result");
    assert_eq!(result.nodes.len(), 2);
    assert_eq!(result.elements.len(), 1);
    assert!((result.max_displacement - 0.04).abs() < 1.0e-12);
    assert!((result.max_force - 1000.0).abs() < 1.0e-9);
}

#[test]
fn handles_spring_2d_rpc_requests() {
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
                    y: 0.0,
                    fix_x: false,
                    fix_y: true,
                    load_x: 1000.0,
                    load_y: 0.0,
                },
            ],
            elements: vec![Spring2dElementInput {
                id: "s0".to_string(),
                node_i: 0,
                node_j: 1,
                stiffness: 25_000.0,
            }],
        })
        .expect("params"),
    };

    let response =
        handle_request_bytes(&serde_json::to_vec(&request).expect("request should serialize"));

    let AgentReply::Stream(progress_frames, final_response) = response;

    assert_eq!(progress_frames.len(), 4);
    assert!(final_response.ok);
    let result: kyuubiki_protocol::SolveSpring2dResult =
        serde_json::from_value(final_response.result.expect("solver result"))
            .expect("spring 2d result");
    assert_eq!(result.nodes.len(), 2);
    assert_eq!(result.elements.len(), 1);
    assert!((result.max_displacement - 0.04).abs() < 1.0e-12);
    assert!((result.max_force - 1000.0).abs() < 1.0e-9);
}

#[test]
fn handles_spring_3d_rpc_requests() {
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
                    y: 0.0,
                    z: 0.0,
                    fix_x: false,
                    fix_y: true,
                    fix_z: true,
                    load_x: 1000.0,
                    load_y: 0.0,
                    load_z: 0.0,
                },
            ],
            elements: vec![Spring3dElementInput {
                id: "s0".to_string(),
                node_i: 0,
                node_j: 1,
                stiffness: 25_000.0,
            }],
        })
        .expect("params"),
    };

    let response =
        handle_request_bytes(&serde_json::to_vec(&request).expect("request should serialize"));

    let AgentReply::Stream(progress_frames, final_response) = response;

    assert_eq!(progress_frames.len(), 4);
    assert!(final_response.ok);
    let result: kyuubiki_protocol::SolveSpring3dResult =
        serde_json::from_value(final_response.result.expect("solver result"))
            .expect("spring 3d result");
    assert_eq!(result.nodes.len(), 2);
    assert_eq!(result.elements.len(), 1);
    assert!((result.max_displacement - 0.04).abs() < 1.0e-12);
    assert!((result.max_force - 1000.0).abs() < 1.0e-9);
}
