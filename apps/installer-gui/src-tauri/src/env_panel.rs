use std::collections::HashMap;
use std::fs;

use kyuubiki_installer::{validate_env_file, workspace_root};
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
pub struct EnvFormPayload {
    pub deployment_mode: String,
    pub agent_discovery: String,
    pub agent_manifest_path: String,
    pub storage_backend: String,
    pub sqlite_database_path: String,
    pub database_url: String,
    pub database_url_configured: bool,
    pub orchestra_lease_name: String,
    pub orchestra_instance_id: String,
    pub orchestra_lease_ttl_ms: String,
    pub orchestra_lease_heartbeat_ms: String,
    pub orchestra_lease_retry_ms: String,
    pub orchestra_lease_query_timeout_ms: String,
    pub agent_endpoints: String,
    pub kyuubiki_api_token: String,
    pub kyuubiki_api_token_configured: bool,
    pub kyuubiki_cluster_api_token: String,
    pub kyuubiki_cluster_api_token_configured: bool,
    pub kyuubiki_cluster_allowed_agent_ids: String,
    pub kyuubiki_cluster_allowed_cluster_ids: String,
    pub kyuubiki_cluster_require_fingerprint: bool,
    pub kyuubiki_cluster_timestamp_window_ms: String,
    pub kyuubiki_protect_reads: bool,
    pub kyuubiki_direct_mesh_enabled: bool,
    pub kyuubiki_direct_mesh_token: String,
    pub kyuubiki_direct_mesh_token_configured: bool,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WriteEnvPayload {
    pub deployment_mode: String,
    pub agent_discovery: String,
    pub agent_manifest_path: String,
    pub storage_backend: String,
    pub sqlite_database_path: String,
    pub database_url: String,
    pub database_url_configured: bool,
    pub orchestra_lease_name: String,
    pub orchestra_instance_id: String,
    pub orchestra_lease_ttl_ms: String,
    pub orchestra_lease_heartbeat_ms: String,
    pub orchestra_lease_retry_ms: String,
    pub orchestra_lease_query_timeout_ms: String,
    pub agent_endpoints: String,
    pub kyuubiki_api_token: String,
    pub kyuubiki_api_token_configured: bool,
    pub kyuubiki_cluster_api_token: String,
    pub kyuubiki_cluster_api_token_configured: bool,
    pub kyuubiki_cluster_allowed_agent_ids: String,
    pub kyuubiki_cluster_allowed_cluster_ids: String,
    pub kyuubiki_cluster_require_fingerprint: bool,
    pub kyuubiki_cluster_timestamp_window_ms: String,
    pub kyuubiki_protect_reads: bool,
    pub kyuubiki_direct_mesh_enabled: bool,
    pub kyuubiki_direct_mesh_token: String,
    pub kyuubiki_direct_mesh_token_configured: bool,
}

fn parse_env_lines(contents: &str) -> HashMap<String, String> {
    let mut entries = HashMap::new();
    for raw_line in contents.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((key, value)) = line.split_once('=') {
            entries.insert(key.trim().to_string(), value.trim().to_string());
        }
    }
    entries
}

fn resolve_sensitive_env_value(
    payload_value: &str,
    preserve_existing: bool,
    existing_entries: &HashMap<String, String>,
    key: &str,
) -> Option<String> {
    let trimmed = payload_value.trim();
    if !trimmed.is_empty() {
        return Some(trimmed.to_string());
    }
    if preserve_existing {
        return existing_entries
            .get(key)
            .cloned()
            .filter(|value| !value.trim().is_empty());
    }
    None
}

fn env_line_value(label: &str, value: &str) -> Result<String, String> {
    let trimmed = value.trim();
    if trimmed
        .chars()
        .any(|ch| ch == '\0' || ch == '\n' || ch == '\r' || ch.is_control())
    {
        return Err(format!("{label} contains unsupported control characters"));
    }
    Ok(trimmed.to_string())
}

fn positive_millisecond_value(label: &str, value: &str) -> Result<String, String> {
    let normalized = env_line_value(label, value)?;
    match normalized.parse::<u64>() {
        Ok(value) if value > 0 => Ok(value.to_string()),
        _ => Err(format!("{label} must be a positive integer")),
    }
}

#[tauri::command]
pub fn read_env_file() -> Result<EnvFormPayload, String> {
    let path = workspace_root().join(".env.local");
    let contents = fs::read_to_string(&path)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;

    let mut form = EnvFormPayload {
        deployment_mode: "local".to_string(),
        agent_discovery: "static".to_string(),
        agent_manifest_path: String::new(),
        storage_backend: "sqlite".to_string(),
        sqlite_database_path: String::new(),
        database_url: String::new(),
        database_url_configured: false,
        orchestra_lease_name: "workflow-recovery".to_string(),
        orchestra_instance_id: String::new(),
        orchestra_lease_ttl_ms: "15000".to_string(),
        orchestra_lease_heartbeat_ms: "5000".to_string(),
        orchestra_lease_retry_ms: "1000".to_string(),
        orchestra_lease_query_timeout_ms: "2000".to_string(),
        agent_endpoints: "127.0.0.1:5001,127.0.0.1:5002".to_string(),
        kyuubiki_api_token: String::new(),
        kyuubiki_api_token_configured: false,
        kyuubiki_cluster_api_token: String::new(),
        kyuubiki_cluster_api_token_configured: false,
        kyuubiki_cluster_allowed_agent_ids: String::new(),
        kyuubiki_cluster_allowed_cluster_ids: String::new(),
        kyuubiki_cluster_require_fingerprint: false,
        kyuubiki_cluster_timestamp_window_ms: "30000".to_string(),
        kyuubiki_protect_reads: false,
        kyuubiki_direct_mesh_enabled: true,
        kyuubiki_direct_mesh_token: String::new(),
        kyuubiki_direct_mesh_token_configured: false,
    };

    for (key, value) in parse_env_lines(&contents) {
        match key.as_str() {
            "KYUUBIKI_DEPLOYMENT_MODE" => form.deployment_mode = value,
            "KYUUBIKI_AGENT_DISCOVERY" => form.agent_discovery = value,
            "KYUUBIKI_AGENT_MANIFEST_PATH" => form.agent_manifest_path = value,
            "KYUUBIKI_STORAGE_BACKEND" => form.storage_backend = value,
            "SQLITE_DATABASE_PATH" => form.sqlite_database_path = value,
            "DATABASE_URL" => form.database_url_configured = !value.is_empty(),
            "KYUUBIKI_ORCHESTRA_LEASE_NAME" => form.orchestra_lease_name = value,
            "KYUUBIKI_ORCHESTRA_INSTANCE_ID" => form.orchestra_instance_id = value,
            "KYUUBIKI_ORCHESTRA_LEASE_TTL_MS" => form.orchestra_lease_ttl_ms = value,
            "KYUUBIKI_ORCHESTRA_LEASE_HEARTBEAT_MS" => form.orchestra_lease_heartbeat_ms = value,
            "KYUUBIKI_ORCHESTRA_LEASE_RETRY_MS" => form.orchestra_lease_retry_ms = value,
            "KYUUBIKI_ORCHESTRA_LEASE_QUERY_TIMEOUT_MS" => {
                form.orchestra_lease_query_timeout_ms = value
            }
            "KYUUBIKI_AGENT_ENDPOINTS" => form.agent_endpoints = value,
            "KYUUBIKI_API_TOKEN" => form.kyuubiki_api_token_configured = !value.is_empty(),
            "KYUUBIKI_CLUSTER_API_TOKEN" => {
                form.kyuubiki_cluster_api_token_configured = !value.is_empty()
            }
            "KYUUBIKI_CLUSTER_ALLOWED_AGENT_IDS" => form.kyuubiki_cluster_allowed_agent_ids = value,
            "KYUUBIKI_CLUSTER_ALLOWED_CLUSTER_IDS" => {
                form.kyuubiki_cluster_allowed_cluster_ids = value
            }
            "KYUUBIKI_CLUSTER_REQUIRE_FINGERPRINT" => {
                form.kyuubiki_cluster_require_fingerprint = value == "true"
            }
            "KYUUBIKI_CLUSTER_TIMESTAMP_WINDOW_MS" => {
                form.kyuubiki_cluster_timestamp_window_ms = value
            }
            "KYUUBIKI_PROTECT_READS" => form.kyuubiki_protect_reads = value == "true",
            "KYUUBIKI_DIRECT_MESH_ENABLED" => form.kyuubiki_direct_mesh_enabled = value != "false",
            "KYUUBIKI_DIRECT_MESH_TOKEN" => {
                form.kyuubiki_direct_mesh_token_configured = !value.is_empty()
            }
            _ => {}
        }
    }

    Ok(form)
}

#[tauri::command]
pub fn write_env_file(payload: WriteEnvPayload) -> Result<String, String> {
    let path = workspace_root().join(".env.local");
    let existing_entries = fs::read_to_string(&path)
        .ok()
        .map(|contents| parse_env_lines(&contents))
        .unwrap_or_default();
    let mut lines = vec![
        format!(
            "KYUUBIKI_DEPLOYMENT_MODE={}",
            env_line_value("deployment mode", &payload.deployment_mode)?
        ),
        format!(
            "KYUUBIKI_AGENT_DISCOVERY={}",
            env_line_value("agent discovery", &payload.agent_discovery)?
        ),
        format!(
            "KYUUBIKI_STORAGE_BACKEND={}",
            env_line_value("storage backend", &payload.storage_backend)?
        ),
    ];

    if !payload.agent_manifest_path.trim().is_empty() {
        lines.push(format!(
            "KYUUBIKI_AGENT_MANIFEST_PATH={}",
            env_line_value("agent manifest path", &payload.agent_manifest_path)?
        ));
    }
    if !payload.sqlite_database_path.trim().is_empty() {
        lines.push(format!(
            "SQLITE_DATABASE_PATH={}",
            env_line_value("sqlite database path", &payload.sqlite_database_path)?
        ));
    }
    if let Some(database_url) = resolve_sensitive_env_value(
        &payload.database_url,
        payload.database_url_configured,
        &existing_entries,
        "DATABASE_URL",
    ) {
        lines.push(format!(
            "DATABASE_URL={}",
            env_line_value("database url", &database_url)?
        ));
    }
    lines.push(format!(
        "KYUUBIKI_ORCHESTRA_LEASE_NAME={}",
        env_line_value("Orchestra lease name", &payload.orchestra_lease_name)?
    ));
    if !payload.orchestra_instance_id.trim().is_empty() {
        lines.push(format!(
            "KYUUBIKI_ORCHESTRA_INSTANCE_ID={}",
            env_line_value("Orchestra instance ID", &payload.orchestra_instance_id)?
        ));
    }
    lines.push(format!(
        "KYUUBIKI_ORCHESTRA_LEASE_TTL_MS={}",
        positive_millisecond_value("Orchestra lease TTL", &payload.orchestra_lease_ttl_ms)?
    ));
    lines.push(format!(
        "KYUUBIKI_ORCHESTRA_LEASE_HEARTBEAT_MS={}",
        positive_millisecond_value(
            "Orchestra lease heartbeat",
            &payload.orchestra_lease_heartbeat_ms
        )?
    ));
    lines.push(format!(
        "KYUUBIKI_ORCHESTRA_LEASE_RETRY_MS={}",
        positive_millisecond_value("Orchestra lease retry", &payload.orchestra_lease_retry_ms)?
    ));
    lines.push(format!(
        "KYUUBIKI_ORCHESTRA_LEASE_QUERY_TIMEOUT_MS={}",
        positive_millisecond_value(
            "Orchestra lease query timeout",
            &payload.orchestra_lease_query_timeout_ms
        )?
    ));
    if !payload.agent_endpoints.trim().is_empty() {
        lines.push(format!(
            "KYUUBIKI_AGENT_ENDPOINTS={}",
            env_line_value("agent endpoints", &payload.agent_endpoints)?
        ));
    }
    if let Some(api_token) = resolve_sensitive_env_value(
        &payload.kyuubiki_api_token,
        payload.kyuubiki_api_token_configured,
        &existing_entries,
        "KYUUBIKI_API_TOKEN",
    ) {
        lines.push(format!(
            "KYUUBIKI_API_TOKEN={}",
            env_line_value("api token", &api_token)?
        ));
    }
    if let Some(cluster_api_token) = resolve_sensitive_env_value(
        &payload.kyuubiki_cluster_api_token,
        payload.kyuubiki_cluster_api_token_configured,
        &existing_entries,
        "KYUUBIKI_CLUSTER_API_TOKEN",
    ) {
        lines.push(format!(
            "KYUUBIKI_CLUSTER_API_TOKEN={}",
            env_line_value("cluster api token", &cluster_api_token)?
        ));
    }
    if !payload.kyuubiki_cluster_allowed_agent_ids.trim().is_empty() {
        lines.push(format!(
            "KYUUBIKI_CLUSTER_ALLOWED_AGENT_IDS={}",
            env_line_value(
                "cluster allowed agent ids",
                &payload.kyuubiki_cluster_allowed_agent_ids
            )?
        ));
    }
    if !payload
        .kyuubiki_cluster_allowed_cluster_ids
        .trim()
        .is_empty()
    {
        lines.push(format!(
            "KYUUBIKI_CLUSTER_ALLOWED_CLUSTER_IDS={}",
            env_line_value(
                "cluster allowed cluster ids",
                &payload.kyuubiki_cluster_allowed_cluster_ids
            )?
        ));
    }
    lines.push(format!(
        "KYUUBIKI_CLUSTER_REQUIRE_FINGERPRINT={}",
        if payload.kyuubiki_cluster_require_fingerprint {
            "true"
        } else {
            "false"
        }
    ));
    if !payload
        .kyuubiki_cluster_timestamp_window_ms
        .trim()
        .is_empty()
    {
        lines.push(format!(
            "KYUUBIKI_CLUSTER_TIMESTAMP_WINDOW_MS={}",
            env_line_value(
                "cluster timestamp window",
                &payload.kyuubiki_cluster_timestamp_window_ms
            )?
        ));
    }
    lines.push(format!(
        "KYUUBIKI_PROTECT_READS={}",
        if payload.kyuubiki_protect_reads {
            "true"
        } else {
            "false"
        }
    ));
    lines.push(format!(
        "KYUUBIKI_DIRECT_MESH_ENABLED={}",
        if payload.kyuubiki_direct_mesh_enabled {
            "true"
        } else {
            "false"
        }
    ));
    if let Some(direct_mesh_token) = resolve_sensitive_env_value(
        &payload.kyuubiki_direct_mesh_token,
        payload.kyuubiki_direct_mesh_token_configured,
        &existing_entries,
        "KYUUBIKI_DIRECT_MESH_TOKEN",
    ) {
        lines.push(format!(
            "KYUUBIKI_DIRECT_MESH_TOKEN={}",
            env_line_value("direct mesh token", &direct_mesh_token)?
        ));
    }

    fs::write(&path, format!("{}\n", lines.join("\n")))
        .map_err(|error| format!("failed to write {}: {error}", path.display()))?;
    validate_env_file()?;
    Ok(format!("wrote {}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::{env_line_value, positive_millisecond_value};

    #[test]
    fn env_line_value_rejects_newline_injection() {
        let error = env_line_value("api token", "safe\nEVIL=true").unwrap_err();
        assert!(error.contains("control characters"));
    }

    #[test]
    fn env_line_value_trims_safe_values() {
        assert_eq!(env_line_value("api token", " token ").unwrap(), "token");
    }

    #[test]
    fn positive_millisecond_value_rejects_zero_and_non_numeric_values() {
        assert!(positive_millisecond_value("lease TTL", "0").is_err());
        assert!(positive_millisecond_value("lease TTL", "later").is_err());
        assert_eq!(
            positive_millisecond_value("lease TTL", " 15000 ").unwrap(),
            "15000"
        );
    }
}
