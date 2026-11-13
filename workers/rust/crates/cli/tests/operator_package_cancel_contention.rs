#[path = "support/operator_package.rs"]
mod support;

use serde_json::{Value, json};
use std::fs;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use support::{LiveAgent, external_operator_task, refresh_task_digest};

const JOB: &str = "held-package-fetch";
const BUSY: &str = "operator_package_job_release_busy";

#[test]
fn cancellation_and_release_do_not_wait_for_a_stalled_package_fetch() {
    exercise_stalled_fetch(false, false);
}

#[test]
fn cancellation_stops_a_partial_download_body_without_leaking_generation() {
    exercise_stalled_fetch(true, false);
}

#[test]
fn download_deadline_stops_a_partial_body_without_cancellation_or_replay() {
    exercise_stalled_fetch(true, true);
}

#[test]
fn stalled_fixture_waits_for_complete_http_headers() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    listener.set_nonblocking(true).unwrap();
    let (entered_tx, entered_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let (stop_tx, stop_rx) = mpsc::channel();
    let (closed_tx, _) = mpsc::channel();
    let stop = SignalOnDrop(stop_tx);
    let server = thread::spawn(move || {
        stalled_endpoint(listener, entered_tx, release_rx, stop_rx, closed_tx, false)
    });
    let release = SignalOnDrop(release_tx);
    let mut client = std::net::TcpStream::connect(address).unwrap();
    client
        .set_read_timeout(Some(Duration::from_secs(3)))
        .unwrap();
    client.write_all(b"GET /api/").unwrap();
    assert!(entered_rx.recv_timeout(Duration::from_millis(20)).is_err());
    client
        .write_all(b"resolve HTTP/1.1\r\nHost: localhost\r\n\r\n")
        .unwrap();
    entered_rx.recv_timeout(Duration::from_secs(3)).unwrap();
    drop(release);
    let mut reply = String::new();
    client.read_to_string(&mut reply).unwrap();
    assert!(reply.starts_with("HTTP/1.1 503"), "{reply}");
    drop(stop);
    assert_eq!(server.join().unwrap(), 1);
}

fn exercise_stalled_fetch(partial_body: bool, timeout: bool) {
    let root = Cleanup(std::env::temp_dir().join(format!(
        "kyuubiki-package-cancel-contention-{}-{partial_body}-{timeout}-{}",
        std::process::id(),
        SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos()
    )));
    let packages = root.0.join("store/packages");
    fs::create_dir_all(&packages).unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let (entered_tx, entered_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let (stop_tx, stop_rx) = mpsc::channel();
    let (closed_tx, closed_rx) = mpsc::channel();
    let stop = SignalOnDrop(stop_tx);
    let server = thread::spawn(move || {
        stalled_endpoint(
            listener,
            entered_tx,
            release_rx,
            stop_rx,
            closed_tx,
            partial_body,
        )
    });
    let agent = LiveAgent::start_orchestrated_with_fetch_timeout(
        &packages,
        &url,
        "test-only-contention-token",
        1,
        if timeout { 2000 } else { 120_000 },
    )
    .expect("start owned Agent");
    let descriptor = call(&agent, "fetch-policy", "describe_agent", json!({}));
    let policy = &descriptor["result"]["operator_package_runtime"]["isolation"]["package_fetch"];
    assert_eq!(policy["loads_operator_code"], false);
    assert_eq!(policy["timeout_ms"], if timeout { 2000 } else { 120_000 });
    thread::scope(|scope| {
        // Drop before scoped joins, including assertion failures, to release blocked I/O.
        let release = SignalOnDrop(release_tx);
        let (finished_tx, finished_rx) = mpsc::channel();
        let agent_ref = &agent;
        let execution = scope.spawn(move || {
            let reply = call(
                agent_ref,
                "held-fetch",
                "run_operator_task_ir",
                json!({
                    "mode":"execute", "job_id":JOB, "task_ir":fetch_task()
                }),
            );
            let _ = finished_tx.send(reply);
        });
        if entered_rx.recv_timeout(Duration::from_secs(10)).is_err() {
            drop(release);
            execution.join().unwrap();
            panic!(
                "package resolve not entered; execution: {:?}",
                finished_rx.try_recv()
            );
        }
        // Model an incomplete download workspace; the killed child cannot run its Drop cleanup.
        let sessions = root.0.join("store/agent-runtime-generations/sessions");
        let mut staged_files = 0;
        for session in fs::read_dir(&sessions).unwrap() {
            for generation in fs::read_dir(session.unwrap().path().join("generations")).unwrap() {
                let temporary = generation.unwrap().path().join(".fetch-temporary");
                fs::write(
                    temporary.join("fixture-partial-download"),
                    b"unfinished-test-bytes",
                )
                .unwrap();
                staged_files += 1;
            }
        }
        assert_eq!(
            staged_files, 1,
            "exactly one unpublished download generation"
        );
        let (cancel_tx, cancel_rx) = mpsc::channel();
        let agent_ref = &agent;
        let cancellation = scope.spawn(move || {
            let response = call(
                agent_ref,
                "cancel-held",
                "cancel_job",
                json!({"job_id":"cleanup-ack-only"}),
            );
            let _ = cancel_tx.send(response);
        });
        let before_release = cancel_rx.recv_timeout(Duration::from_secs(2));
        if before_release.is_err() {
            drop(release);
            cancellation.join().unwrap();
            execution.join().unwrap();
            panic!("cancel acknowledgment waited for package fetch to finish");
        }
        let reply = before_release.unwrap();
        assert_eq!(reply["ok"], true, "{reply}");
        assert_eq!(reply["result"]["cancel_registered"], true);
        assert_eq!(reply["result"]["execution_terminal_confirmed"], false);
        let cleanup = &reply["result"]["operator_package_job_release"];
        assert_eq!(cleanup["error_code"], BUSY, "{reply}");
        assert_eq!(cleanup["status"], "deferred");
        assert_eq!(cleanup["operator_package_cleanup_performed"], false);
        assert_eq!(cleanup["retry_method"], "release_operator_package_job");
        assert_eq!(cleanup["automatic_replay_authorized"], false);

        let descriptor = call(&agent, "still-responsive", "describe_agent", json!({}));
        let active = descriptor["result"]["solver_control"]["active"]
            .as_array()
            .unwrap();
        assert!(
            active
                .iter()
                .any(|row| row["request_id"] == "held-fetch" && row["cancel_requested"] == false)
        );
        assert_eq!(
            descriptor["result"]["operator_package_runtime"]["job_release_policy"]["on_lifecycle_contention"],
            "reject_without_waiting_or_mutation"
        );
        let explicit = call(
            &agent,
            "release-busy",
            "release_operator_package_job",
            json!({"job_id":JOB}),
        );
        assert_eq!(explicit["ok"], false, "{explicit}");
        assert_eq!(explicit["error"]["code"], BUSY);
        assert_eq!(
            explicit["error"]["details"]["retry_method"],
            "release_operator_package_job"
        );
        let invalid = call(
            &agent,
            "invalid-busy",
            "release_operator_package_job",
            json!({"job_id":"bad\njob"}),
        );
        assert_eq!(invalid["error"]["code"], "operator_package_job_id_invalid");
        let excess = call(
            &agent,
            "capacity-still-held",
            "solve_bar_1d",
            solver_params(),
        );
        assert_eq!(excess["error"]["code"], "agent_at_capacity", "{excess}");

        if !timeout {
            let cancelled = call(
                &agent,
                "cancel-download",
                "cancel_job",
                json!({"job_id":JOB}),
            );
            assert_eq!(cancelled["result"]["cancel_registered"], true);
        }
        let finished_before_release = finished_rx.recv_timeout(Duration::from_secs(4));
        let closed_before_release = closed_rx.recv_timeout(Duration::from_secs(2));
        drop(release);
        cancellation.join().unwrap();
        execution.join().unwrap();
        let failed =
            finished_before_release.expect("cancelled fetch must exit before server releases HTTP");
        assert_eq!(failed["ok"], false, "{failed}");
        assert_eq!(
            failed["error"]["code"],
            if timeout {
                "operator_package_fetch_timeout"
            } else {
                "cancelled"
            },
            "{failed}"
        );
        assert_eq!(
            failed["error"]["details"]["operator_task_failure_receipt"]["failure_stage"],
            "fetch_package"
        );
        assert!(failed["result"].is_null());
        closed_before_release.expect("terminal reply must follow closing the downloader socket");
    });
    let retry = call(
        &agent,
        "retry-cleanup-only",
        "release_operator_package_job",
        json!({"job_id":JOB}),
    );
    assert_eq!(retry["ok"], true, "{retry}");
    assert_eq!(retry["result"]["disposition"], "already_released");
    assert_eq!(fs::read_dir(&packages).unwrap().count(), 0);
    let sessions = root.0.join("store/agent-runtime-generations/sessions");
    for session in
        fs::read_dir(&sessions).unwrap_or_else(|error| panic!("{}: {error}", sessions.display()))
    {
        assert_eq!(
            fs::read_dir(session.unwrap().path().join("generations"))
                .unwrap()
                .count(),
            0,
            "cancelled/expired fetch left a generation or temporary files in {}",
            sessions.display()
        );
    }
    let healthy = call(&agent, "healthy-next", "solve_bar_1d", solver_params());
    assert_eq!(healthy["ok"], true, "{healthy}");
    assert!((healthy["result"]["tip_displacement"].as_f64().unwrap() - 0.01).abs() < 1e-12);
    assert_eq!(healthy["result"]["max_stress"], 10.0);
    drop(agent);
    drop(stop);
    assert_eq!(
        server.join().unwrap(),
        1,
        "download must never automatically retry"
    );
}

fn call(agent: &LiveAgent, id: &str, method: &str, params: Value) -> Value {
    let response = agent.request(id, method, params).expect("Agent RPC");
    assert_eq!(response["id"], id);
    response
}

fn solver_params() -> Value {
    json!({"job_id":JOB,"length":1.0,"area":2.0,"youngs_modulus":1000.0,
        "elements":2,"tip_force":20.0})
}

fn fetch_task() -> Value {
    let mut task = external_operator_task(&"a".repeat(64));
    let reference = "orchestra://operator-package/extract.template_summary";
    task["execution_program"]["package_ref"] = json!(reference);
    task["runtime_hints"]["package_ref"] = json!(reference);
    task["runtime_hints"]["authority_mode"] = json!("central_operator_library");
    task["runtime_hints"]["execution_mode"] = json!("orchestra_fetch");
    task["runtime_hints"]["cache_scope"] = json!("job");
    task["runtime_hints"]["agent_fetchable"] = json!(true);
    refresh_task_digest(&mut task);
    task
}

fn stalled_endpoint(
    listener: TcpListener,
    entered: Sender<()>,
    release: Receiver<()>,
    stop: Receiver<()>,
    closed: Sender<()>,
    partial_body: bool,
) -> usize {
    let mut resolve_count = 0;
    loop {
        if !matches!(stop.try_recv(), Err(mpsc::TryRecvError::Empty)) {
            return resolve_count;
        }
        let (mut stream, _) = match listener.accept() {
            Ok(connection) => connection,
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(1));
                continue;
            }
            Err(error) => panic!("accept fixture: {error}"),
        };
        // macOS can inherit the listener's nonblocking flag on accepted sockets.
        stream.set_nonblocking(false).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(3)))
            .unwrap();
        stream
            .set_write_timeout(Some(Duration::from_secs(3)))
            .unwrap();
        let mut header = Vec::new();
        while !header.ends_with(b"\r\n\r\n") && header.len() < 16384 {
            let mut byte = [0];
            if stream.read_exact(&mut byte).is_err() {
                break;
            }
            header.push(byte[0]);
        }
        let resolving = String::from_utf8_lossy(&header)
            .lines()
            .next()
            .is_some_and(|line| line.contains("/resolve"));
        if resolving {
            resolve_count += 1;
            if partial_body {
                write!(stream, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 100\r\nConnection: close\r\n\r\n{{").unwrap();
                stream.flush().unwrap();
            }
            entered.send(()).unwrap();
            stream.set_nonblocking(true).unwrap();
            let deadline = std::time::Instant::now() + Duration::from_secs(15);
            while std::time::Instant::now() < deadline
                && matches!(release.try_recv(), Err(mpsc::TryRecvError::Empty))
            {
                match stream.peek(&mut [0]) {
                    Ok(0) => {
                        let _ = closed.send(());
                        break;
                    }
                    Err(error)
                        if matches!(
                            error.kind(),
                            std::io::ErrorKind::ConnectionReset
                                | std::io::ErrorKind::ConnectionAborted
                        ) =>
                    {
                        let _ = closed.send(());
                        break;
                    }
                    _ => thread::sleep(Duration::from_millis(1)),
                }
            }
            stream.set_nonblocking(false).unwrap();
            if partial_body {
                continue;
            }
        }
        let status = if resolving {
            "503 Service Unavailable"
        } else {
            "200 OK"
        };
        let _ = write!(
            stream,
            "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{{}}"
        );
    }
}

struct SignalOnDrop(Sender<()>);
impl Drop for SignalOnDrop {
    fn drop(&mut self) {
        let _ = self.0.send(());
    }
}

struct Cleanup(PathBuf);
impl Drop for Cleanup {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
