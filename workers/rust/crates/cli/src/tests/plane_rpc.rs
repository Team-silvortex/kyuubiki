use super::*;

#[test]
fn handles_plane_triangle_rpc_requests() {
    let request = RpcRequest {
        rpc_version: RPC_VERSION,
        id: "rpc-plane".to_string(),
        method: RpcMethod::SolvePlaneTriangle2d,
        params: serde_json::to_value(SolvePlaneTriangle2dRequest {
            nodes: vec![
                PlaneNodeInput {
                    id: "n0".to_string(),
                    x: 0.0,
                    y: 0.0,
                    fix_x: true,
                    fix_y: true,
                    load_x: 0.0,
                    load_y: 0.0,
                },
                PlaneNodeInput {
                    id: "n1".to_string(),
                    x: 1.0,
                    y: 0.0,
                    fix_x: false,
                    fix_y: true,
                    load_x: 0.0,
                    load_y: 0.0,
                },
                PlaneNodeInput {
                    id: "n2".to_string(),
                    x: 1.0,
                    y: 1.0,
                    fix_x: false,
                    fix_y: false,
                    load_x: 0.0,
                    load_y: -1000.0,
                },
            ],
            elements: vec![PlaneTriangleElementInput {
                id: "p0".to_string(),
                node_i: 0,
                node_j: 1,
                node_k: 2,
                thickness: 0.02,
                youngs_modulus: 70.0e9,
                poisson_ratio: 0.33,
            }],
        })
        .expect("params"),
    };

    let response =
        handle_request_bytes(&serde_json::to_vec(&request).expect("request should serialize"));

    let AgentReply::Stream(progress_frames, final_response) = response;

    assert_eq!(progress_frames.len(), 4);
    assert!(final_response.ok);
    let result: kyuubiki_protocol::SolvePlaneTriangle2dResult =
        serde_json::from_value(final_response.result.expect("solver result"))
            .expect("plane result");
    assert_eq!(result.nodes.len(), 3);
    assert_eq!(result.elements.len(), 1);
}

#[test]
fn handles_plane_quad_rpc_requests() {
    let request = RpcRequest {
        rpc_version: RPC_VERSION,
        id: "rpc-plane-quad".to_string(),
        method: RpcMethod::SolvePlaneQuad2d,
        params: serde_json::to_value(SolvePlaneQuad2dRequest {
            nodes: vec![
                PlaneNodeInput {
                    id: "n0".to_string(),
                    x: 0.0,
                    y: 0.0,
                    fix_x: true,
                    fix_y: true,
                    load_x: 0.0,
                    load_y: 0.0,
                },
                PlaneNodeInput {
                    id: "n1".to_string(),
                    x: 1.0,
                    y: 0.0,
                    fix_x: false,
                    fix_y: true,
                    load_x: 0.0,
                    load_y: 0.0,
                },
                PlaneNodeInput {
                    id: "n2".to_string(),
                    x: 1.0,
                    y: 1.0,
                    fix_x: false,
                    fix_y: false,
                    load_x: 0.0,
                    load_y: -1000.0,
                },
                PlaneNodeInput {
                    id: "n3".to_string(),
                    x: 0.0,
                    y: 1.0,
                    fix_x: true,
                    fix_y: false,
                    load_x: 0.0,
                    load_y: 0.0,
                },
            ],
            elements: vec![PlaneQuadElementInput {
                id: "q0".to_string(),
                node_i: 0,
                node_j: 1,
                node_k: 2,
                node_l: 3,
                thickness: 0.02,
                youngs_modulus: 70.0e9,
                poisson_ratio: 0.33,
            }],
        })
        .expect("params"),
    };

    let response =
        handle_request_bytes(&serde_json::to_vec(&request).expect("request should serialize"));

    let AgentReply::Stream(progress_frames, final_response) = response;

    assert_eq!(progress_frames.len(), 4);
    assert!(final_response.ok);
    let result: kyuubiki_protocol::SolvePlaneQuad2dResult =
        serde_json::from_value(final_response.result.expect("solver result"))
            .expect("plane quad result");
    assert_eq!(result.nodes.len(), 4);
    assert_eq!(result.elements.len(), 1);
}

#[test]
fn handles_thermal_plane_triangle_2d_rpc_requests() {
    let request = RpcRequest {
        rpc_version: RPC_VERSION,
        id: "rpc-thermal-plane-triangle".to_string(),
        method: RpcMethod::SolveThermalPlaneTriangle2d,
        params: serde_json::to_value(SolveThermalPlaneTriangle2dRequest {
            nodes: vec![
                ThermalPlaneNodeInput {
                    id: "n0".to_string(),
                    x: 0.0,
                    y: 0.0,
                    fix_x: true,
                    fix_y: true,
                    load_x: 0.0,
                    load_y: 0.0,
                    temperature_delta: 40.0,
                },
                ThermalPlaneNodeInput {
                    id: "n1".to_string(),
                    x: 1.0,
                    y: 0.0,
                    fix_x: true,
                    fix_y: true,
                    load_x: 0.0,
                    load_y: 0.0,
                    temperature_delta: 40.0,
                },
                ThermalPlaneNodeInput {
                    id: "n2".to_string(),
                    x: 0.0,
                    y: 1.0,
                    fix_x: true,
                    fix_y: true,
                    load_x: 0.0,
                    load_y: 0.0,
                    temperature_delta: 40.0,
                },
            ],
            elements: vec![ThermalPlaneTriangleElementInput {
                id: "tp0".to_string(),
                node_i: 0,
                node_j: 1,
                node_k: 2,
                thickness: 0.02,
                youngs_modulus: 70.0e9,
                poisson_ratio: 0.33,
                thermal_expansion: 12.0e-6,
            }],
        })
        .expect("params"),
    };

    let response =
        handle_request_bytes(&serde_json::to_vec(&request).expect("request should serialize"));

    let AgentReply::Stream(progress_frames, final_response) = response;

    assert_eq!(progress_frames.len(), 4);
    assert!(final_response.ok);
    let result: kyuubiki_protocol::SolveThermalPlaneTriangle2dResult =
        serde_json::from_value(final_response.result.expect("solver result"))
            .expect("thermal plane triangle result");
    assert_eq!(result.nodes.len(), 3);
    assert_eq!(result.elements.len(), 1);
    assert_eq!(result.max_temperature_delta, 40.0);
}

#[test]
fn handles_thermal_plane_quad_2d_rpc_requests() {
    let request = RpcRequest {
        rpc_version: RPC_VERSION,
        id: "rpc-thermal-plane-quad".to_string(),
        method: RpcMethod::SolveThermalPlaneQuad2d,
        params: serde_json::to_value(SolveThermalPlaneQuad2dRequest {
            nodes: vec![
                ThermalPlaneNodeInput {
                    id: "n0".to_string(),
                    x: 0.0,
                    y: 0.0,
                    fix_x: true,
                    fix_y: true,
                    load_x: 0.0,
                    load_y: 0.0,
                    temperature_delta: 30.0,
                },
                ThermalPlaneNodeInput {
                    id: "n1".to_string(),
                    x: 1.0,
                    y: 0.0,
                    fix_x: true,
                    fix_y: true,
                    load_x: 0.0,
                    load_y: 0.0,
                    temperature_delta: 30.0,
                },
                ThermalPlaneNodeInput {
                    id: "n2".to_string(),
                    x: 1.0,
                    y: 1.0,
                    fix_x: true,
                    fix_y: true,
                    load_x: 0.0,
                    load_y: 0.0,
                    temperature_delta: 30.0,
                },
                ThermalPlaneNodeInput {
                    id: "n3".to_string(),
                    x: 0.0,
                    y: 1.0,
                    fix_x: true,
                    fix_y: true,
                    load_x: 0.0,
                    load_y: 0.0,
                    temperature_delta: 30.0,
                },
            ],
            elements: vec![ThermalPlaneQuadElementInput {
                id: "tq0".to_string(),
                node_i: 0,
                node_j: 1,
                node_k: 2,
                node_l: 3,
                thickness: 0.02,
                youngs_modulus: 70.0e9,
                poisson_ratio: 0.33,
                thermal_expansion: 11.0e-6,
            }],
        })
        .expect("params"),
    };

    let response =
        handle_request_bytes(&serde_json::to_vec(&request).expect("request should serialize"));

    let AgentReply::Stream(progress_frames, final_response) = response;

    assert_eq!(progress_frames.len(), 4);
    assert!(final_response.ok);
    let result: kyuubiki_protocol::SolveThermalPlaneQuad2dResult =
        serde_json::from_value(final_response.result.expect("solver result"))
            .expect("thermal plane quad result");
    assert_eq!(result.nodes.len(), 4);
    assert_eq!(result.elements.len(), 1);
    assert_eq!(result.max_temperature_delta, 30.0);
}
