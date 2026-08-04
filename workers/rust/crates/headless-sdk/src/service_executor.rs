use crate::{
    HeadlessExecutor, HeadlessExecutorError, HeadlessExecutorOutcome, direct_fem_submit_route,
};
use serde_json::{Value, json};
use std::io::{Read, Write};
use std::thread;
use std::time::{Duration, Instant};

use crate::service_executor_artifact::prepare_direct_fem_request_body;
use crate::service_executor_health::with_discovered_solver_endpoints;
use crate::service_executor_http::{
    ARTIFACT_IO_TIMEOUT, REQUEST_IO_TIMEOUT, connect_service_stream, decode_http_response_body,
};
use crate::service_executor_library::{
    execute_model_create, execute_model_version_create, execute_project_create,
    execute_project_delete, execute_project_update,
};
use crate::service_executor_solve::{
    execute_direct_mesh_solve, execute_solve_and_wait_from_model_version,
    execute_solve_from_model_version,
};

const TERMINAL_JOB_STATUSES: &[&str] = &["completed", "failed", "cancelled"];
pub(crate) const MAX_INLINE_JSON_BYTES: usize = 8_000_000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceHeadlessExecutor {
    base_url: String,
    api_token: Option<String>,
}

impl ServiceHeadlessExecutor {
    pub fn new(base_url: &str) -> Self {
        Self::with_token(base_url, None)
    }

    pub fn try_new(base_url: &str) -> Result<Self, HeadlessExecutorError> {
        Self::try_with_token(base_url, None)
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

    pub fn try_with_token(
        base_url: &str,
        api_token: Option<&str>,
    ) -> Result<Self, HeadlessExecutorError> {
        let executor = Self::with_token(base_url, api_token);
        parse_http_url(&executor.base_url)?;
        Ok(executor)
    }
}

pub fn service_executor_supports_action(action: &str) -> bool {
    matches!(
        action,
        "service_health"
            | "project_create"
            | "project_update"
            | "project_delete"
            | "model_create"
            | "model_version_create"
            | "operator_task_prepare"
            | "operator_task_execute"
            | "workflow_submit_catalog"
            | "workflow_submit_graph"
            | "direct_mesh_solve"
            | "solve_from_model_version"
            | "solve_and_wait_from_model_version"
            | "job_fetch"
            | "job_wait"
            | "result_fetch"
            | "solve_composite_thermo_electric_panel"
    ) || direct_fem_submit_route(action).is_some()
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
            "operator_task_prepare" => {
                execute_operator_task_prepare(&self.base_url, self.api_token.as_deref(), payload)
            }
            "operator_task_execute" => {
                execute_operator_task_execute(&self.base_url, self.api_token.as_deref(), payload)
            }
            "project_create" => {
                execute_project_create(&self.base_url, self.api_token.as_deref(), payload)
            }
            "project_update" => {
                execute_project_update(&self.base_url, self.api_token.as_deref(), payload)
            }
            "project_delete" => {
                execute_project_delete(&self.base_url, self.api_token.as_deref(), payload)
            }
            "model_create" => {
                execute_model_create(&self.base_url, self.api_token.as_deref(), payload)
            }
            "model_version_create" => {
                execute_model_version_create(&self.base_url, self.api_token.as_deref(), payload)
            }
            "solve_composite_thermo_electric_panel" => {
                execute_composite_panel_submit(&self.base_url, self.api_token.as_deref(), payload)
            }
            direct_fem_action if direct_fem_submit_route(direct_fem_action).is_some() => {
                execute_direct_fem_submit(
                    &self.base_url,
                    self.api_token.as_deref(),
                    direct_fem_action,
                    payload,
                )
            }
            "workflow_submit_catalog" => {
                execute_workflow_submit_catalog(&self.base_url, self.api_token.as_deref(), payload)
            }
            "workflow_submit_graph" => {
                execute_workflow_submit_graph(&self.base_url, self.api_token.as_deref(), payload)
            }
            "direct_mesh_solve" => {
                execute_direct_mesh_solve(&self.base_url, self.api_token.as_deref(), payload)
            }
            "solve_from_model_version" => {
                execute_solve_from_model_version(&self.base_url, self.api_token.as_deref(), payload)
            }
            "solve_and_wait_from_model_version" => execute_solve_and_wait_from_model_version(
                &self.base_url,
                self.api_token.as_deref(),
                payload,
            ),
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

fn execute_operator_task_prepare(
    base_url: &str,
    api_token: Option<&str>,
    payload: &Value,
) -> Result<HeadlessExecutorOutcome, HeadlessExecutorError> {
    let result = request_json(
        base_url,
        api_token,
        "POST",
        "/api/v1/operator-tasks/prepare",
        Some(payload.clone()),
    )?;
    Ok(HeadlessExecutorOutcome {
        status: "executed".to_string(),
        result,
    })
}

fn execute_operator_task_execute(
    base_url: &str,
    api_token: Option<&str>,
    payload: &Value,
) -> Result<HeadlessExecutorOutcome, HeadlessExecutorError> {
    let result = request_json(
        base_url,
        api_token,
        "POST",
        "/api/v1/operator-tasks/execute",
        Some(payload.clone()),
    )?;
    Ok(HeadlessExecutorOutcome {
        status: "executed".to_string(),
        result,
    })
}

fn execute_service_health(
    base_url: &str,
    api_token: Option<&str>,
    payload: &Value,
) -> Result<HeadlessExecutorOutcome, HeadlessExecutorError> {
    let request_path =
        sanitize_request_path(pick_string(payload, &["path"]).unwrap_or("/api/health"))?;
    let result = request_json(base_url, api_token, "GET", &request_path, None)?;
    Ok(HeadlessExecutorOutcome {
        status: "executed".to_string(),
        result: with_discovered_solver_endpoints(result),
    })
}

pub(crate) fn execute_direct_fem_submit(
    base_url: &str,
    api_token: Option<&str>,
    action: &str,
    payload: &Value,
) -> Result<HeadlessExecutorOutcome, HeadlessExecutorError> {
    let route = direct_fem_submit_route(action).ok_or_else(|| HeadlessExecutorError {
        message: format!("unsupported FEM solve action: {action}"),
    })?;
    let model = payload.get("model").unwrap_or(payload);
    let request_body = prepare_direct_fem_request_body(base_url, api_token, model)?;
    let result = request_json(base_url, api_token, "POST", route, Some(request_body))?;
    Ok(HeadlessExecutorOutcome {
        status: "executed".to_string(),
        result: normalize_job_submission_result(result),
    })
}

fn execute_composite_panel_submit(
    base_url: &str,
    api_token: Option<&str>,
    payload: &Value,
) -> Result<HeadlessExecutorOutcome, HeadlessExecutorError> {
    let result = request_json(
        base_url,
        api_token,
        "POST",
        "/api/v1/fem/composite-thermo-electric-panel/jobs",
        Some(payload.clone()),
    )?;
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
    let workflow_id = required_path_segment(payload, &["workflow_id", "workflowId"])?;
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
    let job_id = required_path_segment(payload, &["job_id", "jobId"])?;
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

pub(crate) fn execute_job_wait(
    base_url: &str,
    api_token: Option<&str>,
    payload: &Value,
) -> Result<HeadlessExecutorOutcome, HeadlessExecutorError> {
    let job_id = required_path_segment(payload, &["job_id", "jobId"])?;
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
            reject_unsuccessful_terminal_job(job_id, &normalized)?;
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

fn reject_unsuccessful_terminal_job(
    job_id: &str,
    job: &Value,
) -> Result<(), HeadlessExecutorError> {
    let status = job
        .get("status")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    if status == "completed" {
        return Ok(());
    }
    let detail = job
        .get("job")
        .and_then(|value| value.get("message"))
        .and_then(Value::as_str)
        .unwrap_or("service job did not complete successfully");
    Err(HeadlessExecutorError {
        message: format!("service job {job_id} reached terminal status {status}: {detail}"),
    })
}

pub(crate) fn execute_result_fetch(
    base_url: &str,
    api_token: Option<&str>,
    payload: &Value,
) -> Result<HeadlessExecutorOutcome, HeadlessExecutorError> {
    let job_id = required_path_segment(payload, &["job_id", "jobId"])?;
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

pub(crate) fn normalize_job_submission_result(result: Value) -> Value {
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

pub(crate) fn request_json(
    base_url: &str,
    api_token: Option<&str>,
    method: &str,
    path: &str,
    body: Option<Value>,
) -> Result<Value, HeadlessExecutorError> {
    let endpoint = parse_http_url(base_url)?;
    let request_path = sanitize_request_path(if path.starts_with('/') {
        path.to_string()
    } else {
        format!("/{path}")
    })?;
    let api_token = sanitize_header_value(api_token, "api token")?;
    let body_text = body
        .map(|value| serde_json::to_string(&value))
        .transpose()
        .map_err(|error| HeadlessExecutorError {
            message: error.to_string(),
        })?;
    validate_inline_json_size(&request_path, body_text.as_deref().map_or(0, str::len))?;
    let mut stream = connect_service_stream(
        &endpoint.host,
        endpoint.port,
        REQUEST_IO_TIMEOUT,
        "service request",
    )?;
    let request = build_request(
        method,
        &endpoint.host,
        &request_path,
        body_text.as_deref(),
        api_token.as_deref(),
    );
    stream
        .write_all(request.as_bytes())
        .map_err(|error| HeadlessExecutorError {
            message: format!("failed to write service request within timeout: {error}"),
        })?;
    let mut response = String::new();
    stream
        .read_to_string(&mut response)
        .map_err(|error| HeadlessExecutorError {
            message: format!("failed to read service response within timeout: {error}"),
        })?;
    parse_json_response(&response, path)
}

pub(crate) fn request_bytes(
    base_url: &str,
    api_token: Option<&str>,
    method: &str,
    path: &str,
    content_type: &str,
    body: &[u8],
) -> Result<Value, HeadlessExecutorError> {
    let endpoint = parse_http_url(base_url)?;
    let request_path = sanitize_request_path(path)?;
    let api_token = sanitize_header_value(api_token, "api token")?;
    let content_type = sanitize_header_value(Some(content_type), "content type")?
        .expect("content type is present");
    let mut stream = connect_service_stream(
        &endpoint.host,
        endpoint.port,
        ARTIFACT_IO_TIMEOUT,
        "model artifact upload",
    )?;
    let mut head = format!(
        "{method} {request_path} HTTP/1.1\r\nHost: {}\r\nAccept: application/json\r\nConnection: close\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\n",
        endpoint.host,
        body.len()
    );
    if let Some(token) = api_token {
        head.push_str(&format!("Authorization: Bearer {token}\r\n"));
    }
    head.push_str("\r\n");
    stream
        .write_all(head.as_bytes())
        .and_then(|_| stream.write_all(body))
        .map_err(|error| HeadlessExecutorError {
            message: format!("failed to upload model artifact: {error}"),
        })?;
    let mut response = String::new();
    stream
        .read_to_string(&mut response)
        .map_err(|error| HeadlessExecutorError {
            message: format!("failed to read model artifact response: {error}"),
        })?;
    parse_json_response(&response, path)
}

fn validate_inline_json_size(path: &str, size_bytes: usize) -> Result<(), HeadlessExecutorError> {
    if size_bytes <= MAX_INLINE_JSON_BYTES {
        return Ok(());
    }
    Err(HeadlessExecutorError {
        message: format!(
            "service payload exceeds inline JSON transport limit: path={path} size_bytes={size_bytes} limit_bytes={MAX_INLINE_JSON_BYTES}; use a persisted model or artifact reference for large meshes"
        ),
    })
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

pub(crate) fn required_path_segment<'a>(
    payload: &'a Value,
    keys: &[&str],
) -> Result<&'a str, HeadlessExecutorError> {
    let value = required_string(payload, keys)?;
    validate_path_segment(value, keys.join("|").as_str())?;
    Ok(value)
}

fn validate_path_segment(value: &str, label: &str) -> Result<(), HeadlessExecutorError> {
    if value.is_empty()
        || value.starts_with('.')
        || value.contains("..")
        || !value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.'))
    {
        return Err(HeadlessExecutorError {
            message: format!("{label} must be a safe path segment"),
        });
    }
    Ok(())
}

fn sanitize_request_path<P>(path: P) -> Result<String, HeadlessExecutorError>
where
    P: AsRef<str>,
{
    let path = path.as_ref();
    if !path.starts_with('/') || path.contains('\\') || path.contains("//") {
        return Err(HeadlessExecutorError {
            message: format!("invalid request path: {path}"),
        });
    }
    if path
        .chars()
        .any(|ch| ch.is_ascii_control() || ch.is_whitespace())
    {
        return Err(HeadlessExecutorError {
            message: "request path contains unsupported whitespace or control characters"
                .to_string(),
        });
    }
    for segment in path.split('/').filter(|segment| !segment.is_empty()) {
        if segment == "." || segment == ".." {
            return Err(HeadlessExecutorError {
                message: format!("request path escapes route boundary: {path}"),
            });
        }
    }
    Ok(path.to_string())
}

fn sanitize_header_value(
    value: Option<&str>,
    label: &str,
) -> Result<Option<String>, HeadlessExecutorError> {
    let Some(value) = value else {
        return Ok(None);
    };
    if value
        .chars()
        .any(|ch| ch == '\r' || ch == '\n' || ch == '\0')
    {
        return Err(HeadlessExecutorError {
            message: format!("{label} contains unsupported control characters"),
        });
    }
    Ok(Some(value.to_string()))
}

fn parse_json_response(response: &str, path: &str) -> Result<Value, HeadlessExecutorError> {
    let (head, body) = response
        .split_once("\r\n\r\n")
        .ok_or_else(|| HeadlessExecutorError {
            message: format!("invalid HTTP response for {path}"),
        })?;
    let body = decode_http_response_body(head, body, path)?;
    let status_line = head.lines().next().unwrap_or_default();
    let status_code = status_line
        .split_whitespace()
        .nth(1)
        .and_then(|value| value.parse::<u16>().ok())
        .unwrap_or(0);
    if !(200..300).contains(&status_code) {
        let payload = parse_error_payload(&body);
        return Err(HeadlessExecutorError {
            message: service_error_message(status_code, path, &payload),
        });
    }
    if body.trim().is_empty() {
        return Ok(Value::Null);
    }
    serde_json::from_str(&body).map_err(|error| HeadlessExecutorError {
        message: format!("failed to parse JSON response for {path}: {error}"),
    })
}

fn parse_error_payload(body: &str) -> Value {
    let trimmed = body.trim();
    if trimmed.is_empty() {
        Value::Null
    } else {
        serde_json::from_str(trimmed).unwrap_or_else(|_| Value::String(trimmed.to_string()))
    }
}

fn service_error_message(status_code: u16, path: &str, payload: &Value) -> String {
    if path == "/api/v1/model-artifacts" {
        return format!(
            "model artifact upload failed {status_code}: {payload}; connect headless directly to the runtime control-plane endpoint (default http://127.0.0.1:4000), not a frontend proxy with a smaller body limit"
        );
    }
    if status_code == 404 {
        return format!("service action endpoint not deployed (404): {path}: {payload}");
    }
    let Some(error_code) = payload.get("error_code").and_then(Value::as_str) else {
        return format!("service request failed {status_code}: {path}: {payload}");
    };
    let error = payload
        .get("error")
        .map(Value::to_string)
        .unwrap_or_else(|| payload.to_string());
    format!("service request failed {status_code}: {path}: {error_code}: {error}")
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
    if base_url.is_empty() {
        return Err(HeadlessExecutorError {
            message: "service base URL must not be empty".to_string(),
        });
    }
    if base_url
        .chars()
        .any(|ch| ch.is_ascii_control() || ch.is_whitespace())
    {
        return Err(HeadlessExecutorError {
            message: "service base URL contains unsupported whitespace or control characters"
                .to_string(),
        });
    }
    let raw = base_url
        .strip_prefix("http://")
        .ok_or_else(|| HeadlessExecutorError {
            message: format!("unsupported base url {base_url}; only http:// is supported"),
        })?;
    if raw.contains(['/', '?', '#']) {
        return Err(HeadlessExecutorError {
            message: format!(
                "service base URL must contain only scheme and authority; paths, queries, and fragments are not supported: {base_url}"
            ),
        });
    }
    if raw.contains('@') {
        return Err(HeadlessExecutorError {
            message: "service base URL must not contain user information".to_string(),
        });
    }
    let authority = raw;
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
    if port == 0 {
        return Err(HeadlessExecutorError {
            message: format!("invalid port in {base_url}: port must be greater than zero"),
        });
    }
    Ok(ParsedHttpUrl { host, port })
}

#[cfg(test)]
#[path = "service_executor_tests.rs"]
mod service_executor_tests;
