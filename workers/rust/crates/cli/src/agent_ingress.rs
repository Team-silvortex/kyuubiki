use std::io::{self, Read};
use std::net::{SocketAddr, TcpStream};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};

use crate::transport::FrameReadError;

#[path = "agent_ingress_recovery.rs"]
mod recovery;
pub(crate) use recovery::allows_method as allows_recovery_method;

const MIB: usize = 1024 * 1024;
const ENV_NAMES: [&str; 4] = [
    "KYUUBIKI_AGENT_MAX_REQUEST_BYTES",
    "KYUUBIKI_AGENT_MAX_IN_FLIGHT_REQUEST_BYTES",
    "KYUUBIKI_AGENT_REQUEST_TIMEOUT_MS",
    "KYUUBIKI_AGENT_MAX_CONNECTIONS",
];
static INGRESS: OnceLock<Ingress> = OnceLock::new();

#[derive(Clone, Copy)]
struct Limits {
    frame_bytes: usize,
    in_flight_bytes: usize,
    timeout_ms: usize,
    connections: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            frame_bytes: 256 * MIB,
            in_flight_bytes: 512 * MIB,
            timeout_ms: 30_000,
            connections: 64,
        }
    }
}

impl Limits {
    fn parse(mut read: impl FnMut(&str) -> Result<Option<String>, String>) -> Result<Self, String> {
        let defaults = Self::default();
        let mut value = |index, default, max| -> Result<usize, String> {
            let name = ENV_NAMES[index];
            match read(name)? {
                None => Ok(default),
                Some(text) => text
                    .parse::<usize>()
                    .ok()
                    .filter(|value| (1..=max).contains(value))
                    .ok_or_else(|| format!("{name} must be within 1..={max}")),
            }
        };
        let limits = Self {
            frame_bytes: value(0, defaults.frame_bytes, 512 * MIB)?,
            in_flight_bytes: value(1, defaults.in_flight_bytes, 1024 * MIB)?,
            timeout_ms: value(2, defaults.timeout_ms, 300_000)?,
            connections: value(3, defaults.connections, 256)?,
        };
        if limits.frame_bytes > limits.in_flight_bytes {
            return Err(format!(
                "{} must be at least {}",
                ENV_NAMES[1], ENV_NAMES[0]
            ));
        }
        Ok(limits)
    }
}

pub(crate) struct Ingress {
    limits: Limits,
    connections: Arc<AtomicUsize>,
    request_bytes: Arc<AtomicUsize>,
    recovery: recovery::RecoveryReserve,
}

impl Ingress {
    fn new(limits: Limits) -> Self {
        Self {
            limits,
            connections: Arc::new(AtomicUsize::new(0)),
            request_bytes: Arc::new(AtomicUsize::new(0)),
            recovery: recovery::RecoveryReserve::new(),
        }
    }

    pub(crate) fn admit_peer(&self, peer: io::Result<SocketAddr>) -> Option<ConnectionPermit> {
        self.admit_peer_with_policy(peer, crate::agent_network_access::current())
    }

    fn admit_peer_with_policy(
        &self,
        peer: io::Result<SocketAddr>,
        policy: &crate::agent_network_access::NetworkAccess,
    ) -> Option<ConnectionPermit> {
        if !policy.admit(peer) {
            return None;
        }
        self.admit_connection()
    }

    fn admit_connection(&self) -> Option<ConnectionPermit> {
        if let Some(permit) = Permit::acquire(&self.connections, 1, self.limits.connections) {
            Some(ConnectionPermit {
                _permit: permit,
                recovery_only: false,
            })
        } else {
            self.recovery.connection().map(|permit| ConnectionPermit {
                _permit: permit,
                recovery_only: true,
            })
        }
    }

    #[cfg(test)]
    pub(crate) fn read_frame(
        &self,
        stream: &mut TcpStream,
    ) -> Result<RequestFrame, FrameReadError> {
        self.read_connection_frame(stream, false)
    }

    pub(crate) fn read_connection_frame(
        &self,
        stream: &mut TcpStream,
        recovery_only: bool,
    ) -> Result<RequestFrame, FrameReadError> {
        self.read_request_in(stream, recovery_only)
            .map_err(|error| {
                if error.kind() == io::ErrorKind::UnexpectedEof {
                    FrameReadError::ConnectionClosed
                } else {
                    FrameReadError::Io(error)
                }
            })
    }

    #[cfg(test)]
    fn read_request(&self, stream: &mut TcpStream) -> io::Result<RequestFrame> {
        self.read_request_in(stream, false)
    }

    fn read_request_in(
        &self,
        stream: &mut TcpStream,
        recovery_only: bool,
    ) -> io::Result<RequestFrame> {
        // Idle/header/body reads share one budget; progress never resets it.
        let started = Instant::now();
        let recovery_deadline = started
            + Duration::from_millis(self.limits.timeout_ms.min(recovery::TIMEOUT_MS) as u64);
        let mut deadline = if recovery_only {
            recovery_deadline
        } else {
            started + Duration::from_millis(self.limits.timeout_ms as u64)
        };
        let mut header = [0; 4];
        read_before(stream, &mut header, deadline)?;
        let length = u32::from_be_bytes(header) as usize;
        if !(1..=self.limits.frame_bytes).contains(&length) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "agent_request_size_limit",
            ));
        }
        let ordinary = if recovery_only {
            None
        } else {
            Permit::acquire(&self.request_bytes, length, self.limits.in_flight_bytes)
        };
        let (reservation, recovery_request) = if let Some(permit) = ordinary {
            (permit, None)
        } else {
            deadline = recovery_deadline;
            remaining(deadline)?;
            let (bytes, request) = self
                .recovery
                .request(length, self.limits)
                .ok_or_else(|| io::Error::other("agent_request_byte_capacity"))?;
            (bytes, Some(request))
        };
        let mut payload = Vec::new();
        payload
            .try_reserve_exact(length)
            .map_err(|_| io::Error::other("agent_request_allocation_failed"))?;
        payload.resize(length, 0);
        read_before(stream, &mut payload, deadline)?;
        Ok(RequestFrame {
            payload,
            recovery_only: recovery_request.is_some(),
            _reservation: reservation,
            _recovery_request: recovery_request,
        })
    }

    fn snapshot(&self) -> serde_json::Value {
        serde_json::json!({
            "schema_version":"kyuubiki.agent-request-ingress/v1",
            "configuration_env":ENV_NAMES,
            "configuration_apply":"process_restart",
            "max_request_bytes":self.limits.frame_bytes,
            "max_in_flight_request_bytes":self.limits.in_flight_bytes,
            "request_timeout_ms":self.limits.timeout_ms,
            "max_connections":self.limits.connections,
            "general_capacity_scope":"configured_limits_exclude_fixed_recovery_reserve",
            "max_total_connections":self.limits.connections + recovery::CONNECTIONS,
            "max_total_in_flight_request_bytes":self.limits.in_flight_bytes + self.limits.frame_bytes.min(recovery::FRAME_BYTES) * recovery::REQUESTS,
            "active_connections":self.connections.load(Ordering::Acquire) + self.recovery.active_connections(),
            "general_active_connections":self.connections.load(Ordering::Acquire),
            "general_reserved_request_bytes":self.request_bytes.load(Ordering::Acquire),
            "reserved_request_bytes":self.request_bytes.load(Ordering::Acquire) + self.recovery.reserved_bytes(),
            "recovery_reserve":self.recovery.snapshot(self.limits),
            "deadline_scope":"idle_header_and_body_per_request_not_execution",
            "byte_budget_scope":"encoded_request_retained_through_reply",
            "overflow_action":"close_connection_before_dispatch",
            "overload_queue":false,
            "automatic_retry":false,
            "authenticated_transport":false,
            "network_access":crate::agent_network_access::snapshot(),
            "whole_process_memory_limit":false
        })
    }
}

pub(crate) struct RequestFrame {
    pub(crate) payload: Vec<u8>,
    pub(crate) recovery_only: bool,
    _reservation: Permit,
    _recovery_request: Option<Permit>,
}

pub(crate) struct ConnectionPermit {
    _permit: Permit,
    pub(crate) recovery_only: bool,
}

pub(crate) struct Permit {
    counter: Arc<AtomicUsize>,
    amount: usize,
}

impl Permit {
    fn acquire(counter: &Arc<AtomicUsize>, amount: usize, limit: usize) -> Option<Self> {
        counter
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |current| {
                current.checked_add(amount).filter(|next| *next <= limit)
            })
            .ok()?;
        Some(Self {
            counter: Arc::clone(counter),
            amount,
        })
    }
}

impl Drop for Permit {
    fn drop(&mut self) {
        self.counter.fetch_sub(self.amount, Ordering::AcqRel);
    }
}

pub(crate) fn configure() -> Result<(), String> {
    let limits = Limits::parse(|name| match std::env::var(name) {
        Ok(value) => Ok(Some(value)),
        Err(std::env::VarError::NotPresent) => Ok(None),
        Err(_) => Err(format!("{name} must be valid Unicode")),
    })?;
    INGRESS
        .set(Ingress::new(limits))
        .map_err(|_| "Agent ingress already configured".into())
}

pub(crate) fn current() -> &'static Ingress {
    INGRESS.get_or_init(|| Ingress::new(Limits::default()))
}

pub(crate) fn snapshot() -> serde_json::Value {
    current().snapshot()
}

fn read_before(stream: &mut TcpStream, mut bytes: &mut [u8], deadline: Instant) -> io::Result<()> {
    while !bytes.is_empty() {
        stream.set_read_timeout(Some(remaining(deadline)?))?;
        match stream.read(bytes) {
            Ok(0) => return Err(io::ErrorKind::UnexpectedEof.into()),
            Ok(count) => bytes = &mut bytes[count..],
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(error),
        }
    }
    remaining(deadline).map(|_| ())
}

fn remaining(deadline: Instant) -> io::Result<Duration> {
    let remaining = deadline.saturating_duration_since(Instant::now());
    if remaining.is_zero() {
        Err(io::Error::new(
            io::ErrorKind::TimedOut,
            "agent_request_timeout",
        ))
    } else {
        Ok(remaining)
    }
}

#[cfg(test)]
#[path = "tests/agent_ingress.rs"]
mod tests;
