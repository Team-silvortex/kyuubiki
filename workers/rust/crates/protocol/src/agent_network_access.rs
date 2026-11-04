use std::net::IpAddr;

use crate::{AgentConnectionAccess, RpcMethod};

pub const AGENT_ALLOWED_PEER_IPS_ENV: &str = "KYUUBIKI_AGENT_ALLOWED_PEER_IPS";
pub const AGENT_TRUSTED_LAN_CONTROL_ENV: &str = "KYUUBIKI_AGENT_TRUSTED_LAN_CONTROL";
pub const AGENT_NETWORK_MAX_PEERS: usize = 64;
pub const AGENT_NETWORK_MAX_CONFIG_BYTES: usize = 4096;

/// Source-IP containment only, not peer identity or controller authorization.
/// Parsing performs no DNS, environment, filesystem or network operations.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AgentNetworkAccessPolicy {
    allowed_peers: Vec<IpAddr>,
    trusted_lan_control: bool,
}

impl AgentNetworkAccessPolicy {
    pub fn parse(value: Option<&str>) -> Result<Self, String> {
        let Some(value) = value else {
            return Ok(Self::default());
        };
        let invalid = || {
            format!(
                "{AGENT_ALLOWED_PEER_IPS_ENV} must contain 1..={AGENT_NETWORK_MAX_PEERS} comma-separated unicast IP literals (at most {AGENT_NETWORK_MAX_CONFIG_BYTES} bytes); unset it for loopback only"
            )
        };
        if value.len() > AGENT_NETWORK_MAX_CONFIG_BYTES {
            return Err(invalid());
        }
        let mut allowed_peers = Vec::new();
        for (index, entry) in value.split(',').enumerate() {
            if index >= AGENT_NETWORK_MAX_PEERS {
                return Err(invalid());
            }
            let ip = entry
                .trim()
                .parse::<IpAddr>()
                .map_err(|_| invalid())?
                .to_canonical();
            if ip.is_unspecified()
                || ip.is_multicast()
                || matches!(ip, IpAddr::V4(ip) if ip.is_broadcast())
            {
                return Err(invalid());
            }
            if !allowed_peers.contains(&ip) {
                allowed_peers.push(ip);
            }
        }
        Ok(Self {
            allowed_peers,
            ..Self::default()
        })
    }

    /// Explicit trust of an isolated network, never proof of caller identity.
    pub fn with_trusted_lan_control(mut self, value: Option<&str>) -> Result<Self, String> {
        self.trusted_lan_control = match value {
            None | Some("false") => false,
            Some("true") => true,
            _ => {
                return Err(format!(
                    "{AGENT_TRUSTED_LAN_CONTROL_ENV} must be exactly true or false; unset it for discovery only"
                ));
            }
        };
        Ok(self)
    }

    pub fn allows_remote_method(&self, method: &RpcMethod) -> bool {
        match method {
            // Host lifecycle controls remain local even with explicit LAN trust.
            RpcMethod::BeginAgentDrain | RpcMethod::ResumeAgentAdmission => false,
            RpcMethod::Ping | RpcMethod::DescribeAgent | RpcMethod::DescribeAgentLifecycle => true,
            _ => self.trusted_lan_control,
        }
    }

    pub fn connection_access(&self, peer_is_loopback: bool) -> AgentConnectionAccess {
        AgentConnectionAccess::new(peer_is_loopback, self.trusted_lan_control)
    }

    pub fn control_mode(&self) -> &'static str {
        if self.trusted_lan_control {
            "trusted_lan_control"
        } else {
            "discovery_only"
        }
    }

    pub fn allows_ip(&self, ip: IpAddr) -> bool {
        let ip = ip.to_canonical();
        ip.is_loopback() || self.allowed_peers.contains(&ip)
    }

    pub fn allowed_peer_ips(&self) -> &[IpAddr] {
        &self.allowed_peers
    }

    pub fn mode(&self) -> &'static str {
        if self.allowed_peers.is_empty() {
            "loopback_only"
        } else {
            "loopback_and_explicit_peer_ips"
        }
    }
}
