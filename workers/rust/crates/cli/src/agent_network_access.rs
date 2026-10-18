use std::io;
use std::net::SocketAddr;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicU64, Ordering};

use kyuubiki_protocol::{
    AGENT_ALLOWED_PEER_IPS_ENV as ENV_NAME, AGENT_NETWORK_MAX_PEERS as MAX_PEERS,
    AgentNetworkAccessPolicy,
};
static ACCESS: OnceLock<NetworkAccess> = OnceLock::new();

#[derive(Default)]
pub(crate) struct NetworkAccess {
    policy: AgentNetworkAccessPolicy,
    denied_connections: AtomicU64,
}

impl NetworkAccess {
    pub(crate) fn parse(value: Option<&str>) -> Result<Self, String> {
        Ok(Self {
            policy: AgentNetworkAccessPolicy::parse(value)?,
            denied_connections: AtomicU64::new(0),
        })
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
            "recovery_reserve_bypass":false,
            "authenticated":false,
            "encrypted":false,
            "trusted_network_required":true,
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
    ACCESS
        .set(NetworkAccess::parse(value.as_deref())?)
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
