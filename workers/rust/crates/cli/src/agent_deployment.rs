use kyuubiki_protocol::AgentDescriptor;
use serde::{Deserialize, Serialize};

use crate::config::AgentConfig;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct AgentDeploymentReadiness {
    pub(crate) schema_version: String,
    pub(crate) ready: bool,
    pub(crate) status: String,
    pub(crate) agent_id: Option<String>,
    pub(crate) runtime_mode: String,
    pub(crate) deployment_modes: Vec<String>,
    pub(crate) update_strategy: String,
    pub(crate) operator_cache_policy: String,
    pub(crate) operator_package_runtime_ready: bool,
    pub(crate) orchestrator_bound: bool,
    pub(crate) peer_mesh_enabled: bool,
    pub(crate) cleanup_policy: String,
    pub(crate) issues: Vec<String>,
}

pub(crate) fn default_agent_deployment_readiness() -> AgentDeploymentReadiness {
    let descriptor = AgentDescriptor::solver_agent_default();
    let config = AgentConfig::from_args(&[]);
    build_agent_deployment_readiness(&config, &descriptor)
}

pub(crate) fn build_agent_deployment_readiness(
    config: &AgentConfig,
    descriptor: &AgentDescriptor,
) -> AgentDeploymentReadiness {
    let mut issues = Vec::new();
    if config.orchestrator_url.is_some() && config.cluster_api_token.is_none() {
        issues.push("orchestrated deployment is missing cluster_api_token".to_string());
    }
    if config.operator_package_host_id.is_some() && config.operator_packages_root.is_none() {
        issues.push("operator package host is declared without operator_packages_root".to_string());
    }
    if config.certificate_id.is_some() && (config.cert_path.is_none() || config.key_path.is_none())
    {
        issues.push("certificate_id is declared without both cert_path and key_path".to_string());
    }

    let operator_package_runtime_ready =
        config.operator_package_host_id.is_some() && config.operator_packages_root.is_some();
    let ready = issues.is_empty();

    AgentDeploymentReadiness {
        schema_version: "kyuubiki.agent-deployment-readiness/v1".to_string(),
        ready,
        status: if ready { "ready" } else { "needs_attention" }.to_string(),
        agent_id: config.agent_id.clone(),
        runtime_mode: descriptor.runtime.runtime_mode.clone(),
        deployment_modes: descriptor.deployment_modes.clone(),
        update_strategy: update_strategy(config),
        operator_cache_policy: descriptor.engine.operator_cache_policy.clone(),
        operator_package_runtime_ready,
        orchestrator_bound: config.orchestrator_url.is_some(),
        peer_mesh_enabled: !config.peers.is_empty(),
        cleanup_policy: "installer_owned_paths_only".to_string(),
        issues,
    }
}

fn update_strategy(config: &AgentConfig) -> String {
    if config.orchestrator_url.is_some() {
        "orchestra_assisted_pull".to_string()
    } else if !config.peers.is_empty() {
        "mesh_manual_pull".to_string()
    } else {
        "local_installer_pull".to_string()
    }
}
