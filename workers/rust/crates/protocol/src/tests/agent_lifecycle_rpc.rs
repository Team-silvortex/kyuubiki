use crate::{
    AGENT_LIFECYCLE_SCHEMA, AgentDescriptor, BeginAgentDrainRequest, RPC_VERSION,
    ResumeAgentAdmissionRequest, RpcMethod, RpcRequest,
};

#[test]
fn lifecycle_methods_and_payloads_round_trip() {
    let requests = [
        RpcRequest {
            rpc_version: RPC_VERSION,
            id: "drain".to_string(),
            method: RpcMethod::BeginAgentDrain,
            params: serde_json::to_value(BeginAgentDrainRequest {
                controller_id: "installer-transaction-1".to_string(),
                reason: "rolling replacement".to_string(),
            })
            .expect("drain request should serialize"),
        },
        RpcRequest {
            rpc_version: RPC_VERSION,
            id: "status".to_string(),
            method: RpcMethod::DescribeAgentLifecycle,
            params: serde_json::json!({}),
        },
        RpcRequest {
            rpc_version: RPC_VERSION,
            id: "resume".to_string(),
            method: RpcMethod::ResumeAgentAdmission,
            params: serde_json::to_value(ResumeAgentAdmissionRequest {
                controller_id: "installer-transaction-1".to_string(),
                drain_generation: 1,
            })
            .expect("resume request should serialize"),
        },
    ];

    for request in requests {
        let encoded = serde_json::to_vec(&request).expect("request should serialize");
        let decoded: RpcRequest =
            serde_json::from_slice(&encoded).expect("request should deserialize");
        assert_eq!(decoded, request);
    }
}

#[test]
fn agent_descriptor_advertises_lifecycle_contract() {
    let descriptor = AgentDescriptor::solver_agent_default();
    assert_eq!(descriptor.lifecycle.schema_version, AGENT_LIFECYCLE_SCHEMA);
    assert_eq!(descriptor.lifecycle.state, "accepting");
    assert!(descriptor.lifecycle.accepting_new_work);
    assert!(!descriptor.lifecycle.safe_to_replace);

    for method in [
        RpcMethod::BeginAgentDrain,
        RpcMethod::DescribeAgentLifecycle,
        RpcMethod::ResumeAgentAdmission,
    ] {
        assert!(descriptor.protocol.methods.contains(&method));
        assert!(
            descriptor
                .capabilities
                .iter()
                .find(|capability| capability.id == "control")
                .expect("control capability")
                .methods
                .contains(&method)
        );
    }
}
