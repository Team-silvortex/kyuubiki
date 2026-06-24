use crate::{
    HeadlessExecutor, HeadlessExecutorError, HeadlessExecutorOutcome, direct_fem_submit_route,
};
use serde_json::{Value, json};
use std::io::{Read, Write};
use std::net::TcpStream;
use std::thread;
use std::time::{Duration, Instant};

const TERMINAL_JOB_STATUSES: &[&str] = &["completed", "failed", "cancelled"];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceHeadlessExecutor {
    base_url: String,
    api_token: Option<String>,
}

impl ServiceHeadlessExecutor {
    pub fn new(base_url: &str) -> Self {
        Self::with_token(base_url, None)
    }

    pub fn with_token(base_url: &str, api_token: Option<&str>) -> Self {
        Self {
            base_url: normalize_base_url(base_url),
            api_token: api_token
                .map(str::trim)
                .filter(|token| !token.is_empty())
                .map(ToString::to_string),
        }
    }
}

impl HeadlessExecutor for ServiceHeadlessExecutor {
    fn name(&self) -> &'static str {
        "service"
    }

    fn execute_step(
        &mut self,
        action: &str,
        _step_index: usize,
        payload: &Value,
    ) -> Result<HeadlessExecutorOutcome, HeadlessExecutorError> {
        match action {
            "service_health" => {
                execute_service_health(&self.base_url, self.api_token.as_deref(), payload)
            }
            "solve_bar_1d"
            | "solve_thermal_bar_1d"
            | "solve_heat_bar_1d"
            | "solve_electrostatic_bar_1d"
            | "solve_magnetostatic_bar_1d"
            | "solve_electrostatic_plane_triangle_2d"
            | "solve_electrostatic_plane_quad_2d"
            | "solve_heat_plane_triangle_2d"
            | "solve_heat_plane_quad_2d"
            | "solve_thermal_truss_2d"
            | "solve_thermal_truss_3d"
            | "solve_beam_1d"
            | "solve_thermal_plane_triangle_2d"
            | "solve_thermal_plane_quad_2d"
            | "solve_thermal_beam_1d"
            | "solve_thermal_frame_2d"
            | "solve_thermal_frame_3d"
            | "solve_torsion_1d"
            | "solve_spring_1d"
            | "solve_spring_2d"
            | "solve_spring_3d"
            | "solve_truss_2d"
            | "solve_truss_3d"
            | "solve_plane_triangle_2d"
            | "solve_plane_quad_2d"
            | "solve_frame_2d"
            | "solve_frame_3d" => execute_direct_fem_submit(
                &self.base_url,
                self.api_token.as_deref(),
                action,
                payload,
            ),
            "workflow_submit_catalog" => {
                execute_workflow_submit_catalog(&self.base_url, self.api_token.as_deref(), payload)
            }
            "workflow_submit_graph" => {
                execute_workflow_submit_graph(&self.base_url, self.api_token.as_deref(), payload)
            }
            "job_fetch" => execute_job_fetch(&self.base_url, self.api_token.as_deref(), payload),
            "job_wait" => execute_job_wait(&self.base_url, self.api_token.as_deref(), payload),
            "result_fetch" => {
                execute_result_fetch(&self.base_url, self.api_token.as_deref(), payload)
            }
            other => Err(HeadlessExecutorError {
                message: format!("unsupported service action: {other}"),
            }),
        }
    }
}

fn execute_service_health(
    base_url: &str,
    api_token: Option<&str>,
    payload: &Value,
) -> Result<HeadlessExecutorOutcome, HeadlessExecutorError> {
    let request_path = pick_string(payload, &["path"]).unwrap_or("/api/health");
    let result = request_json(base_url, api_token, "GET", request_path, None)?;
    Ok(HeadlessExecutorOutcome {
        status: "executed".to_string(),
        result,
    })
}

fn execute_direct_fem_submit(
    base_url: &str,
    api_token: Option<&str>,
    action: &str,
    payload: &Value,
) -> Result<HeadlessExecutorOutcome, HeadlessExecutorError> {
    let route = direct_fem_submit_route(action).ok_or_else(|| HeadlessExecutorError {
        message: format!("unsupported FEM solve action: {action}"),
    })?;
    let request_body = payload
        .get("model")
        .cloned()
        .unwrap_or_else(|| payload.clone());
    let result = request_json(base_url, api_token, "POST", route, Some(request_body))?;
    Ok(HeadlessExecutorOutcome {
        status: "executed".to_string(),
        result: normalize_job_submission_result(result),
    })
}

fn execute_workflow_submit_catalog(
    base_url: &str,
    api_token: Option<&str>,
    payload: &Value,
) -> Result<HeadlessExecutorOutcome, HeadlessExecutorError> {
    let workflow_id = required_string(payload, &["workflow_id", "workflowId"])?;
    let result = request_json(
        base_url,
        api_token,
        "POST",
        &format!("/api/v1/workflows/catalog/{workflow_id}/jobs"),
        Some(json!({
            "input_artifacts": payload.get("input_artifacts").cloned().unwrap_or_else(|| json!({}))
        })),
    )?;
    Ok(HeadlessExecutorOutcome {
        status: "executed".to_string(),
        result: normalize_job_submission_result(result),
    })
}

fn execute_workflow_submit_graph(
    base_url: &str,
    api_token: Option<&str>,
    payload: &Value,
) -> Result<HeadlessExecutorOutcome, HeadlessExecutorError> {
    let graph = payload
        .get("graph")
        .cloned()
        .ok_or_else(|| HeadlessExecutorError {
            message: "workflow_submit_graph requires graph".to_string(),
        })?;
    let result = request_json(
        base_url,
        api_token,
        "POST",
        "/api/v1/workflows/graph/jobs",
        Some(json!({
            "graph": graph,
            "input_artifacts": payload.get("input_artifacts").cloned().unwrap_or_else(|| json!({}))
        })),
    )?;
    Ok(HeadlessExecutorOutcome {
        status: "executed".to_string(),
        result: normalize_job_submission_result(result),
    })
}

fn execute_job_fetch(
    base_url: &str,
    api_token: Option<&str>,
    payload: &Value,
) -> Result<HeadlessExecutorOutcome, HeadlessExecutorError> {
    let job_id = required_string(payload, &["job_id", "jobId"])?;
    let result = request_json(
        base_url,
        api_token,
        "GET",
        &format!("/api/v1/jobs/{job_id}"),
        None,
    )?;
    Ok(HeadlessExecutorOutcome {
        status: "executed".to_string(),
        result: normalize_job_state_result(result),
    })
}

fn execute_job_wait(
    base_url: &str,
    api_token: Option<&str>,
    payload: &Value,
) -> Result<HeadlessExecutorOutcome, HeadlessExecutorError> {
    let job_id = required_string(payload, &["job_id", "jobId"])?;
    let interval_ms = pick_u64(payload, &["interval_ms", "intervalMs"]).unwrap_or(1000);
    let timeout_ms = pick_u64(payload, &["timeout_ms", "timeoutMs"]).unwrap_or(60000);
    let deadline = Instant::now() + Duration::from_millis(timeout_ms);
    loop {
        let result = request_json(
            base_url,
            api_token,
            "GET",
            &format!("/api/v1/jobs/{job_id}"),
            None,
        )?;
        let normalized = normalize_job_state_result(result);
        let terminal = normalized
            .get("status")
            .and_then(Value::as_str)
            .is_some_and(|status| TERMINAL_JOB_STATUSES.contains(&status));
        if terminal {
            return Ok(HeadlessExecutorOutcome {
                status: "executed".to_string(),
                result: normalized,
            });
        }
        if Instant::now() >= deadline {
            return Err(HeadlessExecutorError {
                message: format!("timed out waiting for job {job_id}"),
            });
        }
        thread::sleep(Duration::from_millis(interval_ms));
    }
}

fn execute_result_fetch(
    base_url: &str,
    api_token: Option<&str>,
    payload: &Value,
) -> Result<HeadlessExecutorOutcome, HeadlessExecutorError> {
    let job_id = required_string(payload, &["job_id", "jobId"])?;
    let prefer_job_result = payload
        .get("prefer_job_result")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    if prefer_job_result {
        let envelope = request_json(
            base_url,
            api_token,
            "GET",
            &format!("/api/v1/jobs/{job_id}"),
            None,
        )?;
        let normalized = normalize_job_state_result(envelope);
        if normalized.get("result").is_some() {
            return Ok(HeadlessExecutorOutcome {
                status: "executed".to_string(),
                result: normalized,
            });
        }
    }
    let result = request_json(
        base_url,
        api_token,
        "GET",
        &format!("/api/v1/results/{job_id}"),
        None,
    )?;
    Ok(HeadlessExecutorOutcome {
        status: "executed".to_string(),
        result: normalize_result_fetch_result(job_id, result),
    })
}

fn normalize_job_submission_result(result: Value) -> Value {
    let Some(job) = result.get("job").and_then(Value::as_object) else {
        return result;
    };
    json!({
        "job_id": job.get("job_id").cloned().unwrap_or(Value::Null),
        "status": job.get("status").cloned().unwrap_or(Value::Null),
        "progress": job.get("progress").cloned().unwrap_or(Value::Null),
        "job": result.get("job").cloned().unwrap_or(Value::Null),
        "raw": result,
    })
}

fn normalize_job_state_result(result: Value) -> Value {
    let Some(job) = result.get("job").and_then(Value::as_object) else {
        return result;
    };
    json!({
        "job_id": job.get("job_id").cloned().unwrap_or(Value::Null),
        "status": job.get("status").cloned().unwrap_or(Value::Null),
        "progress": job.get("progress").cloned().unwrap_or(Value::Null),
        "result": result.get("result").cloned().unwrap_or(Value::Null),
        "job": result.get("job").cloned().unwrap_or(Value::Null),
        "raw": result,
    })
}

fn normalize_result_fetch_result(job_id: &str, result: Value) -> Value {
    json!({
        "job_id": job_id,
        "result": result,
        "raw": result,
    })
}

fn request_json(
    base_url: &str,
    api_token: Option<&str>,
    method: &str,
    path: &str,
    body: Option<Value>,
) -> Result<Value, HeadlessExecutorError> {
    let endpoint = parse_http_url(base_url)?;
    let request_path = if path.starts_with('/') {
        path.to_string()
    } else {
        format!("/{path}")
    };
    let body_text = body
        .map(|value| serde_json::to_string(&value))
        .transpose()
        .map_err(|error| HeadlessExecutorError {
            message: error.to_string(),
        })?;
    let mut stream =
        TcpStream::connect((endpoint.host.as_str(), endpoint.port)).map_err(|error| {
            HeadlessExecutorError {
                message: format!(
                    "failed to connect to {}:{}: {error}",
                    endpoint.host, endpoint.port
                ),
            }
        })?;
    let request = build_request(
        method,
        &endpoint.host,
        &request_path,
        body_text.as_deref(),
        api_token,
    );
    stream
        .write_all(request.as_bytes())
        .map_err(|error| HeadlessExecutorError {
            message: format!("failed to write request: {error}"),
        })?;
    let mut response = String::new();
    stream
        .read_to_string(&mut response)
        .map_err(|error| HeadlessExecutorError {
            message: format!("failed to read response: {error}"),
        })?;
    parse_json_response(&response, path)
}

fn build_request(
    method: &str,
    host: &str,
    path: &str,
    body: Option<&str>,
    api_token: Option<&str>,
) -> String {
    let body = body.unwrap_or("");
    let mut request = format!(
        "{method} {path} HTTP/1.1\r\nHost: {host}\r\nAccept: application/json\r\nConnection: close\r\n"
    );
    if let Some(token) = api_token {
        request.push_str(&format!("Authorization: Bearer {token}\r\n"));
    }
    if !body.is_empty() {
        request.push_str("Content-Type: application/json\r\n");
        request.push_str(&format!("Content-Length: {}\r\n", body.len()));
    }
    request.push_str("\r\n");
    request.push_str(body);
    request
}

fn parse_json_response(response: &str, path: &str) -> Result<Value, HeadlessExecutorError> {
    let (head, body) = response
        .split_once("\r\n\r\n")
        .ok_or_else(|| HeadlessExecutorError {
            message: format!("invalid HTTP response for {path}"),
        })?;
    let status_line = head.lines().next().unwrap_or_default();
    let status_code = status_line
        .split_whitespace()
        .nth(1)
        .and_then(|value| value.parse::<u16>().ok())
        .unwrap_or(0);
    let payload = if body.trim().is_empty() {
        Value::Null
    } else {
        serde_json::from_str(body).map_err(|error| HeadlessExecutorError {
            message: format!("failed to parse JSON response for {path}: {error}"),
        })?
    };
    if !(200..300).contains(&status_code) {
        return Err(HeadlessExecutorError {
            message: format!("service request failed {status_code}: {path}"),
        });
    }
    Ok(payload)
}

fn normalize_base_url(base_url: &str) -> String {
    base_url.trim_end_matches('/').to_string()
}

fn required_string<'a>(
    payload: &'a Value,
    keys: &[&str],
) -> Result<&'a str, HeadlessExecutorError> {
    pick_string(payload, keys).ok_or_else(|| HeadlessExecutorError {
        message: format!("missing required payload key {}", keys.join("|")),
    })
}

fn pick_string<'a>(payload: &'a Value, keys: &[&str]) -> Option<&'a str> {
    keys.iter().find_map(|key| {
        payload
            .get(*key)
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
    })
}

fn pick_u64(payload: &Value, keys: &[&str]) -> Option<u64> {
    keys.iter().find_map(|key| {
        payload.get(*key).and_then(|value| {
            value.as_u64().or_else(|| {
                value
                    .as_str()
                    .and_then(|text| text.trim().parse::<u64>().ok())
            })
        })
    })
}

#[derive(Debug)]
struct ParsedHttpUrl {
    host: String,
    port: u16,
}

fn parse_http_url(base_url: &str) -> Result<ParsedHttpUrl, HeadlessExecutorError> {
    let raw = base_url
        .strip_prefix("http://")
        .ok_or_else(|| HeadlessExecutorError {
            message: format!("unsupported base url {base_url}; only http:// is supported"),
        })?;
    let authority = raw.split('/').next().unwrap_or_default();
    let (host, port) = match authority.split_once(':') {
        Some((host, port_text)) => {
            let port = port_text
                .parse::<u16>()
                .map_err(|error| HeadlessExecutorError {
                    message: format!("invalid port in {base_url}: {error}"),
                })?;
            (host.to_string(), port)
        }
        None => (authority.to_string(), 80),
    };
    if host.trim().is_empty() {
        return Err(HeadlessExecutorError {
            message: format!("invalid host in {base_url}"),
        });
    }
    Ok(ParsedHttpUrl { host, port })
}

#[cfg(test)]
#[path = "service_executor_tests.rs"]
mod service_executor_tests;
