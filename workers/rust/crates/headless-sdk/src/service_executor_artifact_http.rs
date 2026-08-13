use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;

use crate::HeadlessExecutorError;
use crate::service_executor::{
    parse_http_url, parse_json_response, sanitize_header_value, sanitize_request_path,
};
use crate::service_executor_http::{ARTIFACT_IO_TIMEOUT, connect_service_stream};

pub(crate) fn request_file(
    base_url: &str,
    api_token: Option<&str>,
    method: &str,
    path: &str,
    content_type: &str,
    body_path: &Path,
) -> Result<serde_json::Value, HeadlessExecutorError> {
    let endpoint = parse_http_url(base_url)?;
    let request_path = sanitize_request_path(path)?;
    let api_token = sanitize_header_value(api_token, "api token")?;
    let content_type = sanitize_header_value(Some(content_type), "content type")?
        .expect("content type is present");
    let mut body = File::open(body_path).map_err(|error| HeadlessExecutorError {
        message: format!("failed to open model artifact: {error}"),
    })?;
    let body_len = body
        .metadata()
        .map_err(|error| HeadlessExecutorError {
            message: format!("failed to inspect model artifact: {error}"),
        })?
        .len();
    let mut stream = connect_service_stream(
        &endpoint.host,
        endpoint.port,
        ARTIFACT_IO_TIMEOUT,
        "model artifact upload",
    )?;
    let mut head = format!(
        "{method} {request_path} HTTP/1.1\r\nHost: {}\r\nAccept: application/json\r\nConnection: close\r\nContent-Type: {content_type}\r\nContent-Length: {body_len}\r\n",
        endpoint.host
    );
    if let Some(token) = api_token {
        head.push_str(&format!("Authorization: Bearer {token}\r\n"));
    }
    head.push_str("\r\n");
    stream
        .write_all(head.as_bytes())
        .map_err(|error| upload_error("headers", error))?;
    let sent =
        std::io::copy(&mut body, &mut stream).map_err(|error| upload_error("body", error))?;
    if sent != body_len {
        return Err(HeadlessExecutorError {
            message: format!(
                "model artifact upload ended early: sent_bytes={sent} expected_bytes={body_len}"
            ),
        });
    }
    stream
        .flush()
        .map_err(|error| upload_error("body", error))?;
    let mut response = String::new();
    stream
        .read_to_string(&mut response)
        .map_err(|error| HeadlessExecutorError {
            message: format!("failed to read model artifact response: {error}"),
        })?;
    parse_json_response(&response, path)
}

fn upload_error(stage: &str, error: std::io::Error) -> HeadlessExecutorError {
    HeadlessExecutorError {
        message: format!("failed to upload model artifact {stage}: {error}"),
    }
}
