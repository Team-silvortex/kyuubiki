use super::*;
use crate::execution_observability::failure_preview;
use crate::service_executor::request_json_with_timeout;
use crate::service_executor_ack_loss_tests::{observe, run};
use crate::service_executor_artifact_http::request_file;
use crate::service_executor_deadline::read_limited_before_deadline;
use crate::service_executor_http::decode_http_response_body;
use crate::{HeadlessExecutor, ServiceHeadlessExecutor};
use serde_json::json;
use std::borrow::Cow;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::thread;
use std::time::Instant;

fn serve<R>(
    reply: impl FnOnce(&mut TcpStream) + Send + 'static,
    call: impl FnOnce(&str) -> R,
) -> (R, Vec<u8>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let server = thread::spawn(move || {
        let deadline = Instant::now() + Duration::from_secs(3);
        let mut stream = loop {
            match listener.accept() {
                Ok((stream, _)) => break stream,
                Err(error)
                    if error.kind() == std::io::ErrorKind::WouldBlock
                        && Instant::now() < deadline =>
                {
                    thread::sleep(Duration::from_millis(2));
                }
                Err(error) => panic!("bounded test accept: {error}"),
            }
        };
        stream.set_nonblocking(false).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        stream
            .set_write_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        let mut query = Vec::new();
        let mut buffer = [0; 8192];
        loop {
            let size = stream.read(&mut buffer).unwrap();
            assert!(size > 0 && query.len() + size <= 16 * 1024 * 1024);
            query.extend_from_slice(&buffer[..size]);
            if let Some(end) = query.windows(4).position(|bytes| bytes == b"\r\n\r\n") {
                let head = std::str::from_utf8(&query[..end]).unwrap();
                let size =
                    crate::service_executor_response::response_body_length(head, "test request")
                        .unwrap()
                        .unwrap_or(0);
                if query.len() == end + 4 + size {
                    break;
                }
            }
        }
        reply(&mut stream);
        query
    });
    let result = call(&url);
    (result, server.join().unwrap())
}

fn header(length: usize) -> String {
    format!("HTTP/1.1 200 OK\r\nContent-Length: {length}\r\nConnection: close\r\n\r\n")
}

#[test]
fn endpoint_budgets_preserve_large_results_and_original_receipt_limits() {
    for path in [
        "/api/health",
        "/api/v1/projects",
        "/api/v1/operator-tasks/execute",
        "/api/v1/jobs",
        "/api/v1/models/a/extra",
    ] {
        assert_eq!(response_budget(path).0, 64 * 1024 * 1024, "{path}");
    }
    for path in [
        "/api/v1/jobs/a/status",
        "/api/v1/jobs/a/status?detail=1",
        "/api/v1/model-artifacts",
    ] {
        assert_eq!(response_budget(path).0, 8_000_000, "{path}");
    }
    for path in [
        "/api/v1/jobs/a",
        "/api/v1/results/a?raw=1",
        "/api/v1/models/a",
        "/api/v1/model-versions/a",
    ] {
        assert_eq!(response_budget(path).0, 512 * 1024 * 1024, "{path}");
    }
    assert_eq!(
        response_budget("/api/v1/operator-tasks/fetch-dispatch-result").0,
        10 * 1024 * 1024
    );
}

#[test]
fn ordinary_requests_have_total_budgets_without_resetting_on_received_bytes() {
    assert_eq!(
        default_request_timeout("/api/health", Duration::from_secs(30)),
        Duration::from_secs(30)
    );
    assert_eq!(
        default_request_timeout("/api/v1/results/a", Duration::from_secs(30)),
        Duration::from_secs(600)
    );
}

#[test]
fn split_header_delimiters_and_exact_wire_limits_are_checked_once() {
    let bytes = format!("{}{{}}", header(2)).into_bytes();
    let split = bytes.len() - 3;
    assert!(!check_response_head(&bytes[..split], 0, bytes.len(), "test").unwrap());
    assert!(check_response_head(&bytes, split.saturating_sub(3), bytes.len(), "test").unwrap());
    assert!(
        check_response_head(&bytes, 0, bytes.len() - 1, "test")
            .unwrap_err()
            .message
            .starts_with(RESPONSE_LIMIT)
    );
    assert!(check_response_head(&bytes, 0, 1, "test").is_err());
}

#[test]
fn oversized_headers_are_rejected_with_or_without_a_delimiter() {
    for terminated in [false, true] {
        let mut bytes = vec![b'x'; MAX_RESPONSE_HEADER_BYTES + 1];
        if terminated {
            bytes.extend_from_slice(b"\r\n\r\n{}");
        }
        assert!(
            check_response_head(&bytes, 0, MAX_LARGE_RESPONSE_BYTES, "large")
                .unwrap_err()
                .message
                .starts_with(RESPONSE_LIMIT)
        );
    }
}

#[test]
fn header_validation_does_not_accept_ambiguous_or_unusable_lengths() {
    for fields in [
        "Content-Length: +2",
        "Content-Length: -2",
        "Content-Length:",
        "Content-Length: 184467440737095516160",
        "Content-Length: 2\r\nContent-Length: 2",
        "Content-Length: 2\r\nTransfer-Encoding: chunked",
    ] {
        let bytes = format!("HTTP/1.1 200 OK\r\n{fields}\r\n\r\n{{}}");
        assert!(
            check_response_head(bytes.as_bytes(), 0, MAX_SERVICE_RESPONSE_BYTES, "test")
                .unwrap_err()
                .message
                .contains("invalid HTTP response")
        );
    }
}

#[test]
fn advertised_oversize_stops_before_waiting_for_body_or_connection_close() {
    let started = Instant::now();
    let (error, query) = serve(
        |stream| {
            stream
                .write_all(header(MAX_SERVICE_RESPONSE_BYTES).as_bytes())
                .unwrap();
            let mut byte = [0];
            assert_eq!(
                stream.read(&mut byte).unwrap(),
                0,
                "caller should close without reading a body"
            );
        },
        |url| {
            request_json_with_timeout(
                url,
                None,
                "GET",
                "/api/health",
                None,
                None,
                Duration::from_secs(1),
            )
            .unwrap_err()
        },
    );
    assert!(error.message.starts_with(RESPONSE_LIMIT));
    assert!(started.elapsed() < Duration::from_secs(1));
    assert!(query.starts_with(b"GET /api/health HTTP/1.1\r\n"));
}

#[test]
fn oversized_reads_stop_bindings_and_later_writes_without_retry_permission() {
    let ((report, calls), _) = observe(header(MAX_LARGE_RESPONSE_BYTES).as_bytes(), |url| {
        run(url, "job_fetch", json!({"job_id":"owned"}))
    });
    assert_eq!(calls, ["job_fetch"]);
    assert_eq!(report.status, "failed");
    assert_eq!(report.executed_step_count, 0);
    let failure = report.execution_summary.failure.unwrap();
    assert_eq!(failure.category, "service_response_limit_exceeded");
    assert_eq!(failure.stage, "transport");
    assert!(!failure.retryable);
    assert_eq!(failure.retry_strategy, "none");
    for key in ["job", "job_id", "result", "raw"] {
        assert!(report.steps[0].result_preview.get(key).is_none());
    }
}

#[test]
fn oversized_write_acknowledgements_remain_unknown_instead_of_replayable() {
    let ((report, calls), _) = observe(header(MAX_SERVICE_RESPONSE_BYTES).as_bytes(), |url| {
        run(url, "project_create", json!({"name":"owned"}))
    });
    assert_eq!(calls, ["project_create"]);
    let failure = report.execution_summary.failure.unwrap();
    assert_eq!(failure.category, "service_request_outcome_unknown");
    assert!(failure.message.contains(RESPONSE_LIMIT));
    assert!(!failure.retryable);
    assert_eq!(failure.retry_strategy, "none");
}

#[test]
fn unframed_and_chunked_growth_hit_the_observed_wire_cap() {
    for fields in ["", "Transfer-Encoding: chunked\r\n"] {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let bytes = format!("HTTP/1.1 200 OK\r\n{fields}\r\n{}", "x".repeat(40_000));
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_write_timeout(Some(Duration::from_secs(1)))
                .unwrap();
            let _ = stream.write_all(bytes.as_bytes());
        });
        let mut stream = TcpStream::connect(address).unwrap();
        let error = read_limited_before_deadline(
            &mut stream,
            Instant::now() + Duration::from_secs(1),
            Duration::from_secs(1),
            32_768,
            "test",
        )
        .unwrap_err();
        assert!(error.message.starts_with(RESPONSE_LIMIT));
        drop(stream);
        server.join().unwrap();
    }
}

#[test]
fn slow_trickle_cannot_renew_ordinary_read_or_write_request_deadlines() {
    for method in ["GET", "POST"] {
        let started = Instant::now();
        let (error, _) = serve(
            |stream| {
                stream
                    .write_all(b"HTTP/1.1 200 OK\r\nConnection: close\r\n\r\n{\"value\":\"")
                    .unwrap();
                for _ in 0..50 {
                    thread::sleep(Duration::from_millis(10));
                    if stream.write_all(b"x").is_err() {
                        break;
                    }
                }
            },
            |url| {
                request_json_with_timeout(
                    url,
                    None,
                    method,
                    "/api/health",
                    None,
                    None,
                    Duration::from_millis(150),
                )
                .unwrap_err()
            },
        );
        assert!(started.elapsed() < Duration::from_secs(1));
        let preview = failure_preview(1, "service_health", error.message);
        let failure = &preview["failure_receipt"];
        assert_eq!(
            failure["category"],
            if method == "GET" {
                "transport_failure"
            } else {
                "service_request_outcome_unknown"
            }
        );
        assert_eq!(failure["retryable"], method == "GET");
    }
}

#[test]
fn nonchunked_json_borrows_the_original_body_without_a_full_string_copy() {
    let body = "{\"result\":{\"value\":1}}";
    let decoded = decode_http_response_body("HTTP/1.1 200 OK", body, "test").unwrap();
    assert!(matches!(decoded, Cow::Borrowed(_)));
    assert_eq!(decoded.as_ptr(), body.as_ptr());
    assert!(matches!(
        decode_http_response_body(
            "HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked",
            "2\r\n{}\r\n0\r\n\r\n",
            "test"
        )
        .unwrap(),
        Cow::Owned(_)
    ));
}

#[test]
fn ordinary_job_and_result_reads_larger_than_status_budget_keep_full_values() {
    for action in ["job_fetch", "result_fetch"] {
        let body = json!({"job":{"job_id":"owned","status":"completed"},"job_id":"owned","result":{"data":"x".repeat(8_000_001)}}).to_string();
        let (result, _) = serve(
            move |stream| {
                stream.write_all(header(body.len()).as_bytes()).unwrap();
                stream.write_all(body.as_bytes()).unwrap();
            },
            |url| {
                ServiceHeadlessExecutor::new(url)
                    .execute_step(action, 1, &json!({"job_id":"owned"}))
                    .unwrap()
            },
        );
        assert_eq!(
            result.result["result"]["data"].as_str().unwrap().len(),
            8_000_001
        );
    }
}

struct OwnedFile(PathBuf);
impl Drop for OwnedFile {
    fn drop(&mut self) {
        std::fs::remove_file(&self.0).unwrap();
    }
}

#[test]
fn artifact_upload_streams_exact_bytes_and_bounds_its_acknowledgement() {
    let directory = std::env::temp_dir();
    let path = directory.join(format!(
        "headless-response-budget-{}-{}.json",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let body = "x".repeat(150_001);
    let mut output = std::fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&path)
        .unwrap();
    let file = OwnedFile(path);
    output.write_all(body.as_bytes()).unwrap();
    drop(output);
    for oversized in [false, true] {
        let (result, query) = serve(
            move |stream| {
                let response = if oversized {
                    header(8_000_000)
                } else {
                    format!("{}{{}}", header(2))
                };
                stream.write_all(response.as_bytes()).unwrap();
            },
            |url| {
                request_file(
                    url,
                    None,
                    "POST",
                    "/api/v1/model-artifacts",
                    "application/json",
                    &file.0,
                )
            },
        );
        let end = query
            .windows(4)
            .position(|bytes| bytes == b"\r\n\r\n")
            .unwrap();
        assert_eq!(&query[end + 4..], body.as_bytes());
        if oversized {
            let error = result.unwrap_err();
            assert!(
                error
                    .message
                    .starts_with("service_request_outcome_unknown:")
            );
            assert!(error.message.contains(RESPONSE_LIMIT));
        } else {
            let result = result.unwrap();
            assert_eq!(result.envelope, json!({}));
            assert_eq!(result.size_bytes, body.len() as u64);
            use sha2::{Digest, Sha256};
            assert_eq!(
                result.sha256,
                format!("{:x}", Sha256::digest(body.as_bytes()))
            );
        }
    }
}

#[test]
fn bounded_transport_keeps_length_and_json_acknowledgement_checks() {
    for bytes in [
        "HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\n{}extra",
        "HTTP/1.1 200 OK\r\nContent-Length: 5\r\n\r\n{}",
        "HTTP/1.1 200 OK\r\nContent-Length: 3\r\n\r\nxxx",
    ] {
        let (result, _) = observe(bytes.as_bytes(), |url| {
            request_json_with_timeout(
                url,
                None,
                "POST",
                "/api/health",
                None,
                None,
                Duration::from_secs(1),
            )
        });
        assert!(
            result
                .unwrap_err()
                .message
                .starts_with("service_request_outcome_unknown:")
        );
    }
}
