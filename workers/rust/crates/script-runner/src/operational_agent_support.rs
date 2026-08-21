use crate::remote_host::ssh_output;
use kyuubiki_protocol::{AgentDescriptor, RpcRequest, RpcResponse};
use std::fs;
use std::io::{Read, Write};
use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpListener, TcpStream};
use std::path::Path;
use std::str::FromStr;
use std::thread;
use std::time::{Duration, Instant};

type RunnerResult<T> = Result<T, String>;

const MAX_RPC_BYTES: usize = 4 * 1024 * 1024;

pub(crate) struct ConnectionProfile {
    pub(crate) local_ip: Ipv4Addr,
    pub(crate) remote_ip: Ipv4Addr,
    pub(crate) remote_architecture: String,
}

pub(crate) fn connection_profile(root: &Path, host: &str) -> RunnerResult<ConnectionProfile> {
    let output = ssh_output(
        root,
        host,
        "printf '%s\n' \"$SSH_CONNECTION\"; uname -s; uname -m".to_string(),
    )?;
    let mut lines = output.lines();
    let connection = lines
        .next()
        .ok_or("remote SSH connection metadata is missing")?
        .split_whitespace()
        .collect::<Vec<_>>();
    if connection.len() != 4 || lines.next() != Some("Linux") {
        return Err("qualification host must be Linux over a direct SSH connection".to_string());
    }
    let local_ip = parse_ipv4(connection[0], "local qualification host")?;
    let remote_ip = parse_ipv4(connection[2], "remote Agent host")?;
    let remote_architecture = lines
        .next()
        .filter(|value| {
            !value.is_empty()
                && value
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
        })
        .ok_or("remote architecture is invalid")?
        .to_string();
    Ok(ConnectionProfile {
        local_ip,
        remote_ip,
        remote_architecture,
    })
}

pub(crate) fn available_local_port() -> RunnerResult<u16> {
    TcpListener::bind((Ipv4Addr::UNSPECIFIED, 0))
        .and_then(|listener| listener.local_addr())
        .map(|address| address.port())
        .map_err(|error| format!("failed to reserve a local qualification port: {error}"))
}

pub(crate) fn query_agent_descriptor(address: SocketAddr) -> RunnerResult<AgentDescriptor> {
    serde_json::from_value(query_agent_descriptor_value(address)?)
        .map_err(|error| format!("invalid Agent descriptor: {error}"))
}

pub(crate) fn query_agent_descriptor_value(address: SocketAddr) -> RunnerResult<serde_json::Value> {
    let response = rpc_request(
        address,
        &RpcRequest {
            rpc_version: kyuubiki_protocol::RPC_VERSION,
            id: "operational-qualification-describe".to_string(),
            method: kyuubiki_protocol::RpcMethod::DescribeAgent,
            params: serde_json::json!({}),
        },
        Duration::from_secs(2),
    )?;
    if !response.ok {
        return Err("Agent descriptor RPC was rejected".to_string());
    }
    response
        .result
        .ok_or_else(|| "Agent descriptor RPC omitted its result".to_string())
}

pub(crate) fn rpc_request(
    address: SocketAddr,
    request: &RpcRequest,
    timeout: Duration,
) -> RunnerResult<RpcResponse> {
    let mut stream = TcpStream::connect_timeout(&address, timeout.min(Duration::from_secs(2)))
        .map_err(|error| format!("Agent RPC unavailable: {error}"))?;
    stream
        .set_read_timeout(Some(timeout))
        .and_then(|_| stream.set_write_timeout(Some(timeout)))
        .map_err(|error| format!("failed to configure Agent RPC timeout: {error}"))?;
    let payload = serde_json::to_vec(request)
        .map_err(|error| format!("failed to encode Agent RPC request: {error}"))?;
    let length =
        u32::try_from(payload.len()).map_err(|_| "Agent RPC request is too large".to_string())?;
    stream
        .write_all(&length.to_be_bytes())
        .and_then(|_| stream.write_all(&payload))
        .map_err(|error| format!("failed to write Agent RPC request: {error}"))?;

    loop {
        let mut header = [0_u8; 4];
        stream
            .read_exact(&mut header)
            .map_err(|error| format!("failed to read Agent RPC response header: {error}"))?;
        let response_length = u32::from_be_bytes(header) as usize;
        if response_length == 0 || response_length > MAX_RPC_BYTES {
            return Err("Agent RPC response length is invalid".to_string());
        }
        let mut response_bytes = vec![0_u8; response_length];
        stream
            .read_exact(&mut response_bytes)
            .map_err(|error| format!("failed to read Agent RPC response: {error}"))?;
        let value: serde_json::Value = serde_json::from_slice(&response_bytes)
            .map_err(|error| format!("invalid Agent RPC response: {error}"))?;
        if value.get("event").and_then(serde_json::Value::as_str) == Some("progress") {
            continue;
        }
        return serde_json::from_value(value)
            .map_err(|error| format!("invalid Agent final response: {error}"));
    }
}

pub(crate) fn wait_endpoint_closed(address: SocketAddr, timeout: Duration) -> RunnerResult<()> {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if TcpStream::connect_timeout(&address, Duration::from_millis(100)).is_err() {
            return Ok(());
        }
        thread::sleep(Duration::from_millis(100));
    }
    Err("qualification endpoint remained open after cleanup".to_string())
}

pub(crate) fn remove_local_work_root(path: &Path) -> RunnerResult<bool> {
    if path.exists() {
        fs::remove_dir_all(path)
            .map_err(|error| format!("failed to remove local qualification work root: {error}"))?;
    }
    Ok(!path.exists())
}

fn parse_ipv4(value: &str, label: &str) -> RunnerResult<Ipv4Addr> {
    match IpAddr::from_str(value) {
        Ok(IpAddr::V4(address)) if !address.is_loopback() && !address.is_unspecified() => {
            Ok(address)
        }
        _ => Err(format!(
            "{label} address must be a non-loopback IPv4 address"
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_loopback_and_non_ipv4_connection_addresses() {
        assert!(parse_ipv4("127.0.0.1", "test").is_err());
        assert!(parse_ipv4("::1", "test").is_err());
        assert!(parse_ipv4("192.0.2.10", "test").is_ok());
    }
}
