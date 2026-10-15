use super::{Limits, Permit};
use kyuubiki_protocol::RpcMethod;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

pub(super) const CONNECTIONS: usize = 4;
pub(super) const REQUESTS: usize = 4;
pub(super) const FRAME_BYTES: usize = 16 * 1024;
pub(super) const TIMEOUT_MS: usize = 1000;

const METHODS: &[RpcMethod] = &[
    RpcMethod::Ping,
    RpcMethod::DescribeAgent,
    RpcMethod::DescribeAgentLifecycle,
    RpcMethod::BeginAgentDrain,
    RpcMethod::ResumeAgentAdmission,
    RpcMethod::FetchDurableAgentExecution,
    RpcMethod::ListDurableAgentExecutions,
    RpcMethod::CancelDurableAgentExecution,
    RpcMethod::FetchOperatorTaskResult,
    RpcMethod::CancelExecution,
    RpcMethod::CancelJob,
];

pub(crate) fn allows_method(method: &RpcMethod) -> bool {
    METHODS.contains(method)
}

pub(super) struct RecoveryReserve {
    connections: Arc<AtomicUsize>,
    requests: Arc<AtomicUsize>,
    bytes: Arc<AtomicUsize>,
}

impl RecoveryReserve {
    pub(super) fn new() -> Self {
        Self {
            connections: Arc::new(AtomicUsize::new(0)),
            requests: Arc::new(AtomicUsize::new(0)),
            bytes: Arc::new(AtomicUsize::new(0)),
        }
    }

    pub(super) fn connection(&self) -> Option<Permit> {
        Permit::acquire(&self.connections, 1, CONNECTIONS)
    }

    pub(super) fn request(&self, length: usize, limits: Limits) -> Option<(Permit, Permit)> {
        let maximum = limits.frame_bytes.min(FRAME_BYTES);
        if length > maximum {
            return None;
        }
        let request = Permit::acquire(&self.requests, 1, REQUESTS)?;
        let bytes = Permit::acquire(&self.bytes, length, maximum * REQUESTS)?;
        Some((bytes, request))
    }

    pub(super) fn active_connections(&self) -> usize {
        self.connections.load(Ordering::Acquire)
    }

    pub(super) fn reserved_bytes(&self) -> usize {
        self.bytes.load(Ordering::Acquire)
    }

    pub(super) fn snapshot(&self, limits: Limits) -> serde_json::Value {
        serde_json::json!({
            "schema_version":"kyuubiki.agent-recovery-ingress/v1",
            "configuration":"fixed_read_only_reserve_same_listener",
            "max_connections":CONNECTIONS, "max_requests":REQUESTS,
            "max_request_bytes":limits.frame_bytes.min(FRAME_BYTES),
            "max_in_flight_request_bytes":limits.frame_bytes.min(FRAME_BYTES) * REQUESTS,
            "request_timeout_ms":limits.timeout_ms.min(TIMEOUT_MS),
            "active_connections":self.active_connections(),
            "active_requests":self.requests.load(Ordering::Acquire),
            "reserved_request_bytes":self.reserved_bytes(),
            "allowed_methods":METHODS,
            "connection_scope":"control_only_until_close_never_promoted",
            "byte_fallback_scope":"control_only_for_current_frame",
            "deadline_scope":"from_frame_receive_start_never_extended",
            "new_execution_allowed":false, "automatic_retry":false,
            "authentication":false, "sustained_overload_availability_guaranteed":false
        })
    }
}
