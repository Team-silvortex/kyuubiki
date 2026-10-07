use super::*;
use crate::executor_outcome_tests::golden_task;
use crate::{HeadlessExecutor, ServiceHeadlessExecutor};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::mpsc;
use std::thread;

fn completed() -> Value {
    let mut receipt = crate::prepare_operator_task_payload(&json!({"task":golden_task()})).unwrap();
    receipt["status"] = json!("executed");
    receipt["result"] = json!({"value":1.0/3.0});
    receipt
}

fn read_request(stream: &mut TcpStream) -> Value {
    stream
        .set_read_timeout(Some(Duration::from_secs(3)))
        .unwrap();
    let mut bytes = Vec::new();
    loop {
        let mut buffer = [0; 4096];
        let count = stream.read(&mut buffer).unwrap();
        assert!(count > 0);
        bytes.extend_from_slice(&buffer[..count]);
        assert!(bytes.len() < 64 * 1024);
        if let Some(split) = bytes.windows(4).position(|part| part == b"\r\n\r\n") {
            let header = std::str::from_utf8(&bytes[..split]).unwrap();
            assert!(header.starts_with("POST /api/v1/operator-tasks/execute HTTP/1.1\r\n"));
            let length = header
                .lines()
                .find_map(|line| {
                    line.strip_prefix("Content-Length: ")
                        .map(|value| value.parse::<usize>().unwrap())
                })
                .unwrap();
            if bytes.len() >= split + 4 + length {
                return serde_json::from_slice(&bytes[split + 4..split + 4 + length]).unwrap();
            }
        }
    }
}

fn send_response(stream: &mut TcpStream, receipt: &Value) -> std::io::Result<()> {
    let body = serde_json::to_vec(receipt).unwrap();
    let header = format!(
        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    stream.write_all(header.as_bytes())?;
    stream.write_all(&body)
}

#[test]
fn budget_is_bounded_and_total_includes_queue_execution_and_response_grace() {
    let schema: Value = serde_json::from_str(include_str!(
        "../../../../../schemas/operator-task-request-budget.schema.json"
    ))
    .unwrap();
    let example: Value = serde_json::from_str(include_str!(
        "../../../../../schemas/examples.operator-task-request-budget.json"
    ))
    .unwrap();
    let default = OperatorTaskRequestBudget::default();
    assert_eq!(example, default.to_value());
    assert_eq!(schema["properties"]["schema_version"]["const"], SCHEMA);
    assert_eq!(schema["additionalProperties"], false);
    for field in ["queue_timeout_ms", "request_timeout_ms"] {
        assert_eq!(schema["properties"][field]["minimum"], 1);
        assert_eq!(schema["properties"][field]["maximum"], MAX_PHASE_MS);
    }
    assert_eq!(default.queue_timeout(), Duration::from_secs(120));
    assert_eq!(default.request_timeout(), Duration::from_secs(120));
    assert_eq!(default.http_timeout(), Duration::from_secs(250));
    for (queue, execution) in [
        (0, 1),
        (1, 0),
        (600_001, 1),
        (1, 600_001),
        (u64::MAX, u64::MAX),
    ] {
        assert!(OperatorTaskRequestBudget::new(queue, execution).is_err());
    }
    assert_eq!(
        OperatorTaskRequestBudget::new(600_000, 600_000)
            .unwrap()
            .http_timeout(),
        Duration::from_secs(1210)
    );
}

#[test]
fn malformed_budget_is_rejected_before_contacting_service() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let mut executor =
        ServiceHeadlessExecutor::new(&format!("http://{}", listener.local_addr().unwrap()));
    let valid = OperatorTaskRequestBudget::default().to_value();
    let mut cases = vec![Value::Null, json!({}), json!({"schema_version":SCHEMA})];
    for (field, value) in [
        ("schema_version", json!("other/v1")),
        ("queue_timeout_ms", json!(0)),
        ("request_timeout_ms", json!(600_001)),
        ("request_timeout_ms", json!(-1)),
        ("request_timeout_ms", json!(true)),
        ("request_timeout_ms", json!(1.0)),
        ("request_timeout_ms", json!("120000")),
        ("unrecognized", json!(1)),
    ] {
        let mut invalid = valid.clone();
        invalid[field] = value;
        cases.push(invalid);
    }
    for budget in cases {
        let error = executor
            .execute_step(
                "operator_task_execute",
                1,
                &json!({"task":golden_task(), "execution_budget":budget}),
            )
            .unwrap_err();
        assert!(
            error
                .message
                .starts_with("invalid operator task execution_budget"),
            "{error:?}"
        );
    }
    assert_eq!(
        listener.accept().unwrap_err().kind(),
        std::io::ErrorKind::WouldBlock
    );
}

#[test]
fn request_override_is_forwarded_without_changing_task_identity_or_executor_default() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let configured = OperatorTaskRequestBudget::new(500, 2500).unwrap();
    let override_budget = OperatorTaskRequestBudget::new(1000, 3000).unwrap();
    let server = thread::spawn(move || {
        for expected in [override_budget, configured] {
            let (mut stream, _) = listener.accept().unwrap();
            let payload = read_request(&mut stream);
            assert_eq!(payload["execution_budget"], expected.to_value());
            assert_eq!(payload["task"], golden_task());
            send_response(&mut stream, &completed()).unwrap();
        }
    });
    let mut executor = ServiceHeadlessExecutor::new(&format!("http://{address}"))
        .with_operator_task_budget(configured);
    for payload in [
        json!({"task":golden_task(), "execution_budget":override_budget.to_value()}),
        json!({"task":golden_task()}),
    ] {
        assert_eq!(
            executor
                .execute_step("operator_task_execute", 1, &payload)
                .unwrap()
                .status,
            "executed"
        );
    }
    assert_eq!(executor.operator_task_budget(), configured);
    server.join().unwrap();
}

#[test]
fn silent_long_task_response_survives_the_old_thirty_second_idle_cutoff() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let payload = read_request(&mut stream);
        assert_eq!(
            payload["execution_budget"],
            OperatorTaskRequestBudget::default().to_value()
        );
        thread::sleep(Duration::from_secs(31));
        send_response(&mut stream, &completed())
    });
    let started = Instant::now();
    let result = ServiceHeadlessExecutor::new(&format!("http://{address}")).execute_step(
        "operator_task_execute",
        1,
        &json!({"task":golden_task()}),
    );
    server.join().unwrap().unwrap();
    assert!(started.elapsed() >= Duration::from_secs(31));
    assert_eq!(result.unwrap().status, "executed");
}

#[test]
fn stalled_task_timeout_stops_batch_without_replay_or_downstream_side_effects() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let (release, released) = mpsc::channel();
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let _ = read_request(&mut stream);
        released.recv_timeout(Duration::from_secs(20)).unwrap();
        drop(stream);
        listener.set_nonblocking(true).unwrap();
        assert_eq!(
            listener.accept().unwrap_err().kind(),
            std::io::ErrorKind::WouldBlock
        );
    });
    let mut executor = ServiceHeadlessExecutor::new(&format!("http://{address}"))
        .with_operator_task_budget(OperatorTaskRequestBudget::new(1, 1).unwrap());
    let workflow: crate::HeadlessExecutionBatch = serde_json::from_value(json!({
        "schema_version":"kyuubiki.headless-execution-batch/v1", "workflow_id":"budget-timeout",
        "exported_at":"1970-01-01T00:00:00.000Z", "language":"en", "warnings":[],
        "steps":[{"index":1,"action":"operator_task_execute","risk":"normal","payload":{"task":golden_task()}},
            {"index":2,"action":"project_create","risk":"normal","payload":{"name":"forbidden-after-timeout"}}]
    })).unwrap();
    let started = Instant::now();
    let report = crate::execute_batch_with_executor(&workflow, &mut executor, false, false);
    release.send(()).unwrap();
    server.join().unwrap();
    assert!(started.elapsed() >= Duration::from_secs(10));
    assert!(started.elapsed() < Duration::from_secs(15));
    assert_eq!(report.status, "failed");
    assert_eq!(report.steps.len(), 1);
    assert_eq!(report.executed_step_count, 0);
    let failure = report.execution_summary.failure.unwrap();
    assert_eq!(failure.category, "operator_task_outcome_unknown");
    assert!(!failure.retryable);
    assert_eq!(failure.retry_strategy, "none");
}

#[test]
fn slow_drip_does_not_renew_the_total_http_deadline() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let _ = read_request(&mut stream);
        for _ in 0..30 {
            if stream.write_all(b"x").is_err() {
                break;
            }
            thread::sleep(Duration::from_millis(20));
        }
        listener.set_nonblocking(true).unwrap();
        assert_eq!(
            listener.accept().unwrap_err().kind(),
            std::io::ErrorKind::WouldBlock
        );
    });
    let started = Instant::now();
    let error = request_json_with_timeout(
        &format!("http://{address}"),
        None,
        "POST",
        "/api/v1/operator-tasks/execute",
        Some(json!({})),
        Some(started + Duration::from_millis(150)),
        Duration::from_secs(1),
    )
    .unwrap_err();
    let elapsed = started.elapsed();
    server.join().unwrap();
    assert!(
        elapsed < Duration::from_millis(500),
        "{elapsed:?}: {error:?}"
    );
}

#[test]
fn task_results_larger_than_status_response_limit_keep_full_receipt_validation() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let _ = read_request(&mut stream);
        let mut receipt = completed();
        receipt["result"]["data"] = json!("x".repeat(8_000_001));
        send_response(&mut stream, &receipt).unwrap();
    });
    let result = ServiceHeadlessExecutor::new(&format!("http://{address}"))
        .execute_step("operator_task_execute", 1, &json!({"task":golden_task()}))
        .unwrap();
    server.join().unwrap();
    assert_eq!(
        result.result["result"]["data"].as_str().unwrap().len(),
        8_000_001
    );
}

#[test]
fn bounded_reader_rejects_response_growth_before_json_allocation() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let _ = stream.write_all(&vec![b'x'; 32_769]);
    });
    let mut stream = TcpStream::connect(address).unwrap();
    let error = crate::service_executor_deadline::read_limited_before_deadline(
        &mut stream,
        Instant::now() + Duration::from_secs(1),
        Duration::from_secs(1),
        32_768,
        "operator task",
    )
    .unwrap_err();
    assert!(
        error
            .message
            .contains("operator task response exceeds the 32768-byte transport limit")
    );
    server.join().unwrap();
}

#[test]
fn unknown_task_transport_outcome_takes_priority_over_retryable_message_fragments() {
    for detail in [
        "agent_queue_timeout",
        "waiting for agent capacity",
        "(404)",
        "unauthorized",
    ] {
        let preview = crate::execution_observability::failure_preview(
            1,
            "operator_task_execute",
            format!("{TRANSPORT_FAILURE_PREFIX}{detail}"),
        );
        assert_eq!(
            preview["failure_receipt"]["category"],
            "operator_task_outcome_unknown"
        );
        assert_eq!(preview["failure_receipt"]["retryable"], false);
    }
}
