use std::io::{self, Read, Write};
use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use kyuubiki_protocol::{
    AgentDescriptor, RPC_VERSION, RpcMethod, RpcRequest, RpcResponse,
    validate_rpc_response_envelope,
};

pub(super) const MAX_FRAME_BYTES: usize = 1024 * 1024;
pub(super) const REQUEST_TIMEOUT_MS: u64 = 1500;
pub(super) const MAX_RESOLVERS: usize = 2;
static RESOLVERS: AtomicUsize = AtomicUsize::new(0);
static NEXT_REQUEST: AtomicU64 = AtomicU64::new(0);

pub(super) fn normalize_endpoint(value: &str) -> Result<String, String> {
    let value = value.trim();
    if value.len() > 255 {
        return Err("peer endpoint exceeds 255 bytes".into());
    }
    if let Ok(address) = value.parse::<SocketAddr>() {
        validate_address(address)?;
        return Ok(address.to_string());
    }
    let (host, port) = value.rsplit_once(':').ok_or("peer requires host:port")?;
    let port: u16 = port.parse().map_err(|_| "invalid peer port")?;
    if port == 0
        || host.is_empty()
        || !host.split('.').all(|label| {
            !label.is_empty()
                && label.len() <= 63
                && !label.starts_with('-')
                && !label.ends_with('-')
                && label
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
        })
    {
        return Err("invalid peer hostname or port".into());
    }
    Ok(format!("{}:{port}", host.to_ascii_lowercase()))
}

fn validate_address(address: SocketAddr) -> Result<(), String> {
    let ip = address.ip().to_canonical();
    if address.port() == 0
        || ip.is_unspecified()
        || ip.is_multicast()
        || matches!(ip, std::net::IpAddr::V4(ip) if ip.is_broadcast())
    {
        return Err("peer address must be a unicast destination with a nonzero port".into());
    }
    Ok(())
}

fn remaining(deadline: Instant) -> io::Result<Duration> {
    let left = deadline.saturating_duration_since(Instant::now());
    if left.is_zero() {
        Err(io::Error::new(
            io::ErrorKind::TimedOut,
            "peer request deadline exhausted",
        ))
    } else {
        Ok(left)
    }
}

struct ResolverSlot;
impl Drop for ResolverSlot {
    fn drop(&mut self) {
        RESOLVERS.fetch_sub(1, Ordering::AcqRel);
    }
}

fn resolve(address: &str, deadline: Instant) -> Result<Vec<SocketAddr>, String> {
    remaining(deadline).map_err(|e| e.to_string())?;
    if let Ok(address) = address.parse() {
        return Ok(vec![address]);
    }
    RESOLVERS
        .fetch_update(Ordering::AcqRel, Ordering::Acquire, |active| {
            (active < MAX_RESOLVERS).then_some(active + 1)
        })
        .map_err(|_| "peer resolver capacity exhausted")?;
    let slot = ResolverSlot;
    let address = address.to_string();
    let (sender, receiver) = mpsc::sync_channel(1);
    // System DNS is not cancellable. Late resolvers cannot connect, and their count is bounded.
    std::thread::Builder::new()
        .name("agent-peer-dns".into())
        .spawn(move || {
            let _slot = slot;
            if Instant::now() < deadline {
                let result = address
                    .to_socket_addrs()
                    .map(|addresses| addresses.take(8).collect::<Vec<_>>());
                let _ = sender.send(result);
            }
        })
        .map_err(|e| e.to_string())?;
    let result = receiver
        .recv_timeout(remaining(deadline).map_err(|e| e.to_string())?)
        .map_err(|_| "peer DNS unavailable before deadline")?;
    remaining(deadline).map_err(|e| e.to_string())?;
    result.map_err(|e| e.to_string())
}

fn read_exact_before(
    stream: &mut TcpStream,
    mut bytes: &mut [u8],
    deadline: Instant,
) -> io::Result<()> {
    while !bytes.is_empty() {
        stream.set_read_timeout(Some(remaining(deadline)?))?;
        match stream.read(bytes) {
            Ok(0) => return Err(io::Error::from(io::ErrorKind::UnexpectedEof)),
            Ok(count) => bytes = &mut bytes[count..],
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(error),
        }
    }
    remaining(deadline).map(|_| ())
}

fn write_before(stream: &mut TcpStream, mut bytes: &[u8], deadline: Instant) -> io::Result<()> {
    while !bytes.is_empty() {
        stream.set_write_timeout(Some(remaining(deadline)?))?;
        match stream.write(bytes) {
            Ok(0) => return Err(io::Error::from(io::ErrorKind::WriteZero)),
            Ok(count) => bytes = &bytes[count..],
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(error),
        }
    }
    remaining(deadline).map(|_| ())
}

pub(super) fn request_agent_descriptor(address: &str) -> Result<AgentDescriptor, String> {
    request_with_timeout(address, Duration::from_millis(REQUEST_TIMEOUT_MS))
}

pub(super) fn request_with_timeout(
    address: &str,
    timeout: Duration,
) -> Result<AgentDescriptor, String> {
    let deadline = Instant::now() + timeout;
    let address = normalize_endpoint(address)?;
    let mut connected = None;
    for address in resolve(&address, deadline)? {
        if validate_address(address).is_err() {
            continue;
        }
        let timeout = remaining(deadline)
            .map_err(|e| e.to_string())?
            .min(Duration::from_millis(500));
        if let Ok(stream) = TcpStream::connect_timeout(&address, timeout) {
            connected = Some(stream);
            break;
        }
    }
    let mut stream = connected.ok_or("peer unavailable before deadline")?;
    let id = format!(
        "peer-describe-{}-{}",
        std::process::id(),
        NEXT_REQUEST.fetch_add(1, Ordering::Relaxed)
    );
    let request = RpcRequest {
        rpc_version: RPC_VERSION,
        id: id.clone(),
        method: RpcMethod::DescribeAgent,
        params: serde_json::json!({}),
    };
    let payload = serde_json::to_vec(&request).map_err(|e| e.to_string())?;
    write_before(&mut stream, &(payload.len() as u32).to_be_bytes(), deadline)
        .and_then(|_| write_before(&mut stream, &payload, deadline))
        .map_err(|e| e.to_string())?;
    let mut header = [0; 4];
    read_exact_before(&mut stream, &mut header, deadline).map_err(|e| e.to_string())?;
    let length = u32::from_be_bytes(header) as usize;
    if !(1..=MAX_FRAME_BYTES).contains(&length) {
        return Err("peer descriptor frame exceeds limit".into());
    }
    let mut payload = vec![0; length];
    read_exact_before(&mut stream, &mut payload, deadline).map_err(|e| e.to_string())?;
    let response: RpcResponse = serde_json::from_slice(&payload).map_err(|e| e.to_string())?;
    validate_rpc_response_envelope(&response).map_err(|e| e.message)?;
    if response.id != id || !response.ok {
        return Err("peer describe response identity or state mismatch".into());
    }
    let descriptor: AgentDescriptor =
        serde_json::from_value(response.result.ok_or("missing peer descriptor")?)
            .map_err(|e| e.to_string())?;
    if descriptor.protocol.rpc_version != RPC_VERSION {
        return Err("unsupported peer protocol".into());
    }
    remaining(deadline).map_err(|e| e.to_string())?;
    Ok(descriptor)
}
