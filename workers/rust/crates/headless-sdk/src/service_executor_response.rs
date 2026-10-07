use crate::HeadlessExecutorError;
use crate::service_executor_http::decode_http_response_body;
use serde_json::Value;

pub(crate) const OUTCOME_UNKNOWN: &str = "service_request_outcome_unknown:";

enum ResponseFailure {
    Protocol(HeadlessExecutorError),
    Rejected(u16, HeadlessExecutorError),
}

impl From<HeadlessExecutorError> for ResponseFailure {
    fn from(error: HeadlessExecutorError) -> Self {
        Self::Protocol(error)
    }
}

pub(crate) fn after_send_failure(
    method: &str,
    error: HeadlessExecutorError,
) -> HeadlessExecutorError {
    if matches!(method, "GET" | "HEAD" | "OPTIONS") {
        return error;
    }
    HeadlessExecutorError {
        message: format!(
            "{OUTCOME_UNKNOWN} the request may have taken effect; {}",
            error.message
        ),
    }
}

pub(crate) fn parse_acknowledgement(
    method: &str,
    response: &str,
    path: &str,
) -> Result<Value, HeadlessExecutorError> {
    match parse_response(response, path) {
        Ok(value) => Ok(value),
        Err(ResponseFailure::Protocol(error)) => Err(after_send_failure(method, error)),
        Err(ResponseFailure::Rejected(status, error)) if status >= 500 => {
            Err(after_send_failure(method, error))
        }
        Err(ResponseFailure::Rejected(_, error)) => Err(error),
    }
}

#[cfg(test)]
pub(crate) fn parse_json_response(
    response: &str,
    path: &str,
) -> Result<Value, HeadlessExecutorError> {
    parse_response(response, path).map_err(|error| match error {
        ResponseFailure::Protocol(error) | ResponseFailure::Rejected(_, error) => error,
    })
}

fn parse_response(response: &str, path: &str) -> Result<Value, ResponseFailure> {
    let protocol = |detail| HeadlessExecutorError {
        message: format!("invalid HTTP response for {path}: {detail}"),
    };
    let (head, body) = response
        .split_once("\r\n\r\n")
        .ok_or_else(|| protocol("missing headers"))?;
    let mut start = head.lines().next().unwrap_or_default().split_whitespace();
    if !matches!(start.next(), Some("HTTP/1.0" | "HTTP/1.1")) {
        return Err(protocol("unsupported status line").into());
    }
    let status_code = start
        .next()
        .and_then(|status| status.parse::<u16>().ok())
        .filter(|status| (100..=599).contains(status))
        .ok_or_else(|| protocol("invalid status code"))?;
    let length = response_body_length(head, path)?;
    if length.is_some_and(|length| length != body.len()) {
        return Err(protocol("body length does not match content length").into());
    }
    let body = decode_http_response_body(head, body, path)?;
    if !(200..300).contains(&status_code) {
        let payload = parse_error_payload(&body);
        return Err(ResponseFailure::Rejected(
            status_code,
            HeadlessExecutorError {
                message: service_error_message(status_code, path, &payload),
            },
        ));
    }
    if body.trim().is_empty() {
        return if status_code == 204 {
            Ok(Value::Null)
        } else {
            Err(protocol("missing JSON acknowledgement").into())
        };
    }
    serde_json::from_str(&body).map_err(|error| {
        ResponseFailure::Protocol(HeadlessExecutorError {
            message: format!("failed to parse JSON response for {path}: {error}"),
        })
    })
}

pub(crate) fn response_body_length(
    head: &str,
    path: &str,
) -> Result<Option<usize>, HeadlessExecutorError> {
    let protocol = |detail| HeadlessExecutorError {
        message: format!("invalid HTTP response for {path}: {detail}"),
    };
    let mut length = None;
    let mut transfer_encoding = false;
    for line in head.lines().skip(1) {
        let Some((name, value)) = line.split_once(':') else {
            continue;
        };
        if name.eq_ignore_ascii_case("content-length") {
            if length.is_some() {
                return Err(protocol("duplicate content length"));
            }
            let value = value.trim();
            if value.is_empty() || !value.bytes().all(|byte| byte.is_ascii_digit()) {
                return Err(protocol("invalid content length"));
            }
            length = Some(
                value
                    .parse::<usize>()
                    .map_err(|_| protocol("invalid content length"))?,
            );
        }
        transfer_encoding |= name.eq_ignore_ascii_case("transfer-encoding");
    }
    if length.is_some() && transfer_encoding {
        return Err(protocol("contradictory body framing"));
    }
    Ok(length)
}

fn parse_error_payload(body: &str) -> Value {
    let trimmed = body.trim();
    if trimmed.is_empty() {
        Value::Null
    } else {
        serde_json::from_str(trimmed).unwrap_or_else(|_| Value::String(trimmed.to_string()))
    }
}

pub(crate) fn service_error_message(status_code: u16, path: &str, payload: &Value) -> String {
    if path == "/api/v1/model-artifacts" && matches!(status_code, 413 | 500) {
        return format!(
            "frontend_proxy_artifact_limit: model artifact upload failed {status_code}: {payload}; connect headless directly to the runtime control-plane endpoint (default http://127.0.0.1:4000), not a frontend proxy with a smaller body limit"
        );
    }
    if path == "/api/v1/model-artifacts" {
        return format!("model artifact upload failed {status_code}: {payload}");
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
