use kyuubiki_headless_sdk::build_composite_panel_steps;
use kyuubiki_protocol::{RPC_VERSION, RpcMethod, RpcRequest};

use crate::{AgentReply, handle_request_bytes};

#[test]
fn agent_executes_composite_thermo_electric_panel_rpc() {
    let step = build_composite_panel_steps()
        .into_iter()
        .next()
        .expect("composite study should include a candidate");
    let request = RpcRequest {
        rpc_version: RPC_VERSION,
        id: "composite-agent-rpc".to_string(),
        method: RpcMethod::SolveCompositeThermoElectricPanel,
        params: step.payload,
    };
    let encoded = serde_json::to_vec(&request).expect("request should serialize");

    let AgentReply::Stream(progress, response) = handle_request_bytes(&encoded);

    assert!(response.ok, "agent solve failed: {:?}", response.error);
    assert!(!progress.is_empty());
    let result = response.result.expect("result should be present");
    assert_eq!(
        result["schema_version"],
        "kyuubiki.composite-thermo-electric-panel-result/v1"
    );
    assert!(result["electrostatic"]["max_electric_field"].is_number());
    assert!(result["heat"]["max_temperature"].is_number());
    assert!(result["thermal"]["max_displacement"].is_number());
}
