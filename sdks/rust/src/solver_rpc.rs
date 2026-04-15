use crate::error::{SdkError, SdkResult};
use serde_json::{Value, json};
use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;

pub struct SolverRpcClient {
    host: String,
    port: u16,
    timeout: Duration,
}

pub struct RpcCallOutcome {
    pub result: Value,
    pub progress_frames: Vec<Value>,
}

impl SolverRpcClient {
    pub fn new(host: impl Into<String>, port: u16) -> Self {
        Self {
            host: host.into(),
            port,
            timeout: Duration::from_secs(15),
        }
    }

    pub fn ping(&self) -> SdkResult<RpcCallOutcome> {
        self.call("ping", json!({}))
    }

    pub fn describe_agent(&self) -> SdkResult<RpcCallOutcome> {
        self.call("describe_agent", json!({}))
    }

    pub fn solve_bar_1d(&self, payload: Value) -> SdkResult<RpcCallOutcome> {
        self.call("solve_bar_1d", payload)
    }

    pub fn solve_truss_2d(&self, payload: Value) -> SdkResult<RpcCallOutcome> {
        self.call("solve_truss_2d", payload)
    }

    pub fn solve_truss_3d(&self, payload: Value) -> SdkResult<RpcCallOutcome> {
        self.call("solve_truss_3d", payload)
    }

    pub fn solve_plane_triangle_2d(&self, payload: Value) -> SdkResult<RpcCallOutcome> {
        self.call("solve_plane_triangle_2d", payload)
    }

    pub fn cancel_job(&self, job_id: &str) -> SdkResult<RpcCallOutcome> {
        self.call("cancel_job", json!({ "job_id": job_id }))
    }

    pub fn call(&self, method: &str, params: Value) -> SdkResult<RpcCallOutcome> {
        let mut stream = TcpStream::connect((self.host.as_str(), self.port))?;
        stream.set_read_timeout(Some(self.timeout))?;
        stream.set_write_timeout(Some(self.timeout))?;

        let request_id = format!("rust-sdk-{}", std::time::SystemTime::now().elapsed().map(|value| value.as_nanos()).unwrap_or(0));
        let payload = serde_json::to_vec(&json!({
            "rpc_version": 1,
            "id": request_id,
            "method": method,
            "params": params,
        }))?;
        let len = (payload.len() as u32).to_be_bytes();
        stream.write_all(&len)?;
        stream.write_all(&payload)?;

        let mut progress_frames = Vec::new();
        loop {
            let mut header = [0_u8; 4];
            stream.read_exact(&mut header)?;
            let size = u32::from_be_bytes(header) as usize;
            let mut frame = vec![0_u8; size];
            stream.read_exact(&mut frame)?;
            let decoded: Value = serde_json::from_slice(&frame)?;
            if decoded.get("event").is_some() {
                progress_frames.push(decoded);
                continue;
            }
            if decoded.get("ok") == Some(&Value::Bool(true)) {
                return Ok(RpcCallOutcome {
                    result: decoded.get("result").cloned().unwrap_or(Value::Null),
                    progress_frames,
                });
            }
            let error = decoded.get("error");
            let message = error
                .and_then(|error| error.get("message"))
                .and_then(Value::as_str)
                .unwrap_or("rpc failed");
            let code = error
                .and_then(|error| error.get("code"))
                .and_then(Value::as_str)
                .map(str::to_string);
            return Err(SdkError::Rpc {
                message: message.to_string(),
                code,
            });
        }
    }
}
