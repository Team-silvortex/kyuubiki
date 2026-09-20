use super::*;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex, mpsc};

struct StatusServer {
    url: String,
    requests: Arc<Mutex<Vec<String>>>,
    stop: mpsc::Sender<()>,
    worker: Option<thread::JoinHandle<()>>,
}

impl StatusServer {
    fn start(respond: impl Fn(usize, &mut TcpStream) + Send + 'static) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        listener.set_nonblocking(true).unwrap();
        let requests = Arc::new(Mutex::new(Vec::new()));
        let observed = Arc::clone(&requests);
        let (stop, stopped) = mpsc::channel();
        let worker = thread::spawn(move || {
            while stopped.try_recv().is_err() {
                let (mut stream, _) = match listener.accept() {
                    Ok(connection) => connection,
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(1));
                        continue;
                    }
                    Err(error) => panic!("accept test connection: {error}"),
                };
                stream.set_nonblocking(false).unwrap();
                stream
                    .set_read_timeout(Some(Duration::from_secs(1)))
                    .unwrap();
                stream
                    .set_write_timeout(Some(Duration::from_secs(1)))
                    .unwrap();
                let mut request = Vec::new();
                let mut buffer = [0; 4096];
                while !request.windows(4).any(|part| part == b"\r\n\r\n") {
                    let length = stream.read(&mut buffer).unwrap();
                    if length == 0 {
                        break;
                    }
                    request.extend_from_slice(&buffer[..length]);
                }
                let index = {
                    let mut requests = observed.lock().unwrap();
                    requests.push(String::from_utf8(request).unwrap());
                    requests.len() - 1
                };
                respond(index, &mut stream);
            }
        });
        Self {
            url,
            requests,
            stop,
            worker: Some(worker),
        }
    }

    fn assert_only_status_reads(&self, expected: usize) {
        let requests = self.requests.lock().unwrap();
        assert_eq!(requests.len(), expected);
        assert!(requests.iter().all(|request| {
            request.starts_with("GET /api/v1/jobs/job-budget/status HTTP/1.1\r\n")
        }));
    }
}

impl Drop for StatusServer {
    fn drop(&mut self) {
        let _ = self.stop.send(());
        self.worker.take().unwrap().join().unwrap();
    }
}

fn response(stream: &mut TcpStream, status: &str) {
    let body = json!({"job": {"job_id": "job-budget", "status": status}}).to_string();
    let _ = write!(
        stream,
        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
}

fn wait(
    server: &StatusServer,
    options: Value,
) -> Result<HeadlessExecutorOutcome, HeadlessExecutorError> {
    let mut payload = json!({"job_id": "job-budget", "timeout_ms": 100, "interval_ms": 100});
    payload
        .as_object_mut()
        .unwrap()
        .extend(options.as_object().unwrap().clone());
    execute_job_wait(&server.url, None, &payload)
}

#[test]
fn rejects_a_completed_response_arriving_after_the_wait_budget() {
    let server = StatusServer::start(|_, stream| {
        thread::sleep(Duration::from_millis(400));
        response(stream, "completed");
    });
    let started = Instant::now();
    let result = wait(&server, json!({}));
    let elapsed = started.elapsed();
    let error = result.expect_err("late completion must not defeat the caller's budget");
    assert!(
        error
            .message
            .contains("timeout_reason=client_total_budget_exhausted"),
        "{error:?}"
    );
    assert!(error.message.contains("job-budget"));
    assert!(elapsed < Duration::from_millis(300), "elapsed={elapsed:?}");
    server.assert_only_status_reads(1);
}

#[test]
fn a_slow_drip_response_cannot_reset_the_wait_budget() {
    let server = StatusServer::start(|_, stream| {
        let body = r#"{"job":{"job_id":"job-budget","status":"completed"}}"#;
        let _ = write!(
            stream,
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n",
            body.len()
        );
        for byte in body.bytes() {
            thread::sleep(Duration::from_millis(10));
            if stream.write_all(&[byte]).is_err() {
                break;
            }
        }
    });
    let started = Instant::now();
    let result = wait(&server, json!({}));
    let elapsed = started.elapsed();
    let error = result.expect_err("progress on the socket must not extend the deadline");
    assert!(
        error
            .message
            .contains("timed out waiting for job job-budget"),
        "{error:?}"
    );
    assert!(elapsed < Duration::from_millis(300), "elapsed={elapsed:?}");
    server.assert_only_status_reads(1);
}

#[test]
fn fixed_wait_does_not_issue_an_extra_poll_after_sleep_exhausts_the_window() {
    let server = StatusServer::start(|index, stream| {
        response(stream, if index == 0 { "running" } else { "completed" });
    });
    let error = wait(&server, json!({"max_total_timeout_ms": 1_000}))
        .expect_err("fixed policy cannot consume a second observation window");
    assert!(
        error
            .message
            .contains("timeout_reason=client_window_exhausted"),
        "{error:?}"
    );
    assert!(error.message.contains("last_status=running"));
    server.assert_only_status_reads(1);
}

#[test]
fn malformed_wait_options_are_rejected_instead_of_silently_defaulted() {
    for options in [
        json!({"timeout_ms": "later"}),
        json!({"timeout_ms": -1}),
        json!({"interval_ms": false}),
        json!({"max_total_timeout_ms": 1.5}),
        json!({"resume_policy": true}),
        json!({"timeoutMs": null}),
        json!({"timeout_ms": 100, "timeoutMs": 200}),
        json!({"resume_policy": "fixed", "resumePolicy": "server_deadline"}),
    ] {
        let mut payload = json!({"job_id": "job-budget"});
        payload
            .as_object_mut()
            .unwrap()
            .extend(options.as_object().unwrap().clone());
        let error = execute_job_wait("http://127.0.0.1:0", None, &payload).unwrap_err();
        assert!(
            error.message.contains("job_wait validation failed"),
            "{options}: {error:?}"
        );
    }
}

#[test]
fn solve_and_wait_validates_timing_before_loading_or_submitting_a_model() {
    let server = StatusServer::start(|_, _| panic!("invalid options must not reach the service"));
    for options in [
        json!({"timeout_ms": 0}),
        json!({"intervalMs": "bad"}),
        json!({"timeout_ms": 200, "maxTotalTimeoutMs": 100}),
        json!({"resumePolicy": false}),
    ] {
        let mut payload = json!({"model_version_id": "version-safe"});
        payload
            .as_object_mut()
            .unwrap()
            .extend(options.as_object().unwrap().clone());
        let error = crate::service_executor_solve::execute_solve_and_wait_from_model_version(
            &server.url,
            None,
            &payload,
        )
        .unwrap_err();
        assert!(
            error.message.contains("job_wait validation failed"),
            "{error:?}"
        );
    }
    server.assert_only_status_reads(0);
}

#[test]
fn timed_out_observation_can_resume_the_same_job_without_submission_or_cancel() {
    let server = StatusServer::start(|index, stream| {
        response(stream, if index == 0 { "running" } else { "completed" });
    });
    let error = wait(&server, json!({})).unwrap_err();
    let receipt = crate::execution_observability::failure_preview(0, "job_wait", error.message);
    assert_eq!(receipt["error_code"], "kyuubiki.headless.job_wait_timeout");
    assert_eq!(receipt["failure_receipt"]["retryable"], true);
    let outcome = wait(&server, json!({"timeout_ms": 1_000})).unwrap();
    assert_eq!(outcome.result["job_id"], "job-budget");
    assert_eq!(outcome.result["status"], "completed");
    assert_eq!(outcome.result["wait"]["poll_attempts"], 1);
    server.assert_only_status_reads(2);
}

#[test]
fn server_authorized_resume_still_bounds_a_stalled_response_by_the_total_budget() {
    let server = StatusServer::start(|index, stream| {
        if index > 0 {
            thread::sleep(Duration::from_millis(500));
            response(stream, "completed");
        } else {
            let body = json!({"job": {"job_id": "job-budget", "status": "running",
                "status_detail": {"timing": {"phase": "execution",
                    "effective_timeout_ms": 10_000, "execution_elapsed_ms": 1}}
            }})
            .to_string();
            let _ = write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{body}",
                body.len()
            );
        }
    });
    let started = Instant::now();
    let error = wait(
        &server,
        json!({"resume_policy": "server_deadline", "max_total_timeout_ms": 200}),
    )
    .expect_err("server permission must not enlarge the client's total budget");
    assert!(started.elapsed() < Duration::from_millis(400));
    assert!(
        error
            .message
            .contains("timeout_reason=client_total_budget_exhausted"),
        "{error:?}"
    );
    assert!(error.message.contains("last_status=running"));
    assert!(error.message.contains("resume_count=1"));
    server.assert_only_status_reads(2);
}

#[test]
fn missing_or_expired_server_timing_does_not_authorize_another_request() {
    for (timing, reason) in [
        (Value::Null, "server_timing_unavailable"),
        (
            json!({"phase": "execution", "effective_timeout_ms": 20, "execution_elapsed_ms": 0}),
            "server_deadline_exhausted",
        ),
    ] {
        let server = StatusServer::start(move |_, stream| {
            let body = json!({"job": {"status": "running", "status_detail": {"timing": timing}}})
                .to_string();
            let _ = write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{body}",
                body.len()
            );
        });
        let error = wait(
            &server,
            json!({"resume_policy": "server_deadline", "max_total_timeout_ms": 1_000}),
        )
        .unwrap_err();
        assert!(
            error.message.contains(&format!("timeout_reason={reason}")),
            "{error:?}"
        );
        server.assert_only_status_reads(1);
    }
}

#[test]
fn connection_refusal_backoff_obeys_the_same_wait_deadline() {
    let reservation = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}", reservation.local_addr().unwrap());
    drop(reservation);
    let started = Instant::now();
    let error = execute_job_wait(
        &url,
        None,
        &json!({"job_id": "job-budget", "timeout_ms": 50}),
    )
    .unwrap_err();
    assert!(started.elapsed() < Duration::from_millis(300));
    assert!(
        error
            .message
            .contains("timeout_reason=client_total_budget_exhausted"),
        "{error:?}"
    );
}

#[test]
fn successful_and_unsuccessful_terminal_statuses_keep_their_semantics() {
    for status in ["completed", "failed", "cancelled"] {
        let server = StatusServer::start(move |_, stream| response(stream, status));
        let result = wait(&server, json!({"timeout_ms": 1_000}));
        if status == "completed" {
            assert_eq!(result.unwrap().result["status"], status);
        } else {
            assert!(
                result
                    .unwrap_err()
                    .message
                    .contains(&format!("terminal status {status}"))
            );
        }
        server.assert_only_status_reads(1);
    }
}

#[test]
fn numeric_string_and_camel_case_wait_options_remain_supported() {
    let server = StatusServer::start(|_, stream| response(stream, "completed"));
    let result = execute_job_wait(
        &server.url.replace("127.0.0.1", "localhost"),
        None,
        &json!({
            "jobId": "job-budget", "timeoutMs": "1000", "intervalMs": "10",
            "resumePolicy": "fixed", "maxTotalTimeoutMs": "2000"
        }),
    )
    .unwrap();
    assert_eq!(result.result["status"], "completed");
    server.assert_only_status_reads(1);
}

#[test]
fn status_transport_rejects_oversized_responses_before_decoding_json() {
    let server = StatusServer::start(|_, stream| {
        let _ = stream.write_all(b"HTTP/1.1 200 OK\r\n\r\n");
        let block = [b' '; 8192];
        for _ in 0..1024 {
            if stream.write_all(&block).is_err() {
                break;
            }
        }
    });
    let error = wait(&server, json!({"timeout_ms": 5_000})).unwrap_err();
    assert!(
        error
            .message
            .contains("response exceeds the 8000000-byte transport limit"),
        "{error:?}"
    );
    server.assert_only_status_reads(1);
}

#[test]
fn server_grant_can_complete_across_a_soft_window_with_auditable_resume_metadata() {
    let server = StatusServer::start(|index, stream| {
        if index == 0 {
            let body = json!({"job": {"job_id": "job-budget", "status": "running",
                "status_detail": {"timing": {"phase": "execution",
                    "effective_timeout_ms": 5_000, "execution_elapsed_ms": 1}}
            }})
            .to_string();
            let _ = write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{body}",
                body.len()
            );
        } else {
            thread::sleep(Duration::from_millis(200));
            response(stream, "completed");
        }
    });
    let outcome = wait(
        &server,
        json!({
            "interval_ms": 10, "resume_policy": "server_deadline", "max_total_timeout_ms": 1_000
        }),
    )
    .unwrap();
    assert_eq!(outcome.result["status"], "completed");
    assert_eq!(outcome.result["wait"]["poll_attempts"], 2);
    assert_eq!(outcome.result["wait"]["resume_count"], 1);
    server.assert_only_status_reads(2);
}
