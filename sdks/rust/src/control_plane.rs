use crate::error::{SdkError, SdkResult};
use serde_json::Value;
use std::io::{Read, Write};
use std::net::TcpStream;

pub struct ControlPlaneClient {
    host: String,
    port: u16,
    base_path: String,
    token: Option<String>,
}

impl ControlPlaneClient {
    pub fn new(base_url: &str) -> SdkResult<Self> {
        Self::new_with_token(base_url, None)
    }

    pub fn new_with_token(base_url: &str, token: Option<String>) -> SdkResult<Self> {
        let trimmed = base_url.trim_end_matches('/');
        let without_scheme = trimmed
            .strip_prefix("http://")
            .ok_or_else(|| SdkError::InvalidUrl("only http:// URLs are supported by the minimal Rust SDK".into()))?;
        let (host_port, base_path) = match without_scheme.split_once('/') {
            Some((host_port, rest)) => (host_port, format!("/{}", rest)),
            None => (without_scheme, String::new()),
        };
        let (host, port) = match host_port.split_once(':') {
            Some((host, port)) => (host.to_string(), port.parse().map_err(|_| SdkError::InvalidUrl(base_url.into()))?),
            None => (host_port.to_string(), 80),
        };

        Ok(Self {
            host,
            port,
            base_path,
            token,
        })
    }

    pub fn health(&self) -> SdkResult<Value> {
        self.request_json("GET", "/api/health", None)
    }

    pub fn protocol(&self) -> SdkResult<Value> {
        self.request_json("GET", "/api/v1/protocol", None)
    }

    pub fn agents(&self) -> SdkResult<Value> {
        self.request_json("GET", "/api/v1/protocol/agents", None)
    }

    pub fn fetch_job(&self, job_id: &str) -> SdkResult<Value> {
        self.request_json("GET", &format!("/api/v1/jobs/{job_id}"), None)
    }

    pub fn cancel_job(&self, job_id: &str) -> SdkResult<Value> {
        self.request_json("POST", &format!("/api/v1/jobs/{job_id}/cancel"), None)
    }

    pub fn create_axial_bar_job(&self, payload: &Value) -> SdkResult<Value> {
        self.request_json("POST", "/api/v1/fem/axial-bar/jobs", Some(payload))
    }

    pub fn create_truss_2d_job(&self, payload: &Value) -> SdkResult<Value> {
        self.request_json("POST", "/api/v1/fem/truss-2d/jobs", Some(payload))
    }

    pub fn create_truss_3d_job(&self, payload: &Value) -> SdkResult<Value> {
        self.request_json("POST", "/api/v1/fem/truss-3d/jobs", Some(payload))
    }

    pub fn create_plane_triangle_2d_job(&self, payload: &Value) -> SdkResult<Value> {
        self.request_json("POST", "/api/v1/fem/plane-triangle-2d/jobs", Some(payload))
    }

    fn request_json(&self, method: &str, path: &str, payload: Option<&Value>) -> SdkResult<Value> {
        let request_path = format!("{}{}", self.base_path, path);
        let body = payload.map(serde_json::to_vec).transpose()?.unwrap_or_default();

        let mut request = format!(
            "{method} {request_path} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\nContent-Type: application/json\r\nContent-Length: {}\r\n",
            self.host,
            body.len()
        );
        if let Some(token) = &self.token {
            request.push_str(&format!("x-kyuubiki-token: {token}\r\n"));
        }
        request.push_str("\r\n");

        let mut stream = TcpStream::connect((self.host.as_str(), self.port))?;
        stream.write_all(request.as_bytes())?;
        if !body.is_empty() {
            stream.write_all(&body)?;
        }

        let mut response = String::new();
        stream.read_to_string(&mut response)?;
        let (headers, body) = response
            .split_once("\r\n\r\n")
            .ok_or_else(|| SdkError::Http("invalid HTTP response".into()))?;

        if !headers.starts_with("HTTP/1.1 2") && !headers.starts_with("HTTP/1.0 2") {
            return Err(SdkError::Http(body.to_string()));
        }

        Ok(serde_json::from_str(body)?)
    }
}
