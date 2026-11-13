use super::*;
use crate::agent_network_access::NetworkAccess;
use kyuubiki_protocol::{AgentDescriptor, RPC_VERSION};
use serde_json::json;

fn request(method: RpcMethod) -> RpcRequest {
    RpcRequest {
        rpc_version: RPC_VERSION,
        id: "lan-policy-request".into(),
        method,
        params: json!({"execution_id":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "store_id":"public-store-label", "controller_id":"public-controller-label",
            "cluster_id":"public-cluster-label", "authenticated":true}),
    }
}

#[test]
fn default_lan_gate_covers_every_advertised_non_discovery_method_before_dispatch() {
    let policy = NetworkAccess::parse(Some("192.0.2.1")).unwrap();
    for method in AgentDescriptor::solver_agent_default().protocol.methods {
        if matches!(
            method,
            RpcMethod::Ping | RpcMethod::DescribeAgent | RpcMethod::DescribeAgentLifecycle
        ) {
            continue;
        }
        let expected = if crate::agent_lifecycle_rpc::is_mutation(&method) {
            "agent_lifecycle_control_requires_loopback"
        } else if matches!(method, RpcMethod::ManageOperatorPackage) {
            "package_deployment_requires_tls"
        } else {
            "agent_lan_control_requires_explicit_trust"
        };
        let AgentReply::Stream(frames, response) =
            handle_request_from_peer_with_policy(request(method.clone()), None, false, &policy);
        assert!(frames.is_empty(), "{method:?}");
        assert!(!response.ok, "{method:?}");
        assert_eq!(response.id, "lan-policy-request");
        let error = response.error.unwrap();
        assert_eq!(error.code, expected, "{method:?}");
        if expected == "agent_lan_control_requires_explicit_trust" {
            let details = error.details.unwrap();
            assert_eq!(details["dispatched"], false);
            assert_eq!(details["automatic_replay_authorized"], false);
            assert!(!details.to_string().contains("public-store-label"));
        }
    }
}

#[test]
fn default_lan_gate_keeps_discovery_available() {
    let policy = NetworkAccess::default();
    for method in [
        RpcMethod::Ping,
        RpcMethod::DescribeAgent,
        RpcMethod::DescribeAgentLifecycle,
    ] {
        let AgentReply::Stream(_, response) =
            handle_request_from_peer_with_policy(request(method), None, false, &policy);
        assert!(response.ok, "{response:?}");
    }
}

#[test]
fn loopback_and_explicit_trust_reach_parameter_validation_not_lan_denial() {
    for (local, trusted) in [(true, None), (false, Some("true"))] {
        let policy = NetworkAccess::default()
            .with_trusted_lan_control(trusted)
            .unwrap();
        let AgentReply::Stream(_, response) = handle_request_from_peer_with_policy(
            request(RpcMethod::CancelExecution),
            None,
            local,
            &policy,
        );
        assert_eq!(response.error.unwrap().code, "invalid_params");
    }
}

#[test]
fn explicit_trust_does_not_grant_remote_lifecycle_mutations() {
    let policy = NetworkAccess::default()
        .with_trusted_lan_control(Some("true"))
        .unwrap();
    for method in [RpcMethod::BeginAgentDrain, RpcMethod::ResumeAgentAdmission] {
        let AgentReply::Stream(_, response) =
            handle_request_from_peer_with_policy(request(method), None, false, &policy);
        assert_eq!(
            response.error.unwrap().code,
            "agent_lifecycle_control_requires_loopback"
        );
    }
}

#[test]
fn trusted_lan_cannot_authorize_unencrypted_package_delivery() {
    let policy = NetworkAccess::default()
        .with_trusted_lan_control(Some("true"))
        .unwrap();
    let AgentReply::Stream(frames, response) = handle_request_from_peer_with_policy(
        request(RpcMethod::ManageOperatorPackage),
        None,
        false,
        &policy,
    );
    assert!(frames.is_empty());
    assert_eq!(
        response.error.unwrap().code,
        "package_deployment_requires_tls"
    );
}

#[test]
fn invalid_envelopes_keep_protocol_errors_before_lan_control_denial() {
    let policy = NetworkAccess::default();
    let mut invalid = request(RpcMethod::SubmitDurableAgentTask);
    invalid.id.clear();
    let AgentReply::Stream(_, response) =
        handle_request_from_peer_with_policy(invalid, None, false, &policy);
    assert_eq!(response.id, "unknown");
    assert_eq!(response.error.unwrap().code, "invalid_request_id");
    let mut invalid = request(RpcMethod::FetchDurableAgentExecution);
    invalid.rpc_version = 255;
    let AgentReply::Stream(_, response) =
        handle_request_from_peer_with_policy(invalid, None, false, &policy);
    assert_eq!(response.error.unwrap().code, "invalid_version");
}

#[test]
fn denied_lan_cancellation_cannot_touch_current_or_future_job_executions() {
    let policy = NetworkAccess::default();
    let job = "lan-denied-owned-job";
    let id = "lan-denied-owned-request";
    let active = crate::agent_execution_control::begin(id.into(), 52, Some(job.into())).unwrap();
    for method in [
        RpcMethod::CancelJob,
        RpcMethod::CancelExecution,
        RpcMethod::ReleaseOperatorPackageJob,
    ] {
        let mut request = request(method);
        request.params = json!({"job_id":job, "request_id":id, "generation":52,
            "process_instance_id":crate::agent_lifecycle::snapshot().process_instance_id});
        let AgentReply::Stream(frames, response) =
            handle_request_from_peer_with_policy(request, None, false, &policy);
        assert!(frames.is_empty());
        assert_eq!(
            response.error.unwrap().code,
            "agent_lan_control_requires_explicit_trust"
        );
        assert!(!active.solver.cancellation_requested());
        assert!(!crate::agent_execution_control::take_cancelled(job));
    }
    drop(active);
    let next = crate::agent_execution_control::begin(id.into(), 53, Some(job.into())).unwrap();
    assert!(!next.solver.cancellation_requested());
}

#[test]
fn connection_access_uses_socket_scope_not_request_claims_or_global_descriptors() {
    for trusted in [None, Some("true")] {
        let policy = NetworkAccess::default()
            .with_trusted_lan_control(trusted)
            .unwrap();
        for local in [false, true, false] {
            let mut request = request(RpcMethod::DescribeAgent);
            request.params["peer_is_loopback"] = json!(!local);
            request.params["connection_access"] =
                json!({"control_allowed":true, "authenticated":true});
            let AgentReply::Stream(_, response) =
                handle_request_from_peer_with_policy(request, None, local, &policy);
            assert!(response.ok);
            let result = response.result.unwrap();
            let access: kyuubiki_protocol::AgentConnectionAccess =
                serde_json::from_value(result["connection_access"].clone()).unwrap();
            assert_eq!(access.validate(), Ok(()));
            assert_eq!(access.control_allowed, local || trusted == Some("true"));
            assert_eq!(access.lifecycle_mutations_allowed, local);
            assert!(!access.authenticated && !access.encrypted);
            assert!(
                crate::agent_state::agent_descriptor_payload()
                    .get("connection_access")
                    .is_none()
            );
        }
    }
}
