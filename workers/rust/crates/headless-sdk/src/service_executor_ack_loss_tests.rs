use crate::{
    HeadlessExecutionBatch, HeadlessExecutionBatchStep, HeadlessExecutor, HeadlessExecutorError,
    HeadlessExecutorOutcome, ServiceHeadlessExecutor, execute_batch_with_executor,
    find_action_contract,
};
use serde_json::{Value, json};
use std::io::{Read, Write};
use std::net::TcpListener;
use std::time::{Duration, Instant};

struct RecordingService {
    service: ServiceHeadlessExecutor,
    calls: Vec<String>,
}

impl HeadlessExecutor for RecordingService {
    fn name(&self) -> &'static str {
        "service"
    }

    fn execute_step(
        &mut self,
        action: &str,
        index: usize,
        payload: &Value,
    ) -> Result<HeadlessExecutorOutcome, HeadlessExecutorError> {
        self.calls.push(action.into());
        self.service.execute_step(action, index, payload)
    }
}

fn observe<R>(response: &[u8], run: impl FnOnce(&str) -> R) -> (R, String) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let response = response.to_vec();
    let worker = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        stream
            .set_write_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        let mut request = Vec::new();
        let mut buffer = [0; 4096];
        loop {
            let size = stream.read(&mut buffer).unwrap();
            assert!(size > 0 && request.len() + size <= 64 * 1024);
            request.extend_from_slice(&buffer[..size]);
            if let Some(split) = request.windows(4).position(|part| part == b"\r\n\r\n") {
                let head = std::str::from_utf8(&request[..split]).unwrap();
                let length = head
                    .lines()
                    .find_map(|line| {
                        let (name, value) = line.split_once(':')?;
                        name.eq_ignore_ascii_case("content-length")
                            .then(|| value.trim().parse::<usize>().unwrap())
                    })
                    .unwrap_or(0);
                if request.len() >= split + 4 + length {
                    break;
                }
            }
        }
        stream.write_all(&response).unwrap();
        String::from_utf8(request).unwrap()
    });
    let result = run(&url);
    (result, worker.join().unwrap())
}

fn run(url: &str, action: &str, payload: Value) -> (crate::HeadlessRunReport, Vec<String>) {
    let batch = HeadlessExecutionBatch {
        schema_version: "kyuubiki.headless-execution-batch/v1".into(),
        exported_at: "2026-10-07T00:00:00Z".into(),
        language: "en".into(),
        workflow_id: "lost-write-ack".into(),
        template_id: None,
        warnings: vec![],
        steps: vec![
            HeadlessExecutionBatchStep {
                index: 1,
                action: action.into(),
                risk: find_action_contract(action).unwrap().risk,
                payload,
            },
            HeadlessExecutionBatchStep {
                index: 2,
                action: "project_create".into(),
                risk: crate::HeadlessRisk::Normal,
                payload: json!({"name":"must-not-follow-unknown-write"}),
            },
        ],
    };
    let mut executor = RecordingService {
        service: ServiceHeadlessExecutor::new(url),
        calls: vec![],
    };
    let report = execute_batch_with_executor(&batch, &mut executor, true, true);
    (report, executor.calls)
}

#[test]
fn lost_or_invalid_write_acknowledgements_never_offer_replay_or_downstream_writes() {
    for response in [
        "",
        "HTTP/1.1 200 OK\r\nContent-Length: 1000\r\n\r\n{\"project\":{\"project_id\":\"owned\"}}",
        "HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n",
        "HTTP/1.1 200 OK\r\nContent-Length: 1\r\n\r\n{",
    ] {
        let ((report, calls), request) = observe(response.as_bytes(), |url| {
            run(url, "project_create", json!({"name":"owned-lost-ack"}))
        });
        assert!(request.starts_with("POST /api/v1/projects HTTP/1.1\r\n"));
        assert!(request.ends_with("{\"name\":\"owned-lost-ack\"}"));
        assert_eq!(report.status, "failed");
        assert_eq!(report.executed_step_count, 0);
        assert_eq!(calls, ["project_create"]);
        let failure = report.execution_summary.failure.unwrap();
        assert_eq!(
            failure.error_code,
            "kyuubiki.headless.service_request_outcome_unknown"
        );
        assert_eq!(failure.stage, "transport");
        assert!(!failure.retryable);
        assert_eq!(failure.retry_strategy, "none");
    }
}

#[test]
fn complete_server_error_on_a_write_does_not_prove_nonexecution() {
    let body = r#"{"error":"failed to connect to a peer after accepting work"}"#;
    let response = format!(
        "HTTP/1.1 500 Server Error\r\nContent-Length: {}\r\n\r\n{body}",
        body.len()
    );
    let ((report, calls), _) = observe(response.as_bytes(), |url| {
        run(url, "project_create", json!({"name":"owned-server-error"}))
    });
    assert_eq!(calls, ["project_create"]);
    let failure = report.execution_summary.failure.unwrap();
    assert_eq!(failure.category, "service_request_outcome_unknown");
    assert!(!failure.retryable);
    assert!(failure.message.contains("500"));
}

#[test]
fn read_only_ack_loss_and_pre_send_connect_failure_keep_transport_diagnostics() {
    let ((report, calls), _) = observe(b"", |url| {
        run(url, "job_fetch", json!({"job_id":"owned-read"}))
    });
    assert_eq!(calls, ["job_fetch"]);
    let failure = report.execution_summary.failure.unwrap();
    assert_eq!(failure.category, "transport_failure");
    assert!(failure.retryable);

    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    drop(listener);
    let (report, calls) = run(&url, "project_create", json!({"name":"not-sent"}));
    assert_eq!(calls, ["project_create"]);
    let failure = report.execution_summary.failure.unwrap();
    assert_eq!(failure.category, "transport_failure");
    assert!(failure.retryable);
    assert!(failure.message.contains("failed to connect"));
}

#[test]
fn deadline_write_and_streamed_upload_share_the_uncertain_ack_gate() {
    for deadline in [false, true] {
        let (result, request) = observe(b"", |url| {
            crate::service_executor::request_json_with_timeout(
                url,
                None,
                "PATCH",
                "/api/v1/projects/owned",
                Some(json!({"name":"changed"})),
                deadline.then(|| Instant::now() + Duration::from_secs(1)),
                Duration::from_secs(1),
            )
        });
        assert!(request.starts_with("PATCH /api/v1/projects/owned HTTP/1.1"));
        assert!(
            result
                .unwrap_err()
                .message
                .starts_with("service_request_outcome_unknown:")
        );
    }
    let file = std::env::temp_dir().join(format!("kyuubiki-ack-upload-{}", std::process::id()));
    std::fs::write(&file, b"{\"owned\":true}").unwrap();
    let (result, request) = observe(b"", |url| {
        crate::service_executor_artifact_http::request_file(
            url,
            None,
            "POST",
            "/api/v1/model-artifacts",
            "application/json",
            &file,
        )
    });
    std::fs::remove_file(file).unwrap();
    assert!(request.ends_with("{\"owned\":true}"));
    assert!(
        result
            .unwrap_err()
            .message
            .starts_with("service_request_outcome_unknown:")
    );
}

#[test]
fn complete_client_rejection_and_204_delete_keep_explicit_acknowledgements() {
    let ((report, _), _) = observe(
        b"HTTP/1.1 403 Forbidden\r\nContent-Length: 2\r\n\r\n{}",
        |url| run(url, "project_create", json!({"name":"forbidden"})),
    );
    let failure = report.execution_summary.failure.unwrap();
    assert_eq!(failure.category, "authorization_failure");
    assert!(!failure.retryable);
    let (result, _) = observe(
        b"HTTP/1.1 204 No Content\r\nContent-Length: 0\r\n\r\n",
        |url| {
            crate::service_executor::request_json(
                url,
                None,
                "DELETE",
                "/api/v1/projects/owned",
                None,
            )
        },
    );
    assert_eq!(result.unwrap(), Value::Null);
}
