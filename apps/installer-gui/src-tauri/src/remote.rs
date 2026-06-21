use std::env;
use std::fs;
use std::path::{Component, Path};
use std::process::Command;

use crate::remote_nodes::{normalize_peer_endpoints, validate_cluster_id, validate_control_mode};
use kyuubiki_installer::workspace_root;
use serde::Serialize;
use serde_json::{Value, json};

pub(crate) const REMOTE_ALLOWED_HOSTS_ENV: &str = "KYUUBIKI_INSTALLER_REMOTE_ALLOWED_HOSTS";
pub(crate) const REMOTE_ALLOWED_WORKSPACE_ROOTS_ENV: &str = "KYUUBIKI_INSTALLER_REMOTE_ALLOWED_WORKSPACE_ROOTS";
pub(crate) const REMOTE_POLICY_SCHEMA_VERSION: &str = "kyuubiki.installer.remote-policy/v1";
#[cfg(test)]
pub(crate) static TEST_ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
#[derive(Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteBootstrapPayload {
    pub target_host: String,
    pub ssh_user: String,
    pub remote_workspace: String,
    pub ssh_port: Option<u16>,
}
#[derive(Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteAgentPayload {
    pub target_host: String,
    pub ssh_user: String,
    pub remote_workspace: String,
    pub control_mode: Option<String>,
    pub orchestrator_url: String,
    pub agent_id: String,
    pub advertise_host: String,
    pub agent_port: u16,
    pub cluster_id: Option<String>,
    pub peer_endpoints: Option<Vec<String>>,
    pub ssh_port: Option<u16>,
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteDeployPolicyPayload {
    pub schema_version: String,
    pub config_path: String,
    pub allowed_hosts: String,
    pub allowed_workspace_roots: String,
    pub effective_allowed_hosts: String,
    pub effective_allowed_workspace_roots: String,
    pub env_allowed_hosts: String,
    pub env_allowed_workspace_roots: String,
    pub rendered: String,
}
#[derive(Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WriteRemoteDeployPolicyPayload {
    pub allowed_hosts: String,
    pub allowed_workspace_roots: String,
}
#[derive(Clone, Debug)]
struct RemoteDeployPolicyConfig {
    schema_version: String,
    allowed_hosts: Vec<String>,
    allowed_workspace_roots: Vec<String>,
}
#[tauri::command]
pub fn remote_deploy_policy() -> Result<RemoteDeployPolicyPayload, String> {
    build_remote_deploy_policy_payload()
}
#[tauri::command]
pub fn write_remote_deploy_policy(
    payload: WriteRemoteDeployPolicyPayload,
) -> Result<String, String> {
    let config = RemoteDeployPolicyConfig {
        schema_version: REMOTE_POLICY_SCHEMA_VERSION.to_string(),
        allowed_hosts: normalize_csv_entries(&payload.allowed_hosts),
        allowed_workspace_roots: normalize_csv_entries(&payload.allowed_workspace_roots),
    };
    let path = remote_policy_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
    }
    fs::write(
        &path,
        serde_json::to_string_pretty(&json!({
            "schema_version": config.schema_version,
            "allowed_hosts": config.allowed_hosts,
            "allowed_workspace_roots": config.allowed_workspace_roots,
        }))
        .map_err(|error| error.to_string())?,
    )
    .map_err(|error| format!("failed to write {}: {error}", path.display()))?;
    Ok(build_remote_deploy_policy_payload()?.rendered)
}
#[tauri::command]
pub fn probe_remote_node(payload: RemoteBootstrapPayload) -> Result<String, String> {
    let ssh_user = validate_ssh_identity(&payload.ssh_user, "ssh user")?;
    let target_host = validate_target_host(&payload.target_host)?;
    let remote_workspace = validate_remote_workspace(&payload.remote_workspace)?;
    let target = format!("{ssh_user}@{target_host}");
    let remote_command = format!(
        "cd {workspace} && printf '%s' 'kyuubiki-remote-ok'",
        workspace = shell_escape(&remote_workspace)
    );
    run_remote_ssh(payload.ssh_port, &target, &remote_command)
}
#[tauri::command]
pub fn remote_bootstrap(payload: RemoteBootstrapPayload) -> Result<String, String> {
    let ssh_user = validate_ssh_identity(&payload.ssh_user, "ssh user")?;
    let target_host = validate_target_host(&payload.target_host)?;
    let remote_workspace = validate_remote_workspace(&payload.remote_workspace)?;
    let target = format!("{ssh_user}@{target_host}");
    let remote_command = format!(
        "cd {workspace} && cargo run -p kyuubiki-installer --manifest-path workers/rust/Cargo.toml -- bootstrap",
        workspace = shell_escape(&remote_workspace)
    );

    run_remote_ssh(payload.ssh_port, &target, &remote_command)
}

#[tauri::command]
pub fn remote_start_agent(payload: RemoteAgentPayload) -> Result<String, String> {
    let ssh_user = validate_ssh_identity(&payload.ssh_user, "ssh user")?;
    let target_host = validate_target_host(&payload.target_host)?;
    let remote_workspace = validate_remote_workspace(&payload.remote_workspace)?;
    let control_mode = validate_control_mode(payload.control_mode.as_deref())?;
    let agent_id = validate_agent_id(&payload.agent_id)?;
    let advertise_host = validate_advertise_host(&payload.advertise_host)?;
    let cluster_id = validate_cluster_id(payload.cluster_id.as_deref())?;
    let peer_endpoints = normalize_peer_endpoints(payload.peer_endpoints.unwrap_or_default())?;
    let target = format!("{ssh_user}@{target_host}");
    let screen_name = format!("kyuubiki_remote_agent_{}", payload.agent_port);
    let remote_command = match control_mode.as_str() {
        "offline_mesh" => build_remote_mesh_agent_command(
            &remote_workspace,
            &screen_name,
            &agent_id,
            &advertise_host,
            payload.agent_port,
            cluster_id.as_deref(),
            &peer_endpoints,
        ),
        _ => build_remote_orchestrated_agent_command(
            &remote_workspace,
            &screen_name,
            &validate_orchestrator_url(&payload.orchestrator_url)?,
            &agent_id,
            &advertise_host,
            payload.agent_port,
        ),
    };
    run_remote_ssh(payload.ssh_port, &target, &remote_command)
}
fn build_remote_orchestrated_agent_command(
    remote_workspace: &str,
    screen_name: &str,
    orchestrator_url: &str,
    agent_id: &str,
    advertise_host: &str,
    agent_port: u16,
) -> String {
    format!(
        "cd {workspace} && screen -S {screen} -X quit >/dev/null 2>&1 || true && screen -dmS {screen} sh -lc 'cd workers/rust && KYUUBIKI_ORCHESTRATOR_URL={orchestrator} KYUUBIKI_AGENT_ID={agent_id} KYUUBIKI_AGENT_ADVERTISE_HOST={advertise_host} cargo run -p kyuubiki-cli -- agent --host 0.0.0.0 --port {port} --agent-id {agent_id} --advertise-host {advertise_host} --orchestrator-url {orchestrator}'",
        workspace = shell_escape(remote_workspace),
        screen = shell_escape(screen_name),
        orchestrator = shell_escape(orchestrator_url),
        agent_id = shell_escape(agent_id),
        advertise_host = shell_escape(advertise_host),
        port = agent_port
    )
}
fn build_remote_mesh_agent_command(
    remote_workspace: &str,
    screen_name: &str,
    agent_id: &str,
    advertise_host: &str,
    agent_port: u16,
    cluster_id: Option<&str>,
    peer_endpoints: &[String],
) -> String {
    let cluster_flag = cluster_id
        .map(|value| format!(" --cluster-id {}", shell_escape(value)))
        .unwrap_or_default();
    let peer_flags = peer_endpoints
        .iter()
        .map(|value| format!(" --peer {}", shell_escape(value)))
        .collect::<String>();
    let cluster_env = cluster_id
        .map(|value| format!("KYUUBIKI_AGENT_CLUSTER_ID={} ", shell_escape(value)))
        .unwrap_or_default();
    format!(
        "cd {workspace} && screen -S {screen} -X quit >/dev/null 2>&1 || true && screen -dmS {screen} sh -lc 'cd workers/rust && {cluster_env}KYUUBIKI_AGENT_ID={agent_id} KYUUBIKI_AGENT_ADVERTISE_HOST={advertise_host} cargo run -p kyuubiki-cli -- agent --host 0.0.0.0 --port {port} --agent-id {agent_id} --advertise-host {advertise_host}{cluster_flag}{peer_flags}'",
        workspace = shell_escape(remote_workspace),
        screen = shell_escape(screen_name),
        cluster_env = cluster_env,
        agent_id = shell_escape(agent_id),
        advertise_host = shell_escape(advertise_host),
        port = agent_port,
        cluster_flag = cluster_flag,
        peer_flags = peer_flags
    )
}
pub(crate) fn validate_target_host(value: &str) -> Result<String, String> {
    let host = validate_host_token(value, "target host")?;
    let allowed = effective_allowed_hosts()?;
    if !allowed.is_empty() && !allowed.iter().any(|candidate| candidate == &host) {
        return Err(format!(
            "target host is not allowed by installer remote policy: {host}"
        ));
    }
    Ok(host)
}
pub(crate) fn validate_advertise_host(value: &str) -> Result<String, String> {
    validate_host_token(value, "advertise host")
}
pub(crate) fn validate_host_token(value: &str, label: &str) -> Result<String, String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(format!("{label} is required"));
    }
    if trimmed.starts_with('-') {
        return Err(format!("{label} must not start with '-'"));
    }
    if trimmed.contains("://") || trimmed.contains('/') || trimmed.contains('?') || trimmed.contains('#') {
        return Err(format!("{label} must be a plain host or IPv4 address"));
    }
    if !trimmed
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '-' | '_'))
    {
        return Err(format!("{label} contains unsupported characters"));
    }
    Ok(trimmed.to_string())
}
pub(crate) fn validate_ssh_identity(value: &str, label: &str) -> Result<String, String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(format!("{label} is required"));
    }
    if trimmed.starts_with('-') {
        return Err(format!("{label} must not start with '-'"));
    }
    if !trimmed
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '-' | '_'))
    {
        return Err(format!("{label} contains unsupported characters"));
    }
    Ok(trimmed.to_string())
}
pub(crate) fn validate_remote_workspace(value: &str) -> Result<String, String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err("remote workspace is required".to_string());
    }
    if !trimmed.starts_with('/') {
        return Err("remote workspace must be an absolute path".to_string());
    }
    if !trimmed
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '/' | '.' | '-' | '_'))
    {
        return Err("remote workspace contains unsupported characters".to_string());
    }

    let path = Path::new(trimmed);
    for component in path.components() {
        match component {
            Component::RootDir | Component::Normal(_) => {}
            Component::CurDir | Component::ParentDir | Component::Prefix(_) => {
                return Err("remote workspace must not contain '.' or '..' segments".to_string());
            }
        }
    }

    let allowed_roots = effective_allowed_workspace_roots()?;
    if !allowed_roots.is_empty()
        && !allowed_roots
            .iter()
            .any(|root| trimmed == root || trimmed.starts_with(&format!("{root}/")))
    {
        return Err(format!("remote workspace is outside installer remote policy: {trimmed}"));
    }

    Ok(trimmed.to_string())
}
pub(crate) fn validate_orchestrator_url(value: &str) -> Result<String, String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err("orchestrator url is required".to_string());
    }
    if !(trimmed.starts_with("http://") || trimmed.starts_with("https://")) {
        return Err("orchestrator url must start with http:// or https://".to_string());
    }
    if trimmed.contains('@') || trimmed.contains('?') || trimmed.contains('#') || trimmed.contains(' ') {
        return Err("orchestrator url contains unsupported components".to_string());
    }
    Ok(trimmed.to_string())
}
pub(crate) fn validate_agent_id(value: &str) -> Result<String, String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err("agent id is required".to_string());
    }
    if !trimmed
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '-' | '_'))
    {
        return Err("agent id contains unsupported characters".to_string());
    }
    Ok(trimmed.to_string())
}
fn env_csv(key: &str) -> Vec<String> {
    env::var(key)
        .ok()
        .map(|value| normalize_csv_entries(&value))
        .unwrap_or_default()
}
fn normalize_csv_entries(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(str::trim)
        .filter(|entry| !entry.is_empty())
        .map(ToString::to_string)
        .collect::<Vec<_>>()
}
fn remote_policy_path() -> std::path::PathBuf {
    workspace_root().join("config").join("installer-remote-policy.json")
}
fn read_remote_policy_config() -> Result<RemoteDeployPolicyConfig, String> {
    let path = remote_policy_path();
    if !path.exists() {
        return Ok(RemoteDeployPolicyConfig {
            schema_version: REMOTE_POLICY_SCHEMA_VERSION.to_string(),
            allowed_hosts: Vec::new(),
            allowed_workspace_roots: Vec::new(),
        });
    }

    let contents = fs::read_to_string(&path)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    let value: Value = serde_json::from_str(&contents)
        .map_err(|error| format!("failed to parse {}: {error}", path.display()))?;
    Ok(RemoteDeployPolicyConfig {
        schema_version: value
            .get("schema_version")
            .and_then(Value::as_str)
            .unwrap_or(REMOTE_POLICY_SCHEMA_VERSION)
            .to_string(),
        allowed_hosts: json_array_strings(value.get("allowed_hosts")),
        allowed_workspace_roots: json_array_strings(value.get("allowed_workspace_roots")),
    })
}
fn json_array_strings(value: Option<&Value>) -> Vec<String> {
    value
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(str::trim)
                .filter(|item| !item.is_empty())
                .map(ToString::to_string)
                .collect()
        })
        .unwrap_or_default()
}
fn effective_allowed_hosts() -> Result<Vec<String>, String> {
    let configured = read_remote_policy_config()?.allowed_hosts;
    let env_hosts = env_csv(REMOTE_ALLOWED_HOSTS_ENV);
    Ok(if env_hosts.is_empty() { configured } else { env_hosts })
}
fn effective_allowed_workspace_roots() -> Result<Vec<String>, String> {
    let configured = read_remote_policy_config()?.allowed_workspace_roots;
    let env_roots = env_csv(REMOTE_ALLOWED_WORKSPACE_ROOTS_ENV);
    Ok(if env_roots.is_empty() {
        configured
    } else {
        env_roots
    })
}
fn build_remote_deploy_policy_payload() -> Result<RemoteDeployPolicyPayload, String> {
    let config = read_remote_policy_config()?;
    let env_allowed_hosts = env_csv(REMOTE_ALLOWED_HOSTS_ENV);
    let env_allowed_workspace_roots = env_csv(REMOTE_ALLOWED_WORKSPACE_ROOTS_ENV);
    let effective_allowed_hosts = if env_allowed_hosts.is_empty() {
        config.allowed_hosts.clone()
    } else {
        env_allowed_hosts.clone()
    };
    let effective_allowed_workspace_roots = if env_allowed_workspace_roots.is_empty() {
        config.allowed_workspace_roots.clone()
    } else {
        env_allowed_workspace_roots.clone()
    };
    let rendered = [
        "installer remote deployment policy".to_string(),
        format!("config_path: {}", remote_policy_path().display()),
        format!("allowed_hosts: {}", csv_or_placeholder(&config.allowed_hosts)),
        format!(
            "allowed_workspace_roots: {}",
            csv_or_placeholder(&config.allowed_workspace_roots)
        ),
        format!(
            "effective_allowed_hosts: {}",
            csv_or_placeholder(&effective_allowed_hosts)
        ),
        format!(
            "effective_allowed_workspace_roots: {}",
            csv_or_placeholder(&effective_allowed_workspace_roots)
        ),
    ]
    .join("\n");

    Ok(RemoteDeployPolicyPayload {
        schema_version: config.schema_version,
        config_path: remote_policy_path().display().to_string(),
        allowed_hosts: config.allowed_hosts.join(","),
        allowed_workspace_roots: config.allowed_workspace_roots.join(","),
        effective_allowed_hosts: effective_allowed_hosts.join(","),
        effective_allowed_workspace_roots: effective_allowed_workspace_roots.join(","),
        env_allowed_hosts: env_allowed_hosts.join(","),
        env_allowed_workspace_roots: env_allowed_workspace_roots.join(","),
        rendered,
    })
}
fn csv_or_placeholder(items: &[String]) -> String {
    if items.is_empty() {
        "(unbounded)".to_string()
    } else {
        items.join(",")
    }
}
fn run_remote_ssh(
    ssh_port: Option<u16>,
    target: &str,
    remote_command: &str,
) -> Result<String, String> {
    let mut command = Command::new("ssh");
    command
        .arg("-o")
        .arg("StrictHostKeyChecking=accept-new")
        .arg("-o")
        .arg("ConnectTimeout=10")
        .arg("-o")
        .arg("ServerAliveInterval=15")
        .arg("-o")
        .arg("ServerAliveCountMax=3");
    if let Some(port) = ssh_port {
        command.arg("-p").arg(port.to_string());
    }

    let output = command
        .arg(target)
        .arg(remote_command)
        .output()
        .map_err(|error| format!("failed to run ssh command: {error}"))?;

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();

    if output.status.success() {
        Ok(if stdout.is_empty() {
            format!("remote command completed on {target}")
        } else {
            stdout
        })
    } else {
        Err(if stderr.is_empty() { stdout } else { stderr })
    }
}
fn shell_escape(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\"'\"'"))
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_relative_remote_workspace() {
        let error = validate_remote_workspace("kyuubiki").unwrap_err();
        assert!(error.contains("absolute path"));
    }

    #[test]
    fn rejects_parent_segments_in_remote_workspace() {
        let error = validate_remote_workspace("/opt/kyuubiki/../other").unwrap_err();
        assert!(error.contains("must not contain '.' or '..'"));
    }

    #[test]
    fn rejects_orchestrator_url_with_query() {
        let error = validate_orchestrator_url("https://orch.example.com:4000?token=1").unwrap_err();
        assert!(error.contains("unsupported"));
    }

    #[test]
    fn rejects_target_host_outside_allowlist() {
        let _guard = TEST_ENV_LOCK.lock().unwrap();
        unsafe {
            env::set_var(REMOTE_ALLOWED_HOSTS_ENV, "192.168.1.12,solver-a");
        }
        let error = validate_target_host("192.168.1.99").unwrap_err();
        assert!(error.contains("installer remote policy"));
        unsafe {
            env::remove_var(REMOTE_ALLOWED_HOSTS_ENV);
        }
    }
}
