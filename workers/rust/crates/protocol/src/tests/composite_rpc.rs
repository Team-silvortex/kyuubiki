use crate::{
    AgentDescriptor, RPC_VERSION, RpcMethod, RpcRequest, SolveCompositeThermoElectricPanelRequest,
};
use serde_json::json;

#[test]
fn serializes_composite_thermo_electric_panel_rpc_round_trip() {
    let request = RpcRequest {
        rpc_version: RPC_VERSION,
        id: "composite-rpc".to_string(),
        method: RpcMethod::SolveCompositeThermoElectricPanel,
        params: serde_json::to_value(sample_request()).expect("params should serialize"),
    };
    let encoded = serde_json::to_string(&request).expect("request should serialize");
    let decoded: RpcRequest = serde_json::from_str(&encoded).expect("request should decode");

    assert_eq!(decoded.method, RpcMethod::SolveCompositeThermoElectricPanel);
    assert_eq!(decoded.params["research"]["candidate_id"], "panel-a");
}

#[test]
fn descriptor_advertises_composite_runtime_capability() {
    let descriptor = AgentDescriptor::solver_agent_default();
    let capability = descriptor
        .capabilities
        .iter()
        .find(|capability| capability.id == "composite-thermo-electric-panel")
        .expect("composite capability should be advertised");

    assert_eq!(
        capability.methods,
        vec![RpcMethod::SolveCompositeThermoElectricPanel]
    );
    assert!(capability.tags.iter().any(|tag| tag == "multiphysics"));
    assert!(
        descriptor
            .protocol
            .methods
            .contains(&RpcMethod::SolveCompositeThermoElectricPanel)
    );
}

#[test]
fn estimates_nodes_across_all_coupled_models() {
    assert_eq!(sample_request().estimated_node_count(), 8);
}

fn sample_request() -> SolveCompositeThermoElectricPanelRequest {
    let model = json!({"nodes": [{"id": "n0"}, {"id": "n1"}]});
    SolveCompositeThermoElectricPanelRequest {
        research: json!({"candidate_id": "panel-a"}),
        electrostatic_model: model.clone(),
        electric_conduction_model: model.clone(),
        heat_model: model.clone(),
        thermal_model: model,
        electrothermal_loss: json!({}),
        electrothermal_feedback: json!({}),
        electric_conduction_feedback: json!({}),
        thermal_expansion_feedback: json!({}),
    }
}
