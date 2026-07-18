use std::collections::HashSet;
use std::sync::{Mutex, OnceLock};

use kyuubiki_protocol::{
    AgentClusterDescriptor, AgentDescriptor, ClusterPeerDescriptor, JobStatus, ProgressEvent,
    RpcProgress, RuntimeAuthorityDescriptor, RuntimeEngineDescriptor,
};

use crate::agent_deployment::{
    AgentDeploymentReadiness, build_agent_deployment_readiness, default_agent_deployment_readiness,
};
use crate::agent_headless_bridge::agent_headless_bridge_manifest;
use crate::agent_watchdog;
use crate::config::AgentConfig;
use crate::operator_task_runtime::{
    operator_package_runtime_snapshot, operator_package_runtime_snapshot_for_config,
    operator_task_execution_reliability_snapshot,
};

pub(crate) fn agent_descriptor() -> AgentDescriptor {
    runtime_descriptor()
        .lock()
        .map(|descriptor| descriptor.clone())
        .unwrap_or_else(|_| AgentDescriptor::solver_agent_default())
}

pub(crate) fn runtime_descriptor() -> &'static Mutex<AgentDescriptor> {
    static DESCRIPTOR: OnceLock<Mutex<AgentDescriptor>> = OnceLock::new();
    DESCRIPTOR.get_or_init(|| Mutex::new(AgentDescriptor::solver_agent_default()))
}

pub(crate) fn store_runtime_descriptor(descriptor: AgentDescriptor) {
    if let Ok(mut current) = runtime_descriptor().lock() {
        *current = descriptor;
    }
}

pub(crate) fn agent_deployment_readiness() -> AgentDeploymentReadiness {
    deployment_readiness()
        .lock()
        .map(|readiness| readiness.clone())
        .unwrap_or_else(|_| default_agent_deployment_readiness())
}

pub(crate) fn deployment_readiness() -> &'static Mutex<AgentDeploymentReadiness> {
    static READINESS: OnceLock<Mutex<AgentDeploymentReadiness>> = OnceLock::new();
    READINESS.get_or_init(|| Mutex::new(default_agent_deployment_readiness()))
}

pub(crate) fn store_deployment_readiness(readiness: AgentDeploymentReadiness) {
    if let Ok(mut current) = deployment_readiness().lock() {
        *current = readiness;
    }
}

pub(crate) fn build_agent_descriptor(config: &AgentConfig) -> AgentDescriptor {
    let mut descriptor = AgentDescriptor::solver_agent_default();
    let runtime_mode = agent_runtime_mode(config);
    descriptor.runtime = AgentClusterDescriptor {
        cluster_id: config.cluster_id.clone(),
        runtime_mode: runtime_mode.to_string(),
        headless: true,
        cluster_size: 1 + config.peers.len(),
        health_score: 100,
        peers: config
            .peers
            .iter()
            .cloned()
            .map(|address| ClusterPeerDescriptor {
                address,
                status: "seed".to_string(),
                failure_count: 0,
                last_seen_unix_s: None,
            })
            .collect(),
    };
    descriptor.authority = build_runtime_authority_descriptor(config, runtime_mode);
    descriptor.engine = build_runtime_engine_descriptor(runtime_mode);
    descriptor
}

pub(crate) fn build_agent_deployment_readiness_for_config(
    config: &AgentConfig,
) -> AgentDeploymentReadiness {
    let descriptor = build_agent_descriptor(config);
    build_agent_deployment_readiness(config, &descriptor)
}

fn build_runtime_authority_descriptor(
    config: &AgentConfig,
    runtime_mode: &str,
) -> RuntimeAuthorityDescriptor {
    match runtime_mode {
        "orchestrated" => RuntimeAuthorityDescriptor {
            control_mode: "orch_managed".to_string(),
            authority_mode: "single_orchestrator".to_string(),
            orchestrator_id: config.orchestrator_url.clone(),
            orchestrator_session_id: None,
            accepts_multi_orchestrator_binding: false,
            agent_library_replication: "central_fetch".to_string(),
        },
        "peer_mesh" => RuntimeAuthorityDescriptor {
            control_mode: "offline_mesh".to_string(),
            authority_mode: "offline_mesh".to_string(),
            orchestrator_id: None,
            orchestrator_session_id: None,
            accepts_multi_orchestrator_binding: false,
            agent_library_replication: "central_fetch".to_string(),
        },
        _ => RuntimeAuthorityDescriptor {
            control_mode: "standalone".to_string(),
            authority_mode: "self_directed".to_string(),
            orchestrator_id: None,
            orchestrator_session_id: None,
            accepts_multi_orchestrator_binding: false,
            agent_library_replication: "central_fetch".to_string(),
        },
    }
}

fn build_runtime_engine_descriptor(runtime_mode: &str) -> RuntimeEngineDescriptor {
    let task_source = match runtime_mode {
        "orchestrated" => "bound_orchestra_dispatch",
        "peer_mesh" => "manual_or_mesh_dispatch",
        _ => "manual_or_sdk",
    };

    RuntimeEngineDescriptor {
        engine_id: "kyuubiki-engine/embedded".to_string(),
        engine_name: "kyuubiki-rust-engine".to_string(),
        lifecycle: "agent_embedded".to_string(),
        task_source: task_source.to_string(),
        operator_source: "bound_orchestra_fetch".to_string(),
        operator_cache_policy: "temporary_execution_cache".to_string(),
    }
}

pub(crate) fn registration_payload(config: &AgentConfig) -> serde_json::Value {
    let descriptor = agent_descriptor();

    serde_json::json!({
        "id": config.agent_id,
        "host": config.advertise_host.clone().unwrap_or_else(|| config.host.clone()),
        "port": config.port,
        "role": "solver",
        "cluster_id": config.cluster_id,
        "tags": registration_tags(config),
        "methods": descriptor.protocol.methods,
        "capabilities": descriptor.capabilities,
        "headless_bridge": agent_headless_bridge_manifest(),
        "operator_package_runtime": operator_package_runtime_snapshot_for_config(config),
        "deployment_readiness": build_agent_deployment_readiness_for_config(config),
        "health_score": descriptor.runtime.health_score,
        "watchdog": agent_watchdog::snapshot()
    })
}

pub(crate) fn agent_descriptor_payload() -> serde_json::Value {
    let mut payload =
        serde_json::to_value(agent_descriptor()).expect("agent descriptor should serialize");

    if let Some(object) = payload.as_object_mut() {
        object.insert(
            "watchdog".to_string(),
            serde_json::to_value(agent_watchdog::snapshot())
                .expect("agent watchdog snapshot should serialize"),
        );
        object.insert(
            "operator_package_runtime".to_string(),
            operator_package_runtime_snapshot(),
        );
        object.insert(
            "operator_task_reliability".to_string(),
            operator_task_execution_reliability_snapshot(),
        );
        object.insert(
            "headless_bridge".to_string(),
            agent_headless_bridge_manifest(),
        );
        object.insert(
            "deployment_readiness".to_string(),
            serde_json::to_value(agent_deployment_readiness())
                .expect("agent deployment readiness should serialize"),
        );
    }

    payload
}

fn registration_tags(config: &AgentConfig) -> Vec<&'static str> {
    let mut tags = if config.peers.is_empty() {
        vec!["headless", "standalone"]
    } else {
        vec!["headless", "peer-mesh"]
    };
    if config.certificate_id.is_some() || config.cert_path.is_some() {
        tags.push("certificate-bound");
    }
    if config.agent_fingerprint.is_some() {
        tags.push("fingerprint-bound");
    }
    tags
}

fn agent_runtime_mode(config: &AgentConfig) -> &'static str {
    if !config.peers.is_empty() {
        "peer_mesh"
    } else if config.orchestrator_url.is_some() {
        "orchestrated"
    } else {
        "standalone"
    }
}

pub(crate) fn build_progress_frames(
    model_name: &str,
    request_id: &str,
    node_count: usize,
) -> Vec<RpcProgress> {
    let steps = [
        (
            JobStatus::Preprocessing,
            0.1_f32,
            Some("normalizing study inputs".to_string()),
        ),
        (
            JobStatus::Partitioning,
            0.25_f32,
            Some(format!("partitioning {model_name} topology")),
        ),
        (
            JobStatus::Solving,
            0.7_f32,
            Some(format!("solving structural system with {node_count} nodes")),
        ),
        (
            JobStatus::Postprocessing,
            0.92_f32,
            Some("collecting nodal and elemental responses".to_string()),
        ),
    ];

    steps
        .into_iter()
        .enumerate()
        .map(|(index, (stage, progress, message))| {
            let mut event = ProgressEvent::new("solver-session", stage, progress);
            event.iteration = Some((index + 1) as u64);
            event.residual = Some(1.0 / ((index + 2) as f64));
            event.peak_memory = Some(512 + (index as u64) * 128);
            event.message = message;

            RpcProgress::new(request_id.to_string(), event)
        })
        .collect()
}

fn cancellation_registry() -> &'static Mutex<HashSet<String>> {
    static REGISTRY: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();
    REGISTRY.get_or_init(|| Mutex::new(HashSet::new()))
}

pub(crate) fn register_cancel(job_id: String) {
    if let Ok(mut registry) = cancellation_registry().lock() {
        registry.insert(job_id);
    }
}

pub(crate) fn take_cancelled(job_id: &str) -> bool {
    if let Ok(mut registry) = cancellation_registry().lock() {
        return registry.remove(job_id);
    }

    false
}

pub(crate) fn extract_job_id(params: &serde_json::Value) -> Option<String> {
    params
        .as_object()
        .and_then(|value| value.get("job_id"))
        .and_then(|value| value.as_str())
        .map(|value| value.to_string())
}
