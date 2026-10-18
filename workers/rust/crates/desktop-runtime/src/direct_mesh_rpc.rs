use std::collections::VecDeque;
#[cfg(test)]
use std::io::Write;
#[cfg(test)]
use std::net::TcpStream;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use kyuubiki_protocol::{
    RpcProgress, RpcResponse, validate_rpc_progress_envelope, validate_rpc_response_envelope,
};

use serde::Serialize;
use serde_json::{Value, json};

const RPC_VERSION: u64 = 1;
const RPC_TIMEOUT: Duration = Duration::from_secs(15);
const MAX_FRAME_BYTES: usize = 64 * 1024 * 1024;
const MAX_METADATA_BYTES: usize = 1024 * 1024;
const MAX_DURABLE_BYTES: usize = 9 * 1024 * 1024;
const MAX_AGENT_ERROR_CODE_BYTES: usize = 128;
static NEXT_REQUEST_ID: AtomicU64 = AtomicU64::new(1);
const MAX_PROGRESS_FRAMES: usize = 128;
const MAX_PROGRESS_BYTES: usize = 1024 * 1024;

#[path = "direct_mesh_selection.rs"]
mod selection;
#[path = "direct_mesh_transport.rs"]
mod transport;

pub(crate) fn selection_policy() -> Value {
    selection::policy()
}

pub(crate) fn transport_policy() -> Value {
    json!({"schema_version":"kyuubiki.direct-mesh-transport/v1",
        "control_request_timeout_ms":RPC_TIMEOUT.as_millis(),
        "solver_frame_timeout_ms":RPC_TIMEOUT.as_millis(), "solver_total_deadline":false,
        "max_pending_dns_lookups":transport::MAX_RESOLVERS,
        "max_resolved_addresses":transport::MAX_ADDRESSES,
        "connect_attempt_timeout_ms":transport::CONNECT_TIMEOUT.as_millis(),
        "dns_capacity_policy":"wait_within_request_deadline",
        "max_metadata_frame_bytes":MAX_METADATA_BYTES, "max_durable_frame_bytes":MAX_DURABLE_BYTES,
        "max_solver_frame_bytes":MAX_FRAME_BYTES,
        "max_retained_progress_frames":MAX_PROGRESS_FRAMES, "max_retained_progress_bytes":MAX_PROGRESS_BYTES,
        "progress_retention":"most_recent_bounded_window", "control_progress_allowed":false,
        "automatic_retry":false,
        "durable_submission":{"identity_binding":["execution_id", "method", "request_digest"],
            "digest_encoding":kyuubiki_protocol::TASK_DIGEST_TYPED_V2,
            "historical_receipt_identity_allowed":true, "lookup_absence_is_admission":false},
        "error_diagnostics":{"agent_code_field":"agent_error_code",
            "max_agent_code_bytes":MAX_AGENT_ERROR_CODE_BYTES,
            "agent_details_forwarded":false, "is_execution_or_owner_evidence":false}})
}

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

#[derive(Debug)]
pub(crate) struct RequestError {
    pub message: String,
    pub agent_code: Option<String>,
}

impl From<String> for RequestError {
    fn from(message: String) -> Self {
        Self {
            message,
            agent_code: None,
        }
    }
}

impl From<&str> for RequestError {
    fn from(message: &str) -> Self {
        message.to_owned().into()
    }
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
    if !matches!(selection_mode, "first_reachable" | "healthiest") {
        return Err("invalid direct mesh selection mode; no task dispatched".into());
    }
    let mut agents = Vec::new();
    let mut rejected = Vec::new();
    for agent in describe_agents(endpoints) {
        let eligibility = if agent.descriptor_error.is_some() {
            Err("descriptor_unavailable")
        } else {
            agent
                .descriptor
                .as_ref()
                .ok_or("descriptor_missing")
                .and_then(|descriptor| selection::eligible_score(descriptor, method))
        };
        match eligibility {
            Ok(score) => agents.push((agent, score)),
            Err(reason) => rejected.push(format!("{}:{}={reason}", agent.host, agent.port)),
        }
    }
    if selection_mode == "healthiest" {
        agents.sort_by_key(|(_, score)| std::cmp::Reverse(*score));
    }
    let (selected, _) = agents.first().ok_or_else(|| {
        format!(
            "no eligible direct mesh agents; no task dispatched; {}",
            rejected.join(", ")
        )
    })?;
    let endpoint = format!("{}:{}", selected.host, selected.port);
    // Selection is read-only. Once dispatch begins, a lost reply cannot authorize migration.
    let (result, progress_frames) = request(&endpoint, method, params).map_err(|error| {
        format!("direct mesh dispatch at {endpoint} failed: {error}; execution outcome may be unknown; automatic cross-agent replay disabled")
    })?;
    Ok(SolveResult {
        endpoint,
        strategy: selection_mode.to_string(),
        result,
        progress_frames,
    })
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

pub(crate) fn request(
    endpoint: &str,
    method: &str,
    params: Value,
) -> Result<(Value, Vec<Value>), String> {
    request_detailed(endpoint, method, params).map_err(|error| error.message)
}

pub(crate) fn request_detailed(
    endpoint: &str,
    method: &str,
    params: Value,
) -> Result<(Value, Vec<Value>), RequestError> {
    exchange_with_timeout(endpoint, method, params, RPC_TIMEOUT)
}

#[cfg(test)]
fn request_with_timeout(
    endpoint: &str,
    method: &str,
    params: Value,
    timeout: Duration,
) -> Result<(Value, Vec<Value>), String> {
    exchange_with_timeout(endpoint, method, params, timeout).map_err(|error| error.message)
}

fn exchange_with_timeout(
    endpoint: &str,
    method: &str,
    params: Value,
    timeout: Duration,
) -> Result<(Value, Vec<Value>), RequestError> {
    let deadline = Instant::now() + timeout;
    let parsed = parse_endpoint(endpoint)?;
    let solver = method.starts_with("solve_");
    let max_reply = match method {
        "describe_agent" | "list_durable_agent_executions" | "cancel_durable_agent_execution" => {
            MAX_METADATA_BYTES
        }
        "submit_durable_agent_task" | "fetch_durable_agent_execution" => MAX_DURABLE_BYTES,
        _ => MAX_FRAME_BYTES,
    };
    let id = request_id();
    let payload = serde_json::to_vec(&json!({
        "rpc_version": RPC_VERSION,
        "id": id,
        "method": method,
        "params": params,
    }))
    .map_err(|error| format!("failed to encode direct mesh request: {error}"))?;
    if payload.len() > MAX_FRAME_BYTES {
        return Err("direct mesh request exceeds 64 MiB".into());
    }
    let submission = if method == "submit_durable_agent_task" {
        Some(crate::direct_mesh_admission::SubmissionIdentity::from_params(&params)?)
    } else {
        None
    };
    let mut stream = transport::connect(&parsed.endpoint, deadline)?;
    transport::write_before(&mut stream, &(payload.len() as u32).to_be_bytes(), deadline)
        .and_then(|_| transport::write_before(&mut stream, &payload, deadline))
        .map_err(|error| format!("failed to write direct mesh request to {endpoint}: {error}"))?;

    let mut progress = VecDeque::<(Value, usize)>::new();
    let mut progress_bytes = 0;
    loop {
        // Legacy solves may legitimately emit progress for longer than the control deadline.
        // Only a complete frame renews that budget; partial bytes can never keep it alive.
        let frame_deadline = if solver {
            Instant::now() + timeout
        } else {
            deadline
        };
        let (frame, bytes) = transport::read_frame(&mut stream, frame_deadline, max_reply)?;
        if frame.get("id").and_then(Value::as_str) != Some(id.as_str()) {
            return Err(
                format!("direct mesh agent {endpoint} returned a mismatched request id").into(),
            );
        }
        if frame.get("event").is_some() {
            if !solver
                || bytes > MAX_PROGRESS_BYTES
                || ["ok", "result", "error"]
                    .iter()
                    .any(|key| frame.get(key).is_some())
            {
                return Err("unexpected or oversized direct mesh progress frame".into());
            }
            let event: RpcProgress = serde_json::from_value(frame).map_err(|e| e.to_string())?;
            validate_rpc_progress_envelope(&event).map_err(|e| e.message)?;
            let frame = serde_json::to_value(event).map_err(|e| e.to_string())?;
            let bytes = serde_json::to_vec(&frame).map_err(|e| e.to_string())?.len();
            if bytes > MAX_PROGRESS_BYTES {
                return Err("normalized direct mesh progress exceeds limit".into());
            }
            while progress.len() >= MAX_PROGRESS_FRAMES
                || progress_bytes + bytes > MAX_PROGRESS_BYTES
            {
                if let Some((_, size)) = progress.pop_front() {
                    progress_bytes -= size;
                }
            }
            progress_bytes += bytes;
            progress.push_back((frame, bytes));
            continue;
        }
        let response: RpcResponse = serde_json::from_value(frame).map_err(|e| e.to_string())?;
        validate_rpc_response_envelope(&response).map_err(|e| e.message)?;
        if response.ok {
            let result = response.result.ok_or("missing direct mesh result")?;
            if let Some(submission) = &submission {
                submission.validate(&result)?;
            }
            if crate::direct_mesh_admission::is_rejection(&result) {
                crate::direct_mesh_admission::validate_rejection(&result, method, &params, &id)?;
            }
            transport::remaining(frame_deadline).map_err(|e| e.to_string())?;
            return Ok((
                result,
                progress.into_iter().map(|(frame, _)| frame).collect(),
            ));
        }
        transport::remaining(frame_deadline).map_err(|e| e.to_string())?;
        let error = response.error.ok_or("missing direct mesh failure")?;
        // Preserve bounded diagnostic codes, never peer-provided retry/ownership instructions.
        let agent_code = (!error.code.is_empty()
            && error.code.len() <= MAX_AGENT_ERROR_CODE_BYTES
            && error
                .code
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-' | b'.')))
        .then_some(error.code);
        return Err(RequestError {
            message: error.message,
            agent_code,
        });
    }
}

#[cfg(test)]
fn read_frame(stream: &mut TcpStream, _endpoint: &str) -> Result<Value, String> {
    transport::read_frame(
        stream,
        Instant::now() + Duration::from_secs(2),
        MAX_FRAME_BYTES,
    )
    .map(|(frame, _)| frame)
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

fn request_id() -> String {
    let epoch = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let sequence = NEXT_REQUEST_ID.fetch_add(1, Ordering::Relaxed);
    format!("native-{epoch:x}-{sequence:x}")
}

#[cfg(test)]
#[path = "direct_mesh_dispatch_tests.rs"]
mod dispatch_tests;

#[cfg(test)]
#[path = "direct_mesh_transport_tests.rs"]
mod transport_tests;

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
