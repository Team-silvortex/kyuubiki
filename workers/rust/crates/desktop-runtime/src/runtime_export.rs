use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;

pub(super) fn export_database(url: Option<&str>) -> Result<String, String> {
    let url = url.unwrap_or("http://127.0.0.1:4000/api/v1/export/database");
    let target = url
        .strip_prefix("http://")
        .ok_or_else(|| "native database export currently requires an HTTP URL".to_string())?;
    let (authority, path) = target
        .split_once('/')
        .map(|(authority, path)| (authority, format!("/{path}")))
        .unwrap_or((target, "/".to_string()));
    let (host, port) = authority
        .rsplit_once(':')
        .map(|(host, port)| {
            port.parse::<u16>()
                .map(|port| (host, port))
                .map_err(|error| format!("invalid export URL port: {error}"))
        })
        .transpose()?
        .unwrap_or((authority, 80));
    if !matches!(host, "127.0.0.1" | "localhost" | "::1") {
        return Err("native database export only permits loopback endpoints".to_string());
    }

    let mut stream = TcpStream::connect((host, port))
        .map_err(|error| format!("failed to connect to database export endpoint: {error}"))?;
    stream
        .set_read_timeout(Some(Duration::from_secs(30)))
        .map_err(|error| format!("failed to configure export timeout: {error}"))?;
    write!(
        stream,
        "GET {path} HTTP/1.1\r\nHost: {authority}\r\nConnection: close\r\nAccept: application/json\r\n\r\n"
    )
    .map_err(|error| format!("failed to request database export: {error}"))?;
    let mut response = Vec::new();
    stream
        .read_to_end(&mut response)
        .map_err(|error| format!("failed to read database export: {error}"))?;
    let rendered = String::from_utf8(response)
        .map_err(|error| format!("database export was not UTF-8: {error}"))?;
    let (headers, body) = rendered
        .split_once("\r\n\r\n")
        .ok_or_else(|| "database export returned an invalid HTTP response".to_string())?;
    if !headers
        .lines()
        .next()
        .is_some_and(|line| line.contains(" 200 "))
    {
        return Err(format!(
            "database export failed: {}",
            headers.lines().next().unwrap_or("unknown HTTP status")
        ));
    }
    Ok(body.to_string())
}
