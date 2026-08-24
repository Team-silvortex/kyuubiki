use std::collections::HashMap;
use std::net::TcpStream;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use kyuubiki_protocol::{
    AgentDescriptor, ClusterPeerDescriptor, RPC_VERSION, RpcMethod, RpcRequest, RpcResponse,
};

use crate::agent_control_link;
use crate::agent_http::{cluster_auth_headers, delete_request, normalize_base_url, post_json};
use crate::agent_state::{registration_payload, runtime_descriptor};
use crate::config::AgentConfig;
use crate::transport::{frame_error_message, read_frame, write_frame};

const MIN_REGISTER_INTERVAL_MS: u64 = 100;
const MAX_REGISTER_INTERVAL_MS: u64 = 300_000;

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
                    &registration_payload(&payload_config),
                    cluster_auth_headers(
                        cluster_api_token.as_deref(),
                        &agent_id_clone,
                        cluster_id.as_deref(),
                        agent_fingerprint.as_deref(),
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
                cluster_auth_headers(
                    cluster_api_token.as_deref(),
                    &agent_id_clone,
                    cluster_id.as_deref(),
                    agent_fingerprint.as_deref(),
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{ErrorKind, Read, Write};
    use std::net::TcpListener;
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
                server_requests
                    .lock()
                    .expect("request log")
                    .push(first_line.clone());
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
    }

    #[test]
    fn registration_interval_is_bounded() {
        assert_eq!(normalized_registration_interval_ms(0), 100);
        assert_eq!(normalized_registration_interval_ms(5_000), 5_000);
        assert_eq!(normalized_registration_interval_ms(u64::MAX), 300_000);
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
}
