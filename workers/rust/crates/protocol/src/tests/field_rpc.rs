use super::prelude::*;

#[test]
fn serializes_thermal_bar_1d_rpc_round_trip() {
    let request = RpcRequest {
        rpc_version: RPC_VERSION,
        id: "rpc-thermal-bar".to_string(),
        method: RpcMethod::SolveThermalBar1d,
        params: serde_json::to_value(SolveThermalBar1dRequest {
            nodes: vec![
                ThermalBar1dNodeInput {
                    id: "n0".to_string(),
                    x: 0.0,
                    fix_x: true,
                    load_x: 0.0,
                    temperature_delta: 30.0,
                },
                ThermalBar1dNodeInput {
                    id: "n1".to_string(),
                    x: 1.0,
                    fix_x: false,
                    load_x: 0.0,
                    temperature_delta: 30.0,
                },
            ],
            elements: vec![ThermalBar1dElementInput {
                id: "tb0".to_string(),
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

    assert_eq!(decoded.method, RpcMethod::SolveThermalBar1d);
    assert_eq!(decoded.id, "rpc-thermal-bar");
}

#[test]
fn serializes_heat_bar_1d_rpc_round_trip() {
    let request = RpcRequest {
        rpc_version: RPC_VERSION,
        id: "rpc-heat-bar".to_string(),
        method: RpcMethod::SolveHeatBar1d,
        params: serde_json::to_value(SolveHeatBar1dRequest {
            nodes: vec![
                HeatBar1dNodeInput {
                    id: "n0".to_string(),
                    x: 0.0,
                    fix_temperature: true,
                    temperature: 100.0,
                    heat_load: 0.0,
                },
                HeatBar1dNodeInput {
                    id: "n1".to_string(),
                    x: 1.0,
                    fix_temperature: true,
                    temperature: 0.0,
                    heat_load: 0.0,
                },
            ],
            elements: vec![HeatBar1dElementInput {
                id: "h0".to_string(),
                node_i: 0,
                node_j: 1,
                area: 0.01,
                conductivity: 45.0,
            }],
        })
        .expect("request params should serialize"),
    };

    let json = serde_json::to_string(&request).expect("request should serialize");
    let decoded: RpcRequest = serde_json::from_str(&json).expect("request should decode");

    assert_eq!(decoded.method, RpcMethod::SolveHeatBar1d);
    assert_eq!(decoded.id, "rpc-heat-bar");
}

#[test]
fn serializes_acoustic_bar_1d_rpc_round_trip() {
    let request = RpcRequest {
        rpc_version: RPC_VERSION,
        id: "rpc-acoustic-bar".to_string(),
        method: RpcMethod::SolveAcousticBar1d,
        params: serde_json::to_value(SolveAcousticBar1dRequest {
            frequency_hz: 100.0,
            nodes: vec![
                AcousticBar1dNodeInput {
                    id: "a0".to_string(),
                    x: 0.0,
                    fix_pressure: true,
                    pressure: 1.0,
                    volume_velocity_source: 0.0,
                },
                AcousticBar1dNodeInput {
                    id: "a1".to_string(),
                    x: 1.0,
                    fix_pressure: false,
                    pressure: 0.0,
                    volume_velocity_source: 0.01,
                },
            ],
            elements: vec![AcousticBar1dElementInput {
                id: "ae0".to_string(),
                node_i: 0,
                node_j: 1,
                area: 0.1,
                density: 1.2,
                bulk_modulus: 142_000.0,
                damping_ratio: 0.02,
            }],
        })
        .expect("request params should serialize"),
    };

    let json = serde_json::to_string(&request).expect("request should serialize");
    let decoded: RpcRequest = serde_json::from_str(&json).expect("request should decode");

    assert_eq!(decoded.method, RpcMethod::SolveAcousticBar1d);
    assert_eq!(decoded.id, "rpc-acoustic-bar");
}

#[test]
fn serializes_electrostatic_bar_1d_rpc_round_trip() {
    let request = RpcRequest {
        rpc_version: RPC_VERSION,
        id: "rpc-electrostatic-bar".to_string(),
        method: RpcMethod::SolveElectrostaticBar1d,
        params: serde_json::to_value(SolveElectrostaticBar1dRequest {
            nodes: vec![
                ElectrostaticBar1dNodeInput {
                    id: "n0".to_string(),
                    x: 0.0,
                    fix_potential: true,
                    potential: 10.0,
                    charge_density: 0.0,
                },
                ElectrostaticBar1dNodeInput {
                    id: "n1".to_string(),
                    x: 1.0,
                    fix_potential: true,
                    potential: 0.0,
                    charge_density: 0.0,
                },
            ],
            elements: vec![ElectrostaticBar1dElementInput {
                id: "eb0".to_string(),
                node_i: 0,
                node_j: 1,
                area: 0.02,
                permittivity: 8.854e-12,
            }],
        })
        .expect("request params should serialize"),
    };

    let json = serde_json::to_string(&request).expect("request should serialize");
    let decoded: RpcRequest = serde_json::from_str(&json).expect("request should decode");

    assert_eq!(decoded.method, RpcMethod::SolveElectrostaticBar1d);
    assert_eq!(decoded.id, "rpc-electrostatic-bar");
}

#[test]
fn serializes_magnetostatic_bar_1d_rpc_round_trip() {
    let request = RpcRequest {
        rpc_version: RPC_VERSION,
        id: "rpc-magnetostatic-bar".to_string(),
        method: RpcMethod::SolveMagnetostaticBar1d,
        params: serde_json::to_value(SolveMagnetostaticBar1dRequest {
            nodes: vec![
                MagnetostaticBar1dNodeInput {
                    id: "n0".to_string(),
                    x: 0.0,
                    fix_magnetic_potential: true,
                    magnetic_potential: 10.0,
                    magnetomotive_source: 0.0,
                },
                MagnetostaticBar1dNodeInput {
                    id: "n1".to_string(),
                    x: 1.0,
                    fix_magnetic_potential: true,
                    magnetic_potential: 0.0,
                    magnetomotive_source: 0.0,
                },
            ],
            elements: vec![MagnetostaticBar1dElementInput {
                id: "mb0".to_string(),
                node_i: 0,
                node_j: 1,
                area: 0.02,
                permeability: 1.25663706212e-6,
            }],
        })
        .expect("request params should serialize"),
    };

    let json = serde_json::to_string(&request).expect("request should serialize");
    let decoded: RpcRequest = serde_json::from_str(&json).expect("request should decode");

    assert_eq!(decoded.method, RpcMethod::SolveMagnetostaticBar1d);
    assert_eq!(decoded.id, "rpc-magnetostatic-bar");
}

#[test]
fn serializes_electrostatic_plane_triangle_2d_rpc_round_trip() {
    let request = RpcRequest {
        rpc_version: RPC_VERSION,
        id: "rpc-electrostatic-plane-triangle".to_string(),
        method: RpcMethod::SolveElectrostaticPlaneTriangle2d,
        params: serde_json::to_value(SolveElectrostaticPlaneTriangle2dRequest {
            nodes: vec![
                ElectrostaticPlaneNodeInput {
                    id: "n0".to_string(),
                    x: 0.0,
                    y: 0.0,
                    fix_potential: true,
                    potential: 10.0,
                    charge_density: 0.0,
                },
                ElectrostaticPlaneNodeInput {
                    id: "n1".to_string(),
                    x: 1.0,
                    y: 0.0,
                    fix_potential: true,
                    potential: 0.0,
                    charge_density: 0.0,
                },
                ElectrostaticPlaneNodeInput {
                    id: "n2".to_string(),
                    x: 0.0,
                    y: 1.0,
                    fix_potential: true,
                    potential: 10.0,
                    charge_density: 0.0,
                },
            ],
            elements: vec![ElectrostaticPlaneTriangleElementInput {
                id: "ep0".to_string(),
                node_i: 0,
                node_j: 1,
                node_k: 2,
                thickness: 0.05,
                permittivity: 2.0,
            }],
        })
        .expect("request params should serialize"),
    };

    let json = serde_json::to_string(&request).expect("request should serialize");
    let decoded: RpcRequest = serde_json::from_str(&json).expect("request should decode");

    assert_eq!(decoded.method, RpcMethod::SolveElectrostaticPlaneTriangle2d);
    assert_eq!(decoded.id, "rpc-electrostatic-plane-triangle");
}

#[test]
fn serializes_electrostatic_plane_quad_2d_rpc_round_trip() {
    let request = RpcRequest {
        rpc_version: RPC_VERSION,
        id: "rpc-electrostatic-plane-quad".to_string(),
        method: RpcMethod::SolveElectrostaticPlaneQuad2d,
        params: serde_json::to_value(SolveElectrostaticPlaneQuad2dRequest {
            nodes: vec![
                ElectrostaticPlaneNodeInput {
                    id: "n0".to_string(),
                    x: 0.0,
                    y: 0.0,
                    fix_potential: true,
                    potential: 10.0,
                    charge_density: 0.0,
                },
                ElectrostaticPlaneNodeInput {
                    id: "n1".to_string(),
                    x: 1.0,
                    y: 0.0,
                    fix_potential: true,
                    potential: 0.0,
                    charge_density: 0.0,
                },
                ElectrostaticPlaneNodeInput {
                    id: "n2".to_string(),
                    x: 1.0,
                    y: 1.0,
                    fix_potential: true,
                    potential: 0.0,
                    charge_density: 0.0,
                },
                ElectrostaticPlaneNodeInput {
                    id: "n3".to_string(),
                    x: 0.0,
                    y: 1.0,
                    fix_potential: true,
                    potential: 10.0,
                    charge_density: 0.0,
                },
            ],
            elements: vec![ElectrostaticPlaneQuadElementInput {
                id: "epq0".to_string(),
                node_i: 0,
                node_j: 1,
                node_k: 2,
                node_l: 3,
                thickness: 0.05,
                permittivity: 2.0,
            }],
        })
        .expect("request params should serialize"),
    };

    let json = serde_json::to_string(&request).expect("request should serialize");
    let decoded: RpcRequest = serde_json::from_str(&json).expect("request should decode");

    assert_eq!(decoded.method, RpcMethod::SolveElectrostaticPlaneQuad2d);
    assert_eq!(decoded.id, "rpc-electrostatic-plane-quad");
}

#[test]
fn serializes_heat_plane_triangle_2d_rpc_round_trip() {
    let request = RpcRequest {
        rpc_version: RPC_VERSION,
        id: "rpc-heat-plane-triangle".to_string(),
        method: RpcMethod::SolveHeatPlaneTriangle2d,
        params: serde_json::to_value(SolveHeatPlaneTriangle2dRequest {
            nodes: vec![
                HeatPlaneNodeInput {
                    id: "n0".to_string(),
                    x: 0.0,
                    y: 0.0,
                    fix_temperature: true,
                    temperature: 100.0,
                    heat_load: 0.0,
                },
                HeatPlaneNodeInput {
                    id: "n1".to_string(),
                    x: 1.0,
                    y: 0.0,
                    fix_temperature: true,
                    temperature: 20.0,
                    heat_load: 0.0,
                },
                HeatPlaneNodeInput {
                    id: "n2".to_string(),
                    x: 0.0,
                    y: 1.0,
                    fix_temperature: false,
                    temperature: 0.0,
                    heat_load: 0.0,
                },
            ],
            elements: vec![HeatPlaneTriangleElementInput {
                id: "hpt0".to_string(),
                node_i: 0,
                node_j: 1,
                node_k: 2,
                thickness: 0.02,
                conductivity: 45.0,
            }],
        })
        .expect("request params should serialize"),
    };

    let json = serde_json::to_string(&request).expect("request should serialize");
    let decoded: RpcRequest = serde_json::from_str(&json).expect("request should decode");

    assert_eq!(decoded.method, RpcMethod::SolveHeatPlaneTriangle2d);
    assert_eq!(decoded.id, "rpc-heat-plane-triangle");
}

#[test]
fn serializes_heat_plane_quad_2d_rpc_round_trip() {
    let request = RpcRequest {
        rpc_version: RPC_VERSION,
        id: "rpc-heat-plane-quad".to_string(),
        method: RpcMethod::SolveHeatPlaneQuad2d,
        params: serde_json::to_value(SolveHeatPlaneQuad2dRequest {
            nodes: vec![
                HeatPlaneNodeInput {
                    id: "n0".to_string(),
                    x: 0.0,
                    y: 0.0,
                    fix_temperature: true,
                    temperature: 100.0,
                    heat_load: 0.0,
                },
                HeatPlaneNodeInput {
                    id: "n1".to_string(),
                    x: 1.0,
                    y: 0.0,
                    fix_temperature: true,
                    temperature: 20.0,
                    heat_load: 0.0,
                },
                HeatPlaneNodeInput {
                    id: "n2".to_string(),
                    x: 1.0,
                    y: 1.0,
                    fix_temperature: false,
                    temperature: 0.0,
                    heat_load: 0.0,
                },
                HeatPlaneNodeInput {
                    id: "n3".to_string(),
                    x: 0.0,
                    y: 1.0,
                    fix_temperature: false,
                    temperature: 0.0,
                    heat_load: 0.0,
                },
            ],
            elements: vec![HeatPlaneQuadElementInput {
                id: "hpq0".to_string(),
                node_i: 0,
                node_j: 1,
                node_k: 2,
                node_l: 3,
                thickness: 0.02,
                conductivity: 45.0,
            }],
        })
        .expect("request params should serialize"),
    };

    let json = serde_json::to_string(&request).expect("request should serialize");
    let decoded: RpcRequest = serde_json::from_str(&json).expect("request should decode");

    assert_eq!(decoded.method, RpcMethod::SolveHeatPlaneQuad2d);
    assert_eq!(decoded.id, "rpc-heat-plane-quad");
}
