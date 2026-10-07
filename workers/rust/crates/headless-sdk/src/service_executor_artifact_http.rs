use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;
use std::time::Instant;

use crate::HeadlessExecutorError;
use crate::service_executor::{parse_http_url, sanitize_header_value, sanitize_request_path};
use crate::service_executor_deadline::{read_service_response, write_before_deadline};
use crate::service_executor_http::{ARTIFACT_IO_TIMEOUT, connect_service_stream_with_deadline};
use crate::service_executor_response::{after_send_failure, parse_acknowledgement};

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
    let deadline = Instant::now() + ARTIFACT_IO_TIMEOUT;
    let mut stream = connect_service_stream_with_deadline(
        &endpoint.host,
        endpoint.port,
        ARTIFACT_IO_TIMEOUT,
        "model artifact upload",
        Some(deadline),
    )?;
    let mut head = format!(
        "{method} {request_path} HTTP/1.1\r\nHost: {}\r\nAccept: application/json\r\nConnection: close\r\nContent-Type: {content_type}\r\nContent-Length: {body_len}\r\n",
        endpoint.host
    );
    if let Some(token) = api_token {
        head.push_str(&format!("Authorization: Bearer {token}\r\n"));
    }
    head.push_str("\r\n");
    write_before_deadline(&mut stream, head.as_bytes(), deadline, ARTIFACT_IO_TIMEOUT)
        .map_err(|error| after_send_failure(method, error))?;
    let mut sent = 0;
    let mut buffer = [0; 64 * 1024];
    loop {
        let length = body
            .read(&mut buffer)
            .map_err(|error| after_send_failure(method, upload_error("body", error)))?;
        if length == 0 {
            break;
        }
        write_before_deadline(
            &mut stream,
            &buffer[..length],
            deadline,
            ARTIFACT_IO_TIMEOUT,
        )
        .map_err(|error| after_send_failure(method, error))?;
        sent += length as u64;
    }
    if sent != body_len {
        return Err(after_send_failure(
            method,
            HeadlessExecutorError {
                message: format!(
                    "model artifact upload ended early: sent_bytes={sent} expected_bytes={body_len}"
                ),
            },
        ));
    }
    stream
        .flush()
        .map_err(|error| after_send_failure(method, upload_error("body", error)))?;
    let response = read_service_response(&mut stream, &request_path, deadline, ARTIFACT_IO_TIMEOUT)
        .map_err(|error| after_send_failure(method, error))?;
    parse_acknowledgement(method, &response, path)
}

fn upload_error(stage: &str, error: std::io::Error) -> HeadlessExecutorError {
    HeadlessExecutorError {
        message: format!("failed to upload model artifact {stage}: {error}"),
    }
}
