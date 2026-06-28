use std::fs;

use kyuubiki_installer::workspace_root;
use serde::Serialize;
use serde_json::{Value, json};

use crate::remote::{
    REMOTE_POLICY_SCHEMA_VERSION, validate_advertise_host, validate_agent_id,
    validate_orchestrator_url, validate_remote_workspace, validate_ssh_identity,
    validate_target_host,
};

#[derive(Clone, Debug, Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteNodeWorkflowSnapshot {
    pub workflow_kind: String,
    pub stage: String,
    pub status: String,
    pub summary: Option<String>,
    pub recorded_at_unix_ms: Option<u64>,
    pub details: Option<Value>,
}

#[derive(Clone, Debug, Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteNodeRecord {
    pub label: String,
    pub target_host: String,
    pub ssh_user: String,
    pub remote_workspace: String,
    pub ssh_port: Option<u16>,
    pub control_mode: Option<String>,
    pub orchestrator_url: String,
    pub agent_id: String,
    pub advertise_host: String,
    pub agent_port: u16,
    pub cluster_id: Option<String>,
    pub peer_endpoints: Option<Vec<String>>,
    pub certificate_id: Option<String>,
    pub last_probe_status: Option<String>,
    pub last_probe_summary: Option<String>,
    pub last_probe_unix_ms: Option<u64>,
    pub last_action: Option<String>,
    pub last_action_unix_ms: Option<u64>,
    pub workflow_snapshots: Option<Vec<RemoteNodeWorkflowSnapshot>>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteNodeRegistryPayload {
    pub config_path: String,
    pub nodes: Vec<RemoteNodeRecord>,
    pub rendered: String,
}

#[derive(Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WriteRemoteNodeRegistryPayload {
    pub nodes: Vec<RemoteNodeRecord>,
}

#[tauri::command]
pub fn remote_node_registry() -> Result<RemoteNodeRegistryPayload, String> {
    let nodes = read_remote_node_registry()?;
    Ok(RemoteNodeRegistryPayload {
        config_path: remote_nodes_path().display().to_string(),
        rendered: render_remote_nodes(&nodes),
        nodes,
    })
}

#[tauri::command]
pub fn write_remote_node_registry(
    payload: WriteRemoteNodeRegistryPayload,
) -> Result<String, String> {
    let nodes = payload
        .nodes
        .into_iter()
        .map(validate_remote_node_record)
        .collect::<Result<Vec<_>, _>>()?;
    let path = remote_nodes_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
    }
    fs::write(
        &path,
        serde_json::to_string_pretty(&json!({
            "schema_version": REMOTE_POLICY_SCHEMA_VERSION,
            "nodes": nodes,
        }))
        .map_err(|error| error.to_string())?,
    )
    .map_err(|error| format!("failed to write {}: {error}", path.display()))?;
    Ok(render_remote_nodes(&read_remote_node_registry()?))
}

pub(crate) fn validate_control_mode(value: Option<&str>) -> Result<String, String> {
    match value.unwrap_or("orchestrated").trim() {
        "" | "orchestrated" => Ok("orchestrated".to_string()),
        "offline_mesh" => Ok("offline_mesh".to_string()),
        _ => Err("control mode must be orchestrated or offline_mesh".to_string()),
    }
}

pub(crate) fn validate_cluster_id(value: Option<&str>) -> Result<Option<String>, String> {
    let Some(trimmed) = value.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(None);
    };
    if !trimmed
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '-' | '_'))
    {
        return Err("cluster id contains unsupported characters".to_string());
    }
    Ok(Some(trimmed.to_string()))
}

pub(crate) fn normalize_peer_endpoints(values: Vec<String>) -> Result<Vec<String>, String> {
    let mut peers = values
        .into_iter()
        .map(|value| validate_peer_endpoint(&value))
        .collect::<Result<Vec<_>, _>>()?;
    peers.sort();
    peers.dedup();
    Ok(peers)
}

fn validate_peer_endpoint(value: &str) -> Result<String, String> {
    let trimmed = value.trim();
    let Some((host, port)) = trimmed.rsplit_once(':') else {
        return Err("peer endpoint must be host:port".to_string());
    };
    crate::remote::validate_host_token(host, "peer endpoint host")?;
    let port = port
        .parse::<u16>()
        .map_err(|_| "peer endpoint port must be 1-65535".to_string())?;
    if port == 0 {
        return Err("peer endpoint port must be 1-65535".to_string());
    }
    Ok(format!("{}:{port}", host.trim()))
}

fn remote_nodes_path() -> std::path::PathBuf {
    workspace_root()
        .join("config")
        .join("installer-remote-nodes.json")
}

fn read_remote_node_registry() -> Result<Vec<RemoteNodeRecord>, String> {
    let path = remote_nodes_path();
    if !path.exists() {
        return Ok(Vec::new());
    }
    let contents = fs::read_to_string(&path)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    let value: Value = serde_json::from_str(&contents)
        .map_err(|error| format!("failed to parse {}: {error}", path.display()))?;
    value
        .get("nodes")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .cloned()
                .map(serde_json::from_value::<RemoteNodeRecord>)
                .collect::<Result<Vec<_>, _>>()
                .map_err(|error| error.to_string())
        })
        .unwrap_or_else(|| Ok(Vec::new()))?
        .into_iter()
        .map(validate_remote_node_record)
        .collect()
}

fn validate_remote_node_record(node: RemoteNodeRecord) -> Result<RemoteNodeRecord, String> {
    let target_host = validate_target_host(&node.target_host)?;
    let control_mode = validate_control_mode(node.control_mode.as_deref())?;
    let cluster_id = validate_cluster_id(node.cluster_id.as_deref())?;
    let peer_endpoints = normalize_peer_endpoints(node.peer_endpoints.unwrap_or_default())?;
    let workflow_snapshots = node
        .workflow_snapshots
        .unwrap_or_default()
        .into_iter()
        .map(validate_workflow_snapshot)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(RemoteNodeRecord {
        label: if node.label.trim().is_empty() {
            target_host.clone()
        } else {
            node.label.trim().to_string()
        },
        target_host,
        ssh_user: validate_ssh_identity(&node.ssh_user, "ssh user")?,
        remote_workspace: validate_remote_workspace(&node.remote_workspace)?,
        ssh_port: node.ssh_port,
        control_mode: Some(control_mode.clone()),
        orchestrator_url: if control_mode == "offline_mesh" {
            String::new()
        } else {
            validate_orchestrator_url(&node.orchestrator_url)?
        },
        agent_id: validate_agent_id(&node.agent_id)?,
        advertise_host: validate_advertise_host(&node.advertise_host)?,
        agent_port: node.agent_port,
        cluster_id,
        peer_endpoints: if peer_endpoints.is_empty() {
            None
        } else {
            Some(peer_endpoints)
        },
        certificate_id: validate_certificate_id(node.certificate_id.as_deref())?,
        last_probe_status: node
            .last_probe_status
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty()),
        last_probe_summary: node
            .last_probe_summary
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty()),
        last_probe_unix_ms: node.last_probe_unix_ms,
        last_action: node
            .last_action
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty()),
        last_action_unix_ms: node.last_action_unix_ms,
        workflow_snapshots: if workflow_snapshots.is_empty() {
            None
        } else {
            Some(
                workflow_snapshots
                    .into_iter()
                    .rev()
                    .take(16)
                    .collect::<Vec<_>>()
                    .into_iter()
                    .rev()
                    .collect(),
            )
        },
    })
}

fn render_remote_nodes(nodes: &[RemoteNodeRecord]) -> String {
    let mut lines = vec![
        "installer remote nodes".to_string(),
        format!("config_path: {}", remote_nodes_path().display()),
    ];
    if nodes.is_empty() {
        lines.push("nodes: (none)".to_string());
        return lines.join("\n");
    }
    for node in nodes {
        lines.push(format!(
            "[node] {} mode={} ssh={}@{}:{} workspace={} agent={} advertise={} route={}",
            node.label,
            node.control_mode.as_deref().unwrap_or("orchestrated"),
            node.ssh_user,
            node.target_host,
            node.ssh_port.unwrap_or(22),
            node.remote_workspace,
            node.agent_id,
            node.advertise_host,
            if node.control_mode.as_deref() == Some("offline_mesh") {
                node.cluster_id.as_deref().unwrap_or("(mesh)")
            } else {
                node.orchestrator_url.as_str()
            }
        ));
        if let Some(certificate_id) = &node.certificate_id {
            lines.push(format!("  certificate_id: {}", certificate_id));
        }
        if let Some(peers) = &node.peer_endpoints {
            if !peers.is_empty() {
                lines.push(format!("  peers: {}", peers.join(",")));
            }
        }
        if let Some(status) = &node.last_probe_status {
            lines.push(format!(
                "  probe: {} @ {}",
                status,
                node.last_probe_unix_ms.unwrap_or(0)
            ));
        }
        if let Some(action) = &node.last_action {
            lines.push(format!(
                "  action: {} @ {}",
                action,
                node.last_action_unix_ms.unwrap_or(0)
            ));
        }
        if let Some(snapshot) = node
            .workflow_snapshots
            .as_ref()
            .and_then(|snapshots| snapshots.last())
        {
            lines.push(format!(
                "  workflow: {} stage={} status={} @ {}",
                snapshot.workflow_kind,
                snapshot.stage,
                snapshot.status,
                snapshot.recorded_at_unix_ms.unwrap_or(0)
            ));
        }
    }
    lines.join("\n")
}

fn validate_snapshot_token(value: &str, label: &str) -> Result<String, String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(format!("{label} must not be empty"));
    }
    if !trimmed
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '-' | '_'))
    {
        return Err(format!("{label} contains unsupported characters"));
    }
    Ok(trimmed.to_string())
}

fn validate_workflow_snapshot(
    snapshot: RemoteNodeWorkflowSnapshot,
) -> Result<RemoteNodeWorkflowSnapshot, String> {
    Ok(RemoteNodeWorkflowSnapshot {
        workflow_kind: validate_snapshot_token(&snapshot.workflow_kind, "workflow snapshot kind")?,
        stage: validate_snapshot_token(&snapshot.stage, "workflow snapshot stage")?,
        status: validate_snapshot_token(&snapshot.status, "workflow snapshot status")?,
        summary: snapshot
            .summary
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty()),
        recorded_at_unix_ms: snapshot.recorded_at_unix_ms,
        details: snapshot.details,
    })
}

fn validate_certificate_id(value: Option<&str>) -> Result<Option<String>, String> {
    let Some(trimmed) = value.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(None);
    };
    if !trimmed
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '-' | '_'))
    {
        return Err("certificate id contains unsupported characters".to_string());
    }
    Ok(Some(trimmed.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_offline_mesh_node_without_orchestrator() {
        let _guard = crate::remote::TEST_ENV_LOCK.lock().unwrap();
        unsafe {
            std::env::remove_var(crate::remote::REMOTE_ALLOWED_HOSTS_ENV);
            std::env::remove_var(crate::remote::REMOTE_ALLOWED_WORKSPACE_ROOTS_ENV);
        }
        let node = validate_remote_node_record(RemoteNodeRecord {
            label: "mesh-a".to_string(),
            target_host: "10.0.0.10".to_string(),
            ssh_user: "ubuntu".to_string(),
            remote_workspace: "/opt/kyuubiki".to_string(),
            ssh_port: Some(22),
            control_mode: Some("offline_mesh".to_string()),
            orchestrator_url: String::new(),
            agent_id: "mesh-a".to_string(),
            advertise_host: "10.0.0.10".to_string(),
            agent_port: 5001,
            cluster_id: Some("lan-a".to_string()),
            peer_endpoints: Some(vec!["10.0.0.11:5001".to_string()]),
            certificate_id: Some("mesh-a-cert".to_string()),
            last_probe_status: None,
            last_probe_summary: None,
            last_probe_unix_ms: None,
            last_action: None,
            last_action_unix_ms: None,
            workflow_snapshots: Some(vec![RemoteNodeWorkflowSnapshot {
                workflow_kind: "mesh_rollout_stage".to_string(),
                stage: "mesh_preflight".to_string(),
                status: "succeeded".to_string(),
                summary: Some("mesh preflight ok".to_string()),
                recorded_at_unix_ms: Some(1),
                details: None,
            }]),
        })
        .unwrap();
        assert_eq!(node.control_mode.as_deref(), Some("offline_mesh"));
        assert_eq!(node.cluster_id.as_deref(), Some("lan-a"));
        assert_eq!(
            node.workflow_snapshots
                .as_ref()
                .and_then(|items| items.last())
                .map(|item| item.stage.as_str()),
            Some("mesh_preflight")
        );
    }
}
