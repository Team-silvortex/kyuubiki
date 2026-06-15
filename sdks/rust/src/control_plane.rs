use crate::auth::KyuubikiAuth;
use crate::error::{SdkError, SdkResult};
use serde_json::Value;
use std::io::{Read, Write};
use std::net::TcpStream;

const FEM_JOB_PATHS: &[(&str, &str)] = &[
    ("bar_1d", "/api/v1/fem/axial-bar/jobs"),
    ("thermal_bar_1d", "/api/v1/fem/thermal-bar-1d/jobs"),
    ("heat_bar_1d", "/api/v1/fem/heat-bar-1d/jobs"),
    ("electrostatic_bar_1d", "/api/v1/fem/electrostatic-bar-1d/jobs"),
    ("beam_1d", "/api/v1/fem/beam-1d/jobs"),
    ("thermal_beam_1d", "/api/v1/fem/thermal-beam-1d/jobs"),
    ("torsion_1d", "/api/v1/fem/torsion-1d/jobs"),
    ("spring_1d", "/api/v1/fem/spring-1d/jobs"),
    ("spring_2d", "/api/v1/fem/spring-2d/jobs"),
    ("spring_3d", "/api/v1/fem/spring-3d/jobs"),
    ("truss_2d", "/api/v1/fem/truss-2d/jobs"),
    ("thermal_truss_2d", "/api/v1/fem/thermal-truss-2d/jobs"),
    ("frame_2d", "/api/v1/fem/frame-2d/jobs"),
    ("thermal_frame_2d", "/api/v1/fem/thermal-frame-2d/jobs"),
    ("plane_triangle_2d", "/api/v1/fem/plane-triangle-2d/jobs"),
    ("heat_plane_triangle_2d", "/api/v1/fem/heat-plane-triangle-2d/jobs"),
    ("thermal_plane_triangle_2d", "/api/v1/fem/thermal-plane-triangle-2d/jobs"),
    ("electrostatic_plane_triangle_2d", "/api/v1/fem/electrostatic-plane-triangle-2d/jobs"),
    ("plane_quad_2d", "/api/v1/fem/plane-quad-2d/jobs"),
    ("heat_plane_quad_2d", "/api/v1/fem/heat-plane-quad-2d/jobs"),
    ("thermal_plane_quad_2d", "/api/v1/fem/thermal-plane-quad-2d/jobs"),
    ("electrostatic_plane_quad_2d", "/api/v1/fem/electrostatic-plane-quad-2d/jobs"),
    ("truss_3d", "/api/v1/fem/truss-3d/jobs"),
    ("thermal_truss_3d", "/api/v1/fem/thermal-truss-3d/jobs"),
    ("frame_3d", "/api/v1/fem/frame-3d/jobs"),
    ("thermal_frame_3d", "/api/v1/fem/thermal-frame-3d/jobs"),
];

pub struct ControlPlaneClient {
    host: String,
    port: u16,
    base_path: String,
    auth: Option<KyuubikiAuth>,
}

impl ControlPlaneClient {
    pub fn new(base_url: &str) -> SdkResult<Self> {
        Self::new_with_auth(base_url, None)
    }

    pub fn new_with_token(base_url: &str, token: Option<String>) -> SdkResult<Self> {
        Self::new_with_auth(base_url, token.map(KyuubikiAuth::access_token))
    }

    pub fn new_with_auth(base_url: &str, auth: Option<KyuubikiAuth>) -> SdkResult<Self> {
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
            auth,
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

    pub fn list_workflow_catalog(&self) -> SdkResult<Value> {
        self.request_json("GET", "/api/v1/workflows/catalog", None)
    }

    pub fn fetch_workflow_catalog_workflow(&self, workflow_id: &str) -> SdkResult<Value> {
        self.request_json(
            "GET",
            &format!("/api/v1/workflows/catalog/{}", percent_encode(workflow_id)),
            None,
        )
    }

    pub fn list_workflow_operators(&self) -> SdkResult<Value> {
        self.request_json("GET", "/api/v1/operators", None)
    }

    pub fn list_workflow_operators_with_query(&self, query: Option<&[(&str, String)]>) -> SdkResult<Value> {
        let path = append_query("/api/v1/operators", query);
        self.request_json("GET", &path, None)
    }

    pub fn fetch_workflow_operator(&self, operator_id: &str) -> SdkResult<Value> {
        self.request_json("GET", &format!("/api/v1/operators/{}", percent_encode(operator_id)), None)
    }

    pub fn list_jobs(&self) -> SdkResult<Value> {
        self.request_json("GET", "/api/v1/jobs", None)
    }

    pub fn fetch_job(&self, job_id: &str) -> SdkResult<Value> {
        self.request_json("GET", &format!("/api/v1/jobs/{job_id}"), None)
    }

    pub fn update_job(&self, job_id: &str, payload: &Value) -> SdkResult<Value> {
        self.request_json("PATCH", &format!("/api/v1/jobs/{job_id}"), Some(payload))
    }

    pub fn cancel_job(&self, job_id: &str) -> SdkResult<Value> {
        self.request_json("POST", &format!("/api/v1/jobs/{job_id}/cancel"), None)
    }

    pub fn delete_job(&self, job_id: &str) -> SdkResult<Value> {
        self.request_json("DELETE", &format!("/api/v1/jobs/{job_id}"), None)
    }

    pub fn create_axial_bar_job(&self, payload: &Value) -> SdkResult<Value> {
        self.submit_fem_job("bar_1d", payload)
    }

    pub fn create_truss_2d_job(&self, payload: &Value) -> SdkResult<Value> {
        self.submit_fem_job("truss_2d", payload)
    }

    pub fn create_truss_3d_job(&self, payload: &Value) -> SdkResult<Value> {
        self.submit_fem_job("truss_3d", payload)
    }

    pub fn create_plane_triangle_2d_job(&self, payload: &Value) -> SdkResult<Value> {
        self.submit_fem_job("plane_triangle_2d", payload)
    }

    pub fn submit_fem_job(&self, solve_kind: &str, payload: &Value) -> SdkResult<Value> {
        let normalized = normalize_solve_kind(solve_kind);
        let path = FEM_JOB_PATHS
            .iter()
            .find_map(|(kind, path)| (*kind == normalized).then_some(*path))
            .ok_or_else(|| SdkError::Rpc {
                message: format!("unsupported solve kind: {solve_kind}"),
                code: None,
            })?;
        self.request_json("POST", path, Some(payload))
    }

    pub fn submit_workflow_catalog_job(&self, workflow_id: &str, input_artifacts: &Value) -> SdkResult<Value> {
        self.request_json(
            "POST",
            &format!("/api/v1/workflows/catalog/{workflow_id}/jobs"),
            Some(&serde_json::json!({ "input_artifacts": input_artifacts })),
        )
    }

    pub fn submit_workflow_graph_job(&self, graph: &Value, input_artifacts: &Value) -> SdkResult<Value> {
        self.request_json(
            "POST",
            "/api/v1/workflows/graph/jobs",
            Some(&serde_json::json!({ "graph": graph, "input_artifacts": input_artifacts })),
        )
    }

    pub fn list_results(&self) -> SdkResult<Value> {
        self.request_json("GET", "/api/v1/results", None)
    }

    pub fn fetch_result(&self, job_id: &str) -> SdkResult<Value> {
        self.request_json("GET", &format!("/api/v1/results/{job_id}"), None)
    }

    pub fn fetch_result_chunk(&self, job_id: &str, kind: &str, offset: Option<usize>, limit: Option<usize>) -> SdkResult<Value> {
        let mut path = format!("/api/v1/results/{job_id}/chunks/{kind}");
        let mut query = Vec::new();
        if let Some(offset) = offset {
            query.push(format!("offset={offset}"));
        }
        if let Some(limit) = limit {
            query.push(format!("limit={limit}"));
        }
        if !query.is_empty() {
            path.push('?');
            path.push_str(&query.join("&"));
        }
        self.request_json("GET", &path, None)
    }

    pub fn update_result(&self, job_id: &str, result: &Value) -> SdkResult<Value> {
        self.request_json("PATCH", &format!("/api/v1/results/{job_id}"), Some(&serde_json::json!({ "result": result })))
    }

    pub fn delete_result(&self, job_id: &str) -> SdkResult<Value> {
        self.request_json("DELETE", &format!("/api/v1/results/{job_id}"), None)
    }

    pub fn export_database(&self) -> SdkResult<Value> {
        self.request_json("GET", "/api/v1/export/database", None)
    }

    pub fn export_security_events(&self, query: Option<&[(&str, String)]>) -> SdkResult<Value> {
        let path = append_query("/api/v1/export/security-events", query);
        self.request_json("GET", &path, None)
    }

    pub fn export_security_events_csv(&self, query: Option<&[(&str, String)]>) -> SdkResult<String> {
        let path = append_query("/api/v1/export/security-events.csv", query);
        self.request_text("GET", &path)
    }

    fn request_json(&self, method: &str, path: &str, payload: Option<&Value>) -> SdkResult<Value> {
        let request_path = format!("{}{}", self.base_path, path);
        let body = payload.map(serde_json::to_vec).transpose()?.unwrap_or_default();

        let mut request = format!(
            "{method} {request_path} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\nContent-Type: application/json\r\nContent-Length: {}\r\n",
            self.host,
            body.len()
        );
        if let Some(auth) = &self.auth {
            request.push_str(&format!("{}: {}\r\n", auth.header_name, auth.header_value));
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
            .ok_or_else(|| SdkError::Transport("invalid HTTP response".into()))?;

        let status_code = headers
            .split_whitespace()
            .nth(1)
            .and_then(|value| value.parse::<u16>().ok())
            .ok_or_else(|| SdkError::Transport("invalid HTTP status line".into()))?;

        if !(200..300).contains(&status_code) {
            return Err(SdkError::HttpStatus {
                status_code,
                body: body.to_string(),
            });
        }

        Ok(serde_json::from_str(body)?)
    }

    fn request_text(&self, method: &str, path: &str) -> SdkResult<String> {
        let request_path = format!("{}{}", self.base_path, path);
        let mut request = format!(
            "{method} {request_path} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\nContent-Type: application/json\r\nContent-Length: 0\r\n",
            self.host
        );
        if let Some(auth) = &self.auth {
            request.push_str(&format!("{}: {}\r\n", auth.header_name, auth.header_value));
        }
        request.push_str("\r\n");

        let mut stream = TcpStream::connect((self.host.as_str(), self.port))?;
        stream.write_all(request.as_bytes())?;

        let mut response = String::new();
        stream.read_to_string(&mut response)?;
        let (headers, body) = response
            .split_once("\r\n\r\n")
            .ok_or_else(|| SdkError::Transport("invalid HTTP response".into()))?;

        let status_code = headers
            .split_whitespace()
            .nth(1)
            .and_then(|value| value.parse::<u16>().ok())
            .ok_or_else(|| SdkError::Transport("invalid HTTP status line".into()))?;

        if !(200..300).contains(&status_code) {
            return Err(SdkError::HttpStatus {
                status_code,
                body: body.to_string(),
            });
        }

        Ok(body.to_string())
    }
}

fn append_query(path: &str, query: Option<&[(&str, String)]>) -> String {
    let mut built = String::from(path);
    if let Some(query) = query {
        let query = query
            .iter()
            .filter(|(_, value)| !value.is_empty())
            .map(|(key, value)| format!("{}={}", percent_encode(key), percent_encode(value)))
            .collect::<Vec<_>>();
        if !query.is_empty() {
            built.push('?');
            built.push_str(&query.join("&"));
        }
    }
    built
}

fn normalize_solve_kind(kind: &str) -> &str {
    match kind {
        "axial_bar_1d" => "bar_1d",
        other => other,
    }
}

fn percent_encode(value: &str) -> String {
    let mut encoded = String::new();
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => encoded.push(byte as char),
            _ => encoded.push_str(&format!("%{byte:02X}")),
        }
    }
    encoded
}
