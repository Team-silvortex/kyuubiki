use std::fs::File;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::path::Path;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub(crate) fn normalize_base_url(url: &str) -> String {
    url.trim_end_matches('/').to_string()
}

pub(crate) fn post_json(
    url: &str,
    payload: &serde_json::Value,
    extra_headers: Vec<(String, String)>,
) -> Result<(), String> {
    let body = serde_json::to_string(payload)
        .map_err(|error| format!("failed to serialize registration payload: {error}"))?;
    send_http_request(
        "POST",
        url,
        Some(("application/json", body.as_bytes())),
        extra_headers,
    )
}

pub(crate) fn delete_request(
    url: &str,
    extra_headers: Vec<(String, String)>,
) -> Result<(), String> {
    send_http_request("DELETE", url, None, extra_headers)
}

pub(crate) fn get_to_writer(
    url: &str,
    extra_headers: Vec<(String, String)>,
    max_bytes: usize,
    writer: &mut impl Write,
) -> Result<usize, String> {
    let parsed = parse_http_url(url)?;
    let address = format!("{}:{}", parsed.host, parsed.port);
    let socket_addr = address
        .to_socket_addrs()
        .map_err(|error| format!("failed to resolve {address}: {error}"))?
        .next()
        .ok_or_else(|| format!("failed to resolve {address}"))?;
    let mut stream = TcpStream::connect_timeout(&socket_addr, Duration::from_secs(10))
        .map_err(|error| format!("failed to connect to {address}: {error}"))?;
    let _ = stream.set_read_timeout(Some(Duration::from_secs(120)));
    let _ = stream.set_write_timeout(Some(Duration::from_secs(30)));

    let mut request = format!(
        "GET {path} HTTP/1.1\r\nHost: {host}\r\nConnection: close\r\n",
        path = parsed.path,
        host = parsed.host
    );
    for (header, value) in extra_headers {
        request.push_str(&format!("{header}: {value}\r\n"));
    }
    request.push_str("\r\n");
    stream
        .write_all(request.as_bytes())
        .map_err(|error| format!("failed to write artifact request: {error}"))?;
    stream
        .flush()
        .map_err(|error| format!("failed to flush artifact request: {error}"))?;

    let mut reader = BufReader::new(stream);
    let mut status = String::new();
    reader
        .read_line(&mut status)
        .map_err(|error| format!("failed to read artifact response status: {error}"))?;
    if !(status.starts_with("HTTP/1.1 2") || status.starts_with("HTTP/1.0 2")) {
        return Err(format!("artifact request returned {}", status.trim_end()));
    }

    let mut content_length = None;
    loop {
        let mut header = String::new();
        reader
            .read_line(&mut header)
            .map_err(|error| format!("failed to read artifact response headers: {error}"))?;
        if header == "\r\n" || header == "\n" {
            break;
        }
        let Some((name, value)) = header.split_once(':') else {
            return Err("artifact response contained an invalid header".to_string());
        };
        if name.eq_ignore_ascii_case("content-length") {
            content_length = value.trim().parse::<usize>().ok();
        }
        if name.eq_ignore_ascii_case("transfer-encoding")
            && !value.trim().eq_ignore_ascii_case("identity")
        {
            return Err("artifact response must use a bounded content-length body".to_string());
        }
    }

    let content_length =
        content_length.ok_or_else(|| "artifact response omitted content-length".to_string())?;
    if content_length > max_bytes {
        return Err(format!(
            "artifact response exceeds limit: size_bytes={content_length} limit_bytes={max_bytes}"
        ));
    }

    let mut received = 0usize;
    let mut buffer = vec![0_u8; 1_048_576];
    while received < content_length {
        let remaining = content_length - received;
        let read_length = remaining.min(buffer.len());
        let count = reader
            .read(&mut buffer[..read_length])
            .map_err(|error| format!("failed to read artifact response body: {error}"))?;
        if count == 0 {
            return Err(format!(
                "artifact response ended early: received_bytes={received} expected_bytes={content_length}"
            ));
        }
        writer
            .write_all(&buffer[..count])
            .map_err(|error| format!("failed to persist artifact response: {error}"))?;
        received += count;
    }

    Ok(received)
}

pub(crate) fn post_file(
    url: &str,
    content_type: &str,
    path: &Path,
    extra_headers: Vec<(String, String)>,
) -> Result<Vec<u8>, String> {
    let parsed = parse_http_url(url)?;
    let address = format!("{}:{}", parsed.host, parsed.port);
    let socket_addr = address
        .to_socket_addrs()
        .map_err(|error| format!("failed to resolve {address}: {error}"))?
        .next()
        .ok_or_else(|| format!("failed to resolve {address}"))?;
    let mut file = File::open(path)
        .map_err(|error| format!("failed to open result artifact for upload: {error}"))?;
    let content_length = file
        .metadata()
        .map_err(|error| format!("failed to inspect result artifact: {error}"))?
        .len();
    let mut stream = TcpStream::connect_timeout(&socket_addr, Duration::from_secs(10))
        .map_err(|error| format!("failed to connect to {address}: {error}"))?;
    let _ = stream.set_read_timeout(Some(Duration::from_secs(120)));
    let _ = stream.set_write_timeout(Some(Duration::from_secs(120)));

    let mut request = format!(
        "POST {path} HTTP/1.1\r\nHost: {host}\r\nConnection: close\r\nAccept: application/json\r\nContent-Type: {content_type}\r\nContent-Length: {content_length}\r\n",
        path = parsed.path,
        host = parsed.host
    );
    for (header, value) in extra_headers {
        request.push_str(&format!("{header}: {value}\r\n"));
    }
    request.push_str("\r\n");
    stream
        .write_all(request.as_bytes())
        .map_err(|error| format!("failed to write result artifact headers: {error}"))?;
    let sent = std::io::copy(&mut file, &mut stream)
        .map_err(|error| format!("failed to upload result artifact: {error}"))?;
    if sent != content_length {
        return Err(format!(
            "result artifact upload ended early: sent_bytes={sent} expected_bytes={content_length}"
        ));
    }
    stream
        .flush()
        .map_err(|error| format!("failed to flush result artifact upload: {error}"))?;

    let mut response = Vec::new();
    stream
        .take(1_048_577)
        .read_to_end(&mut response)
        .map_err(|error| format!("failed to read result artifact response: {error}"))?;
    if response.len() > 1_048_576 {
        return Err("result artifact response exceeds 1 MiB".to_string());
    }
    let separator = response
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .ok_or_else(|| "result artifact upload returned invalid HTTP".to_string())?;
    let headers = String::from_utf8_lossy(&response[..separator]);
    if !(headers.starts_with("HTTP/1.1 2") || headers.starts_with("HTTP/1.0 2")) {
        return Err(format!(
            "result artifact upload returned {}",
            headers.lines().next().unwrap_or("unknown HTTP status")
        ));
    }
    Ok(response[(separator + 4)..].to_vec())
}

fn send_http_request(
    method: &str,
    url: &str,
    body: Option<(&str, &[u8])>,
    extra_headers: Vec<(String, String)>,
) -> Result<(), String> {
    let parsed = parse_http_url(url)?;
    let address = format!("{}:{}", parsed.host, parsed.port);
    let socket_addr = address
        .to_socket_addrs()
        .map_err(|error| format!("failed to resolve {address}: {error}"))?
        .next()
        .ok_or_else(|| format!("failed to resolve {address}"))?;

    let mut stream = TcpStream::connect_timeout(&socket_addr, Duration::from_millis(1_500))
        .map_err(|error| {
            format!(
                "failed to connect to {}:{}: {error}",
                parsed.host, parsed.port
            )
        })?;
    let _ = stream.set_read_timeout(Some(Duration::from_millis(2_000)));
    let _ = stream.set_write_timeout(Some(Duration::from_millis(2_000)));

    let (content_type, bytes) = body.unwrap_or(("application/json", &[]));
    let mut request = format!(
        "{method} {path} HTTP/1.1\r\nHost: {host}\r\nConnection: close\r\nContent-Type: {content_type}\r\nContent-Length: {length}\r\n",
        method = method,
        path = parsed.path,
        host = parsed.host,
        content_type = content_type,
        length = bytes.len()
    );

    for (header, value) in extra_headers {
        request.push_str(&format!("{header}: {value}\r\n"));
    }

    request.push_str("\r\n");

    stream
        .write_all(request.as_bytes())
        .map_err(|error| format!("failed to write HTTP request: {error}"))?;
    if !bytes.is_empty() {
        stream
            .write_all(bytes)
            .map_err(|error| format!("failed to write HTTP request body: {error}"))?;
    }
    let _ = stream.flush();

    let mut response = String::new();
    stream
        .read_to_string(&mut response)
        .map_err(|error| format!("failed to read HTTP response: {error}"))?;

    if response.starts_with("HTTP/1.1 2") || response.starts_with("HTTP/1.0 2") {
        Ok(())
    } else {
        Err(format!("unexpected HTTP response from {url}: {response}"))
    }
}

pub(crate) fn cluster_auth_headers(
    token: Option<&str>,
    agent_id: &str,
    cluster_id: Option<&str>,
    fingerprint: Option<&str>,
) -> Vec<(String, String)> {
    let mut headers = vec![
        ("x-kyuubiki-agent-id".to_string(), agent_id.to_string()),
        (
            "x-kyuubiki-cluster-ts".to_string(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|duration| duration.as_millis().to_string())
                .unwrap_or_else(|_| "0".to_string()),
        ),
        (
            "x-kyuubiki-cluster-nonce".to_string(),
            cluster_request_nonce(),
        ),
    ];
    if let Some(token) = token.filter(|value| !value.trim().is_empty()) {
        headers.push(("x-kyuubiki-token".to_string(), token.trim().to_string()));
    }
    if let Some(cluster_id) = cluster_id.filter(|value| !value.trim().is_empty()) {
        headers.push((
            "x-kyuubiki-cluster-id".to_string(),
            cluster_id.trim().to_string(),
        ));
    }
    if let Some(fingerprint) = fingerprint.filter(|value| !value.trim().is_empty()) {
        headers.push((
            "x-kyuubiki-agent-fingerprint".to_string(),
            fingerprint.trim().to_string(),
        ));
    }

    headers
}

fn cluster_request_nonce() -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    format!("agent-{now}")
}

pub(crate) struct ParsedHttpUrl {
    pub(crate) host: String,
    pub(crate) port: u16,
    pub(crate) path: String,
}

pub(crate) fn parse_http_url(url: &str) -> Result<ParsedHttpUrl, String> {
    let raw = url
        .strip_prefix("http://")
        .ok_or_else(|| format!("unsupported orchestrator URL: {url} (expected http://...)"))?;
    let (authority, path) = match raw.split_once('/') {
        Some((authority, path)) => (authority, format!("/{path}")),
        None => (raw, "/".to_string()),
    };
    let (host, port) = match authority.split_once(':') {
        Some((host, port)) => {
            let port = port
                .parse::<u16>()
                .map_err(|_| format!("invalid orchestrator port in URL: {url}"))?;
            (host.to_string(), port)
        }
        None => (authority.to_string(), 80),
    };

    if host.trim().is_empty() {
        return Err(format!("invalid orchestrator host in URL: {url}"));
    }

    Ok(ParsedHttpUrl { host, port, path })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::ErrorKind;
    use std::net::TcpListener;
    use std::thread;

    #[test]
    fn post_json_keeps_the_request_connection_open_until_response() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("HTTP listener");
        let address = listener.local_addr().expect("listener address");
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("HTTP request");
            stream
                .set_read_timeout(Some(Duration::from_secs(1)))
                .expect("request timeout");
            let mut request = Vec::new();
            while !complete_http_request(&request) {
                let mut chunk = [0_u8; 4096];
                let count = stream.read(&mut chunk).expect("request bytes");
                assert!(count > 0, "client closed before sending the request body");
                request.extend_from_slice(&chunk[..count]);
            }

            stream
                .set_read_timeout(Some(Duration::from_millis(100)))
                .expect("half-close probe timeout");
            let mut probe = [0_u8; 1];
            let closed_before_response = match stream.read(&mut probe) {
                Ok(0) => true,
                Err(error)
                    if matches!(error.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut) =>
                {
                    false
                }
                Ok(_) => panic!("unexpected trailing request bytes"),
                Err(error) => panic!("request connection probe failed: {error}"),
            };
            stream
                .write_all(
                    b"HTTP/1.1 201 Created\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                )
                .expect("HTTP response");
            closed_before_response
        });

        post_json(
            &format!("http://{address}/api/v1/agents/register"),
            &serde_json::json!({"id": "connection-probe"}),
            Vec::new(),
        )
        .expect("POST response");
        assert!(
            !server.join().expect("HTTP server thread"),
            "HTTP client must not half-close before the server responds"
        );
    }

    fn complete_http_request(request: &[u8]) -> bool {
        let Some(header_end) = request.windows(4).position(|window| window == b"\r\n\r\n") else {
            return false;
        };
        let headers = String::from_utf8_lossy(&request[..header_end]);
        let content_length = headers
            .lines()
            .find_map(|line| {
                let (name, value) = line.split_once(':')?;
                name.eq_ignore_ascii_case("content-length")
                    .then(|| value.trim().parse::<usize>().ok())
                    .flatten()
            })
            .unwrap_or(0);
        request.len() >= header_end + 4 + content_length
    }
}
