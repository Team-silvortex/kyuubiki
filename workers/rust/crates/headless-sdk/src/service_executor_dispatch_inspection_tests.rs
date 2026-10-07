use super::*;
use crate::ServiceHeadlessExecutor;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::thread;

fn receipt() -> Value {
    serde_json::from_str(include_str!(
        "../../../../../schemas/examples.operator-task-dispatch-inspection.json"
    ))
    .unwrap()
}

#[test]
fn inspection_rejects_mismatched_identity_authority_and_false_terminal_claims() {
    let valid = receipt();
    validate(&valid, "task", &"a".repeat(64)).unwrap();
    for field in ["task_id", "task_digest", "schema_version"] {
        let mut bad = valid.clone();
        bad[field] = json!("wrong");
        assert!(validate(&bad, "task", &"a".repeat(64)).is_err());
    }
    for field in ["automatic_replay_authorized", "terminal_result_available"] {
        let mut bad = valid.clone();
        bad[field] = json!(true);
        assert!(validate(&bad, "task", &"a".repeat(64)).is_err());
    }
    for field in ["retained_attempt_count", "truncated", "attempts"] {
        let mut bad = valid.clone();
        bad[field] = Value::Null;
        assert!(validate(&bad, "task", &"a".repeat(64)).is_err());
    }
    let mut bad = valid.clone();
    bad["attempts"][0]["observation"]["outcome"] = json!("executed");
    assert!(validate(&bad, "task", &"a".repeat(64)).is_err());
    let mut terminal = valid;
    terminal["attempts"][0]["state"] = json!("observed_executed");
    assert!(validate(&terminal, "task", &"a".repeat(64)).is_err());
    terminal["attempts"][0]["observation"] =
        json!({"status":"retained_dispatch_observation","outcome":"observed_executed"});
    validate(&terminal, "task", &"a".repeat(64)).unwrap();
}

#[test]
fn malformed_inspection_queries_fail_before_network_access() {
    let executor = ServiceHeadlessExecutor::new("http://127.0.0.1:1");
    for (id, digest) in [
        ("", "a".repeat(64)),
        ("task", "A".repeat(64)),
        ("task", "a".repeat(63)),
        ("task", "z".repeat(64)),
    ] {
        assert!(
            executor
                .inspect_operator_task_dispatch(id, &digest)
                .unwrap_err()
                .message
                .contains("query identity")
        );
    }
}

#[test]
fn inspection_cancellation_target_must_match_every_observed_identity() {
    let mut valid = receipt();
    valid["attempts"][0]["observation"] = json!({
        "status":"original_endpoint_reports_active_request", "outcome":"unknown",
        "process_instance_id":"owner-boot", "generation":3, "cancel_requested":false,
        "execution_target":{"process_instance_id":"owner-boot", "request_id":"request", "generation":3, "job_id":"task"}
    });
    validate(&valid, "task", &"a".repeat(64)).unwrap();
    for (field, wrong) in [
        ("process_instance_id", json!("other-boot")),
        ("request_id", json!("other-request")),
        ("generation", json!(4)),
        ("job_id", json!("other-task")),
        ("extra", json!(true)),
    ] {
        let mut bad = valid.clone();
        bad["attempts"][0]["observation"]["execution_target"][field] = wrong;
        assert!(validate(&bad, "task", &"a".repeat(64)).is_err());
    }
    valid["attempts"][0]["observation"]["status"] = json!("request_not_observed_active");
    assert!(validate(&valid, "task", &"a".repeat(64)).is_err());
}

#[test]
fn journal_failure_after_dispatch_is_unknown_and_never_retryable() {
    let preview = crate::execution_observability::failure_preview(
        1,
        "operator_task_execute",
        "operator_task_dispatch_outcome_unknown: waiting for agent capacity".into(),
    );
    assert_eq!(
        preview["failure_receipt"]["category"],
        "operator_task_outcome_unknown"
    );
    assert_eq!(preview["failure_receipt"]["retryable"], false);
    assert_eq!(preview["failure_receipt"]["retry_strategy"], "none");
}

#[test]
fn fresh_sdk_session_uses_only_read_only_inspection_route_and_minimal_identity() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let worker = thread::spawn(move || {
        let (mut socket, _) = listener.accept().unwrap();
        socket
            .set_read_timeout(Some(Duration::from_secs(3)))
            .unwrap();
        let mut bytes = Vec::new();
        loop {
            let mut buffer = [0; 4096];
            let count = socket.read(&mut buffer).unwrap();
            assert!(count > 0);
            bytes.extend_from_slice(&buffer[..count]);
            assert!(bytes.len() < 8192);
            if let Some(split) = bytes.windows(4).position(|part| part == b"\r\n\r\n") {
                let header = std::str::from_utf8(&bytes[..split]).unwrap();
                let length = header
                    .lines()
                    .find_map(|line| line.strip_prefix("Content-Length: "))
                    .unwrap()
                    .parse::<usize>()
                    .unwrap();
                if bytes.len() < split + 4 + length {
                    continue;
                }
                assert!(
                    header.starts_with("POST /api/v1/operator-tasks/inspect-dispatch HTTP/1.1\r\n")
                );
                assert!(header.contains("Authorization: Bearer test-token"));
                let body: Value =
                    serde_json::from_slice(&bytes[split + 4..split + 4 + length]).unwrap();
                assert_eq!(body, json!({"task_id":"task","task_digest":"a".repeat(64)}));
                break;
            }
        }
        let body = serde_json::to_vec(&receipt()).unwrap();
        write!(
            socket,
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        )
        .unwrap();
        socket.write_all(&body).unwrap();
        listener.set_nonblocking(true).unwrap();
        thread::sleep(Duration::from_millis(40));
        assert!(listener.accept().is_err());
    });
    let inspected = ServiceHeadlessExecutor::with_token(&url, Some("test-token"))
        .inspect_operator_task_dispatch("task", &"a".repeat(64))
        .unwrap();
    assert_eq!(inspected, receipt());
    worker.join().unwrap();
}
