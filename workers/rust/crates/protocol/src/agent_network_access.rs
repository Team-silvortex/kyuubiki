use std::net::IpAddr;

pub const AGENT_ALLOWED_PEER_IPS_ENV: &str = "KYUUBIKI_AGENT_ALLOWED_PEER_IPS";
pub const AGENT_NETWORK_MAX_PEERS: usize = 64;
pub const AGENT_NETWORK_MAX_CONFIG_BYTES: usize = 4096;

/// Source-IP containment only, not peer identity or controller authorization.
/// Parsing performs no DNS, environment, filesystem or network operations.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AgentNetworkAccessPolicy {
    allowed_peers: Vec<IpAddr>,
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
        Ok(Self { allowed_peers })
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
