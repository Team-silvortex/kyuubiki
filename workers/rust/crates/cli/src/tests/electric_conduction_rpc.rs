use super::*;
use kyuubiki_protocol::{
    ElectricConductionPlaneNodeInput, ElectricConductionPlaneQuadElementInput,
    ElectricConductionTerminalInput, SolveElectricConductionPlaneQuad2dRequest,
    SolveElectricConductionPlaneQuad2dResult,
};

#[test]
fn handles_electric_conduction_plane_quad_2d_rpc_requests() {
    let conductivity = 1.0 / 1.68e-8;
    let voltage = 3.36e-5;
    let request = RpcRequest {
        rpc_version: RPC_VERSION,
        id: "rpc-electric-conduction-plane-quad".to_string(),
        method: RpcMethod::SolveElectricConductionPlaneQuad2d,
        params: serde_json::to_value(SolveElectricConductionPlaneQuad2dRequest {
            nodes: vec![
                node("n0", 0.0, 0.0, 0.0),
                node("n1", 0.03, 0.0, voltage),
                node("n2", 0.03, 0.03, voltage),
                node("n3", 0.0, 0.03, 0.0),
            ],
            elements: vec![ElectricConductionPlaneQuadElementInput {
                id: "conductor".to_string(),
                node_i: 0,
                node_j: 1,
                node_k: 2,
                node_l: 3,
                thickness: 0.001,
                electrical_conductivity_s_m: conductivity,
            }],
            contact_interfaces: vec![],
            terminals: vec![],
        })
        .expect("params"),
    };

    let AgentReply::Stream(progress, response) =
        handle_request_bytes(&serde_json::to_vec(&request).expect("request"));
    let result: SolveElectricConductionPlaneQuad2dResult =
        serde_json::from_value(response.result.expect("result")).expect("conduction result");

    assert_eq!(progress.len(), 4);
    assert!(response.ok);
    assert!((result.max_current_density_a_m2 - 2.0 / 3.0e-5).abs() < 1.0e-8);
    assert!((result.total_joule_power_w - 6.72e-5).abs() < 1.0e-15);
    assert!((result.total_injected_current_a - 2.0).abs() < 1.0e-12);
    assert!(result.current_balance_relative_error < 1.0e-12);
    assert!(result.free_current_residual_relative_error < 1.0e-12);
    assert!(result.power_balance_relative_error < 1.0e-12);
}

#[test]
fn carries_impedance_terminals_and_source_power_through_rpc() {
    let request = RpcRequest {
        rpc_version: RPC_VERSION,
        id: "rpc-electric-conduction-terminals".to_string(),
        method: RpcMethod::SolveElectricConductionPlaneQuad2d,
        params: serde_json::to_value(SolveElectricConductionPlaneQuad2dRequest {
            nodes: vec![
                free_node("n0", 0.0, 0.0),
                free_node("n1", 1.0, 0.0),
                free_node("n2", 1.0, 1.0),
                free_node("n3", 0.0, 1.0),
            ],
            elements: vec![ElectricConductionPlaneQuadElementInput {
                id: "bulk".to_string(),
                node_i: 0,
                node_j: 1,
                node_k: 2,
                node_l: 3,
                thickness: 1.0,
                electrical_conductivity_s_m: 1.0,
            }],
            contact_interfaces: vec![],
            terminals: vec![
                terminal("left-bottom", 0, 0.0),
                terminal("right-bottom", 1, 3.0),
                terminal("right-top", 2, 3.0),
                terminal("left-top", 3, 0.0),
            ],
        })
        .expect("terminal params"),
    };

    let AgentReply::Stream(_, response) =
        handle_request_bytes(&serde_json::to_vec(&request).expect("request"));
    let result: SolveElectricConductionPlaneQuad2dResult =
        serde_json::from_value(response.result.expect("result")).expect("terminal result");

    assert_eq!(result.terminals.len(), 4);
    assert!((result.total_source_power_w - 3.0).abs() < 1.0e-12);
    assert!((result.total_terminal_impedance_power_w - 2.0).abs() < 1.0e-12);
    assert!(result.source_power_balance_relative_error < 1.0e-12);
}

fn node(id: &str, x: f64, y: f64, electric_potential_v: f64) -> ElectricConductionPlaneNodeInput {
    ElectricConductionPlaneNodeInput {
        id: id.to_string(),
        x,
        y,
        fix_electric_potential: true,
        electric_potential_v,
        current_source_a: 0.0,
    }
}

fn free_node(id: &str, x: f64, y: f64) -> ElectricConductionPlaneNodeInput {
    ElectricConductionPlaneNodeInput {
        id: id.to_string(),
        x,
        y,
        fix_electric_potential: false,
        electric_potential_v: 0.0,
        current_source_a: 0.0,
    }
}

fn terminal(id: &str, node: usize, external_potential_v: f64) -> ElectricConductionTerminalInput {
    ElectricConductionTerminalInput {
        id: id.to_string(),
        node,
        external_potential_v,
        impedance_ohm: 2.0,
    }
}
