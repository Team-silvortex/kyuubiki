use std::io;
use std::net::SocketAddr;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicU64, Ordering};

use kyuubiki_protocol::{
    AGENT_ALLOWED_PEER_IPS_ENV as ENV_NAME, AGENT_NETWORK_MAX_PEERS as MAX_PEERS,
    AGENT_TRUSTED_LAN_CONTROL_ENV, AgentNetworkAccessPolicy, RpcMethod, RpcResponse,
};
static ACCESS: OnceLock<NetworkAccess> = OnceLock::new();

#[derive(Default)]
pub(crate) struct NetworkAccess {
    policy: AgentNetworkAccessPolicy,
    denied_connections: AtomicU64,
    denied_requests: AtomicU64,
}

impl NetworkAccess {
    pub(crate) fn parse(value: Option<&str>) -> Result<Self, String> {
        Ok(Self {
            policy: AgentNetworkAccessPolicy::parse(value)?,
            denied_connections: AtomicU64::new(0),
            denied_requests: AtomicU64::new(0),
        })
    }

    pub(crate) fn with_trusted_lan_control(mut self, value: Option<&str>) -> Result<Self, String> {
        self.policy = self.policy.with_trusted_lan_control(value)?;
        Ok(self)
    }

    pub(crate) fn connection_access(&self, peer_is_loopback: bool) -> serde_json::Value {
        serde_json::to_value(self.policy.connection_access(peer_is_loopback))
            .expect("connection access policy should serialize")
    }

    pub(crate) fn reject_remote_request(
        &self,
        id: &str,
        method: &RpcMethod,
    ) -> Option<RpcResponse> {
        if self.policy.allows_remote_method(method) {
            return None;
        }
        let _ = self
            .denied_requests
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |count| {
                Some(count.saturating_add(1))
            });
        Some(RpcResponse::error_with_details(
            id,
            "agent_lan_control_requires_explicit_trust",
            "non-loopback control requires explicit trusted-LAN configuration; source IP and task identifiers are not authentication",
            serde_json::json!({"schema_version":"kyuubiki.agent-lan-control-denial/v1",
                "configuration_env":AGENT_TRUSTED_LAN_CONTROL_ENV,
                "configuration_apply":"process_restart", "dispatched":false,
                "automatic_retry":false, "automatic_replay_authorized":false,
                "authenticated":false,
                "required_action":"inspect_agent_policy_do_not_replay_unknown_work"}),
        ))
    }

    pub(crate) fn admit(&self, peer: io::Result<SocketAddr>) -> bool {
        let allowed = peer.is_ok_and(|peer| self.policy.allows_ip(peer.ip()));
        if !allowed {
            let _ = self.denied_connections.fetch_update(
                Ordering::Relaxed,
                Ordering::Relaxed,
                |count| Some(count.saturating_add(1)),
            );
        }
        allowed
    }

    fn snapshot(&self) -> serde_json::Value {
        serde_json::json!({
            "schema_version":"kyuubiki.agent-network-access/v1",
            "mode":self.policy.mode(),
            "configuration_env":ENV_NAME,
            "configuration_apply":"process_restart",
            "allowed_peer_ips":self.policy.allowed_peer_ips(),
            "max_configured_peer_ips":MAX_PEERS,
            "loopback_allowed":true,
            "peer_source":"kernel_socket_peer_ip",
            "check_before":"connection_capacity_thread_and_request_frame",
            "denial_action":"close_without_reading_or_reply",
            "denied_connections":self.denied_connections.load(Ordering::Relaxed),
            "lan_control": {
                "mode":self.policy.control_mode(),
                "configuration_env":AGENT_TRUSTED_LAN_CONTROL_ENV,
                "configuration_apply":"process_restart",
                "default_remote_methods":["ping", "describe_agent", "describe_agent_lifecycle"],
                "loopback_control_allowed":true,
                "remote_lifecycle_mutations_allowed":false,
                "denied_requests":self.denied_requests.load(Ordering::Relaxed),
                "check_before":"dispatch_store_access_and_execution_admission",
                "trusted_mode_is_authentication":false
            },
            "recovery_reserve_bypass":false,
            "authenticated":crate::agent_api_security::enabled(),
            "encrypted":crate::agent_api_security::enabled(),
            "trusted_network_required":!crate::agent_api_security::enabled(),
            "api_security": {
                "enabled":crate::agent_api_security::enabled(),
                "protocol":"kyuubiki.agent-auth/v1",
                "tls_version":"1.3",
                "configuration_apply":"process_restart",
                "token_environment":"KYUUBIKI_AGENT_API_TOKEN",
                "certificate_environment":"KYUUBIKI_AGENT_TLS_CERT_FILE",
                "private_key_environment":"KYUUBIKI_AGENT_TLS_KEY_FILE",
                "identity_scope":"shared_token_not_orchestra_binding_or_individual_user",
                "plaintext_fallback":false
            },
            "local_process_identity_verified":false,
            "authority_binding_verified":false,
            "proxy_and_nat_scope":"observed_socket_peer_not_original_caller"
        })
    }
}

pub(crate) fn configure() -> Result<(), String> {
    let value = match std::env::var(ENV_NAME) {
        Ok(value) => Some(value),
        Err(std::env::VarError::NotPresent) => None,
        Err(_) => return Err(format!("{ENV_NAME} must be valid Unicode")),
    };
    let control = match std::env::var(AGENT_TRUSTED_LAN_CONTROL_ENV) {
        Ok(value) => Some(value),
        Err(std::env::VarError::NotPresent) => None,
        Err(_) => {
            return Err(format!(
                "{AGENT_TRUSTED_LAN_CONTROL_ENV} must be valid Unicode"
            ));
        }
    };
    let access =
        NetworkAccess::parse(value.as_deref())?.with_trusted_lan_control(control.as_deref())?;
    ACCESS
        .set(access)
        .map_err(|_| "Agent network access already configured".into())
}

pub(crate) fn current() -> &'static NetworkAccess {
    ACCESS.get_or_init(NetworkAccess::default)
}

pub(crate) fn snapshot() -> serde_json::Value {
    current().snapshot()
}

#[cfg(test)]
#[path = "tests/agent_network_access.rs"]
mod tests;
