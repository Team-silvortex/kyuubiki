use std::io::{Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::Serialize;
use serde_json::{Value, json};

const RPC_VERSION: u64 = 1;
const RPC_TIMEOUT: Duration = Duration::from_secs(15);
const MAX_FRAME_BYTES: usize = 64 * 1024 * 1024;
static NEXT_REQUEST_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct DirectMeshEndpoint {
    pub endpoint: String,
    pub host: String,
    pub port: u16,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct AgentSummary {
    pub id: String,
    pub host: String,
    pub port: u16,
    pub role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub descriptor: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub descriptor_error: Option<String>,
}

#[derive(Clone, Debug)]
pub(crate) struct SolveResult {
    pub endpoint: String,
    pub strategy: String,
    pub result: Value,
    pub progress_frames: Vec<Value>,
}

pub(crate) fn parse_endpoint(value: &str) -> Result<DirectMeshEndpoint, String> {
    let normalized = value.trim();
    if normalized.is_empty()
        || normalized.len() > 255
        || normalized.contains("://")
        || normalized.contains(['/', '?', '#', '@'])
        || normalized.starts_with(':')
        || normalized.ends_with(':')
    {
        return Err(format!("invalid direct mesh endpoint: {value}"));
    }
    let (host, port) = normalized
        .split_once(':')
        .ok_or_else(|| format!("invalid direct mesh endpoint: {value}"))?;
    if port.contains(':') || !valid_host(host) {
        return Err(format!("invalid direct mesh endpoint host: {value}"));
    }
    if matches!(host, "*" | "0.0.0.0" | "::" | "[::]") {
        return Err(format!(
            "direct mesh endpoint must target a concrete host: {value}"
        ));
    }
    let port = port
        .parse::<u16>()
        .ok()
        .filter(|port| *port > 0)
        .ok_or_else(|| format!("invalid direct mesh endpoint: {value}"))?;
    Ok(DirectMeshEndpoint {
        endpoint: format!("{host}:{port}"),
        host: host.to_string(),
        port,
    })
}

pub(crate) fn describe_agents(endpoints: &[String]) -> Vec<AgentSummary> {
    let handles = endpoints
        .iter()
        .cloned()
        .map(|endpoint| std::thread::spawn(move || describe_agent(&endpoint)))
        .collect::<Vec<_>>();
    handles
        .into_iter()
        .map(|handle| {
            handle.join().unwrap_or_else(|_| AgentSummary {
                id: "direct-agent@unknown".to_string(),
                host: "unknown".to_string(),
                port: 0,
                role: "solver".to_string(),
                descriptor: None,
                descriptor_error: Some("direct mesh descriptor worker panicked".to_string()),
            })
        })
        .collect()
}

pub(crate) fn solve(
    method: &str,
    params: Value,
    endpoints: &[String],
    selection_mode: &str,
) -> Result<SolveResult, String> {
    let mut agents = describe_agents(endpoints)
        .into_iter()
        .filter(|agent| agent.descriptor_error.is_none())
        .collect::<Vec<_>>();
    if selection_mode == "healthiest" {
        agents.sort_by(|left, right| {
            health_score(right)
                .partial_cmp(&health_score(left))
                .unwrap_or(std::cmp::Ordering::Equal)
        });
    }
    let ordered = if agents.is_empty() {
        endpoints.to_vec()
    } else {
        agents
            .iter()
            .map(|agent| format!("{}:{}", agent.host, agent.port))
            .collect()
    };
    let mut last_error = None;
    for endpoint in ordered {
        match request(&endpoint, method, params.clone()) {
            Ok((result, progress_frames)) => {
                return Ok(SolveResult {
                    endpoint,
                    strategy: selection_mode.to_string(),
                    result,
                    progress_frames,
                });
            }
            Err(error) => last_error = Some(error),
        }
    }
    Err(last_error.unwrap_or_else(|| "no reachable direct mesh agents".to_string()))
}

fn describe_agent(endpoint: &str) -> AgentSummary {
    let parsed = match parse_endpoint(endpoint) {
        Ok(parsed) => parsed,
        Err(error) => {
            return AgentSummary {
                id: format!("direct-agent@{endpoint}"),
                host: endpoint.to_string(),
                port: 0,
                role: "solver".to_string(),
                descriptor: None,
                descriptor_error: Some(error),
            };
        }
    };
    match request(&parsed.endpoint, "describe_agent", json!({})) {
        Ok((descriptor, _)) => {
            let program = descriptor
                .get("program")
                .and_then(Value::as_str)
                .unwrap_or("direct-agent");
            AgentSummary {
                id: format!("{program}@{}", parsed.endpoint),
                host: parsed.host,
                port: parsed.port,
                role: "solver".to_string(),
                descriptor: Some(descriptor),
                descriptor_error: None,
            }
        }
        Err(error) => AgentSummary {
            id: format!("direct-agent@{}", parsed.endpoint),
            host: parsed.host,
            port: parsed.port,
            role: "solver".to_string(),
            descriptor: None,
            descriptor_error: Some(error),
        },
    }
}

fn request(endpoint: &str, method: &str, params: Value) -> Result<(Value, Vec<Value>), String> {
    let parsed = parse_endpoint(endpoint)?;
    let mut stream = connect(&parsed)?;
    stream
        .set_read_timeout(Some(RPC_TIMEOUT))
        .and_then(|_| stream.set_write_timeout(Some(RPC_TIMEOUT)))
        .map_err(|error| format!("failed to configure direct mesh socket: {error}"))?;
    let id = request_id();
    let payload = serde_json::to_vec(&json!({
        "rpc_version": RPC_VERSION,
        "id": id,
        "method": method,
        "params": params,
    }))
    .map_err(|error| format!("failed to encode direct mesh request: {error}"))?;
    if payload.len() > MAX_FRAME_BYTES {
        return Err("direct mesh request exceeds 64 MiB".to_string());
    }
    stream
        .write_all(&(payload.len() as u32).to_be_bytes())
        .and_then(|_| stream.write_all(&payload))
        .map_err(|error| format!("failed to write direct mesh request to {endpoint}: {error}"))?;

    let mut progress = Vec::new();
    loop {
        let frame = read_frame(&mut stream, endpoint)?;
        if frame.get("id").and_then(Value::as_str) != Some(id.as_str()) {
            return Err(format!(
                "direct mesh agent {endpoint} returned a mismatched request id"
            ));
        }
        if frame.get("event").is_some() {
            progress.push(frame);
            continue;
        }
        if frame.get("ok").and_then(Value::as_bool) == Some(true) {
            return Ok((
                frame.get("result").cloned().unwrap_or(Value::Null),
                progress,
            ));
        }
        let message = frame
            .pointer("/error/message")
            .and_then(Value::as_str)
            .unwrap_or("direct mesh rpc failed");
        return Err(message.to_string());
    }
}

fn read_frame(stream: &mut TcpStream, endpoint: &str) -> Result<Value, String> {
    let mut header = [0_u8; 4];
    stream
        .read_exact(&mut header)
        .map_err(|error| format!("failed to read direct mesh frame from {endpoint}: {error}"))?;
    let length = u32::from_be_bytes(header) as usize;
    if length == 0 || length > MAX_FRAME_BYTES {
        return Err(format!(
            "direct mesh agent {endpoint} returned an invalid frame length"
        ));
    }
    let mut payload = vec![0_u8; length];
    stream
        .read_exact(&mut payload)
        .map_err(|error| format!("failed to read direct mesh payload from {endpoint}: {error}"))?;
    serde_json::from_slice(&payload)
        .map_err(|error| format!("direct mesh agent {endpoint} returned invalid JSON: {error}"))
}

fn connect(endpoint: &DirectMeshEndpoint) -> Result<TcpStream, String> {
    let addresses = endpoint
        .endpoint
        .to_socket_addrs()
        .map_err(|error| format!("failed to resolve {}: {error}", endpoint.endpoint))?;
    let mut last_error = None;
    for address in addresses {
        match TcpStream::connect_timeout(&address, RPC_TIMEOUT) {
            Ok(stream) => return Ok(stream),
            Err(error) => last_error = Some(error),
        }
    }
    Err(format!(
        "failed to connect to direct mesh agent {}: {}",
        endpoint.endpoint,
        last_error
            .map(|error| error.to_string())
            .unwrap_or_else(|| "no address".to_string())
    ))
}

fn valid_host(host: &str) -> bool {
    host == "localhost" || valid_ipv4(host) || host.split('.').all(valid_hostname_label)
}

fn valid_ipv4(host: &str) -> bool {
    let segments = host.split('.').collect::<Vec<_>>();
    segments.len() == 4
        && segments.iter().all(|segment| {
            !segment.is_empty()
                && segment.len() <= 3
                && segment.bytes().all(|byte| byte.is_ascii_digit())
                && segment.parse::<u8>().is_ok()
        })
}

fn valid_hostname_label(label: &str) -> bool {
    !label.is_empty()
        && label.len() <= 63
        && label
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
        && label
            .as_bytes()
            .first()
            .is_some_and(u8::is_ascii_alphanumeric)
        && label
            .as_bytes()
            .last()
            .is_some_and(u8::is_ascii_alphanumeric)
}

fn health_score(agent: &AgentSummary) -> f64 {
    agent
        .descriptor
        .as_ref()
        .and_then(|value| value.pointer("/runtime/health_score"))
        .and_then(Value::as_f64)
        .unwrap_or(0.0)
}

fn request_id() -> String {
    let epoch = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let sequence = NEXT_REQUEST_ID.fetch_add(1, Ordering::Relaxed);
    format!("native-{epoch:x}-{sequence:x}")
}

#[cfg(test)]
mod tests {
    use super::parse_endpoint;

    #[test]
    fn endpoint_parser_rejects_wildcards_and_urls() {
        assert_eq!(parse_endpoint("127.0.0.1:5001").unwrap().port, 5001);
        assert!(parse_endpoint("http://127.0.0.1:5001").is_err());
        assert!(parse_endpoint("0.0.0.0:5001").is_err());
        assert!(parse_endpoint("host:0").is_err());
    }
}
