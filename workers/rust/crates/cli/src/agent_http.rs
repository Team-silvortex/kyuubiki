use std::io::{Read, Write};
use std::net::{Shutdown, TcpStream, ToSocketAddrs};
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
    let _ = stream.shutdown(Shutdown::Write);

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
    match token {
        Some(token) if !token.trim().is_empty() => {
            let mut headers = vec![
                ("x-kyuubiki-token".to_string(), token.trim().to_string()),
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
        _ => vec![],
    }
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
        Some((authority, path)) => (authority, format!("/{}", path)),
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
