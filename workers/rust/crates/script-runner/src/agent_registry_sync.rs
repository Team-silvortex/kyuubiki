use std::collections::HashMap;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde_json::json;

type RunnerResult<T> = Result<T, String>;

pub(crate) fn run_agent_registry_sync(root: &Path, args: Vec<OsString>) -> RunnerResult<u8> {
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        print_help();
        return Ok(0);
    }
    if args.iter().any(|arg| arg == "--once") {
        let config = RegistryConfig::load(root)?;
        register_once(root, &config)?;
        heartbeat_once(root, &config)?;
        unregister_once(root, &config).ok();
        return Ok(0);
    }
    if !args.is_empty() {
        return Err("agent-registry-sync only accepts --help or --once".to_string());
    }

    let config = RegistryConfig::load(root)?;
    register_once(root, &config)?;
    loop {
        thread::sleep(Duration::from_millis(config.heartbeat_interval_ms));
        heartbeat_once(root, &config)?;
    }
}

struct RegistryConfig {
    orch_base_url: String,
    heartbeat_interval_ms: u64,
    cluster_token: String,
    agent_id: String,
    agent_host: String,
    agent_port: u16,
    fingerprint: String,
    cluster_id: Option<String>,
}

impl RegistryConfig {
    fn load(root: &Path) -> RunnerResult<Self> {
        let home = std::env::var("KYUUBIKI_HOME").unwrap_or_else(|_| root.display().to_string());
        let home_path = PathBuf::from(&home);
        let orch_env_path = std::env::var("KYUUBIKI_ORCHESTRATOR_ENV_FILE")
            .map(PathBuf::from)
            .unwrap_or_else(|_| home_path.join("deploy/kyuubiki-orchestrator.env"));
        let agent_env_path = std::env::var("KYUUBIKI_AGENT_ENV_FILE")
            .map(PathBuf::from)
            .unwrap_or_else(|_| home_path.join("deploy/kyuubiki-agent.env"));
        let orch_env = read_env_file(&orch_env_path)?;
        let agent_env = read_env_file(&agent_env_path)?;

        let cluster_token = env_value(&orch_env, "KYUUBIKI_CLUSTER_API_TOKEN").unwrap_or_default();
        let agent_id = env_value(&agent_env, "KYUUBIKI_AGENT_ID").unwrap_or_default();
        let fingerprint = env_value(&agent_env, "KYUUBIKI_AGENT_FINGERPRINT").unwrap_or_default();
        if cluster_token.is_empty() || agent_id.is_empty() || fingerprint.is_empty() {
            return Err("missing registry sync configuration".to_string());
        }

        let agent_host = env_value(&agent_env, "KYUUBIKI_AGENT_ADVERTISE_HOST")
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| {
                std::env::var("KYUUBIKI_AGENT_ADVERTISE_HOST_FALLBACK")
                    .unwrap_or_else(|_| "kyuubiki-lab.local".to_string())
            });
        let agent_port = env_value(&agent_env, "KYUUBIKI_AGENT_PORT")
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| {
                std::env::var("KYUUBIKI_AGENT_PORT_FALLBACK").unwrap_or_else(|_| "5001".to_string())
            })
            .parse::<u16>()
            .map_err(|error| format!("invalid KYUUBIKI_AGENT_PORT: {error}"))?;

        Ok(Self {
            orch_base_url: std::env::var("KYUUBIKI_AGENT_REGISTRY_URL")
                .unwrap_or_else(|_| "http://127.0.0.1:4000".to_string()),
            heartbeat_interval_ms: std::env::var("KYUUBIKI_AGENT_REGISTRY_INTERVAL_MS")
                .unwrap_or_else(|_| "5000".to_string())
                .parse::<u64>()
                .map_err(|error| format!("invalid KYUUBIKI_AGENT_REGISTRY_INTERVAL_MS: {error}"))?,
            cluster_token,
            agent_id,
            agent_host,
            agent_port,
            fingerprint,
            cluster_id: env_value(&agent_env, "KYUUBIKI_AGENT_CLUSTER_ID")
                .filter(|value| !value.is_empty()),
        })
    }

    fn payload(&self) -> serde_json::Value {
        let mut payload = json!({
            "id": self.agent_id,
            "host": self.agent_host,
            "port": self.agent_port,
            "role": "solver",
            "control_mode": "orch_managed",
            "orch_id": self.orch_base_url,
            "tags": ["headless", "standalone"],
            "methods": [],
            "capabilities": [],
            "health_score": 100
        });
        if let Some(cluster_id) = &self.cluster_id {
            payload["cluster_id"] = json!(cluster_id);
        }
        payload
    }

    fn headers(&self) -> Vec<(String, String)> {
        let ts = unix_millis().to_string();
        let nonce = format!("registry-{}-{ts}", self.agent_id);
        vec![
            ("content-type".to_string(), "application/json".to_string()),
            ("x-kyuubiki-token".to_string(), self.cluster_token.clone()),
            ("x-kyuubiki-agent-id".to_string(), self.agent_id.clone()),
            ("x-kyuubiki-cluster-ts".to_string(), ts),
            ("x-kyuubiki-cluster-nonce".to_string(), nonce),
            (
                "x-kyuubiki-agent-fingerprint".to_string(),
                self.fingerprint.clone(),
            ),
        ]
    }
}

fn register_once(root: &Path, config: &RegistryConfig) -> RunnerResult<()> {
    curl_json(
        root,
        "POST",
        &format!("{}/api/v1/agents/register", config.orch_base_url),
        Some(config.payload().to_string()),
        config,
    )
}

fn heartbeat_once(root: &Path, config: &RegistryConfig) -> RunnerResult<()> {
    curl_json(
        root,
        "POST",
        &format!(
            "{}/api/v1/agents/{}/heartbeat",
            config.orch_base_url, config.agent_id
        ),
        Some(config.payload().to_string()),
        config,
    )
}

fn unregister_once(root: &Path, config: &RegistryConfig) -> RunnerResult<()> {
    curl_json(
        root,
        "DELETE",
        &format!("{}/api/v1/agents/{}", config.orch_base_url, config.agent_id),
        None,
        config,
    )
}

fn curl_json(
    root: &Path,
    method: &str,
    url: &str,
    payload: Option<String>,
    config: &RegistryConfig,
) -> RunnerResult<()> {
    let mut command = Command::new("curl");
    command
        .args(["-fsS", "--max-time", "8", "-X", method])
        .arg(url);
    for (key, value) in config.headers() {
        command.args(["-H", &format!("{key}: {value}")]);
    }
    if let Some(payload) = payload {
        command.arg("--data-binary").arg(payload);
    }
    let status = command
        .current_dir(root)
        .stdout(Stdio::null())
        .status()
        .map_err(|error| format!("failed to run curl: {error}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("registry sync {method} {url} failed"))
    }
}

fn read_env_file(path: &Path) -> RunnerResult<HashMap<String, String>> {
    let text = std::fs::read_to_string(path)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    let mut values = HashMap::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if let Some((key, value)) = trimmed.split_once('=') {
            values.insert(key.trim().to_string(), unquote_env_value(value.trim()));
        }
    }
    Ok(values)
}

fn env_value(values: &HashMap<String, String>, key: &str) -> Option<String> {
    values.get(key).cloned()
}

fn unquote_env_value(value: &str) -> String {
    value
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
        .or_else(|| {
            value
                .strip_prefix('\'')
                .and_then(|value| value.strip_suffix('\''))
        })
        .unwrap_or(value)
        .to_string()
}

fn unix_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}

fn print_help() {
    println!(
        "agent-registry-sync\n\n\
Usage:\n  ./scripts/kyuubiki agent-registry-sync [--once]\n\n\
Registers the local agent with the orchestrator registry and sends heartbeats.\n\
Configuration is loaded from deploy/kyuubiki-orchestrator.env and \
deploy/kyuubiki-agent.env unless overridden by environment variables."
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_simple_env_values() {
        assert_eq!(unquote_env_value("abc"), "abc");
        assert_eq!(unquote_env_value("\"abc\""), "abc");
        assert_eq!(unquote_env_value("'abc'"), "abc");
    }

    #[test]
    fn payload_includes_optional_cluster_id() {
        let config = RegistryConfig {
            orch_base_url: "http://orch".to_string(),
            heartbeat_interval_ms: 1000,
            cluster_token: "token".to_string(),
            agent_id: "agent-a".to_string(),
            agent_host: "host-a".to_string(),
            agent_port: 5001,
            fingerprint: "fp".to_string(),
            cluster_id: Some("cluster-a".to_string()),
        };
        let payload = config.payload();
        assert_eq!(payload["id"], "agent-a");
        assert_eq!(payload["cluster_id"], "cluster-a");
        assert_eq!(payload["health_score"], 100);
    }
}
