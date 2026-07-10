use serde::{Deserialize, Serialize};

const REMOTE_HOST_TRUST_SCHEMA_VERSION: &str = "kyuubiki.remote-host-trust/v1";

#[cfg(test)]
#[path = "remote_host_trust_fuzz.rs"]
mod remote_host_trust_fuzz;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemoteHostTrustPlan {
    pub schema_version: String,
    pub current_mode: String,
    pub target_mode: String,
    pub dev_known_hosts_path: String,
    pub managed_known_hosts_path: String,
    pub options: Vec<RemoteHostTrustOption>,
    pub required_before_managed_execution: Vec<String>,
    pub notes: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemoteHostTrustOption {
    pub phase: String,
    pub key: String,
    pub value: String,
}

impl RemoteHostTrustPlan {
    pub fn render(&self) -> String {
        let mut lines = vec![
            "kyuubiki remote host trust plan".to_string(),
            format!("schema: {}", self.schema_version),
            format!("current_mode: {}", self.current_mode),
            format!("target_mode: {}", self.target_mode),
            format!("dev_known_hosts_path: {}", self.dev_known_hosts_path),
            format!(
                "managed_known_hosts_path: {}",
                self.managed_known_hosts_path
            ),
            "ssh_options:".to_string(),
        ];

        for option in &self.options {
            lines.push(format!(
                "  - [{}] {}={}",
                option.phase, option.key, option.value
            ));
        }

        lines.push("required_before_managed_execution:".to_string());
        for item in &self.required_before_managed_execution {
            lines.push(format!("  - {item}"));
        }

        lines.push("notes:".to_string());
        for note in &self.notes {
            lines.push(format!("  - {note}"));
        }

        lines.join("\n")
    }
}

pub fn default_remote_host_trust_plan() -> RemoteHostTrustPlan {
    RemoteHostTrustPlan {
        schema_version: REMOTE_HOST_TRUST_SCHEMA_VERSION.to_string(),
        current_mode: "dev-accept-new".to_string(),
        target_mode: "pinned-known-host".to_string(),
        dev_known_hosts_path: "tests/integration/remote-ssh-fixture/runtime/known_hosts".to_string(),
        managed_known_hosts_path: ".kyuubiki/credentials/installer/remote-trust/known_hosts"
            .to_string(),
        options: vec![
            option("dev", "StrictHostKeyChecking", "accept-new"),
            option(
                "dev",
                "UserKnownHostsFile",
                "tests/integration/remote-ssh-fixture/runtime/known_hosts",
            ),
            option("managed", "StrictHostKeyChecking", "yes"),
            option(
                "managed",
                "UserKnownHostsFile",
                ".kyuubiki/credentials/installer/remote-trust/known_hosts",
            ),
            option("managed", "HostKeyAlias", "<node-id-or-fingerprint>"),
        ],
        required_before_managed_execution: vec![
            "installer records the expected host key fingerprint for the target node".to_string(),
            "operator can review the pinned host identity before execution".to_string(),
            "managed SSH execution refuses hosts missing a pinned key".to_string(),
            "project files never store SSH passwords or private keys".to_string(),
        ],
        notes: vec![
            "this command is read-only and does not write known_hosts files".to_string(),
            "accept-new is limited to local fixture and explicit development workflows".to_string(),
            "managed deployment should bind node identity, host key, artifact integrity, and journal records".to_string(),
        ],
    }
}

fn option(phase: &str, key: &str, value: &str) -> RemoteHostTrustOption {
    RemoteHostTrustOption {
        phase: phase.to_string(),
        key: key.to_string(),
        value: value.to_string(),
    }
}
