use crate::HeadlessExecutorError;
use std::io::{self, Read, Write};
use std::net::{IpAddr, SocketAddr, TcpStream, ToSocketAddrs};
use std::sync::{
    Arc, LazyLock,
    atomic::{AtomicUsize, Ordering},
    mpsc,
};
use std::thread;
use std::time::{Duration, Instant};

const MAX_RESOLVERS: usize = 4;
const MAX_STATUS_RESPONSE_BYTES: usize = 8_000_000;
static RESOLVERS: LazyLock<Arc<AtomicUsize>> = LazyLock::new(|| Arc::new(AtomicUsize::new(0)));

pub(crate) fn remaining_timeout(
    deadline: Instant,
    limit: Duration,
) -> Result<Duration, HeadlessExecutorError> {
    let remaining = deadline.saturating_duration_since(Instant::now());
    if remaining.is_zero() {
        return Err(error("service request deadline exhausted"));
    }
    Ok(remaining.min(limit))
}

pub(crate) fn resolve_before_deadline(
    host: &str,
    port: u16,
    deadline: Instant,
) -> Result<Vec<SocketAddr>, HeadlessExecutorError> {
    remaining_timeout(deadline, Duration::MAX)?;
    if let Ok(ip) = host.parse::<IpAddr>() {
        return Ok(vec![SocketAddr::new(ip, port)]);
    }
    let host = host.to_string();
    resolve_with(deadline, Arc::clone(&RESOLVERS), move || {
        (host.as_str(), port)
            .to_socket_addrs()
            .map(Iterator::collect)
    })
}

// System DNS is not cancellable. Bound outstanding lookups, discard late answers,
// and never let a resolver thread connect or send a request after its caller exits.
fn resolve_with(
    deadline: Instant,
    active: Arc<AtomicUsize>,
    resolve: impl FnOnce() -> io::Result<Vec<SocketAddr>> + Send + 'static,
) -> Result<Vec<SocketAddr>, HeadlessExecutorError> {
    remaining_timeout(deadline, Duration::MAX)?;
    active
        .fetch_update(Ordering::AcqRel, Ordering::Acquire, |count| {
            (count < MAX_RESOLVERS).then_some(count + 1)
        })
        .map_err(|_| error("service resolver capacity exhausted; retry observation later"))?;
    let slot = ResolverSlot(active);
    let (sender, receiver) = mpsc::channel();
    thread::Builder::new()
        .name("headless-dns".into())
        .spawn(move || {
            let _slot = slot;
            if Instant::now() < deadline {
                let _ = sender.send(resolve());
            }
        })
        .map_err(|cause| error(format!("failed to start service resolver: {cause}")))?;
    let result = receiver
        .recv_timeout(remaining_timeout(deadline, Duration::MAX)?)
        .map_err(|cause| match cause {
            mpsc::RecvTimeoutError::Timeout => {
                error("service request deadline exhausted during DNS resolution")
            }
            mpsc::RecvTimeoutError::Disconnected => {
                error("service resolver stopped without a result")
            }
        })?;
    remaining_timeout(deadline, Duration::MAX)?;
    result.map_err(|cause| error(format!("failed to resolve service address: {cause}")))
}

struct ResolverSlot(Arc<AtomicUsize>);

impl Drop for ResolverSlot {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::AcqRel);
    }
}

pub(crate) fn write_before_deadline(
    stream: &mut TcpStream,
    mut bytes: &[u8],
    deadline: Instant,
    io_timeout: Duration,
) -> Result<(), HeadlessExecutorError> {
    while !bytes.is_empty() {
        stream
            .set_write_timeout(Some(remaining_timeout(deadline, io_timeout)?))
            .map_err(|cause| {
                error(format!(
                    "failed to configure service write timeout: {cause}"
                ))
            })?;
        match stream.write(bytes) {
            Ok(0) => return Err(error("failed to write service request: connection closed")),
            Ok(length) => bytes = &bytes[length..],
            Err(cause) if cause.kind() == io::ErrorKind::Interrupted => continue,
            Err(cause) => {
                return Err(error(format!(
                    "failed to write service request within timeout: {cause}"
                )));
            }
        }
    }
    remaining_timeout(deadline, io_timeout).map(|_| ())
}

pub(crate) fn read_before_deadline(
    stream: &mut TcpStream,
    deadline: Instant,
    io_timeout: Duration,
) -> Result<String, HeadlessExecutorError> {
    let mut response = Vec::new();
    let mut buffer = [0; 8192];
    loop {
        stream
            .set_read_timeout(Some(remaining_timeout(deadline, io_timeout)?))
            .map_err(|cause| error(format!("failed to configure service read timeout: {cause}")))?;
        match stream.read(&mut buffer) {
            Ok(0) => break,
            Ok(length) => {
                if response.len() + length > MAX_STATUS_RESPONSE_BYTES {
                    return Err(error(
                        "job status response exceeds the 8000000-byte transport limit",
                    ));
                }
                response.extend_from_slice(&buffer[..length]);
            }
            Err(cause) if cause.kind() == io::ErrorKind::Interrupted => continue,
            Err(cause) => {
                return Err(error(format!(
                    "failed to read service response within timeout: {cause}"
                )));
            }
        }
    }
    remaining_timeout(deadline, io_timeout)?;
    String::from_utf8(response)
        .map_err(|cause| error(format!("service response is not UTF-8: {cause}")))
}

fn error(message: impl Into<String>) -> HeadlessExecutorError {
    HeadlessExecutorError {
        message: message.into(),
    }
}

#[cfg(test)]
#[path = "service_executor_deadline_tests.rs"]
mod tests;
