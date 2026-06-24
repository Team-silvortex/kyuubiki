use super::prelude::*;

#[test]
fn serializes_thermal_truss_2d_rpc_round_trip() {
    let request = RpcRequest {
        rpc_version: RPC_VERSION,
        id: "rpc-thermal-truss-2d".to_string(),
        method: RpcMethod::SolveThermalTruss2d,
        params: serde_json::to_value(SolveThermalTruss2dRequest {
            nodes: vec![
                ThermalTruss2dNodeInput {
                    id: "n0".to_string(),
                    x: 0.0,
                    y: 0.0,
                    fix_x: true,
                    fix_y: true,
                    load_x: 0.0,
                    load_y: 0.0,
                    temperature_delta: 25.0,
                },
                ThermalTruss2dNodeInput {
                    id: "n1".to_string(),
                    x: 1.0,
                    y: 0.0,
                    fix_x: false,
                    fix_y: false,
                    load_x: 0.0,
                    load_y: 0.0,
                    temperature_delta: 25.0,
                },
            ],
            elements: vec![ThermalTruss2dElementInput {
                id: "tt0".to_string(),
                node_i: 0,
                node_j: 1,
                area: 0.01,
                youngs_modulus: 210.0e9,
                thermal_expansion: 12.0e-6,
            }],
        })
        .expect("request params should serialize"),
    };

    let json = serde_json::to_string(&request).expect("request should serialize");
    let decoded: RpcRequest = serde_json::from_str(&json).expect("request should decode");

    assert_eq!(decoded.method, RpcMethod::SolveThermalTruss2d);
    assert_eq!(decoded.id, "rpc-thermal-truss-2d");
}

#[test]
fn serializes_plane_triangle_rpc_round_trip() {
    let request = RpcRequest {
        rpc_version: RPC_VERSION,
        id: "rpc-plane".to_string(),
        method: RpcMethod::SolvePlaneTriangle2d,
        params: serde_json::to_value(SolvePlaneTriangle2dRequest {
            nodes: vec![],
            elements: vec![],
        })
        .expect("request params should serialize"),
    };

    let json = serde_json::to_string(&request).expect("request should serialize");
    let decoded: RpcRequest = serde_json::from_str(&json).expect("request should decode");

    assert_eq!(decoded.method, RpcMethod::SolvePlaneTriangle2d);
    assert_eq!(decoded.id, "rpc-plane");
}

#[test]
fn serializes_plane_quad_rpc_round_trip() {
    let request = RpcRequest {
        rpc_version: RPC_VERSION,
        id: "rpc-plane-quad".to_string(),
        method: RpcMethod::SolvePlaneQuad2d,
        params: serde_json::to_value(SolvePlaneQuad2dRequest {
            nodes: vec![],
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
        .expect("request params should serialize"),
    };

    let json = serde_json::to_string(&request).expect("request should serialize");
    let decoded: RpcRequest = serde_json::from_str(&json).expect("request should decode");

    assert_eq!(decoded.method, RpcMethod::SolvePlaneQuad2d);
    assert_eq!(decoded.id, "rpc-plane-quad");
}

#[test]
fn serializes_thermal_plane_triangle_2d_rpc_round_trip() {
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
                    fix_x: false,
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
                    fix_y: false,
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
        .expect("request params should serialize"),
    };

    let json = serde_json::to_string(&request).expect("request should serialize");
    let decoded: RpcRequest = serde_json::from_str(&json).expect("request should decode");

    assert_eq!(decoded.method, RpcMethod::SolveThermalPlaneTriangle2d);
    assert_eq!(decoded.id, "rpc-thermal-plane-triangle");
}

#[test]
fn serializes_thermal_plane_quad_2d_rpc_round_trip() {
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
                    temperature_delta: 25.0,
                },
                ThermalPlaneNodeInput {
                    id: "n1".to_string(),
                    x: 1.0,
                    y: 0.0,
                    fix_x: false,
                    fix_y: true,
                    load_x: 0.0,
                    load_y: 0.0,
                    temperature_delta: 30.0,
                },
                ThermalPlaneNodeInput {
                    id: "n2".to_string(),
                    x: 1.0,
                    y: 1.0,
                    fix_x: false,
                    fix_y: false,
                    load_x: 0.0,
                    load_y: 0.0,
                    temperature_delta: 35.0,
                },
                ThermalPlaneNodeInput {
                    id: "n3".to_string(),
                    x: 0.0,
                    y: 1.0,
                    fix_x: true,
                    fix_y: false,
                    load_x: 0.0,
                    load_y: 0.0,
                    temperature_delta: 40.0,
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
        .expect("request params should serialize"),
    };

    let json = serde_json::to_string(&request).expect("request should serialize");
    let decoded: RpcRequest = serde_json::from_str(&json).expect("request should decode");

    assert_eq!(decoded.method, RpcMethod::SolveThermalPlaneQuad2d);
    assert_eq!(decoded.id, "rpc-thermal-plane-quad");
}
