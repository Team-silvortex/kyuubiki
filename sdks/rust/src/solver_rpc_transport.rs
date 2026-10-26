use crate::{
    RpcCallOutcome, SdkError, SdkResult,
    solver_rpc_wire::{self, Reply, invalid},
};
use serde_json::{Value, json};
use std::{
    io::{self, Read, Write},
    net::{TcpStream, ToSocketAddrs},
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

/// Encoded-byte budgets, not a promise about parsed JSON's total heap footprint.
/// Timeouts bound network I/O, not synchronous OS hostname resolution or solver work.
#[derive(Debug, Clone)]
pub struct SolverRpcLimits {
    pub max_request_bytes: usize,
    pub max_response_bytes: usize,
    pub max_progress_bytes: usize,
    pub max_progress_frames: usize,
    pub io_timeout: Duration,
    pub call_timeout: Duration,
}

impl Default for SolverRpcLimits {
    fn default() -> Self {
        Self {
            max_request_bytes: 256 * 1024 * 1024,
            max_response_bytes: 256 * 1024 * 1024,
            max_progress_bytes: 4 * 1024 * 1024,
            max_progress_frames: 4096,
            io_timeout: Duration::from_secs(15),
            call_timeout: Duration::from_secs(30 * 60),
        }
    }
}

impl SolverRpcLimits {
    pub(crate) fn validate(&self) -> SdkResult<()> {
        if self.max_request_bytes == 0
            || self.max_response_bytes == 0
            || u32::try_from(self.max_request_bytes).is_err()
            || u32::try_from(self.max_response_bytes).is_err()
            || self.io_timeout.is_zero()
            || self.call_timeout.is_zero()
            || Instant::now().checked_add(self.call_timeout).is_none()
        {
            return Err(invalid(
                "invalid RPC limits: finite nonzero frame and timeout budgets required",
            ));
        }
        Ok(())
    }
}

static NEXT_REQUEST: AtomicU64 = AtomicU64::new(0);

fn next_id() -> SdkResult<String> {
    let sequence = NEXT_REQUEST
        .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_add(1))
        .map_err(|_| invalid("RPC request sequence exhausted"))?;
    let epoch = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    Ok(format!(
        "rust-sdk-{}-{epoch:x}-{sequence:x}",
        std::process::id()
    ))
}

pub(crate) fn call_identified(
    host: &str,
    port: u16,
    limits: &SolverRpcLimits,
    method: &str,
    params: Value,
) -> SdkResult<(String, RpcCallOutcome)> {
    limits.validate()?;
    let id = next_id()?;
    let payload = encode(
        &json!({"rpc_version":1, "id":id, "method":method, "params":params}),
        limits.max_request_bytes,
    )?;
    let deadline = Instant::now()
        .checked_add(limits.call_timeout)
        .ok_or_else(|| invalid("RPC deadline overflow"))?;
    // DNS is synchronous in std; elapsed resolution time still consumes this budget.
    let addresses = (host, port).to_socket_addrs()?;
    let mut last = None;
    let mut connected = None;
    for address in addresses {
        let timeout = remaining(deadline, limits.io_timeout)?;
        match TcpStream::connect_timeout(&address, timeout) {
            Ok(stream) => {
                connected = Some(stream);
                break;
            }
            Err(error) => last = Some(error),
        }
    }
    let mut stream = connected.ok_or_else(|| {
        last.map(SdkError::from)
            .unwrap_or_else(|| invalid("RPC host has no addresses"))
    })?;
    let length =
        u32::try_from(payload.len()).map_err(|_| invalid("RPC request exceeds wire length"))?;
    write_until(
        &mut stream,
        &length.to_be_bytes(),
        deadline,
        limits.io_timeout,
    )?;
    write_until(&mut stream, &payload, deadline, limits.io_timeout)?;
    drop(payload);
    let mut progress_frames = Vec::new();
    let mut progress_bytes = 0usize;
    loop {
        let mut header = [0; 4];
        read_until(&mut stream, &mut header, deadline, limits.io_timeout)?;
        let size = u32::from_be_bytes(header) as usize;
        if size == 0 || size > limits.max_response_bytes {
            return Err(invalid(
                "RPC response frame exceeds configured byte limit or is empty",
            ));
        }
        let mut bytes = Vec::new();
        bytes
            .try_reserve_exact(size)
            .map_err(|_| invalid("cannot allocate bounded RPC response"))?;
        bytes.resize(size, 0);
        read_until(&mut stream, &mut bytes, deadline, limits.io_timeout)?;
        let reply =
            solver_rpc_wire::decode(&bytes, &id, limits.max_progress_bytes - progress_bytes)?;
        remaining(deadline, limits.io_timeout)?;
        match reply {
            Reply::Progress(value) => {
                if progress_frames.len() >= limits.max_progress_frames {
                    return Err(invalid("RPC progress frame budget exceeded"));
                }
                progress_bytes += size;
                progress_frames.push(value);
            }
            Reply::Complete(result) => {
                return Ok((
                    id,
                    RpcCallOutcome {
                        result,
                        progress_frames,
                    },
                ));
            }
        }
    }
}

fn remaining(deadline: Instant, idle: Duration) -> SdkResult<Duration> {
    let remaining = deadline.saturating_duration_since(Instant::now());
    if remaining.is_zero() {
        return Err(SdkError::Timeout(
            "RPC call deadline exceeded; execution outcome may be unknown".into(),
        ));
    }
    Ok(remaining.min(idle))
}

fn io_error(error: io::Error) -> SdkError {
    if matches!(
        error.kind(),
        io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut
    ) {
        SdkError::Timeout("RPC I/O timeout; execution outcome may be unknown".into())
    } else {
        error.into()
    }
}

fn read_until(
    stream: &mut TcpStream,
    mut bytes: &mut [u8],
    deadline: Instant,
    idle: Duration,
) -> SdkResult<()> {
    while !bytes.is_empty() {
        stream.set_read_timeout(Some(remaining(deadline, idle)?))?;
        match stream.read(bytes) {
            Ok(0) => return Err(io::Error::from(io::ErrorKind::UnexpectedEof).into()),
            Ok(count) => bytes = &mut bytes[count..],
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(io_error(error)),
        }
    }
    Ok(())
}

fn write_until(
    stream: &mut TcpStream,
    mut bytes: &[u8],
    deadline: Instant,
    idle: Duration,
) -> SdkResult<()> {
    while !bytes.is_empty() {
        stream.set_write_timeout(Some(remaining(deadline, idle)?))?;
        match stream.write(bytes) {
            Ok(0) => return Err(io::Error::from(io::ErrorKind::WriteZero).into()),
            Ok(count) => bytes = &bytes[count..],
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(io_error(error)),
        }
    }
    Ok(())
}

struct BoundedRequest {
    bytes: Vec<u8>,
    limit: usize,
    exceeded: bool,
}

impl Write for BoundedRequest {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if bytes.len() > self.limit - self.bytes.len() {
            self.exceeded = true;
            return Err(io::Error::other("RPC request byte limit exceeded"));
        }
        let required = self.bytes.len() + bytes.len();
        if required > self.bytes.capacity() {
            let capacity = required
                .max(self.bytes.capacity().saturating_mul(2))
                .min(self.limit);
            self.bytes
                .try_reserve_exact(capacity - self.bytes.len())
                .map_err(io::Error::other)?;
        }
        self.bytes.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn encode(value: &Value, limit: usize) -> SdkResult<Vec<u8>> {
    let mut output = BoundedRequest {
        bytes: Vec::new(),
        limit,
        exceeded: false,
    };
    let result = serde_json::to_writer(&mut output, value);
    if output.exceeded {
        return Err(invalid(
            "RPC request exceeds configured byte limit; not dispatched",
        ));
    }
    result?;
    Ok(output.bytes)
}
