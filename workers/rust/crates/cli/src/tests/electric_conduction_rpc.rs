use super::*;
use kyuubiki_protocol::{
    ElectricConductionPlaneNodeInput, ElectricConductionPlaneQuadElementInput,
    SolveElectricConductionPlaneQuad2dRequest, SolveElectricConductionPlaneQuad2dResult,
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
