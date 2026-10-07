use super::*;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::thread;

fn query() -> Value {
    json!({"task_id":"task", "task_digest":"a".repeat(64), "attempt_id":"b".repeat(32),
        "execution_target":{"process_instance_id":"original-process", "request_id":"original-rpc", "generation":3, "job_id":"task"}})
}

fn receipt(query: &Value, status: &str) -> Value {
    let delivered = matches!(
        status,
        "requested" | "target_not_observed" | "cancellation_outcome_unknown"
    );
    let registered = if status == "cancellation_outcome_unknown" {
        Value::Null
    } else {
        json!(status == "requested")
    };
    let ack = if matches!(status, "requested" | "target_not_observed") {
        json!({"schema_version":kyuubiki_protocol::AGENT_EXECUTION_CANCELLATION_SCHEMA,
            "execution_target":query["execution_target"], "status":status, "cancel_registered":registered,
            "execution_terminal_confirmed":false, "pending_cancellation_created":false,
            "operator_package_cleanup_performed":false, "automatic_replay_authorized":false})
    } else {
        Value::Null
    };
    let mut result = query.clone();
    for (key, value) in json!({"schema_version":SCHEMA, "status":status, "cancel_registered":registered,
        "delivery_attempted":delivered, "agent_acknowledgement":ack,
        "uncertainty_reason":if status == "cancellation_outcome_unknown" {json!("original_endpoint_unreachable")} else {Value::Null},
        "execution_terminal_confirmed":false, "automatic_replay_authorized":false,
        "journal_mutation_performed":false, "publication_performed":false, "job_wide_fallback_performed":false})
        .as_object().unwrap() { result[key] = value.clone(); }
    result
}

#[test]
fn cancellation_statuses_separate_registration_absence_and_unknown_without_terminal_authority() {
    let query = query();
    for status in [
        "requested",
        "target_not_observed",
        "cancellation_outcome_unknown",
        "no_retained_dispatch",
        "not_dispatched",
        "retained_terminal_dispatch",
        "original_endpoint_not_configured",
    ] {
        validate(&receipt(&query, status), &query).unwrap();
    }
    let mut unknown = receipt(&query, "cancellation_outcome_unknown");
    unknown["uncertainty_reason"] = json!("agent_acknowledgement_invalid");
    validate(&unknown, &query).unwrap();
}

#[test]
fn cancellation_receipt_rejects_identity_drift_missing_fields_and_broadened_authority() {
    let query = query();
    let original = receipt(&query, "requested");
    let mut mutations = Vec::new();
    for field in [
        "task_id",
        "task_digest",
        "attempt_id",
        "execution_target",
        "schema_version",
    ] {
        let mut value = original.clone();
        value[field] = json!("foreign");
        mutations.push(value);
    }
    for field in [
        "execution_terminal_confirmed",
        "automatic_replay_authorized",
        "publication_performed",
        "journal_mutation_performed",
        "job_wide_fallback_performed",
    ] {
        let mut value = original.clone();
        value[field] = json!(true);
        mutations.push(value);
    }
    for field in FIELDS {
        let mut value = original.clone();
        value.as_object_mut().unwrap().remove(*field);
        mutations.push(value);
    }
    let mut extra = original.clone();
    extra["extra"] = json!(true);
    mutations.push(extra);
    let mut status = original.clone();
    status["status"] = json!("cancelled");
    mutations.push(status);
    for value in mutations {
        assert!(validate(&value, &query).is_err(), "accepted {value}");
    }
}

#[test]
fn cancellation_receipt_rejects_foreign_or_contradictory_agent_acknowledgements() {
    let query = query();
    let original = receipt(&query, "requested");
    for (field, wrong) in [
        ("schema_version", json!("foreign")),
        ("execution_target", Value::Null),
        ("status", json!("target_not_observed")),
        ("cancel_registered", json!(false)),
        ("execution_terminal_confirmed", json!(true)),
        ("pending_cancellation_created", json!(true)),
        ("operator_package_cleanup_performed", json!(true)),
        ("automatic_replay_authorized", json!(true)),
    ] {
        let mut value = original.clone();
        value["agent_acknowledgement"][field] = wrong;
        assert!(validate(&value, &query).is_err());
    }
    for status in ["no_retained_dispatch", "cancellation_outcome_unknown"] {
        let baseline = receipt(&query, status);
        for (field, wrong) in [
            ("cancel_registered", json!(true)),
            ("agent_acknowledgement", json!({})),
            (
                "delivery_attempted",
                json!(status != "cancellation_outcome_unknown"),
            ),
        ] {
            let mut value = baseline.clone();
            value[field] = wrong;
            assert!(validate(&value, &query).is_err());
        }
    }
}

#[test]
fn malformed_cancellation_queries_fail_before_any_connection_attempt() {
    let query = query();
    let target: CancelExecutionRequest =
        serde_json::from_value(query["execution_target"].clone()).unwrap();
    let executor = ServiceHeadlessExecutor::try_new("http://127.0.0.1:1").unwrap();
    let mut variants = vec![];
    let mut value = target.clone();
    value.generation = 0;
    variants.push(value);
    for invalid in ["", " ", "x\ny", &"x".repeat(257), "unavailable"] {
        let mut value = target.clone();
        value.process_instance_id = invalid.into();
        variants.push(value);
    }
    let mut value = target.clone();
    value.job_id = "another-job".into();
    variants.push(value);
    for value in variants {
        let error = executor
            .cancel_operator_task_dispatch("task", &"a".repeat(64), &"b".repeat(32), &value)
            .unwrap_err();
        assert!(
            error
                .message
                .starts_with("invalid operator task dispatch cancellation")
        );
    }
    for (digest, attempt) in [
        ("A".repeat(64), "b".repeat(32)),
        ("a".repeat(64), "b".repeat(31)),
    ] {
        assert!(
            executor
                .cancel_operator_task_dispatch("task", &digest, &attempt, &target)
                .unwrap_err()
                .message
                .starts_with("invalid operator task dispatch cancellation")
        );
    }
}

fn server(bad_body: bool) -> (String, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    listener.set_nonblocking(true).unwrap();
    let worker = thread::spawn(move || {
        let deadline = Instant::now() + Duration::from_secs(5);
        let mut stream = loop {
            match listener.accept() {
                Ok((stream, _)) => break stream,
                Err(error)
                    if error.kind() == std::io::ErrorKind::WouldBlock
                        && Instant::now() < deadline =>
                {
                    thread::sleep(Duration::from_millis(5))
                }
                Err(error) => panic!("cancel test listener: {error}"),
            }
        };
        stream
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        stream
            .set_write_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        let mut bytes = Vec::new();
        while !bytes.ends_with(b"\r\n\r\n") {
            let mut byte = [0];
            stream.read_exact(&mut byte).unwrap();
            bytes.push(byte[0]);
            assert!(bytes.len() < 8192);
        }
        let head = String::from_utf8(bytes).unwrap();
        assert!(head.starts_with("POST /api/v1/operator-tasks/cancel-dispatch HTTP/1.1"));
        assert!(head.contains("Authorization: Bearer test-token"));
        let length: usize = head
            .lines()
            .find_map(|line| line.strip_prefix("Content-Length: "))
            .unwrap()
            .parse()
            .unwrap();
        assert!(length < 8192);
        let mut body = vec![0; length];
        stream.read_exact(&mut body).unwrap();
        let payload: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(payload, query());
        let body = if bad_body {
            "not json".into()
        } else {
            receipt(&payload, "requested").to_string()
        };
        write!(
            stream,
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        )
        .unwrap();
        stream.flush().unwrap();
        assert!(
            matches!(listener.accept(), Err(error) if error.kind() == std::io::ErrorKind::WouldBlock)
        );
    });
    (url, worker)
}

#[test]
fn public_executor_sends_exact_query_and_validates_registration_without_replay() {
    let (url, server) = server(false);
    let executor = ServiceHeadlessExecutor::try_with_token(&url, Some("test-token")).unwrap();
    let query = query();
    let target = serde_json::from_value(query["execution_target"].clone()).unwrap();
    let result = executor
        .cancel_operator_task_dispatch("task", &"a".repeat(64), &"b".repeat(32), &target)
        .unwrap();
    server.join().unwrap();
    assert_eq!(result, receipt(&query, "requested"));
    assert!(!format!("{executor:?}").contains("test-token"));
}

#[test]
fn malformed_http_reply_keeps_registration_unconfirmed_and_does_not_retry() {
    let (url, server) = server(true);
    let executor = ServiceHeadlessExecutor::try_with_token(&url, Some("test-token")).unwrap();
    let target = serde_json::from_value(query()["execution_target"].clone()).unwrap();
    let error = executor
        .cancel_operator_task_dispatch("task", &"a".repeat(64), &"b".repeat(32), &target)
        .unwrap_err();
    server.join().unwrap();
    assert!(
        error
            .message
            .contains("registration is unconfirmed and automatic retry is not authorized")
    );
}
