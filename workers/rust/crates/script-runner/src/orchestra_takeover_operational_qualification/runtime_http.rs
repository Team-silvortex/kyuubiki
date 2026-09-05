use serde_json::Value;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::time::Duration;

type RunnerResult<T> = Result<T, String>;

const MAX_HTTP_BYTES: u64 = 4 * 1024 * 1024;

pub(super) struct HttpResponse {
    pub(super) status: u16,
    pub(super) body: Value,
}

pub(super) fn get_json(port: u16, path: &str, token: &str) -> RunnerResult<Value> {
    let response = request_json(port, "GET", path, token, None)?;
    if (200..300).contains(&response.status) {
        Ok(response.body)
    } else {
        Err(format!(
            "Orchestra returned HTTP status {}",
            response.status
        ))
    }
}

pub(super) fn post_json(
    port: u16,
    path: &str,
    token: &str,
    body: &Value,
) -> RunnerResult<HttpResponse> {
    request_json(port, "POST", path, token, Some(body))
}

fn request_json(
    port: u16,
    method: &str,
    path: &str,
    token: &str,
    body: Option<&Value>,
) -> RunnerResult<HttpResponse> {
    let address = SocketAddr::from(([127, 0, 0, 1], port));
    let mut stream = TcpStream::connect_timeout(&address, Duration::from_millis(500))
        .map_err(|error| format!("Orchestra HTTP unavailable: {error}"))?;
    stream
        .set_read_timeout(Some(Duration::from_secs(2)))
        .map_err(|error| format!("failed to configure Orchestra HTTP timeout: {error}"))?;
    let encoded = body
        .map(serde_json::to_vec)
        .transpose()
        .map_err(|error| format!("failed to encode Orchestra HTTP request: {error}"))?
        .unwrap_or_default();
    let request = format!(
        "{method} {path} HTTP/1.1\r\nHost: 127.0.0.1\r\nAuthorization: Bearer {token}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        encoded.len()
    );
    stream
        .write_all(request.as_bytes())
        .and_then(|_| stream.write_all(&encoded))
        .map_err(|error| format!("failed to write Orchestra HTTP request: {error}"))?;
    let mut response = Vec::new();
    stream
        .take(MAX_HTTP_BYTES + 1)
        .read_to_end(&mut response)
        .map_err(|error| format!("failed to read Orchestra HTTP response: {error}"))?;
    if response.len() as u64 > MAX_HTTP_BYTES {
        return Err("Orchestra HTTP response exceeded 4 MiB".to_string());
    }
    parse_response(&response)
}

fn parse_response(response: &[u8]) -> RunnerResult<HttpResponse> {
    let separator = response
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .ok_or("Orchestra returned invalid HTTP")?;
    let headers = String::from_utf8_lossy(&response[..separator]);
    let status = headers
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .and_then(|value| value.parse::<u16>().ok())
        .ok_or("Orchestra returned an invalid HTTP status")?;
    let body = if headers.lines().any(|line| {
        line.to_ascii_lowercase()
            .starts_with("transfer-encoding: chunked")
    }) {
        decode_chunked(&response[(separator + 4)..])?
    } else {
        response[(separator + 4)..].to_vec()
    };
    let body = serde_json::from_slice(&body)
        .map_err(|error| format!("invalid Orchestra JSON: {error}"))?;
    Ok(HttpResponse { status, body })
}

fn decode_chunked(mut bytes: &[u8]) -> RunnerResult<Vec<u8>> {
    let mut decoded = Vec::new();
    loop {
        let line_end = bytes
            .windows(2)
            .position(|window| window == b"\r\n")
            .ok_or("invalid chunked HTTP length")?;
        let length_text = std::str::from_utf8(&bytes[..line_end])
            .map_err(|_| "invalid chunked HTTP length encoding")?;
        let length = usize::from_str_radix(length_text.split(';').next().unwrap_or(""), 16)
            .map_err(|_| "invalid chunked HTTP length value")?;
        bytes = &bytes[(line_end + 2)..];
        if length == 0 {
            return Ok(decoded);
        }
        if bytes.len() < length + 2 || &bytes[length..(length + 2)] != b"\r\n" {
            return Err("truncated chunked HTTP body".to_string());
        }
        decoded.extend_from_slice(&bytes[..length]);
        bytes = &bytes[(length + 2)..];
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_success_and_error_json_responses() {
        let success = parse_response(b"HTTP/1.1 200 OK\r\nContent-Length: 11\r\n\r\n{\"ok\":true}")
            .expect("success response");
        assert_eq!(success.status, 200);
        assert_eq!(success.body["ok"], true);

        let rejected = parse_response(
            b"HTTP/1.1 422 Unprocessable Entity\r\nContent-Length: 31\r\n\r\n{\"error\":\":orchestra_standby\"}",
        )
        .expect("error response");
        assert_eq!(rejected.status, 422);
        assert_eq!(rejected.body["error"], ":orchestra_standby");
    }

    #[test]
    fn decodes_chunked_json_body() {
        let response = parse_response(
            b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\nB\r\n{\"ok\":true}\r\n0\r\n\r\n",
        )
        .expect("chunked response");
        assert_eq!(response.body["ok"], true);
    }
}
