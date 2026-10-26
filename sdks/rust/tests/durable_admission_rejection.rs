use kyuubiki_headless_sdk::{
    DurableExecutionHandle, DurableExecutionIntent, FailureClass, KyuubikiAgentClient, SdkError,
    SdkResult, SolverRpcClient, SolverRpcLimits,
};
use serde_json::{Value, json};
use std::{
    io::{Read, Write},
    net::TcpListener,
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

fn intent() -> DurableExecutionIntent {
    DurableExecutionIntent::new(&"b".repeat(32), &"a".repeat(32), "solve_bar_1d", &json!({}))
        .unwrap()
}

fn rejection(request: &Value) -> Value {
    let intent = intent();
    json!({
        "schema_version":"kyuubiki.agent-durable-admission-rejection/v1",
        "store_id":intent.store_id, "execution_id":intent.execution_id,
        "request_digest":intent.request_digest, "method":intent.method,
        "request_id":request["id"], "process_instance_id":"process-a",
        "status":"not_admitted", "reason":"store_capacity",
        "message":"execution store admission blocked by capacity",
        "execution_started":false, "automatic_replay_authorized":false
    })
}

fn attempt(
    prepared: bool,
    change: impl FnOnce(&mut Value) + Send + 'static,
) -> (SdkResult<Value>, Value) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    listener.set_nonblocking(true).unwrap();
    let (done, finished) = mpsc::channel();
    let worker = thread::spawn(move || {
        let deadline = Instant::now() + Duration::from_secs(3);
        let mut stream = loop {
            match listener.accept() {
                Ok((stream, _)) => break stream,
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    assert!(Instant::now() < deadline, "expected one submission");
                    thread::sleep(Duration::from_millis(1));
                }
                Err(error) => panic!("test accept failed: {error}"),
            }
        };
        stream.set_nonblocking(false).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(3)))
            .unwrap();
        stream
            .set_write_timeout(Some(Duration::from_secs(3)))
            .unwrap();
        let mut header = [0; 4];
        stream.read_exact(&mut header).unwrap();
        let length = u32::from_be_bytes(header) as usize;
        assert!(length < 65536);
        let mut bytes = vec![0; length];
        stream.read_exact(&mut bytes).unwrap();
        let request: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(request["method"], "submit_durable_agent_task");
        let mut reply = json!({"rpc_version":1, "id":request["id"], "ok":true,
            "result":rejection(&request)});
        change(&mut reply);
        let bytes = serde_json::to_vec(&reply).unwrap();
        stream
            .write_all(&(bytes.len() as u32).to_be_bytes())
            .unwrap();
        stream.write_all(&bytes).unwrap();
        drop(stream);
        finished.recv_timeout(Duration::from_secs(3)).unwrap();
        assert!(
            matches!(listener.accept(), Err(e) if e.kind() == std::io::ErrorKind::WouldBlock),
            "SDK must not reconnect or resubmit on rejection"
        );
        reply
    });
    let client = SolverRpcClient::new("127.0.0.1", port)
        .with_limits(SolverRpcLimits {
            io_timeout: Duration::from_secs(1),
            call_timeout: Duration::from_secs(2),
            ..Default::default()
        })
        .unwrap();
    let intent = intent();
    let result = if prepared {
        client.submit_prepared_durable_agent_task(&intent, json!({}))
    } else {
        client.submit_durable_agent_task(
            &intent.store_id,
            &intent.execution_id,
            &intent.method,
            json!({}),
        )
    };
    done.send(()).unwrap();
    (result, worker.join().unwrap())
}

#[test]
fn explicit_rejection_preserves_attempt_evidence_in_both_submission_apis() {
    for prepared in [false, true] {
        let (result, wire) = attempt(prepared, |_| {});
        let error = result.unwrap_err();
        assert_eq!(
            KyuubikiAgentClient::classify_error(&error),
            FailureClass::Rpc
        );
        assert!(error.to_string().contains("capacity"));
        let SdkError::DurableAdmissionRejected { report } = error else {
            panic!("lost validated rejection: {error}");
        };
        assert_eq!(report, wire["result"]);
        assert_eq!(report["request_id"], wire["id"]);
        assert!(DurableExecutionHandle::from_report(&report).is_err());
        assert!(intent().handle_from_report(&report).is_err());
    }
}

#[test]
fn foreign_incomplete_or_contradictory_reports_are_not_known_rejections() {
    for (key, value) in [
        ("schema_version", json!("unknown")),
        ("store_id", json!("c".repeat(32))),
        ("execution_id", json!("c".repeat(32))),
        ("request_digest", json!("c".repeat(64))),
        ("method", json!("solve_modal_frame_2d")),
        ("request_id", json!("old-submission")),
        ("process_instance_id", json!(" \n")),
        ("process_instance_id", json!("x".repeat(257))),
        ("status", json!("completed")),
        ("reason", json!("new-unknown-reason")),
        ("message", json!("")),
        ("message", json!("text\ncontrol")),
        ("message", json!("x".repeat(1025))),
        ("execution_started", json!(true)),
        ("automatic_replay_authorized", json!(true)),
        ("resumable", json!(true)),
        ("response", json!({"ok":true,"result":42})),
        ("result", json!(42)),
        ("error", json!({"code":"failure"})),
        ("progress", json!({})),
        ("event", json!("progress")),
    ] {
        let (result, _) = attempt(true, move |reply| reply["result"][key] = value);
        let error = result.unwrap_err();
        assert!(matches!(error, SdkError::Transport(_)), "{key}: {error}");
    }
    for key in rejection(&json!({"id":"request"}))
        .as_object()
        .unwrap()
        .keys()
    {
        let key = key.clone();
        for remove in [false, true] {
            let key = key.clone();
            let (result, _) = attempt(true, move |reply| {
                if remove {
                    reply["result"].as_object_mut().unwrap().remove(&key);
                } else {
                    reply["result"][key] = Value::Null;
                }
            });
            assert!(matches!(result, Err(SdkError::Transport(_))));
        }
    }
}

#[test]
fn outer_rpc_failure_or_foreign_envelope_cannot_assert_known_non_admission() {
    let (result, _) = attempt(true, |reply| reply["id"] = json!("foreign"));
    assert!(matches!(result, Err(SdkError::Transport(_))));
    let (result, _) = attempt(true, |reply| {
        let claimed = reply["result"].take();
        reply["ok"] = json!(false);
        reply["error"] = json!({"code":"store_capacity", "message":"claimed rejection",
            "details":claimed});
    });
    assert!(matches!(result, Err(SdkError::Rpc { .. })));
}

#[test]
fn existing_execution_receipt_keeps_its_original_admission_request_id() {
    let (result, wire) = attempt(true, |reply| {
        let report = &mut reply["result"];
        for key in ["reason", "message", "execution_started"] {
            report.as_object_mut().unwrap().remove(key);
        }
        report["schema_version"] = json!("kyuubiki.agent-durable-execution/v1");
        report["request_id"] = json!("original-admission");
        report["status"] = json!("running");
        report["resumable"] = json!(false);
        report["response"] = Value::Null;
    });
    let report = result.unwrap();
    assert_eq!(report, wire["result"]);
    assert_ne!(report["request_id"], wire["id"]);
    intent().handle_from_report(&report).unwrap();
}
