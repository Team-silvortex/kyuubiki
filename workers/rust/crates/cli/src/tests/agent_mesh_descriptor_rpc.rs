use std::collections::HashMap;

use super::*;

#[test]
fn handles_ping_rpc_requests() {
    let request = RpcRequest {
        rpc_version: RPC_VERSION,
        id: "rpc-ping".to_string(),
        method: RpcMethod::Ping,
        params: serde_json::json!({}),
    };

    let AgentReply::Stream(progress_frames, final_response) =
        handle_request_bytes(&serde_json::to_vec(&request).expect("request should serialize"));

    assert!(progress_frames.is_empty());
    assert!(final_response.ok);
    assert_eq!(
        final_response.result.expect("ping result"),
        serde_json::json!({ "pong": true })
    );
}

#[test]
fn handles_describe_agent_rpc_requests() {
    agent_watchdog::reset_for_tests();

    let request = RpcRequest {
        rpc_version: RPC_VERSION,
        id: "rpc-describe".to_string(),
        method: RpcMethod::DescribeAgent,
        params: serde_json::json!({}),
    };

    let AgentReply::Stream(progress_frames, final_response) =
        handle_request_bytes(&serde_json::to_vec(&request).expect("request should serialize"));

    assert!(progress_frames.is_empty());
    assert!(final_response.ok);

    let descriptor_payload = final_response.result.expect("descriptor result");
    assert!(descriptor_payload["watchdog"]["state"].is_string());
    assert!(descriptor_payload["watchdog"]["active_execution_count"].is_number());
    assert_eq!(
        descriptor_payload["operator_package_runtime"]["ready"],
        false
    );
    assert_eq!(
        descriptor_payload["operator_package_runtime"]["status"],
        "not_attached"
    );

    let descriptor: AgentDescriptor =
        serde_json::from_value(descriptor_payload).expect("agent descriptor");

    assert_eq!(descriptor.program, "kyuubiki-rust-agent");
    assert_eq!(descriptor.protocol.rpc_version, RPC_VERSION);
    assert!(
        descriptor
            .protocol
            .methods
            .contains(&RpcMethod::SolveTruss3d)
    );
    assert_eq!(descriptor.runtime.runtime_mode, "standalone");
    assert_eq!(descriptor.authority.control_mode, "standalone");
    assert_eq!(descriptor.authority.authority_mode, "self_directed");
    assert_eq!(descriptor.engine.lifecycle, "agent_embedded");
    assert_eq!(descriptor.engine.task_source, "manual_or_sdk");
    assert_eq!(descriptor.engine.operator_source, "bound_orchestra_fetch");
}

#[test]
fn builds_peer_mesh_runtime_descriptor() {
    let descriptor = build_agent_descriptor(&AgentConfig {
        host: "127.0.0.1".to_string(),
        port: 5001,
        agent_id: Some("solver-a".to_string()),
        advertise_host: Some("10.0.0.20".to_string()),
        orchestrator_url: None,
        cluster_api_token: None,
        agent_fingerprint: None,
        certificate_id: None,
        cert_path: None,
        key_path: None,
        ca_cert_path: None,
        register_interval_ms: 5_000,
        cluster_id: Some("lan-a".to_string()),
        peers: vec!["10.0.0.11:5001".to_string(), "10.0.0.12:5001".to_string()],
        operator_package_host_id: None,
        operator_packages_root: None,
        operator_activated_package_count: 0,
    });

    assert_eq!(descriptor.runtime.runtime_mode, "peer_mesh");
    assert_eq!(descriptor.runtime.cluster_id.as_deref(), Some("lan-a"));
    assert!(descriptor.runtime.headless);
    assert_eq!(descriptor.runtime.cluster_size, 3);
    assert_eq!(descriptor.runtime.health_score, 100);
    assert_eq!(descriptor.runtime.peers.len(), 2);
    assert_eq!(descriptor.runtime.peers[0].status, "seed");
    assert_eq!(descriptor.authority.control_mode, "offline_mesh");
    assert_eq!(descriptor.authority.authority_mode, "offline_mesh");
    assert_eq!(descriptor.engine.lifecycle, "agent_embedded");
    assert_eq!(descriptor.engine.task_source, "manual_or_mesh_dispatch");
    assert_eq!(
        descriptor.engine.operator_cache_policy,
        "temporary_execution_cache"
    );
}

#[test]
fn builds_orchestrated_runtime_engine_descriptor() {
    let descriptor = build_agent_descriptor(&AgentConfig {
        host: "127.0.0.1".to_string(),
        port: 5001,
        agent_id: Some("solver-orch-a".to_string()),
        advertise_host: None,
        orchestrator_url: Some("https://orchestra.example.com".to_string()),
        cluster_api_token: None,
        agent_fingerprint: None,
        certificate_id: None,
        cert_path: None,
        key_path: None,
        ca_cert_path: None,
        register_interval_ms: 5_000,
        cluster_id: Some("cluster-a".to_string()),
        peers: vec![],
        operator_package_host_id: None,
        operator_packages_root: None,
        operator_activated_package_count: 0,
    });

    assert_eq!(descriptor.runtime.runtime_mode, "orchestrated");
    assert_eq!(descriptor.authority.control_mode, "orch_managed");
    assert_eq!(
        descriptor.authority.orchestrator_id.as_deref(),
        Some("https://orchestra.example.com")
    );
    assert_eq!(descriptor.engine.lifecycle, "agent_embedded");
    assert_eq!(descriptor.engine.task_source, "bound_orchestra_dispatch");
    assert_eq!(descriptor.engine.operator_source, "bound_orchestra_fetch");
}

#[test]
fn computes_cluster_health_score_from_peer_states() {
    let peers = vec![
        ClusterPeerDescriptor {
            address: "10.0.0.10:5001".to_string(),
            status: "healthy".to_string(),
            failure_count: 0,
            last_seen_unix_s: Some(1),
        },
        ClusterPeerDescriptor {
            address: "10.0.0.11:5001".to_string(),
            status: "degraded".to_string(),
            failure_count: 2,
            last_seen_unix_s: Some(1),
        },
        ClusterPeerDescriptor {
            address: "10.0.0.12:5001".to_string(),
            status: "unreachable".to_string(),
            failure_count: 4,
            last_seen_unix_s: None,
        },
    ];

    assert_eq!(compute_cluster_health_score(&peers), 50);
}

#[test]
fn builds_peer_descriptors_from_failures_and_last_seen() {
    let peers = vec!["10.0.0.10:5001".to_string(), "10.0.0.11:5001".to_string()];
    let failures = HashMap::from([
        ("10.0.0.10:5001".to_string(), 0_u32),
        ("10.0.0.11:5001".to_string(), 2_u32),
    ]);
    let last_seen = HashMap::from([("10.0.0.10:5001".to_string(), 123_u64)]);

    let descriptors = build_peer_descriptors(&peers, &failures, &last_seen);

    assert_eq!(descriptors[0].status, "healthy");
    assert_eq!(descriptors[1].status, "unreachable");
    assert_eq!(descriptors[1].failure_count, 2);
}
