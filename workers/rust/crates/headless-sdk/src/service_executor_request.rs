use crate::HeadlessExecutorError;
use serde_json::Value;
use std::io::{self, Write};

pub(crate) const MAX_INLINE_JSON_BYTES: usize = 8_000_000;

pub(crate) fn serialize_inline_json(
    path: &str,
    value: Value,
) -> Result<Vec<u8>, HeadlessExecutorError> {
    let mut writer = InlineJsonWriter::new(MAX_INLINE_JSON_BYTES);
    serde_json::to_writer(&mut writer, &value).map_err(|error| HeadlessExecutorError {
        message: error.to_string(),
    })?;
    validate_inline_json_size(path, writer.size_bytes)?;
    Ok(writer.bytes)
}

pub(crate) fn validate_inline_json_size(
    path: &str,
    size_bytes: usize,
) -> Result<(), HeadlessExecutorError> {
    if size_bytes <= MAX_INLINE_JSON_BYTES {
        return Ok(());
    }
    Err(HeadlessExecutorError {
        message: format!(
            "service payload exceeds inline JSON transport limit: path={path} size_bytes={size_bytes} limit_bytes={MAX_INLINE_JSON_BYTES}; use a persisted model or artifact reference for large meshes"
        ),
    })
}

pub(crate) fn build_request_head(
    method: &str,
    host: &str,
    path: &str,
    body_bytes: usize,
    api_token: Option<&str>,
) -> String {
    let mut head = format!(
        "{method} {path} HTTP/1.1\r\nHost: {host}\r\nAccept: application/json\r\nConnection: close\r\n"
    );
    if let Some(token) = api_token {
        head.push_str(&format!("Authorization: Bearer {token}\r\n"));
    }
    if body_bytes > 0 {
        head.push_str("Content-Type: application/json\r\n");
        head.push_str(&format!("Content-Length: {body_bytes}\r\n"));
    }
    head.push_str("\r\n");
    head
}

struct InlineJsonWriter {
    bytes: Vec<u8>,
    size_bytes: usize,
    limit: usize,
}

impl InlineJsonWriter {
    fn new(limit: usize) -> Self {
        Self {
            bytes: Vec::new(),
            size_bytes: 0,
            limit,
        }
    }
}

impl Write for InlineJsonWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let size_bytes = self
            .size_bytes
            .checked_add(bytes.len())
            .ok_or_else(|| io::Error::other("serialized JSON size overflow"))?;
        if size_bytes <= self.limit {
            if size_bytes > self.bytes.capacity() {
                let capacity = self
                    .bytes
                    .capacity()
                    .saturating_mul(2)
                    .max(1024)
                    .max(size_bytes)
                    .min(self.limit);
                self.bytes
                    .try_reserve_exact(capacity - self.bytes.len())
                    .map_err(|_| io::Error::other("failed to reserve inline JSON buffer"))?;
            }
            self.bytes.extend_from_slice(bytes);
        } else if self.size_bytes <= self.limit {
            // Keep counting for the exact size diagnostic, but retain no oversized body.
            self.bytes = Vec::new();
        }
        self.size_bytes = size_bytes;
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
#[path = "service_executor_request_tests.rs"]
mod tests;
