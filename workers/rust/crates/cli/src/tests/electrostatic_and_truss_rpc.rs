use super::*;

#[test]
fn handles_electrostatic_plane_triangle_2d_rpc_requests() {
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
        .expect("params"),
    };

    let response =
        handle_request_bytes(&serde_json::to_vec(&request).expect("request should serialize"));

    let AgentReply::Stream(progress_frames, final_response) = response;

    assert_eq!(progress_frames.len(), 4);
    assert!(final_response.ok);
    let result: kyuubiki_protocol::SolveElectrostaticPlaneTriangle2dResult =
        serde_json::from_value(final_response.result.expect("solver result"))
            .expect("electrostatic plane triangle result");
    assert_eq!(result.max_potential, 10.0);
    assert!((result.max_electric_field - 10.0).abs() < 1.0e-6);
    assert!((result.max_flux_density - 20.0).abs() < 1.0e-6);
}

#[test]
fn handles_electrostatic_plane_quad_2d_rpc_requests() {
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
        .expect("params"),
    };

    let response =
        handle_request_bytes(&serde_json::to_vec(&request).expect("request should serialize"));

    let AgentReply::Stream(progress_frames, final_response) = response;

    assert_eq!(progress_frames.len(), 4);
    assert!(final_response.ok);
    let result: kyuubiki_protocol::SolveElectrostaticPlaneQuad2dResult =
        serde_json::from_value(final_response.result.expect("solver result"))
            .expect("electrostatic plane quad result");
    assert_eq!(result.max_potential, 10.0);
    assert!((result.max_electric_field - 10.0).abs() < 1.0e-6);
    assert!((result.max_flux_density - 20.0).abs() < 1.0e-6);
}

#[test]
fn handles_thermal_truss_2d_rpc_requests() {
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
                    temperature_delta: 40.0,
                },
                ThermalTruss2dNodeInput {
                    id: "n1".to_string(),
                    x: 1.0,
                    y: 0.0,
                    fix_x: true,
                    fix_y: true,
                    load_x: 0.0,
                    load_y: 0.0,
                    temperature_delta: 40.0,
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

    let response =
        handle_request_bytes(&serde_json::to_vec(&request).expect("request should serialize"));

    let AgentReply::Stream(progress_frames, final_response) = response;

    assert_eq!(progress_frames.len(), 4);
    assert!(final_response.ok);
    let result: kyuubiki_protocol::SolveThermalTruss2dResult =
        serde_json::from_value(final_response.result.expect("solver result"))
            .expect("thermal truss 2d result");
    assert_eq!(result.nodes.len(), 2);
    assert_eq!(result.elements.len(), 1);
    assert_eq!(result.max_temperature_delta, 40.0);
}

#[test]
fn handles_thermal_truss_3d_rpc_requests() {
    let request = RpcRequest {
        rpc_version: RPC_VERSION,
        id: "rpc-thermal-truss-3d".to_string(),
        method: RpcMethod::SolveThermalTruss3d,
        params: serde_json::to_value(SolveThermalTruss3dRequest {
            nodes: vec![
                ThermalTruss3dNodeInput {
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
                    temperature_delta: 40.0,
                },
                ThermalTruss3dNodeInput {
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
                    temperature_delta: 40.0,
                },
                ThermalTruss3dNodeInput {
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
                    temperature_delta: 40.0,
                },
            ],
            elements: vec![
                ThermalTruss3dElementInput {
                    id: "tt0".to_string(),
                    node_i: 0,
                    node_j: 1,
                    area: 0.01,
                    youngs_modulus: 210.0e9,
                    thermal_expansion: 12.0e-6,
                },
                ThermalTruss3dElementInput {
                    id: "tt1".to_string(),
                    node_i: 1,
                    node_j: 2,
                    area: 0.01,
                    youngs_modulus: 210.0e9,
                    thermal_expansion: 12.0e-6,
                },
                ThermalTruss3dElementInput {
                    id: "tt2".to_string(),
                    node_i: 2,
                    node_j: 0,
                    area: 0.01,
                    youngs_modulus: 210.0e9,
                    thermal_expansion: 12.0e-6,
                },
            ],
        })
        .expect("request params should serialize"),
    };

    let response =
        handle_request_bytes(&serde_json::to_vec(&request).expect("request should serialize"));

    let AgentReply::Stream(progress_frames, final_response) = response;

    assert_eq!(progress_frames.len(), 4);
    assert!(final_response.ok);
    let result: kyuubiki_protocol::SolveThermalTruss3dResult =
        serde_json::from_value(final_response.result.expect("solver result"))
            .expect("thermal truss 3d result");
    assert_eq!(result.nodes.len(), 3);
    assert_eq!(result.elements.len(), 3);
    assert_eq!(result.max_temperature_delta, 40.0);
}
