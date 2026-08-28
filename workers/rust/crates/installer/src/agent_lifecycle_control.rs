use kyuubiki_protocol::{
    AgentLifecycleDescriptor, BeginAgentDrainRequest, RPC_VERSION, ResumeAgentAdmissionRequest,
    RpcMethod, RpcRequest, RpcResponse, validate_agent_lifecycle_descriptor,
    validate_rpc_response_envelope,
};
use serde::Serialize;
use serde_json::Value;
use std::fmt;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, Instant};

const MAX_AGENT_LIFECYCLE_FRAME_BYTES: usize = 1024 * 1024;
const DEFAULT_POLL_INTERVAL: Duration = Duration::from_millis(50);

#[derive(Clone, Debug)]
pub struct AgentLifecycleClient {
    address: SocketAddr,
    timeout: Duration,
    poll_interval: Duration,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AgentLifecycleControlError {
    pub code: String,
    pub message: String,
    pub retryable: bool,
}

pub trait AgentLifecycleControl {
    fn describe(&self) -> Result<AgentLifecycleDescriptor, AgentLifecycleControlError>;
    fn begin_drain(
        &self,
        controller_id: &str,
        reason: &str,
    ) -> Result<AgentLifecycleDescriptor, AgentLifecycleControlError>;
    fn wait_until_quiescent(
        &self,
        controller_id: &str,
        drain_generation: u64,
    ) -> Result<AgentLifecycleDescriptor, AgentLifecycleControlError>;
    fn wait_until_replaced(
        &self,
        previous_process_instance_id: &str,
    ) -> Result<AgentLifecycleDescriptor, AgentLifecycleControlError>;
    fn resume_admission(
        &self,
        controller_id: &str,
        drain_generation: u64,
    ) -> Result<AgentLifecycleDescriptor, AgentLifecycleControlError>;
}

impl fmt::Display for AgentLifecycleControlError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for AgentLifecycleControlError {}

impl AgentLifecycleClient {
    pub fn new(address: SocketAddr, timeout: Duration) -> Result<Self, AgentLifecycleControlError> {
        if timeout.is_zero() {
            return Err(control_error(
                "invalid_timeout",
                "agent lifecycle timeout must be greater than zero",
            ));
        }
        Ok(Self {
            address,
            timeout,
            poll_interval: DEFAULT_POLL_INTERVAL.min(timeout),
        })
    }

    pub fn describe(&self) -> Result<AgentLifecycleDescriptor, AgentLifecycleControlError> {
        self.lifecycle_request(RpcMethod::DescribeAgentLifecycle, serde_json::json!({}))
    }

    pub fn begin_drain(
        &self,
        controller_id: &str,
        reason: &str,
    ) -> Result<AgentLifecycleDescriptor, AgentLifecycleControlError> {
        self.lifecycle_request(
            RpcMethod::BeginAgentDrain,
            serialize_params(BeginAgentDrainRequest {
                controller_id: controller_id.to_string(),
                reason: reason.to_string(),
            })?,
        )
    }

    pub fn wait_until_quiescent(
        &self,
        controller_id: &str,
        drain_generation: u64,
    ) -> Result<AgentLifecycleDescriptor, AgentLifecycleControlError> {
        let deadline = Instant::now() + self.timeout;
        loop {
            let lifecycle = self.describe()?;
            if lifecycle.drain_generation != drain_generation
                || lifecycle.drain_owner_id.as_deref() != Some(controller_id)
            {
                return Err(control_error(
                    "agent_drain_lease_lost",
                    "agent drain owner or generation changed while waiting for quiescence",
                ));
            }
            if lifecycle.state == "quiescent" && lifecycle.safe_to_replace {
                return Ok(lifecycle);
            }
            if lifecycle.state != "draining" {
                return Err(control_error(
                    "agent_drain_state_lost",
                    "agent left draining state before reaching quiescence",
                ));
            }
            if Instant::now() >= deadline {
                return Err(retryable_error(
                    "agent_drain_timeout",
                    format!(
                        "agent {} did not quiesce within {} ms",
                        self.address,
                        self.timeout.as_millis()
                    ),
                ));
            }
            thread::sleep(self.poll_interval);
        }
    }

    pub fn resume_admission(
        &self,
        controller_id: &str,
        drain_generation: u64,
    ) -> Result<AgentLifecycleDescriptor, AgentLifecycleControlError> {
        self.lifecycle_request(
            RpcMethod::ResumeAgentAdmission,
            serialize_params(ResumeAgentAdmissionRequest {
                controller_id: controller_id.to_string(),
                drain_generation,
            })?,
        )
    }

    pub fn wait_until_replaced(
        &self,
        previous_process_instance_id: &str,
    ) -> Result<AgentLifecycleDescriptor, AgentLifecycleControlError> {
        let deadline = Instant::now() + self.timeout;
        loop {
            match self.describe() {
                Ok(lifecycle)
                    if lifecycle.process_instance_id != previous_process_instance_id
                        && lifecycle.state == "accepting"
                        && lifecycle.accepting_new_work =>
                {
                    return Ok(lifecycle);
                }
                Ok(_) => {}
                Err(error) if error.retryable => {}
                Err(error) => return Err(error),
            }
            if Instant::now() >= deadline {
                return Err(retryable_error(
                    "agent_replacement_timeout",
                    format!(
                        "agent {} did not expose a new accepting process instance within {} ms",
                        self.address,
                        self.timeout.as_millis()
                    ),
                ));
            }
            thread::sleep(self.poll_interval);
        }
    }

    fn lifecycle_request(
        &self,
        method: RpcMethod,
        params: Value,
    ) -> Result<AgentLifecycleDescriptor, AgentLifecycleControlError> {
        let response = self.rpc_request(method, params)?;
        if !response.ok {
            let error = response.error.ok_or_else(|| {
                control_error(
                    "invalid_agent_response",
                    "failed Agent lifecycle response omitted its error",
                )
            })?;
            return Err(AgentLifecycleControlError {
                retryable: retryable_code(&error.code),
                code: error.code,
                message: error.message,
            });
        }
        let lifecycle: AgentLifecycleDescriptor =
            serde_json::from_value(response.result.ok_or_else(|| {
                control_error(
                    "invalid_agent_response",
                    "successful Agent lifecycle response omitted its result",
                )
            })?)
            .map_err(|error| {
                control_error(
                    "invalid_agent_lifecycle",
                    format!("Agent lifecycle result is invalid: {error}"),
                )
            })?;
        validate_agent_lifecycle_descriptor(&lifecycle).map_err(|errors| {
            control_error(
                "invalid_agent_lifecycle",
                errors
                    .into_iter()
                    .map(|error| format!("{}: {}", error.code, error.message))
                    .collect::<Vec<_>>()
                    .join("; "),
            )
        })?;
        if lifecycle.state == "unavailable" {
            return Err(retryable_error(
                "agent_lifecycle_unavailable",
                "Agent lifecycle state is unavailable",
            ));
        }
        Ok(lifecycle)
    }

    fn rpc_request(
        &self,
        method: RpcMethod,
        params: Value,
    ) -> Result<RpcResponse, AgentLifecycleControlError> {
        let request = RpcRequest {
            rpc_version: RPC_VERSION,
            id: next_request_id(),
            method,
            params,
        };
        let payload = serde_json::to_vec(&request).map_err(|error| {
            control_error(
                "agent_request_encode_failed",
                format!("failed to encode Agent lifecycle request: {error}"),
            )
        })?;
        let length = u32::try_from(payload.len()).map_err(|_| {
            control_error(
                "agent_request_too_large",
                "Agent lifecycle request exceeds the transport frame limit",
            )
        })?;
        let mut stream =
            TcpStream::connect_timeout(&self.address, self.timeout.min(Duration::from_secs(2)))
                .map_err(|error| {
                    retryable_error(
                        "agent_unreachable",
                        format!("failed to connect to Agent {}: {error}", self.address),
                    )
                })?;
        stream
            .set_read_timeout(Some(self.timeout))
            .and_then(|_| stream.set_write_timeout(Some(self.timeout)))
            .map_err(|error| {
                retryable_error(
                    "agent_transport_failed",
                    format!("failed to configure Agent lifecycle transport: {error}"),
                )
            })?;
        stream
            .write_all(&length.to_be_bytes())
            .and_then(|_| stream.write_all(&payload))
            .map_err(|error| {
                retryable_error(
                    "agent_transport_failed",
                    format!("failed to send Agent lifecycle request: {error}"),
                )
            })?;

        loop {
            let value = read_json_frame(&mut stream)?;
            if value.get("event").and_then(Value::as_str) == Some("progress") {
                continue;
            }
            let response: RpcResponse = serde_json::from_value(value).map_err(|error| {
                control_error(
                    "invalid_agent_response",
                    format!("failed to decode Agent lifecycle response: {error}"),
                )
            })?;
            validate_rpc_response_envelope(&response).map_err(|error| {
                control_error(
                    "invalid_agent_response",
                    format!("{}: {}", error.code.as_str(), error.message),
                )
            })?;
            if response.id != request.id {
                return Err(control_error(
                    "agent_response_id_mismatch",
                    "Agent lifecycle response id does not match its request",
                ));
            }
            return Ok(response);
        }
    }
}

impl AgentLifecycleControl for AgentLifecycleClient {
    fn describe(&self) -> Result<AgentLifecycleDescriptor, AgentLifecycleControlError> {
        AgentLifecycleClient::describe(self)
    }

    fn begin_drain(
        &self,
        controller_id: &str,
        reason: &str,
    ) -> Result<AgentLifecycleDescriptor, AgentLifecycleControlError> {
        AgentLifecycleClient::begin_drain(self, controller_id, reason)
    }

    fn wait_until_quiescent(
        &self,
        controller_id: &str,
        drain_generation: u64,
    ) -> Result<AgentLifecycleDescriptor, AgentLifecycleControlError> {
        AgentLifecycleClient::wait_until_quiescent(self, controller_id, drain_generation)
    }

    fn wait_until_replaced(
        &self,
        previous_process_instance_id: &str,
    ) -> Result<AgentLifecycleDescriptor, AgentLifecycleControlError> {
        AgentLifecycleClient::wait_until_replaced(self, previous_process_instance_id)
    }

    fn resume_admission(
        &self,
        controller_id: &str,
        drain_generation: u64,
    ) -> Result<AgentLifecycleDescriptor, AgentLifecycleControlError> {
        AgentLifecycleClient::resume_admission(self, controller_id, drain_generation)
    }
}

fn read_json_frame(stream: &mut TcpStream) -> Result<Value, AgentLifecycleControlError> {
    let mut header = [0_u8; 4];
    stream.read_exact(&mut header).map_err(|error| {
        retryable_error(
            "agent_transport_failed",
            format!("failed to read Agent lifecycle frame header: {error}"),
        )
    })?;
    let length = u32::from_be_bytes(header) as usize;
    if length == 0 || length > MAX_AGENT_LIFECYCLE_FRAME_BYTES {
        return Err(control_error(
            "invalid_agent_frame_length",
            format!(
                "Agent lifecycle frame length must be within 1..={MAX_AGENT_LIFECYCLE_FRAME_BYTES}"
            ),
        ));
    }
    let mut payload = vec![0_u8; length];
    stream.read_exact(&mut payload).map_err(|error| {
        retryable_error(
            "agent_transport_failed",
            format!("failed to read Agent lifecycle frame payload: {error}"),
        )
    })?;
    serde_json::from_slice(&payload).map_err(|error| {
        control_error(
            "invalid_agent_response",
            format!("Agent lifecycle frame is not valid JSON: {error}"),
        )
    })
}

fn serialize_params(params: impl Serialize) -> Result<Value, AgentLifecycleControlError> {
    serde_json::to_value(params).map_err(|error| {
        control_error(
            "agent_request_encode_failed",
            format!("failed to encode Agent lifecycle parameters: {error}"),
        )
    })
}

fn next_request_id() -> String {
    static NEXT_REQUEST: AtomicU64 = AtomicU64::new(1);
    format!(
        "installer-agent-lifecycle-{}-{}",
        std::process::id(),
        NEXT_REQUEST.fetch_add(1, Ordering::Relaxed)
    )
}

fn retryable_code(code: &str) -> bool {
    matches!(
        code,
        "agent_unreachable"
            | "agent_transport_failed"
            | "agent_lifecycle_unavailable"
            | "agent_drain_timeout"
            | "agent_replacement_timeout"
    )
}

fn control_error(code: &str, message: impl Into<String>) -> AgentLifecycleControlError {
    AgentLifecycleControlError {
        code: code.to_string(),
        message: message.into(),
        retryable: false,
    }
}

fn retryable_error(code: &str, message: impl Into<String>) -> AgentLifecycleControlError {
    AgentLifecycleControlError {
        code: code.to_string(),
        message: message.into(),
        retryable: true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kyuubiki_protocol::AGENT_LIFECYCLE_SCHEMA;
    use std::net::TcpListener;

    #[test]
    fn client_reads_and_validates_a_framed_lifecycle_response() {
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let address = listener.local_addr().unwrap();
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let request = read_test_frame(&mut stream);
            assert_eq!(request["method"], "describe_agent_lifecycle");
            let response = RpcResponse::success(
                request["id"].as_str().unwrap(),
                serde_json::to_value(AgentLifecycleDescriptor::default()).unwrap(),
            );
            write_test_frame(&mut stream, &response);
        });

        let client = AgentLifecycleClient::new(address, Duration::from_secs(2)).unwrap();
        let lifecycle = client.describe().unwrap();
        assert_eq!(lifecycle.schema_version, AGENT_LIFECYCLE_SCHEMA);
        assert_eq!(lifecycle.state, "accepting");
        server.join().unwrap();
    }

    #[test]
    fn client_rejects_an_inconsistent_safe_to_replace_claim() {
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let address = listener.local_addr().unwrap();
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let request = read_test_frame(&mut stream);
            let invalid = AgentLifecycleDescriptor {
                state: "quiescent".to_string(),
                safe_to_replace: true,
                quiescent: true,
                active_execution_count: 1,
                ..AgentLifecycleDescriptor::default()
            };
            write_test_frame(
                &mut stream,
                &RpcResponse::success(
                    request["id"].as_str().unwrap(),
                    serde_json::to_value(invalid).unwrap(),
                ),
            );
        });

        let client = AgentLifecycleClient::new(address, Duration::from_secs(2)).unwrap();
        let error = client.describe().unwrap_err();
        assert_eq!(error.code, "invalid_agent_lifecycle");
        assert!(!error.retryable);
        server.join().unwrap();
    }

    fn read_test_frame(stream: &mut TcpStream) -> Value {
        let mut header = [0_u8; 4];
        stream.read_exact(&mut header).unwrap();
        let mut payload = vec![0_u8; u32::from_be_bytes(header) as usize];
        stream.read_exact(&mut payload).unwrap();
        serde_json::from_slice(&payload).unwrap()
    }

    fn write_test_frame(stream: &mut TcpStream, value: &impl Serialize) {
        let payload = serde_json::to_vec(value).unwrap();
        stream
            .write_all(&u32::try_from(payload.len()).unwrap().to_be_bytes())
            .unwrap();
        stream.write_all(&payload).unwrap();
    }
}
