//! Bounded loopback-only installed-WebView acceptance fixture, never a runtime service.
//! cargo run -p kyuubiki-desktop-runtime --example checkpoint_response_fault -- PROJECT_ID
//! Set the Workbench API override to the printed URL only for the acceptance script.
//! Restore it before exiting. The proxy expires after ten minutes; Ctrl-C stops it.
//! Only new four-node truss models under PROJECT_ID can be written. No credentials,
//! geometry, or full responses are logged. Existing models cannot be mutated.

#[allow(dead_code)]
#[path = "../src/frontend_http.rs"]
mod frontend_http;

use frontend_http::HttpRequest;
use serde_json::{Value, json};
use std::collections::HashSet;
use std::io::{Read, Write};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicUsize, Ordering},
};
use std::time::{Duration, Instant};

const ORIGIN: &str = "http://127.0.0.1:3000";
const PREFIX: &str = "Acceptance response fault ";
const LIMIT: usize = 1024 * 1024;

#[derive(Default)]
struct State {
    models: HashSet<String>,
    stalled_version: bool,
    dropped_model: bool,
}

fn main() -> Result<(), String> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() != 1 || args[0].len() != 16 || !args[0].bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err("usage: checkpoint_response_fault <exact 16-hex disposable project ID>".into());
    }
    let project = Arc::new(args[0].clone());
    let listener = TcpListener::bind("127.0.0.1:0").map_err(|e| e.to_string())?;
    listener.set_nonblocking(true).map_err(|e| e.to_string())?;
    println!(
        "{}",
        json!({"event":"ready", "url":format!("http://{}", listener.local_addr().unwrap()),
        "upstream":"http://127.0.0.1:4000", "project":project.as_str(), "ttl_seconds":600})
    );
    let state = Arc::new(Mutex::new(State::default()));
    let active = Arc::new(AtomicUsize::new(0));
    let deadline = Instant::now() + Duration::from_secs(600);
    while Instant::now() < deadline {
        match listener.accept() {
            Ok((mut client, _)) => {
                if active.load(Ordering::SeqCst) >= 8 {
                    continue;
                }
                active.fetch_add(1, Ordering::SeqCst);
                let (project, state, active) = (project.clone(), state.clone(), active.clone());
                std::thread::spawn(move || {
                    if let Err(error) = handle(&mut client, &project, &state) {
                        eprintln!("{}", json!({"event":"fixture_error", "reason":error}));
                    }
                    active.fetch_sub(1, Ordering::SeqCst);
                });
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(10))
            }
            Err(e) => return Err(e.to_string()),
        }
    }
    println!("{}", json!({"event":"expired"}));
    Ok(())
}

fn allowed_write(request: &HttpRequest, project: &str, state: &State, body: &Value) -> bool {
    if request.method != "POST"
        || request.body.len() > LIMIT
        || request.target != request.path
        || !body["name"]
            .as_str()
            .is_some_and(|name| name.starts_with(PREFIX))
        || !body["request_id"].as_str().is_some_and(|id| !id.is_empty())
        || body["kind"] != "truss_3d"
        || body["payload"]["nodes"].as_array().map(Vec::len) != Some(4)
        || body["payload"]["elements"].as_array().map(Vec::len) != Some(6)
    {
        return false;
    }
    request.path == format!("/api/v1/projects/{project}/models")
        || state
            .models
            .iter()
            .any(|id| request.path == format!("/api/v1/models/{id}/versions"))
}

fn handle(client: &mut TcpStream, project: &str, shared: &Mutex<State>) -> Result<(), String> {
    client
        .set_write_timeout(Some(Duration::from_secs(20)))
        .map_err(|e| e.to_string())?;
    let request = frontend_http::read_request(client)?;
    if request
        .headers
        .get("origin")
        .is_some_and(|origin| origin != ORIGIN)
    {
        return reply(client, 403, b"{\"error\":\"fixture_origin_rejected\"}");
    }
    if request.method == "OPTIONS" {
        return reply(client, 204, b"");
    }
    let body: Value = serde_json::from_slice(&request.body).unwrap_or(Value::Null);
    let write = request.method != "GET";
    if !request.path.starts_with("/api/")
        || (write && !allowed_write(&request, project, &shared.lock().unwrap(), &body))
    {
        return reply(client, 403, b"{\"error\":\"fixture_scope_rejected\"}");
    }
    let (status, payload) = upstream(&request)?;
    let mut fault = "none";
    if write && status == 201 {
        let receipt: Value = serde_json::from_slice(&payload).map_err(|e| e.to_string())?;
        let mut state = shared.lock().unwrap();
        if let Some(id) = receipt["model"]["model_id"].as_str() {
            if receipt["model"]["project_id"] != project {
                return Err("unexpected receipt project".into());
            }
            state.models.insert(id.to_string());
            if body["name"] == format!("{PREFIX}fork") && !state.dropped_model {
                state.dropped_model = true;
                fault = "drop_after_commit";
            }
        } else if receipt["version"]["version_id"].is_string() {
            if !state
                .models
                .contains(receipt["version"]["model_id"].as_str().unwrap_or_default())
            {
                return Err("unexpected receipt model".into());
            }
            if !state.stalled_version {
                state.stalled_version = true;
                fault = "stall_body_after_commit";
            }
        } else {
            return Err("missing checkpoint receipt".into());
        }
        println!(
            "{}",
            json!({"event":"committed_response", "path":request.path,
            "request_id":body["request_id"], "status":status, "fault":fault,
            "model_id":receipt["model"]["model_id"], "version_id":receipt["version"]["version_id"],
            "initial_version_id":receipt["model"]["latest_version_id"]})
        );
    }
    match fault {
        "drop_after_commit" => client.shutdown(Shutdown::Both).map_err(|e| e.to_string()),
        "stall_body_after_commit" => {
            write_head(client, status, payload.len())?;
            client.write_all(&payload[..1]).map_err(|e| e.to_string())?;
            client.flush().map_err(|e| e.to_string())?;
            // Longer than the shipped 15-second request timeout, but bounded even
            // when testing a broken build. The retry runs on another connection.
            std::thread::sleep(Duration::from_secs(25));
            let _ = client.shutdown(Shutdown::Both);
            Ok(())
        }
        _ => reply(client, status, &payload),
    }
}

fn upstream(request: &HttpRequest) -> Result<(u16, Vec<u8>), String> {
    let mut server =
        TcpStream::connect_timeout(&"127.0.0.1:4000".parse().unwrap(), Duration::from_secs(5))
            .map_err(|e| e.to_string())?;
    server
        .set_read_timeout(Some(Duration::from_secs(20)))
        .map_err(|e| e.to_string())?;
    server
        .set_write_timeout(Some(Duration::from_secs(20)))
        .map_err(|e| e.to_string())?;
    let mut head = format!(
        "{} {} HTTP/1.1\r\nHost: 127.0.0.1:4000\r\nConnection: close\r\nContent-Length: {}\r\n",
        request.method,
        request.target,
        request.body.len()
    );
    for key in [
        "accept",
        "content-type",
        "authorization",
        "x-kyuubiki-token",
        "x-kyuubiki-cluster-token",
    ] {
        if let Some(value) = request.headers.get(key) {
            head.push_str(&format!("{key}: {}\r\n", value.replace(['\r', '\n'], "")));
        }
    }
    head.push_str("\r\n");
    server
        .write_all(head.as_bytes())
        .and_then(|_| server.write_all(&request.body))
        .map_err(|e| e.to_string())?;
    server
        .shutdown(Shutdown::Write)
        .map_err(|e| e.to_string())?;
    let mut bytes = Vec::new();
    server
        .take((LIMIT + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    parse_response(&bytes)
}

fn parse_response(bytes: &[u8]) -> Result<(u16, Vec<u8>), String> {
    if bytes.len() > LIMIT {
        return Err("acceptance response exceeds 1 MiB".into());
    }
    let end = bytes
        .windows(4)
        .position(|part| part == b"\r\n\r\n")
        .ok_or("incomplete upstream headers")?;
    let head = std::str::from_utf8(&bytes[..end]).map_err(|e| e.to_string())?;
    let mut lines = head.split("\r\n");
    let status = lines
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .and_then(|value| value.parse::<u16>().ok())
        .ok_or("invalid upstream status")?;
    let mut length = None;
    for line in lines {
        let (key, value) = line.split_once(':').ok_or("invalid upstream header")?;
        if key.eq_ignore_ascii_case("transfer-encoding") {
            return Err("fixture requires a length-delimited response".into());
        }
        if key.eq_ignore_ascii_case("content-length") {
            if length.is_some() {
                return Err("duplicate response length".into());
            }
            length = Some(value.trim().parse::<usize>().map_err(|e| e.to_string())?);
        }
    }
    let body = &bytes[end + 4..];
    if length != Some(body.len()) {
        return Err("incomplete or ambiguous upstream body".into());
    }
    Ok((status, body.to_vec()))
}

fn write_head(client: &mut TcpStream, status: u16, size: usize) -> Result<(), String> {
    write!(client, "HTTP/1.1 {status} Acceptance\r\nContent-Type: application/json\r\nContent-Length: {size}\r\nConnection: close\r\nAccess-Control-Allow-Origin: {ORIGIN}\r\nVary: Origin\r\nAccess-Control-Allow-Methods: GET, POST, OPTIONS\r\nAccess-Control-Allow-Headers: content-type, authorization, x-kyuubiki-token, x-kyuubiki-cluster-token\r\nCache-Control: no-store\r\n\r\n")
        .map_err(|e| e.to_string())
}

fn reply(client: &mut TcpStream, status: u16, body: &[u8]) -> Result<(), String> {
    write_head(client, status, body.len())?;
    client.write_all(body).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_owned_fixture_models_can_be_written() {
        let mut state = State::default();
        let mut request = HttpRequest {
            method: "POST".into(),
            target: "/api/v1/projects/abcd/models".into(),
            path: "/api/v1/projects/abcd/models".into(),
            headers: Default::default(),
            body: vec![],
        };
        let body = json!({"name":format!("{PREFIX}baseline"), "request_id":"key", "kind":"truss_3d",
            "payload":{"nodes":[0,1,2,3], "elements":[0,1,2,3,4,5]}});
        assert!(allowed_write(&request, "abcd", &state, &body));
        assert!(!allowed_write(&request, "other", &state, &body));
        request.path = "/api/v1/models/existing/versions".into();
        request.target = request.path.clone();
        assert!(!allowed_write(&request, "abcd", &state, &body));
        state.models.insert("existing".into());
        assert!(allowed_write(&request, "abcd", &state, &body));
        assert!(!allowed_write(
            &request,
            "abcd",
            &state,
            &json!({"name":"research"})
        ));
        request.method = "DELETE".into();
        assert!(!allowed_write(&request, "abcd", &state, &body));
    }

    #[test]
    fn upstream_must_finish_before_a_commit_can_be_observed() {
        assert_eq!(
            parse_response(b"HTTP/1.1 201 Created\r\nContent-Length: 2\r\n\r\n{}").unwrap(),
            (201, b"{}".to_vec())
        );
        for bytes in [
            b"HTTP/1.1 201 Created\r\nContent-Length: 3\r\n\r\n{}".as_slice(),
            b"HTTP/1.1 201 Created\r\nContent-Length: 2\r\nTransfer-Encoding: chunked\r\n\r\n{}",
            b"HTTP/1.1 201 Created\r\nContent-Length: 2\r\nContent-Length: 2\r\n\r\n{}",
        ] {
            assert!(parse_response(bytes).is_err());
        }
    }
}
