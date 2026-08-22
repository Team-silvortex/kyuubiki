use std::collections::HashMap;
use std::fs;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::{Component, Path, PathBuf};
use std::time::Duration;

use serde_json::Value;

const MAX_HEADER_BYTES: usize = 64 * 1024;
const MAX_BODY_BYTES: usize = 32 * 1024 * 1024;

#[derive(Clone, Debug)]
pub(crate) struct HttpRequest {
    pub method: String,
    pub target: String,
    pub path: String,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
}

#[derive(Clone, Debug)]
pub(crate) struct HttpResponse {
    pub status: u16,
    pub content_type: String,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

pub(crate) fn read_request(stream: &mut TcpStream) -> Result<HttpRequest, String> {
    stream
        .set_read_timeout(Some(Duration::from_secs(30)))
        .map_err(|error| format!("failed to configure request timeout: {error}"))?;
    let mut bytes = Vec::with_capacity(4096);
    let header_end = loop {
        if let Some(index) = find_bytes(&bytes, b"\r\n\r\n") {
            break index + 4;
        }
        if bytes.len() >= MAX_HEADER_BYTES {
            return Err("request headers exceed 64 KiB".to_string());
        }
        let mut chunk = [0_u8; 4096];
        let read = stream
            .read(&mut chunk)
            .map_err(|error| format!("failed to read request: {error}"))?;
        if read == 0 {
            return Err("connection closed before request headers completed".to_string());
        }
        bytes.extend_from_slice(&chunk[..read]);
    };

    let head = std::str::from_utf8(&bytes[..header_end - 4])
        .map_err(|_| "request headers must be UTF-8".to_string())?;
    let mut lines = head.split("\r\n");
    let request_line = lines.next().ok_or_else(|| "empty request".to_string())?;
    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or_default().to_ascii_uppercase();
    let target = parts.next().unwrap_or_default().to_string();
    let version = parts.next().unwrap_or_default();
    if method.is_empty()
        || !target.starts_with('/')
        || !matches!(version, "HTTP/1.0" | "HTTP/1.1")
        || parts.next().is_some()
    {
        return Err("invalid HTTP request line".to_string());
    }

    let mut headers = HashMap::new();
    for line in lines {
        insert_header(&mut headers, line)?;
    }
    if headers.contains_key("transfer-encoding") {
        return Err("transfer-encoded request bodies are not supported".to_string());
    }
    let body_length = headers
        .get("content-length")
        .map(|value| {
            value
                .parse::<usize>()
                .map_err(|_| "invalid content-length".to_string())
        })
        .transpose()?
        .unwrap_or(0);
    if body_length > MAX_BODY_BYTES {
        return Err("request body exceeds 32 MiB".to_string());
    }
    let total = header_end + body_length;
    while bytes.len() < total {
        let remaining = total - bytes.len();
        let mut chunk = vec![0_u8; remaining.min(64 * 1024)];
        let read = stream
            .read(&mut chunk)
            .map_err(|error| format!("failed to read request body: {error}"))?;
        if read == 0 {
            return Err("connection closed before request body completed".to_string());
        }
        bytes.extend_from_slice(&chunk[..read]);
    }
    let path = target.split('?').next().unwrap_or("/").to_string();
    Ok(HttpRequest {
        method,
        target,
        path,
        headers,
        body: bytes[header_end..total].to_vec(),
    })
}

fn insert_header(headers: &mut HashMap<String, String>, line: &str) -> Result<(), String> {
    let (name, value) = line
        .split_once(':')
        .ok_or_else(|| "invalid HTTP header".to_string())?;
    let name = name.trim().to_ascii_lowercase();
    if name.is_empty() || !name.bytes().all(valid_header_name_byte) {
        return Err("invalid HTTP header name".to_string());
    }
    if headers
        .insert(name.clone(), value.trim().to_string())
        .is_some()
    {
        return Err(format!("duplicate HTTP header is not supported: {name}"));
    }
    Ok(())
}

pub(crate) fn write_response(
    stream: &mut TcpStream,
    response: HttpResponse,
    head_only: bool,
) -> Result<(), String> {
    let mut head = format!(
        "HTTP/1.1 {} {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n",
        response.status,
        reason_phrase(response.status),
        response.content_type,
        response.body.len()
    );
    for (name, value) in response.headers {
        head.push_str(&name);
        head.push_str(": ");
        head.push_str(&value.replace(['\r', '\n'], ""));
        head.push_str("\r\n");
    }
    head.push_str("\r\n");
    stream
        .write_all(head.as_bytes())
        .map_err(|error| format!("failed to write response headers: {error}"))?;
    if !head_only {
        stream
            .write_all(&response.body)
            .map_err(|error| format!("failed to write response body: {error}"))?;
    }
    Ok(())
}

pub(crate) fn json_response(status: u16, value: Value) -> HttpResponse {
    response(
        status,
        "application/json; charset=utf-8",
        value.to_string().into_bytes(),
    )
}

pub(crate) fn text_response(status: u16, message: impl Into<String>) -> HttpResponse {
    response(
        status,
        "text/plain; charset=utf-8",
        message.into().into_bytes(),
    )
}

pub(crate) fn response(status: u16, content_type: &str, body: Vec<u8>) -> HttpResponse {
    HttpResponse {
        status,
        content_type: content_type.to_string(),
        headers: security_headers(),
        body,
    }
}

pub(crate) fn static_response(root: &Path, request: &HttpRequest) -> HttpResponse {
    if !matches!(request.method.as_str(), "GET" | "HEAD") {
        return json_response(405, serde_json::json!({"error": "method_not_allowed"}));
    }
    let relative = match safe_static_path(&request.path) {
        Ok(path) => path,
        Err(error) => return json_response(400, serde_json::json!({"error": error})),
    };
    let mut candidate = root.join(&relative);
    let exported_html = candidate.with_extension("html");
    if relative.extension().is_none() && exported_html.is_file() {
        candidate = exported_html;
    } else if candidate.is_dir() {
        candidate = candidate.join("index.html");
    }
    if !candidate.is_file() && relative.extension().is_none() {
        candidate = root.join("index.html");
    }
    let canonical = match candidate.canonicalize() {
        Ok(path) if path.starts_with(root) && path.is_file() => path,
        _ => {
            return json_response(
                404,
                serde_json::json!({"error": "frontend_asset_not_found"}),
            );
        }
    };
    match fs::read(&canonical) {
        Ok(body) => {
            let mut response = response(200, mime_type(&canonical), body);
            response.headers.push((
                "Cache-Control".to_string(),
                cache_control(&canonical).to_string(),
            ));
            response
        }
        Err(error) => json_response(
            500,
            serde_json::json!({"error": "frontend_asset_read_failed", "message": error.to_string()}),
        ),
    }
}

pub(crate) fn query_parameter(target: &str, name: &str) -> Option<String> {
    let query = target.split_once('?')?.1;
    query.split('&').find_map(|part| {
        let (key, value) = part.split_once('=').unwrap_or((part, ""));
        (key == name).then(|| percent_decode(value).ok()).flatten()
    })
}

fn safe_static_path(path: &str) -> Result<PathBuf, String> {
    let decoded = percent_decode(path)?;
    if decoded.contains(['\\', '\0']) {
        return Err("invalid frontend asset path".to_string());
    }
    let mut relative = PathBuf::new();
    for component in Path::new(decoded.trim_start_matches('/')).components() {
        match component {
            Component::Normal(value) => relative.push(value),
            Component::CurDir => {}
            _ => return Err("frontend asset path escapes its root".to_string()),
        }
    }
    if relative.as_os_str().is_empty() {
        relative.push("index.html");
    }
    Ok(relative)
}

fn percent_decode(value: &str) -> Result<String, String> {
    let bytes = value.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' {
            if index + 2 >= bytes.len() {
                return Err("invalid percent-encoded path".to_string());
            }
            let high =
                hex(bytes[index + 1]).ok_or_else(|| "invalid percent encoding".to_string())?;
            let low =
                hex(bytes[index + 2]).ok_or_else(|| "invalid percent encoding".to_string())?;
            decoded.push((high << 4) | low);
            index += 3;
        } else {
            decoded.push(bytes[index]);
            index += 1;
        }
    }
    String::from_utf8(decoded).map_err(|_| "decoded path is not UTF-8".to_string())
}

fn security_headers() -> Vec<(String, String)> {
    vec![
        ("X-Content-Type-Options".to_string(), "nosniff".to_string()),
        ("X-Frame-Options".to_string(), "SAMEORIGIN".to_string()),
        ("Referrer-Policy".to_string(), "no-referrer".to_string()),
        (
            "Cross-Origin-Resource-Policy".to_string(),
            "same-origin".to_string(),
        ),
    ]
}

fn cache_control(path: &Path) -> &'static str {
    if path
        .components()
        .any(|component| component.as_os_str() == "_next")
    {
        "public, max-age=31536000, immutable"
    } else {
        "no-cache"
    }
}

fn mime_type(path: &Path) -> &'static str {
    match path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("")
    {
        "css" => "text/css; charset=utf-8",
        "html" => "text/html; charset=utf-8",
        "js" | "mjs" => "text/javascript; charset=utf-8",
        "json" => "application/json; charset=utf-8",
        "png" => "image/png",
        "svg" => "image/svg+xml",
        "ico" => "image/x-icon",
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        "woff" => "font/woff",
        "woff2" => "font/woff2",
        "wasm" => "application/wasm",
        "map" => "application/json; charset=utf-8",
        _ => "application/octet-stream",
    }
}

fn valid_header_name_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || b"!#$%&'*+-.^_`|~".contains(&byte)
}

fn find_bytes(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

fn hex(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

fn reason_phrase(status: u16) -> &'static str {
    match status {
        200 => "OK",
        201 => "Created",
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        413 => "Payload Too Large",
        500 => "Internal Server Error",
        502 => "Bad Gateway",
        503 => "Service Unavailable",
        _ => "Response",
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::{HttpRequest, insert_header, query_parameter, safe_static_path, static_response};

    fn request(path: &str) -> HttpRequest {
        HttpRequest {
            method: "GET".to_string(),
            target: path.to_string(),
            path: path.to_string(),
            headers: Default::default(),
            body: Vec::new(),
        }
    }

    #[test]
    fn static_paths_reject_encoded_traversal() {
        assert!(safe_static_path("/%2e%2e/secrets").is_err());
        assert!(safe_static_path("/assets\\escape").is_err());
        assert_eq!(
            safe_static_path("/").unwrap().to_string_lossy(),
            "index.html"
        );
    }

    #[test]
    fn query_parameters_are_decoded() {
        assert_eq!(
            query_parameter("/items?offset=12&name=a%20b", "name").as_deref(),
            Some("a b")
        );
    }

    #[test]
    fn request_shape_remains_owned_by_native_runtime() {
        let request = request("/");
        assert_eq!(request.path, "/");
    }

    #[test]
    fn duplicate_headers_are_rejected_before_body_parsing() {
        let mut headers = std::collections::HashMap::new();
        insert_header(&mut headers, "Content-Length: 4").unwrap();
        let error = insert_header(&mut headers, "content-length: 8").unwrap_err();
        assert!(error.contains("duplicate HTTP header"));
    }

    #[test]
    fn next_export_html_routes_are_resolved_before_spa_fallback() {
        let root = std::env::temp_dir().join(format!(
            "kyuubiki-native-frontend-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(root.join("docs")).unwrap();
        fs::write(root.join("index.html"), "home").unwrap();
        fs::write(root.join("docs.html"), "documentation").unwrap();
        let root = root.canonicalize().unwrap();
        let response = static_response(&root, &request("/docs"));
        assert_eq!(response.status, 200);
        assert_eq!(response.body, b"documentation");
        fs::remove_dir_all(root).unwrap();
    }
}
