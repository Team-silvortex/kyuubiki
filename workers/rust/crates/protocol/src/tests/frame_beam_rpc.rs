use super::prelude::*;

#[test]
fn serializes_frame_2d_rpc_round_trip() {
    let request = RpcRequest {
        rpc_version: RPC_VERSION,
        id: "rpc-frame-2d".to_string(),
        method: RpcMethod::SolveFrame2d,
        params: serde_json::to_value(SolveFrame2dRequest {
            nodes: vec![
                Frame2dNodeInput {
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
                Frame2dNodeInput {
                    id: "n1".to_string(),
                    x: 1.0,
                    y: 0.0,
                    fix_x: false,
                    fix_y: false,
                    fix_rz: false,
                    load_x: 0.0,
                    load_y: -1000.0,
                    moment_z: 0.0,
                },
            ],
            elements: vec![Frame2dElementInput {
                id: "f0".to_string(),
                node_i: 0,
                node_j: 1,
                area: 0.01,
                youngs_modulus: 210.0e9,
                moment_of_inertia: 8.0e-6,
                section_modulus: 1.6e-4,
            }],
        })
        .expect("request params should serialize"),
    };

    let json = serde_json::to_string(&request).expect("request should serialize");
    let decoded: RpcRequest = serde_json::from_str(&json).expect("request should decode");

    assert_eq!(decoded.method, RpcMethod::SolveFrame2d);
    assert_eq!(decoded.id, "rpc-frame-2d");
}

#[test]
fn serializes_frame_3d_rpc_round_trip() {
    let request = RpcRequest {
        rpc_version: RPC_VERSION,
        id: "rpc-frame-3d".to_string(),
        method: RpcMethod::SolveFrame3d,
        params: serde_json::to_value(SolveFrame3dRequest {
            nodes: vec![
                Frame3dNodeInput {
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
                Frame3dNodeInput {
                    id: "n1".to_string(),
                    x: 1.0,
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
            elements: vec![Frame3dElementInput {
                id: "f0".to_string(),
                node_i: 0,
                node_j: 1,
                area: 0.01,
                youngs_modulus: 210.0e9,
                shear_modulus: 80.0e9,
                torsion_constant: 2.0e-6,
                moment_of_inertia_y: 8.0e-6,
                moment_of_inertia_z: 6.0e-6,
                section_modulus_y: 1.6e-4,
                section_modulus_z: 1.3e-4,
            }],
        })
        .expect("request params should serialize"),
    };

    let json = serde_json::to_string(&request).expect("request should serialize");
    let decoded: RpcRequest = serde_json::from_str(&json).expect("request should decode");

    assert_eq!(decoded.method, RpcMethod::SolveFrame3d);
    assert_eq!(decoded.id, "rpc-frame-3d");
}

#[test]
fn serializes_thermal_frame_2d_rpc_round_trip() {
    let request = RpcRequest {
        rpc_version: RPC_VERSION,
        id: "rpc-thermal-frame-2d".to_string(),
        method: RpcMethod::SolveThermalFrame2d,
        params: serde_json::to_value(SolveThermalFrame2dRequest {
            nodes: vec![
                ThermalFrame2dNodeInput {
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
                ThermalFrame2dNodeInput {
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
            elements: vec![ThermalFrame2dElementInput {
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
        .expect("request params should serialize"),
    };

    let json = serde_json::to_string(&request).expect("request should serialize");
    let decoded: RpcRequest = serde_json::from_str(&json).expect("request should decode");

    assert_eq!(decoded.method, RpcMethod::SolveThermalFrame2d);
    assert_eq!(decoded.id, "rpc-thermal-frame-2d");
}

#[test]
fn serializes_thermal_frame_3d_rpc_round_trip() {
    let request = RpcRequest {
        rpc_version: RPC_VERSION,
        id: "rpc-thermal-frame-3d".to_string(),
        method: RpcMethod::SolveThermalFrame3d,
        params: serde_json::to_value(SolveThermalFrame3dRequest {
            nodes: vec![
                ThermalFrame3dNodeInput {
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
                ThermalFrame3dNodeInput {
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
            elements: vec![ThermalFrame3dElementInput {
                id: "tf0".to_string(),
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
                thermal_expansion: 12.0e-6,
                section_depth_y: 0.2,
                section_depth_z: 0.2,
                temperature_gradient_y: 30.0,
                temperature_gradient_z: 25.0,
            }],
        })
        .expect("request params should serialize"),
    };

    let json = serde_json::to_string(&request).expect("request should serialize");
    let decoded: RpcRequest = serde_json::from_str(&json).expect("request should decode");

    assert_eq!(decoded.method, RpcMethod::SolveThermalFrame3d);
    assert_eq!(decoded.id, "rpc-thermal-frame-3d");
}

#[test]
fn serializes_beam_1d_rpc_round_trip() {
    let request = RpcRequest {
        rpc_version: RPC_VERSION,
        id: "rpc-beam-1d".to_string(),
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
        .expect("request params should serialize"),
    };

    let json = serde_json::to_string(&request).expect("request should serialize");
    let decoded: RpcRequest = serde_json::from_str(&json).expect("request should decode");

    assert_eq!(decoded.method, RpcMethod::SolveBeam1d);
    assert_eq!(decoded.id, "rpc-beam-1d");
}

#[test]
fn serializes_thermal_beam_1d_rpc_round_trip() {
    let request = RpcRequest {
        rpc_version: RPC_VERSION,
        id: "rpc-thermal-beam-1d".to_string(),
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
        .expect("request params should serialize"),
    };

    let json = serde_json::to_string(&request).expect("request should serialize");
    let decoded: RpcRequest = serde_json::from_str(&json).expect("request should decode");

    assert_eq!(decoded.method, RpcMethod::SolveThermalBeam1d);
    assert_eq!(decoded.id, "rpc-thermal-beam-1d");
}
