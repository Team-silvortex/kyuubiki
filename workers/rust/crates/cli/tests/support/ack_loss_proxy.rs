use serde_json::{Value, json};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

type Result<T> = std::result::Result<T, String>;
const LIMIT: usize = 1024 * 1024;

#[derive(Clone)]
enum Wire {
    Agent {
        drop_execution: bool,
    },
    Http {
        method: &'static str,
        path: String,
        remove_identity: Option<(&'static str, &'static str)>,
        advertised_length: Option<usize>,
        substitute_artifact: Option<Value>,
        request_limit: usize,
    },
}

#[derive(Default, Clone, Debug)]
pub(super) struct Capture {
    pub requests: Vec<Value>,
    pub discarded: Vec<Value>,
    pub replies: Vec<Value>,
    pub errors: Vec<String>,
}

// Test-only relay: capture an actual upstream reply before dropping it, or
// explicitly corrupt one record identity for semantic acknowledgement tests.
pub(super) struct AckLossProxy {
    pub port: u16,
    stop: Arc<AtomicBool>,
    capture: Arc<Mutex<Capture>>,
    worker: Option<JoinHandle<()>>,
}

impl AckLossProxy {
    pub fn agent(port: u16, drop_execution: bool) -> Result<Self> {
        Self::start(port, Wire::Agent { drop_execution })
    }

    pub fn http(port: u16) -> Result<Self> {
        Self::http_post(port, "/api/v1/operator-tasks/cancel-dispatch")
    }

    pub fn http_post(port: u16, path: &'static str) -> Result<Self> {
        Self::start(
            port,
            Wire::Http {
                method: "POST",
                path: path.into(),
                remove_identity: None,
                advertised_length: None,
                substitute_artifact: None,
                request_limit: LIMIT,
            },
        )
    }

    pub fn http_without_job_identity(port: u16, path: &'static str) -> Result<Self> {
        Self::http_without_record_identity(port, "POST", path, "job", "job_id")
    }

    pub fn http_without_record_identity(
        port: u16,
        method: &'static str,
        path: &str,
        record: &'static str,
        identity: &'static str,
    ) -> Result<Self> {
        Self::start(
            port,
            Wire::Http {
                method,
                path: path.into(),
                remove_identity: Some((record, identity)),
                advertised_length: None,
                substitute_artifact: None,
                request_limit: LIMIT,
            },
        )
    }

    pub fn http_with_oversized_length(
        port: u16,
        method: &'static str,
        path: &str,
        length: usize,
    ) -> Result<Self> {
        Self::start(
            port,
            Wire::Http {
                method,
                path: path.into(),
                remove_identity: None,
                advertised_length: Some(length),
                substitute_artifact: None,
                request_limit: LIMIT,
            },
        )
    }

    pub fn http_with_artifact_reference(port: u16, reference: Value) -> Result<Self> {
        Self::start(
            port,
            Wire::Http {
                method: "POST",
                path: "/api/v1/model-artifacts".into(),
                remove_identity: None,
                advertised_length: None,
                substitute_artifact: Some(reference),
                request_limit: 9_000_000,
            },
        )
    }

    fn start(upstream: u16, wire: Wire) -> Result<Self> {
        let listener = TcpListener::bind(("127.0.0.1", 0)).map_err(|e| e.to_string())?;
        let port = listener.local_addr().map_err(|e| e.to_string())?.port();
        listener.set_nonblocking(true).map_err(|e| e.to_string())?;
        let stop = Arc::new(AtomicBool::new(false));
        let capture = Arc::new(Mutex::new(Capture::default()));
        let signal = Arc::clone(&stop);
        let log = Arc::clone(&capture);
        let worker = thread::spawn(move || {
            let mut connections = Vec::new();
            while !signal.load(Ordering::Acquire) {
                match listener.accept() {
                    Ok((client, _)) => {
                        if connections.len() >= 128 {
                            log.lock()
                                .unwrap()
                                .errors
                                .push("connection cap reached".into());
                            break;
                        }
                        let log = Arc::clone(&log);
                        let wire = wire.clone();
                        connections.push(thread::spawn(move || {
                            if let Err(error) = relay(client, upstream, wire, &log) {
                                log.lock().unwrap().errors.push(error);
                            }
                        }));
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(5));
                    }
                    Err(e) => {
                        log.lock().unwrap().errors.push(e.to_string());
                        break;
                    }
                }
            }
            for connection in connections {
                connection.join().expect("relay panicked");
            }
        });
        Ok(Self {
            port,
            stop,
            capture,
            worker: Some(worker),
        })
    }

    pub fn capture(&self) -> Capture {
        self.capture.lock().unwrap().clone()
    }
}

impl Drop for AckLossProxy {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(worker) = self.worker.take() {
            worker.join().expect("acknowledgement relay panicked");
        }
    }
}

fn read(stream: &mut TcpStream, bytes: &mut [u8], deadline: Instant) -> Result<()> {
    let mut filled = 0;
    while filled < bytes.len() {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Err("relay deadline exceeded".into());
        }
        stream
            .set_read_timeout(Some(remaining))
            .map_err(|e| e.to_string())?;
        match stream.read(&mut bytes[filled..]) {
            Ok(0) => return Err("relay received an incomplete frame".into()),
            Ok(size) => filled += size,
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(e.to_string()),
        }
    }
    Ok(())
}

fn frame(stream: &mut TcpStream, deadline: Instant) -> Result<(Vec<u8>, Value)> {
    let mut header = [0; 4];
    read(stream, &mut header, deadline)?;
    let size = u32::from_be_bytes(header) as usize;
    if size == 0 || size > LIMIT {
        return Err("relay frame cap exceeded".into());
    }
    let mut payload = vec![0; size];
    read(stream, &mut payload, deadline)?;
    let json = serde_json::from_slice(&payload).map_err(|e| e.to_string())?;
    Ok((payload, json))
}

fn send(stream: &mut TcpStream, bytes: &[u8]) -> Result<()> {
    stream
        .write_all(&(bytes.len() as u32).to_be_bytes())
        .map_err(|e| e.to_string())?;
    stream.write_all(bytes).map_err(|e| e.to_string())
}

fn http_message(
    stream: &mut TcpStream,
    deadline: Instant,
    request: Option<(&str, &str)>,
    limit: usize,
) -> Result<(Vec<u8>, Value)> {
    let mut bytes = Vec::new();
    while !bytes.ends_with(b"\r\n\r\n") {
        if bytes.len() >= 32 * 1024 {
            return Err("relay HTTP header cap exceeded".into());
        }
        let mut byte = [0];
        read(stream, &mut byte, deadline)?;
        bytes.push(byte[0]);
    }
    let head = std::str::from_utf8(&bytes).map_err(|e| e.to_string())?;
    let first = head.lines().next().ok_or("missing HTTP start line")?;
    let expected = match request {
        Some((method, path)) => first == format!("{method} {path} HTTP/1.1"),
        None => {
            first.starts_with("HTTP/1.1 ")
                && first
                    .split_whitespace()
                    .nth(1)
                    .and_then(|status| status.parse::<u16>().ok())
                    .is_some_and(|status| (200..300).contains(&status))
        }
    };
    if !expected {
        return Err(format!("unexpected relay HTTP start line: {first}"));
    }
    let length = head
        .lines()
        .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            name.eq_ignore_ascii_case("content-length")
                .then(|| value.trim().parse::<usize>())
        })
        .or_else(|| matches!(request, Some(("DELETE" | "GET", _))).then_some(Ok(0)))
        .ok_or("missing HTTP body length")?
        .map_err(|e| e.to_string())?;
    if (length == 0 && request.is_none()) || length > limit {
        return Err("relay HTTP body cap exceeded".into());
    }
    let mut body = vec![0; length];
    read(stream, &mut body, deadline)?;
    let json = if body.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&body).map_err(|e| e.to_string())?
    };
    bytes.extend(body);
    Ok((bytes, json))
}

fn relay(mut client: TcpStream, port: u16, wire: Wire, log: &Mutex<Capture>) -> Result<()> {
    let deadline = Instant::now() + Duration::from_secs(12);
    // Accepted sockets can inherit the listener's nonblocking mode on macOS.
    // Restore blocking I/O before applying the bounded per-connection deadlines.
    client
        .set_nonblocking(false)
        .map_err(|e| format!("configure accepted stream: {e}"))?;
    let address = ([127, 0, 0, 1], port).into();
    let mut upstream = TcpStream::connect_timeout(&address, Duration::from_secs(2))
        .map_err(|e| format!("connect upstream: {e}"))?;
    for stream in [&client, &upstream] {
        stream
            .set_write_timeout(Some(Duration::from_secs(2)))
            .map_err(|e| e.to_string())?;
    }
    match wire {
        Wire::Agent { drop_execution } => {
            let (bytes, request) =
                frame(&mut client, deadline).map_err(|e| format!("read caller RPC: {e}"))?;
            log.lock().unwrap().requests.push(request.clone());
            let method = request["method"].as_str().unwrap_or("unknown");
            send(&mut upstream, &bytes).map_err(|e| format!("send upstream {method}: {e}"))?;
            // Bound both duration and count, including heartbeat/progress frames.
            for _ in 0..256 {
                let (bytes, response) = frame(&mut upstream, deadline)
                    .map_err(|e| format!("read upstream {method}: {e}"))?;
                if response["id"] != request["id"] {
                    return Err("relay RPC identity mismatch".into());
                }
                let terminal = response.get("ok").is_some();
                if terminal {
                    log.lock().unwrap().replies.push(json!({
                        "id":response["id"], "method":request["method"],
                        "ok":response["ok"], "status":response["result"]["status"],
                        "error_code":response["error"]["code"]
                    }));
                }
                let discard = terminal
                    && (request["method"] == "cancel_execution"
                        || (drop_execution && request["method"] == "run_operator_task_ir"));
                if discard {
                    log.lock().unwrap().discarded.push(response);
                } else {
                    send(&mut client, &bytes).map_err(|e| format!("send caller {method}: {e}"))?;
                }
                if terminal {
                    return Ok(());
                }
            }
            Err("relay progress cap exceeded".into())
        }
        Wire::Http {
            method,
            path,
            remove_identity,
            advertised_length,
            substitute_artifact,
            request_limit,
        } => {
            let (bytes, query) =
                http_message(&mut client, deadline, Some((method, &path)), request_limit)
                    .map_err(|e| format!("read caller HTTP: {e}"))?;
            log.lock().unwrap().requests.push(query);
            upstream
                .write_all(&bytes)
                .map_err(|e| format!("send upstream HTTP: {e}"))?;
            let (_, acknowledgement) = http_message(&mut upstream, deadline, None, LIMIT)
                .map_err(|e| format!("read upstream HTTP: {e}"))?;
            log.lock().unwrap().discarded.push(acknowledgement.clone());
            if let Some(length) = advertised_length {
                client.write_all(format!("HTTP/1.1 200 OK\r\nContent-Length: {length}\r\nConnection: close\r\n\r\n").as_bytes())
                    .map_err(|e| e.to_string())?;
                log.lock()
                    .unwrap()
                    .replies
                    .push(json!({"advertised_length":length}));
                return Ok(());
            }
            if remove_identity.is_some() || substitute_artifact.is_some() {
                let mut corrupted = acknowledgement;
                if let Some((record, identity)) = remove_identity {
                    corrupted
                        .get_mut(record)
                        .and_then(Value::as_object_mut)
                        .ok_or("actual acknowledgement has no record object")?
                        .remove(identity)
                        .ok_or("actual acknowledgement has no record identity")?;
                }
                if let Some(reference) = substitute_artifact {
                    *corrupted
                        .get_mut("artifact")
                        .ok_or("missing actual artifact")? = reference;
                }
                let body = serde_json::to_vec(&corrupted).map_err(|e| e.to_string())?;
                client
                    .write_all(
                        format!(
                            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                            body.len()
                        )
                        .as_bytes(),
                    )
                    .map_err(|e| e.to_string())?;
                client.write_all(&body).map_err(|e| e.to_string())?;
                log.lock().unwrap().replies.push(corrupted);
            }
            Ok(())
        }
    }
}

#[test]
fn accepted_nonblocking_socket_waits_for_delayed_rpc_without_rewriting_bytes() -> Result<()> {
    let upstream = TcpListener::bind(("127.0.0.1", 0)).map_err(|e| e.to_string())?;
    upstream.set_nonblocking(true).map_err(|e| e.to_string())?;
    let port = upstream.local_addr().map_err(|e| e.to_string())?.port();
    let listener = TcpListener::bind(("127.0.0.1", 0)).map_err(|e| e.to_string())?;
    let mut caller =
        TcpStream::connect(listener.local_addr().unwrap()).map_err(|e| e.to_string())?;
    let (accepted, _) = listener.accept().map_err(|e| e.to_string())?;
    // Force the inherited state on every platform so this regression is not
    // dependent on whether the host happens to inherit O_NONBLOCK on accept.
    accepted.set_nonblocking(true).map_err(|e| e.to_string())?;
    let query = br#"{ "id": "delayed", "method": "describe_agent", "params": {} }"#;
    let reply = br#"{ "id": "delayed", "ok": true, "result": { "value": 1.000 } }"#;
    let log = Mutex::new(Capture::default());
    let (ready, started) = std::sync::mpsc::channel();
    thread::scope(|scope| -> Result<()> {
        let server = scope.spawn(|| -> Result<()> {
            let deadline = Instant::now() + Duration::from_secs(2);
            let mut stream = loop {
                match upstream.accept() {
                    Ok((stream, _)) => break stream,
                    Err(error)
                        if error.kind() == std::io::ErrorKind::WouldBlock
                            && Instant::now() < deadline =>
                    {
                        thread::sleep(Duration::from_millis(5))
                    }
                    Err(error) => return Err(format!("test upstream accept: {error}")),
                }
            };
            stream.set_nonblocking(false).map_err(|e| e.to_string())?;
            ready.send(()).map_err(|e| e.to_string())?;
            let (bytes, _) = frame(&mut stream, Instant::now() + Duration::from_secs(5))?;
            assert_eq!(bytes, query, "relay rewrote the request");
            send(&mut stream, reply)
        });
        let proxy = scope.spawn(|| {
            relay(
                accepted,
                port,
                Wire::Agent {
                    drop_execution: false,
                },
                &log,
            )
        });
        started
            .recv_timeout(Duration::from_secs(2))
            .map_err(|e| e.to_string())?;
        thread::sleep(Duration::from_millis(50));
        caller
            .set_write_timeout(Some(Duration::from_secs(2)))
            .map_err(|e| e.to_string())?;
        send(&mut caller, query)?;
        let (bytes, _) = frame(&mut caller, Instant::now() + Duration::from_secs(5))?;
        assert_eq!(bytes, reply, "relay rewrote the response");
        server.join().map_err(|_| "upstream panicked")??;
        proxy.join().map_err(|_| "proxy panicked")??;
        assert_eq!(log.lock().unwrap().requests.len(), 1);
        Ok(())
    })
}
