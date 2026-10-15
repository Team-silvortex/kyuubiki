use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use kyuubiki_protocol::{AgentDescriptor, ClusterPeerDescriptor};
#[cfg(test)]
use kyuubiki_protocol::{RpcRequest, RpcResponse};

use crate::agent_control_link;
use crate::agent_http::{
    cluster_auth_headers, delete_request, normalize_base_url, post_json, with_agent_session_header,
};
use crate::agent_state::{registration_payload_for_session, runtime_descriptor};
use crate::config::AgentConfig;
#[cfg(test)]
use crate::transport::{frame_error_message, read_frame, write_frame};

#[path = "agent_peer_rpc.rs"]
mod peer_rpc;
#[cfg(test)]
use peer_rpc::request_agent_descriptor;

#[path = "agent_mesh_candidates.rs"]
mod candidates;
use candidates::CandidatePool;

#[path = "agent_mesh_probe.rs"]
mod probe;

const MAX_PEERS: usize = 64;
const PROBES_PER_ROUND: usize = 8;

const MIN_REGISTER_INTERVAL_MS: u64 = 100;
const MAX_REGISTER_INTERVAL_MS: u64 = 300_000;
static AGENT_SESSION_COUNTER: AtomicU64 = AtomicU64::new(0);

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
        let interval_ms = normalized_registration_interval_ms(config.register_interval_ms);
        let cluster_api_token = config.cluster_api_token.clone();
        let agent_fingerprint = config.agent_fingerprint.clone();
        let running = Arc::new(AtomicBool::new(true));
        let running_clone = Arc::clone(&running);
        let cluster_id = config.cluster_id.clone();
        let payload_config = config.clone();
        let orchestrator_url_clone = orchestrator_url.clone();
        let agent_id_clone = agent_id;
        let agent_session_id = next_agent_session_id();
        agent_control_link::configure(interval_ms);

        let join_handle = thread::spawn(move || {
            let base_url = normalize_base_url(&orchestrator_url_clone);
            let mut registered = false;
            while running_clone.load(Ordering::SeqCst) {
                let operation = if registered { "heartbeat" } else { "register" };
                let url = if registered {
                    format!("{base_url}/api/v1/agents/{agent_id_clone}/heartbeat")
                } else {
                    format!("{base_url}/api/v1/agents/register")
                };
                agent_control_link::record_attempt(operation);
                let result = post_json(
                    &url,
                    &registration_payload_for_session(&payload_config, &agent_session_id),
                    with_agent_session_header(
                        cluster_auth_headers(
                            cluster_api_token.as_deref(),
                            &agent_id_clone,
                            cluster_id.as_deref(),
                            agent_fingerprint.as_deref(),
                        ),
                        &agent_session_id,
                    ),
                );

                let retry_delay_ms = match result {
                    Ok(()) => {
                        let snapshot = agent_control_link::record_success(operation, interval_ms);
                        if !registered && snapshot.attempt_count > 1 {
                            eprintln!(
                                "agent control link recovered: registrations={} attempts={}",
                                snapshot.successful_registration_count, snapshot.attempt_count
                            );
                        }
                        registered = true;
                        if operation == "register" {
                            0
                        } else {
                            interval_ms
                        }
                    }
                    Err(error) => {
                        let snapshot =
                            agent_control_link::record_failure(operation, &error, interval_ms);
                        eprintln!(
                            "agent control link degraded: operation={} code={} failures={} retry_ms={}",
                            operation,
                            snapshot.last_failure_code.as_deref().unwrap_or("unknown"),
                            snapshot.consecutive_failure_count,
                            snapshot.next_retry_delay_ms
                        );
                        registered = false;
                        snapshot.next_retry_delay_ms
                    }
                };

                if !sleep_while_running(&running_clone, retry_delay_ms) {
                    break;
                }
            }

            let unregister = delete_request(
                &format!("{base_url}/api/v1/agents/{agent_id_clone}"),
                with_agent_session_header(
                    cluster_auth_headers(
                        cluster_api_token.as_deref(),
                        &agent_id_clone,
                        cluster_id.as_deref(),
                        agent_fingerprint.as_deref(),
                    ),
                    &agent_session_id,
                ),
            );
            agent_control_link::record_stopped(unregister.as_ref().err().map(String::as_str));
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

fn next_agent_session_id() -> String {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    let counter = AGENT_SESSION_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("agent-session-{}-{nonce}-{counter}", std::process::id())
}

fn sleep_while_running(running: &AtomicBool, delay_ms: u64) -> bool {
    let mut remaining = delay_ms;
    while running.load(Ordering::SeqCst) && remaining > 0 {
        let chunk = remaining.min(100);
        thread::sleep(Duration::from_millis(chunk));
        remaining -= chunk;
    }
    running.load(Ordering::SeqCst)
}

fn normalized_registration_interval_ms(configured: u64) -> u64 {
    configured.clamp(MIN_REGISTER_INTERVAL_MS, MAX_REGISTER_INTERVAL_MS)
}

impl PeerMeshHandle {
    pub(crate) fn maybe_spawn(config: &AgentConfig) -> Result<Option<Self>, String> {
        validate_mesh_config(config)?;
        if config.peers.is_empty() {
            return Ok(None);
        }

        let running = Arc::new(AtomicBool::new(true));
        let running_clone = Arc::clone(&running);
        let seed_peers = configured_peers(config);
        let self_addresses = self_addresses(config);
        let cluster_id = config.cluster_id.clone();
        let sync_interval_ms = config
            .register_interval_ms
            .clamp(1_000, MAX_REGISTER_INTERVAL_MS);

        let join_handle = thread::Builder::new()
            .name("agent-peer-mesh".into())
            .spawn(move || {
                let mut candidates = CandidatePool::new(seed_peers, self_addresses);

                while running_clone.load(Ordering::SeqCst) {
                    let round = candidates.next_round();
                    probe::probe_round(round, &running_clone, |peer, result| {
                        if let Ok(descriptor) = result
                            && compatible_peer(&descriptor, cluster_id.as_deref())
                        {
                            candidates.record_success(
                                &peer,
                                descriptor.runtime.peers,
                                Instant::now(),
                                unix_now_s(),
                            );
                        } else {
                            candidates.record_failure(&peer);
                        }
                        update_runtime_mesh(
                            cluster_id.clone(),
                            candidates.finish_round(Instant::now()),
                        );
                    });

                    if running_clone.load(Ordering::SeqCst) {
                        update_runtime_mesh(
                            cluster_id.clone(),
                            candidates.finish_round(Instant::now()),
                        );
                    }

                    if !sleep_while_running(&running_clone, sync_interval_ms) {
                        break;
                    }
                }
            })
            .map_err(|error| format!("failed to spawn peer mesh: {error}"))?;

        Ok(Some(Self {
            running,
            join_handle: Some(join_handle),
        }))
    }

    pub(crate) fn stop(mut self) {
        self.running.store(false, Ordering::SeqCst);

        if let Some(join_handle) = self.join_handle.take() {
            let _ = join_handle.join();
        }
    }
}

pub(crate) fn validate_mesh_config(config: &AgentConfig) -> Result<(), String> {
    if config.peers.is_empty() {
        return Ok(());
    }
    if config.orchestrator_url.is_some() {
        return Err("peer mesh and Orchestra binding are mutually exclusive".into());
    }
    if config.peers.len() > MAX_PEERS {
        return Err("peer mesh supports at most 64 configured peers".into());
    }
    if config.cluster_id.as_ref().is_some_and(|id| {
        id.trim().is_empty() || id.len() > 256 || id.chars().any(char::is_control)
    }) {
        return Err(
            "mesh cluster identity must be nonblank, control-free and at most 256 bytes".into(),
        );
    }
    for peer in &config.peers {
        peer_rpc::normalize_endpoint(peer)?;
    }
    Ok(())
}

pub(crate) fn configured_peers(config: &AgentConfig) -> Vec<String> {
    filter_self_peers(
        normalize_peer_addresses(config.peers.clone()),
        &self_addresses(config),
    )
}

fn compatible_peer(descriptor: &AgentDescriptor, cluster_id: Option<&str>) -> bool {
    descriptor.runtime.cluster_id.as_deref() == cluster_id
        && descriptor.runtime.runtime_mode == "peer_mesh"
        && descriptor.authority.control_mode == "offline_mesh"
        && descriptor.authority.authority_mode == "offline_mesh"
        && descriptor.authority.orchestrator_id.is_none()
        && descriptor.authority.orchestrator_session_id.is_none()
        && !descriptor.authority.accepts_multi_orchestrator_binding
        && descriptor.runtime.peers.len() <= MAX_PEERS
}

pub(crate) fn peer_mesh_policy() -> serde_json::Value {
    serde_json::json!({"schema_version":"kyuubiki.agent-peer-discovery/v1",
        "max_known_peers":MAX_PEERS, "probes_per_round":PROBES_PER_ROUND,
        "max_concurrent_probes":probe::MAX_CONCURRENT_PROBES,
        "state_publication":"after_each_probe_completion",
        "probe_worker_failure":"remaining_worker_or_bounded_serial_fallback",
        "shutdown_probes":"stop_queue_and_join_in_flight",
        "overlapping_probe_rounds":false,
        "probe_order":"retained_fifo_new_candidates_at_tail",
        "max_probe_rounds_for_retained_peer":MAX_PEERS.div_ceil(PROBES_PER_ROUND),
        "probe_round_bound_scope":"complete_running_rounds_not_wall_time",
        "configured_seed_retention":"until_reconfiguration",
        "learned_peer_failure_threshold":candidates::FAILURE_THRESHOLD,
        "learned_peer_cooldown_ms":candidates::COOLDOWN_MS,
        "max_retired_peer_cooldowns":candidates::MAX_RETIRED,
        "retirement_when_cooldown_full":"retain_failed_candidate",
        "candidate_retirement_scope":"discovery_only",
        "cooldown_clock":"process_monotonic",
        "request_timeout_ms":peer_rpc::REQUEST_TIMEOUT_MS, "max_frame_bytes":peer_rpc::MAX_FRAME_BYTES,
        "max_pending_dns_lookups":peer_rpc::MAX_RESOLVERS, "cluster_match":"exact_including_unset",
        "authority":"offline_mesh_only", "trusted_network_required":true,
        "cluster_id_is_authentication":false, "hint_is_execution_authorization":false,
        "task_replication":false, "automatic_cross_peer_replay":false,
        "cluster_size":"self_plus_known_candidates_not_live_quorum",
        "health_score_scope":"retained_candidates_not_task_recovery",
        "topology_persistence":"process_local_rebuilt_from_configured_seeds"})
}

fn self_addresses(config: &AgentConfig) -> Vec<String> {
    let advertise_host = config
        .advertise_host
        .clone()
        .unwrap_or_else(|| config.host.clone());

    let endpoint = |host: &str| match host.parse::<std::net::IpAddr>() {
        Ok(ip) => std::net::SocketAddr::new(ip, config.port).to_string(),
        Err(_) => format!("{host}:{}", config.port),
    };
    let mut addresses = vec![endpoint(&config.host), endpoint(&advertise_host)];
    if config.host == "0.0.0.0" || config.host == "::" {
        addresses.extend([
            format!("127.0.0.1:{}", config.port),
            format!("[::1]:{}", config.port),
        ]);
    }
    normalize_peer_addresses(addresses)
}

pub(crate) fn normalize_peer_addresses(peers: Vec<String>) -> Vec<String> {
    let mut normalized = peers
        .into_iter()
        .filter_map(|peer| peer_rpc::normalize_endpoint(&peer).ok())
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
    if let Ok(mut current) = runtime_descriptor().lock()
        && current.runtime.runtime_mode == "peer_mesh"
        && current.runtime.cluster_id == cluster_id
    {
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

#[cfg(test)]
#[path = "tests/agent_mesh_reliability.rs"]
mod reliability_tests;

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{ErrorKind, Read, Write};
    use std::net::{TcpListener, TcpStream};
    use std::sync::Mutex;
    use std::time::Instant;

    #[test]
    fn heartbeat_failure_falls_back_to_registration_and_recovers() {
        let _guard = agent_control_link::test_guard();
        let listener = TcpListener::bind("127.0.0.1:0").expect("fake Orchestra listener");
        listener
            .set_nonblocking(true)
            .expect("nonblocking listener");
        let address = listener.local_addr().expect("listener address");
        let requests = Arc::new(Mutex::new(Vec::new()));
        let server_requests = Arc::clone(&requests);

        let server = thread::spawn(move || {
            let deadline = Instant::now() + Duration::from_secs(5);
            let mut heartbeat_rejected = false;
            while Instant::now() < deadline {
                let (mut stream, _) = match listener.accept() {
                    Ok(connection) => connection,
                    Err(error) if error.kind() == ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(5));
                        continue;
                    }
                    Err(error) => panic!("fake Orchestra accept failed: {error}"),
                };
                stream
                    .set_nonblocking(false)
                    .expect("blocking HTTP request stream");
                let _ = stream.set_read_timeout(Some(Duration::from_secs(1)));
                let request = read_http_request(&mut stream);
                let first_line = request.lines().next().unwrap_or_default().to_string();
                server_requests.lock().expect("request log").push(request);
                let reject = first_line.contains("/heartbeat") && !heartbeat_rejected;
                heartbeat_rejected |= reject;
                let status = if reject {
                    "503 Service Unavailable"
                } else {
                    "200 OK"
                };
                write!(
                    stream,
                    "HTTP/1.1 {status}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                )
                .expect("HTTP response");
                if first_line.starts_with("DELETE ") {
                    return;
                }
            }
            panic!("fake Orchestra did not receive unregister before timeout");
        });

        let args = vec![
            "--agent-id".to_string(),
            "rejoin-agent".to_string(),
            "--orchestrator-url".to_string(),
            format!("http://{address}"),
            "--register-interval-ms".to_string(),
            "20".to_string(),
        ];
        let config = AgentConfig::from_args(&args);
        let handle = AgentRegistrationHandle::maybe_spawn(&config).expect("registration handle");
        let deadline = Instant::now() + Duration::from_secs(3);
        while {
            let snapshot = agent_control_link::snapshot();
            (snapshot.successful_registration_count < 2 || snapshot.successful_heartbeat_count < 1)
                && Instant::now() < deadline
        } {
            thread::sleep(Duration::from_millis(10));
        }
        let recovered = agent_control_link::snapshot();
        handle.stop();
        server.join().expect("fake Orchestra thread");

        assert_eq!(recovered.state, "registered");
        assert_eq!(recovered.successful_registration_count, 2);
        assert_eq!(recovered.successful_heartbeat_count, 1);
        assert_eq!(recovered.consecutive_failure_count, 0);
        assert_eq!(recovered.next_retry_delay_ms, MIN_REGISTER_INTERVAL_MS);
        let requests = requests.lock().expect("request log");
        assert!(
            requests
                .first()
                .is_some_and(|line| line.contains("/register"))
        );
        assert!(requests.iter().any(|line| line.contains("/heartbeat")));
        assert!(
            requests
                .windows(2)
                .any(|window| window[0].contains("/heartbeat") && window[1].contains("/register"))
        );
        assert!(
            requests
                .last()
                .is_some_and(|line| line.starts_with("DELETE "))
        );

        let session_header = request_header(&requests[0], "x-kyuubiki-agent-session-id")
            .expect("registration session header");
        assert!(requests.iter().all(|request| {
            request_header(request, "x-kyuubiki-agent-session-id") == Some(session_header)
        }));
        assert!(
            requests
                .iter()
                .filter(|request| request.starts_with("POST "))
                .all(|request| request
                    .contains(&format!("\"agent_session_id\":\"{session_header}\"")))
        );
    }

    #[test]
    fn registration_interval_is_bounded() {
        assert_eq!(normalized_registration_interval_ms(0), 100);
        assert_eq!(normalized_registration_interval_ms(5_000), 5_000);
        assert_eq!(normalized_registration_interval_ms(u64::MAX), 300_000);
    }

    #[test]
    fn agent_process_sessions_are_unique_and_header_safe() {
        let first = next_agent_session_id();
        let second = next_agent_session_id();

        assert_ne!(first, second);
        assert!(first.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':')
        }));
    }

    fn read_http_request(stream: &mut TcpStream) -> String {
        let mut request = Vec::new();
        while !complete_http_request(&request) {
            let mut chunk = [0_u8; 4096];
            let count = stream.read(&mut chunk).expect("HTTP request bytes");
            assert!(count > 0, "HTTP request ended before its declared body");
            request.extend_from_slice(&chunk[..count]);
        }
        String::from_utf8(request).expect("UTF-8 HTTP request")
    }

    fn complete_http_request(request: &[u8]) -> bool {
        let Some(header_end) = request.windows(4).position(|window| window == b"\r\n\r\n") else {
            return false;
        };
        let headers = String::from_utf8_lossy(&request[..header_end]);
        let content_length = headers
            .lines()
            .find_map(|line| {
                let (name, value) = line.split_once(':')?;
                name.eq_ignore_ascii_case("content-length")
                    .then(|| value.trim().parse::<usize>().ok())
                    .flatten()
            })
            .unwrap_or(0);
        request.len() >= header_end + 4 + content_length
    }

    fn request_header<'a>(request: &'a str, expected_name: &str) -> Option<&'a str> {
        request.lines().find_map(|line| {
            let (name, value) = line.split_once(':')?;
            name.eq_ignore_ascii_case(expected_name)
                .then_some(value.trim())
        })
    }
}
