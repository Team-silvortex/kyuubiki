use std::collections::HashMap;
use std::net::TcpStream;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use kyuubiki_protocol::{
    AgentDescriptor, ClusterPeerDescriptor, RPC_VERSION, RpcMethod, RpcRequest, RpcResponse,
};

use crate::agent_http::{cluster_auth_headers, delete_request, normalize_base_url, post_json};
use crate::agent_state::{registration_payload, runtime_descriptor};
use crate::config::AgentConfig;
use crate::transport::{frame_error_message, read_frame, write_frame};

pub(crate) struct AgentRegistrationHandle {
    running: Arc<AtomicBool>,
    join_handle: Option<thread::JoinHandle<()>>,
}

pub(crate) struct PeerMeshHandle {
    running: Arc<AtomicBool>,
    join_handle: Option<thread::JoinHandle<()>>,
}

impl AgentRegistrationHandle {
    pub(crate) fn maybe_spawn(config: &AgentConfig) -> Option<Self> {
        let agent_id = config.agent_id.clone()?;
        let orchestrator_url = config.orchestrator_url.clone()?;
        let interval_ms = config.register_interval_ms;
        let cluster_api_token = config.cluster_api_token.clone();
        let agent_fingerprint = config.agent_fingerprint.clone();
        let running = Arc::new(AtomicBool::new(true));
        let running_clone = Arc::clone(&running);
        let cluster_id = config.cluster_id.clone();
        let initial_payload = registration_payload(config);
        let orchestrator_url_clone = orchestrator_url.clone();
        let agent_id_clone = agent_id.clone();

        let join_handle = thread::spawn(move || {
            let _ = post_json(
                &format!(
                    "{}/api/v1/agents/register",
                    normalize_base_url(&orchestrator_url_clone)
                ),
                &initial_payload,
                cluster_auth_headers(
                    cluster_api_token.as_deref(),
                    &agent_id,
                    cluster_id.as_deref(),
                    agent_fingerprint.as_deref(),
                ),
            );

            while running_clone.load(Ordering::SeqCst) {
                thread::sleep(Duration::from_millis(interval_ms));

                if !running_clone.load(Ordering::SeqCst) {
                    break;
                }

                let _ = post_json(
                    &format!(
                        "{}/api/v1/agents/{}/heartbeat",
                        normalize_base_url(&orchestrator_url_clone),
                        agent_id_clone
                    ),
                    &initial_payload,
                    cluster_auth_headers(
                        cluster_api_token.as_deref(),
                        &agent_id_clone,
                        cluster_id.as_deref(),
                        agent_fingerprint.as_deref(),
                    ),
                );
            }

            let _ = delete_request(
                &format!(
                    "{}/api/v1/agents/{}",
                    normalize_base_url(&orchestrator_url_clone),
                    agent_id_clone
                ),
                cluster_auth_headers(
                    cluster_api_token.as_deref(),
                    &agent_id_clone,
                    cluster_id.as_deref(),
                    agent_fingerprint.as_deref(),
                ),
            );
        });

        Some(Self {
            running,
            join_handle: Some(join_handle),
        })
    }

    pub(crate) fn stop(mut self) {
        self.running.store(false, Ordering::SeqCst);

        if let Some(join_handle) = self.join_handle.take() {
            let _ = join_handle.join();
        }
    }
}

impl PeerMeshHandle {
    pub(crate) fn maybe_spawn(config: &AgentConfig) -> Option<Self> {
        if config.peers.is_empty() {
            return None;
        }

        let running = Arc::new(AtomicBool::new(true));
        let running_clone = Arc::clone(&running);
        let seed_peers = normalize_peer_addresses(config.peers.clone());
        let self_addresses = self_addresses(config);
        let cluster_id = config.cluster_id.clone();
        let sync_interval_ms = config.register_interval_ms.max(1_000);

        let join_handle = thread::spawn(move || {
            let mut known_peers = seed_peers;
            let mut peer_failures: HashMap<String, u32> = HashMap::new();
            let mut peer_last_seen: HashMap<String, u64> = HashMap::new();

            while running_clone.load(Ordering::SeqCst) {
                let mut discovered = known_peers.clone();

                for peer in known_peers.clone() {
                    if let Ok(descriptor) = request_agent_descriptor(&peer) {
                        discovered.extend(
                            descriptor
                                .runtime
                                .peers
                                .into_iter()
                                .map(|peer| peer.address),
                        );
                        peer_failures.insert(peer.clone(), 0);
                        peer_last_seen.insert(peer, unix_now_s());
                    } else {
                        let failure_count = peer_failures.entry(peer).or_insert(0);
                        *failure_count += 1;
                    }
                }

                known_peers =
                    filter_self_peers(normalize_peer_addresses(discovered), &self_addresses);
                update_runtime_mesh(
                    cluster_id.clone(),
                    build_peer_descriptors(&known_peers, &peer_failures, &peer_last_seen),
                );

                thread::sleep(Duration::from_millis(sync_interval_ms));
            }
        });

        Some(Self {
            running,
            join_handle: Some(join_handle),
        })
    }

    pub(crate) fn stop(mut self) {
        self.running.store(false, Ordering::SeqCst);

        if let Some(join_handle) = self.join_handle.take() {
            let _ = join_handle.join();
        }
    }
}

fn self_addresses(config: &AgentConfig) -> Vec<String> {
    let advertise_host = config
        .advertise_host
        .clone()
        .unwrap_or_else(|| config.host.clone());

    normalize_peer_addresses(vec![
        format!("{}:{}", config.host, config.port),
        format!("{}:{}", advertise_host, config.port),
    ])
}

pub(crate) fn normalize_peer_addresses(peers: Vec<String>) -> Vec<String> {
    let mut normalized = peers
        .into_iter()
        .map(|peer| peer.trim().to_string())
        .filter(|peer| !peer.is_empty())
        .collect::<Vec<_>>();
    normalized.sort();
    normalized.dedup();
    normalized
}

pub(crate) fn filter_self_peers(peers: Vec<String>, self_addresses: &[String]) -> Vec<String> {
    peers
        .into_iter()
        .filter(|peer| {
            !self_addresses
                .iter()
                .any(|self_address| self_address == peer)
        })
        .collect()
}

pub(crate) fn build_peer_descriptors(
    peers: &[String],
    failures: &HashMap<String, u32>,
    last_seen: &HashMap<String, u64>,
) -> Vec<ClusterPeerDescriptor> {
    peers
        .iter()
        .cloned()
        .map(|address| {
            let failure_count = failures.get(&address).copied().unwrap_or(0);
            let status = if last_seen.contains_key(&address) && failure_count == 0 {
                "healthy"
            } else if last_seen.contains_key(&address) {
                "degraded"
            } else {
                "unreachable"
            };

            ClusterPeerDescriptor {
                address: address.clone(),
                status: status.to_string(),
                failure_count,
                last_seen_unix_s: last_seen.get(&address).copied(),
            }
        })
        .collect()
}

fn update_runtime_mesh(cluster_id: Option<String>, peers: Vec<ClusterPeerDescriptor>) {
    if let Ok(mut current) = runtime_descriptor().lock() {
        current.runtime.cluster_id = cluster_id;
        current.runtime.runtime_mode = if peers.is_empty() {
            "standalone".to_string()
        } else {
            "peer_mesh".to_string()
        };
        current.runtime.headless = true;
        current.runtime.cluster_size = 1 + peers.len();
        current.runtime.health_score = compute_cluster_health_score(&peers);
        current.runtime.peers = peers;
    }
}

pub(crate) fn compute_cluster_health_score(peers: &[ClusterPeerDescriptor]) -> u8 {
    if peers.is_empty() {
        return 100;
    }

    let total = peers.len() as f32;
    let healthy = peers.iter().filter(|peer| peer.status == "healthy").count() as f32;
    let degraded = peers
        .iter()
        .filter(|peer| peer.status == "degraded")
        .count() as f32;
    let score = ((healthy + degraded * 0.5) / total) * 100.0;
    score.round().clamp(0.0, 100.0) as u8
}

fn unix_now_s() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

fn request_agent_descriptor(address: &str) -> Result<AgentDescriptor, String> {
    let mut stream = TcpStream::connect(address)
        .map_err(|error| format!("failed to connect to peer {address}: {error}"))?;
    let _ = stream.set_read_timeout(Some(Duration::from_millis(1_500)));
    let _ = stream.set_write_timeout(Some(Duration::from_millis(1_500)));

    let request = RpcRequest {
        rpc_version: RPC_VERSION,
        id: "peer-describe".to_string(),
        method: RpcMethod::DescribeAgent,
        params: serde_json::json!({}),
    };

    let payload = serde_json::to_vec(&request)
        .map_err(|error| format!("failed to encode peer describe request: {error}"))?;
    write_frame(&mut stream, &payload)
        .map_err(|error| format!("failed to write peer request frame: {error}"))?;

    let response_payload = read_frame(&mut stream).map_err(|error| {
        format!(
            "failed to read peer response: {}",
            frame_error_message(error)
        )
    })?;

    let response: RpcResponse = serde_json::from_slice(&response_payload)
        .map_err(|error| format!("failed to decode peer response: {error}"))?;

    if !response.ok {
        let error = response
            .error
            .map(|error| format!("{}: {}", error.code, error.message))
            .unwrap_or_else(|| "unknown peer error".to_string());
        return Err(format!("peer describe failed: {error}"));
    }

    serde_json::from_value(response.result.unwrap_or_default())
        .map_err(|error| format!("failed to decode peer descriptor: {error}"))
}
