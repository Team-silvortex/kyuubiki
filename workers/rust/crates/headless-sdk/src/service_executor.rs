use crate::{
    HeadlessExecutor, HeadlessExecutorError, HeadlessExecutorOutcome, direct_fem_submit_route,
};
use serde_json::{Map, Value, json};
use std::fmt;
use std::io::IoSlice;

use crate::service_executor_artifact::prepare_direct_fem_request_body;
use crate::service_executor_deadline::{read_service_response, write_parts_before_deadline};
use crate::service_executor_health::with_discovered_solver_endpoints;
use crate::service_executor_http::{REQUEST_IO_TIMEOUT, connect_service_stream_with_deadline};
use crate::service_executor_job_read::JobReadRequest;
use crate::service_executor_job_receipt::{
    submission_context, validate_job_receipt, validate_submission_context,
    validate_submission_receipt,
};
use crate::service_executor_job_wait::{execute_job_wait, reject_unsuccessful_terminal_job};
use crate::service_executor_library::{
    execute_model_create, execute_model_version_create, execute_project_create,
    execute_project_delete, execute_project_update,
};
pub(crate) use crate::service_executor_request::MAX_INLINE_JSON_BYTES;
#[cfg(test)]
use crate::service_executor_request::validate_inline_json_size;
use crate::service_executor_request::{build_request_head, serialize_inline_json};
use crate::service_executor_response::{after_send_failure, parse_acknowledgement};
#[cfg(test)]
use crate::service_executor_response::{parse_json_response, service_error_message};
use crate::service_executor_response_budget::default_request_timeout;
pub(crate) use crate::service_executor_result::execute_result_fetch;
#[cfg(test)]
use crate::service_executor_result::normalize_result_fetch_result;
use crate::service_executor_solve::{
    execute_direct_mesh_solve, execute_solve_and_wait_from_model_version,
    execute_solve_from_model_version,
};
use crate::service_executor_task_budget::{OperatorTaskRequestBudget, execute_operator_task};
use std::time::{Duration, Instant};

#[path = "service_executor_dispatch_cancellation.rs"]
mod dispatch_cancellation;

#[derive(Clone, PartialEq, Eq)]
pub struct ServiceHeadlessExecutor {
    base_url: String,
    api_token: Option<String>,
    operator_task_budget: OperatorTaskRequestBudget,
}

impl fmt::Debug for ServiceHeadlessExecutor {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ServiceHeadlessExecutor")
            .field("base_url", &self.base_url)
            .field("api_token_configured", &self.api_token.is_some())
            .field("operator_task_budget", &self.operator_task_budget)
            .finish()
    }
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
            operator_task_budget: OperatorTaskRequestBudget::default(),
        }
    }

    pub fn try_with_token(
        base_url: &str,
        api_token: Option<&str>,
    ) -> Result<Self, HeadlessExecutorError> {
        let executor = Self::with_token(base_url, api_token);
        parse_http_url(&executor.base_url)?;
        sanitize_header_value(executor.api_token.as_deref(), "api token")?;
        Ok(executor)
    }

    /// Configure synchronous TaskIR requests without extending metadata or job polling timeouts.
    pub fn with_operator_task_budget(mut self, budget: OperatorTaskRequestBudget) -> Self {
        self.operator_task_budget = budget;
        self
    }

    pub fn operator_task_budget(&self) -> OperatorTaskRequestBudget {
        self.operator_task_budget
    }

    /// Retrieve one original TaskIR attempt, without execution or job publication.
    pub fn fetch_operator_task_result(
        &self,
        task: &Value,
        attempt_id: &str,
    ) -> Result<Value, HeadlessExecutorError> {
        crate::service_executor_dispatch_result::fetch(
            &self.base_url,
            self.api_token.as_deref(),
            task,
            attempt_id,
        )
    }

    /// Observe retained dispatches and the original configured Agent without replaying the task.
    /// An absent active request never proves completion, cancellation, or safe rerun.
    pub fn inspect_operator_task_dispatch(
        &self,
        task_id: &str,
        task_digest: &str,
    ) -> Result<Value, HeadlessExecutorError> {
        crate::service_executor_dispatch_inspection::inspect(
            &self.base_url,
            self.api_token.as_deref(),
            task_id,
            task_digest,
        )
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
            "operator_task_execute" => execute_operator_task(
                &self.base_url,
                self.api_token.as_deref(),
                payload,
                self.operator_task_budget,
            ),
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
    let explicit_context = submission_context(payload)?;
    let mut context = submission_context(model)?;
    context.extend(explicit_context);
    let prepared = prepare_direct_fem_request_body(base_url, api_token, model)?;
    let mut request_body = prepared.body;
    if !context.is_empty() {
        request_body
            .as_object_mut()
            .ok_or_else(|| HeadlessExecutorError {
                message: "direct FEM context requires an object model".into(),
            })?
            .extend(context.clone());
    }
    let result = request_json(base_url, api_token, "POST", route, Some(request_body))?;
    let mut result = normalize_job_submission_with_context(result, &context)?;
    result
        .as_object_mut()
        .expect("normalized submission is an object")
        .insert(
            "model_artifact_upload".into(),
            prepared.upload.unwrap_or(Value::Null),
        );
    Ok(HeadlessExecutorOutcome {
        status: "executed".to_string(),
        result,
    })
}

fn execute_composite_panel_submit(
    base_url: &str,
    api_token: Option<&str>,
    payload: &Value,
) -> Result<HeadlessExecutorOutcome, HeadlessExecutorError> {
    let context = submission_context(payload)?;
    let result = request_json(
        base_url,
        api_token,
        "POST",
        "/api/v1/fem/composite-thermo-electric-panel/jobs",
        Some(payload.clone()),
    )?;
    Ok(HeadlessExecutorOutcome {
        status: "executed".to_string(),
        result: normalize_job_submission_with_context(result, &context)?,
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
        result: normalize_job_submission_result(result)?,
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
        result: normalize_job_submission_result(result)?,
    })
}

fn execute_job_fetch(
    base_url: &str,
    api_token: Option<&str>,
    payload: &Value,
) -> Result<HeadlessExecutorOutcome, HeadlessExecutorError> {
    let read = JobReadRequest::parse(payload)?;
    let job_id = read.job_id;
    let result = request_json(
        base_url,
        api_token,
        "GET",
        &format!("/api/v1/jobs/{job_id}"),
        None,
    )?;
    validate_job_receipt(job_id, &result)?;
    read.validate_context(&result)?;
    Ok(HeadlessExecutorOutcome {
        status: "executed".to_string(),
        result: normalize_job_state_result(result),
    })
}

pub(crate) fn normalize_job_submission_result(
    result: Value,
) -> Result<Value, HeadlessExecutorError> {
    normalize_job_submission_with_context(result, &Map::new())
}

pub(crate) fn normalize_job_submission_with_context(
    result: Value,
    context: &Map<String, Value>,
) -> Result<Value, HeadlessExecutorError> {
    // A parsed 2xx reply alone cannot acknowledge a submitted write safely.
    let job =
        validate_submission_receipt(&result).map_err(|error| after_send_failure("POST", error))?;
    validate_submission_context(&result, job, context)
        .map_err(|error| after_send_failure("POST", error))?;
    let normalized = Value::Object(Map::from_iter([
        (
            "job_id".into(),
            job.get("job_id").cloned().unwrap_or(Value::Null),
        ),
        (
            "status".into(),
            job.get("status").cloned().unwrap_or(Value::Null),
        ),
        (
            "progress".into(),
            job.get("progress").cloned().unwrap_or(Value::Null),
        ),
        (
            "job".into(),
            result.get("job").cloned().unwrap_or(Value::Null),
        ),
        ("raw".into(), result),
    ]));
    if matches!(normalized["status"].as_str(), Some("failed" | "cancelled")) {
        let job_id = normalized["job_id"]
            .as_str()
            .expect("validated job identity");
        reject_unsuccessful_terminal_job(job_id, &normalized)?;
    }
    Ok(normalized)
}

pub(crate) fn normalize_job_state_result(result: Value) -> Value {
    let Some(job) = result.get("job").and_then(Value::as_object) else {
        return result;
    };
    // Public mirrors still require independent values. Move the original raw
    // envelope and clone each exposed field once, without reserializing Values.
    Value::Object(Map::from_iter([
        (
            "job_id".into(),
            job.get("job_id").cloned().unwrap_or(Value::Null),
        ),
        (
            "status".into(),
            job.get("status").cloned().unwrap_or(Value::Null),
        ),
        (
            "progress".into(),
            job.get("progress").cloned().unwrap_or(Value::Null),
        ),
        (
            "result".into(),
            result.get("result").cloned().unwrap_or(Value::Null),
        ),
        (
            "job".into(),
            result.get("job").cloned().unwrap_or(Value::Null),
        ),
        ("raw".into(), result),
    ]))
}

pub(crate) fn request_json(
    base_url: &str,
    api_token: Option<&str>,
    method: &str,
    path: &str,
    body: Option<Value>,
) -> Result<Value, HeadlessExecutorError> {
    request_json_with_deadline(base_url, api_token, method, path, body, None)
}

pub(crate) fn request_json_with_deadline(
    base_url: &str,
    api_token: Option<&str>,
    method: &str,
    path: &str,
    body: Option<Value>,
    deadline: Option<Instant>,
) -> Result<Value, HeadlessExecutorError> {
    request_json_with_timeout(
        base_url,
        api_token,
        method,
        path,
        body,
        deadline,
        REQUEST_IO_TIMEOUT,
    )
}

pub(crate) fn request_json_with_timeout(
    base_url: &str,
    api_token: Option<&str>,
    method: &str,
    path: &str,
    body: Option<Value>,
    deadline: Option<Instant>,
    io_timeout: Duration,
) -> Result<Value, HeadlessExecutorError> {
    let endpoint = parse_http_url(base_url)?;
    let request_path = sanitize_request_path(if path.starts_with('/') {
        path.to_string()
    } else {
        format!("/{path}")
    })?;
    let api_token = sanitize_header_value(api_token, "api token")?;
    let body_bytes = body
        .map(|value| serialize_inline_json(&request_path, value))
        .transpose()?
        .unwrap_or_default();
    let deadline = deadline
        .unwrap_or_else(|| Instant::now() + default_request_timeout(&request_path, io_timeout));
    let mut stream = connect_service_stream_with_deadline(
        &endpoint.host,
        endpoint.port,
        io_timeout,
        "service request",
        Some(deadline),
    )?;
    let head = build_request_head(
        method,
        &endpoint.host,
        &request_path,
        body_bytes.len(),
        api_token.as_deref(),
    );
    write_parts_before_deadline(
        &mut stream,
        &mut [IoSlice::new(head.as_bytes()), IoSlice::new(&body_bytes)],
        deadline,
        io_timeout,
    )
    .map_err(|error| after_send_failure(method, error))?;
    drop(body_bytes);
    let response = read_service_response(&mut stream, &request_path, deadline, io_timeout)
        .map_err(|error| after_send_failure(method, error))?;
    parse_acknowledgement(method, &response, path)
}

pub(crate) fn required_path_segment<'a>(
    payload: &'a Value,
    keys: &[&str],
) -> Result<&'a str, HeadlessExecutorError> {
    let value = required_string(payload, keys)?;
    validate_path_segment(value, keys.join("|").as_str())?;
    Ok(value)
}

pub(crate) fn validate_path_segment(value: &str, label: &str) -> Result<(), HeadlessExecutorError> {
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

pub(crate) fn sanitize_request_path<P>(path: P) -> Result<String, HeadlessExecutorError>
where
    P: AsRef<str>,
{
    let path = path.as_ref();
    if !path.starts_with('/') || path.contains('\\') || path.contains("//") || path.contains('%') {
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

pub(crate) fn sanitize_header_value(
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
    if value.is_empty()
        || value.len() > 8 * 1024
        || !value.bytes().all(|byte| byte.is_ascii_graphic())
    {
        return Err(HeadlessExecutorError {
            message: format!("{label} contains unsupported characters or exceeds the size limit"),
        });
    }
    Ok(Some(value.to_string()))
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

#[derive(Debug)]
pub(crate) struct ParsedHttpUrl {
    pub(crate) host: String,
    pub(crate) port: u16,
}

pub(crate) fn parse_http_url(base_url: &str) -> Result<ParsedHttpUrl, HeadlessExecutorError> {
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
            message: "service base URL contains whitespace or control characters".into(),
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

#[cfg(test)]
#[path = "service_executor_security_tests.rs"]
mod service_executor_security_tests;
