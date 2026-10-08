use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;
use std::time::Instant;

use crate::HeadlessExecutorError;
use crate::service_executor::{parse_http_url, sanitize_header_value, sanitize_request_path};
use crate::service_executor_deadline::{read_service_response, write_before_deadline};
use crate::service_executor_http::{ARTIFACT_IO_TIMEOUT, connect_service_stream_with_deadline};
use crate::service_executor_response::{after_send_failure, parse_acknowledgement};
use sha2::{Digest, Sha256};

#[derive(Debug)]
pub(crate) struct FileUploadAcknowledgement {
    pub envelope: serde_json::Value,
    pub sha256: String,
    pub size_bytes: u64,
}

pub(crate) fn request_file(
    base_url: &str,
    api_token: Option<&str>,
    method: &str,
    path: &str,
    content_type: &str,
    body_path: &Path,
) -> Result<FileUploadAcknowledgement, HeadlessExecutorError> {
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
    let mut digest = Sha256::new();
    let mut buffer = [0; 64 * 1024];
    while sent < body_len {
        // Never send a file's growth beyond the advertised HTTP body length.
        let remaining = (body_len - sent).min(buffer.len() as u64) as usize;
        let length = body
            .read(&mut buffer[..remaining])
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
        digest.update(&buffer[..length]);
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
    Ok(FileUploadAcknowledgement {
        envelope: parse_acknowledgement(method, &response, path)?,
        sha256: format!("{:x}", digest.finalize()),
        size_bytes: sent,
    })
}

fn upload_error(stage: &str, error: std::io::Error) -> HeadlessExecutorError {
    HeadlessExecutorError {
        message: format!("failed to upload model artifact {stage}: {error}"),
    }
}
