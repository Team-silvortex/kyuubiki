use crate::HeadlessExecutorError;
use crate::service_executor_response::response_body_length;
use std::time::Duration;

pub(crate) const RESPONSE_LIMIT: &str = "service_response_limit_exceeded:";
pub(crate) const MAX_RESPONSE_HEADER_BYTES: usize = 64 * 1024;
const MAX_STATUS_RESPONSE_BYTES: usize = 8_000_000;
const MAX_SERVICE_RESPONSE_BYTES: usize = 64 * 1024 * 1024;
const MAX_LARGE_RESPONSE_BYTES: usize = 512 * 1024 * 1024;

pub(crate) fn response_budget(path: &str) -> (usize, &'static str) {
    let path = path.split('?').next().unwrap_or(path);
    match path {
        "/api/v1/operator-tasks/execute" => (MAX_SERVICE_RESPONSE_BYTES, "operator task"),
        "/api/v1/operator-tasks/fetch-dispatch-result" => {
            (10 * 1024 * 1024, "original task result")
        }
        "/api/v1/model-artifacts" => (MAX_STATUS_RESPONSE_BYTES, "model artifact receipt"),
        _ if path.starts_with("/api/v1/jobs/") && path.ends_with("/status") => {
            (MAX_STATUS_RESPONSE_BYTES, "job status")
        }
        _ if ["jobs", "results", "models", "model-versions"]
            .iter()
            .any(|route| single_record_route(path, route)) =>
        {
            (MAX_LARGE_RESPONSE_BYTES, "large model or result")
        }
        _ => (MAX_SERVICE_RESPONSE_BYTES, "service"),
    }
}

fn single_record_route(path: &str, route: &str) -> bool {
    path.strip_prefix("/api/v1/")
        .and_then(|path| path.strip_prefix(route))
        .and_then(|path| path.strip_prefix('/'))
        .is_some_and(|id| !id.is_empty() && !id.contains('/'))
}

pub(crate) fn default_request_timeout(path: &str, io_timeout: Duration) -> Duration {
    if response_budget(path).0 == MAX_LARGE_RESPONSE_BYTES {
        io_timeout.saturating_mul(20)
    } else {
        io_timeout
    }
}

// Check only the header until its delimiter is observed. Never scan a growing
// large body repeatedly, or reserve storage based on an untrusted length.
pub(crate) fn check_response_head(
    response: &[u8],
    scan_from: usize,
    max_bytes: usize,
    context: &str,
) -> Result<bool, HeadlessExecutorError> {
    let Some(end) = response[scan_from..]
        .windows(4)
        .position(|bytes| bytes == b"\r\n\r\n")
        .map(|offset| scan_from + offset + 4)
    else {
        if response.len() > MAX_RESPONSE_HEADER_BYTES {
            return Err(limit_error("HTTP header", MAX_RESPONSE_HEADER_BYTES));
        }
        return Ok(false);
    };
    if end > MAX_RESPONSE_HEADER_BYTES {
        return Err(limit_error("HTTP header", MAX_RESPONSE_HEADER_BYTES));
    }
    let head = std::str::from_utf8(&response[..end - 4]).map_err(|_| HeadlessExecutorError {
        message: "invalid HTTP response: header is not UTF-8".into(),
    })?;
    let remaining = max_bytes
        .checked_sub(end)
        .ok_or_else(|| limit_error(context, max_bytes))?;
    if response_body_length(head, context)?.is_some_and(|length| length > remaining) {
        return Err(limit_error(context, max_bytes));
    }
    Ok(true)
}

pub(crate) fn limit_error(context: &str, max_bytes: usize) -> HeadlessExecutorError {
    HeadlessExecutorError {
        message: format!(
            "{RESPONSE_LIMIT} {context} response exceeds the {max_bytes}-byte transport limit"
        ),
    }
}

#[cfg(test)]
#[path = "service_executor_response_budget_tests.rs"]
mod tests;
