use kyuubiki_headless_sdk::{DurableExecutionHandle, DurableExecutionIntent, SolverRpcClient};
use serde_json::{Value, json};
use std::{
    io::{Read, Write},
    net::TcpListener,
    thread,
    time::{Duration, Instant},
};

fn intent() -> DurableExecutionIntent {
    DurableExecutionIntent::new(&"b".repeat(32), &"a".repeat(32), "solve_bar_1d", &json!({}))
        .unwrap()
}

fn missing() -> Value {
    json!({"schema_version":"kyuubiki.agent-durable-execution/v1",
        "store_id":"b".repeat(32), "execution_id":"a".repeat(32),
        "status":"not_found", "automatic_replay_authorized":false})
}

fn peer(report: Value) -> (SolverRpcClient, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    listener.set_nonblocking(true).unwrap();
    let worker = thread::spawn(move || {
        let deadline = Instant::now() + Duration::from_secs(3);
        let mut stream = loop {
            match listener.accept() {
                Ok((stream, _)) => break stream,
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    assert!(
                        Instant::now() < deadline,
                        "expected original-task inspection"
                    );
                    thread::sleep(Duration::from_millis(1));
                }
                Err(e) => panic!("test peer: {e}"),
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
        let size = u32::from_be_bytes(header) as usize;
        assert!(size < 65536);
        let mut bytes = vec![0; size];
        stream.read_exact(&mut bytes).unwrap();
        let request: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(request["method"], "fetch_durable_agent_execution");
        assert_eq!(
            request["params"],
            json!({
                "expected_store_id":"b".repeat(32), "execution_id":"a".repeat(32)
            })
        );
        let reply = json!({"rpc_version":1,"id":request["id"],"ok":true,"result":report});
        let bytes = serde_json::to_vec(&reply).unwrap();
        stream
            .write_all(&(bytes.len() as u32).to_be_bytes())
            .unwrap();
        stream.write_all(&bytes).unwrap();
    });
    (SolverRpcClient::new("127.0.0.1", port), worker)
}

#[test]
fn missing_observation_cannot_claim_execution_or_resume_authority() {
    for (field, value) in [
        ("resumable", json!(true)),
        ("execution_started", json!(true)),
        ("request_digest", json!(intent().request_digest)),
        ("request_id", json!("original-admission")),
        ("process_instance_id", json!("original-process")),
        ("method", json!("solve_bar_1d")),
        ("response", json!({"ok":true,"result":{}})),
        ("result", json!({"value":42})),
        ("error", json!({"code":"failed"})),
        ("event", json!("progress")),
        ("progress", json!({"progress":0.5})),
    ] {
        for prepared in [false, true] {
            let mut report = missing();
            report[field] = value.clone();
            let (client, worker) = peer(report);
            let outcome = if prepared {
                client.inspect_durable_agent_task(&intent())
            } else {
                client.inspect_durable_agent_execution(&intent().store_id, &intent().execution_id)
            };
            worker.join().unwrap();
            assert!(outcome.is_err(), "{field} was accepted as a missing task");
        }
    }
}

#[test]
fn missing_observation_accepts_neutral_fields_but_never_creates_a_handle() {
    for neutral in [Value::Null, json!(false)] {
        let mut report = missing();
        for field in [
            "response",
            "result",
            "error",
            "event",
            "progress",
            "request_digest",
            "request_id",
            "process_instance_id",
            "method",
        ] {
            report[field] = Value::Null;
        }
        report["resumable"] = neutral.clone();
        report["execution_started"] = neutral;
        report["observed_at"] = json!("future diagnostic metadata");
        let (client, worker) = peer(report.clone());
        let observed = client.inspect_durable_agent_task(&intent());
        worker.join().unwrap();
        assert_eq!(observed.unwrap(), report);
        assert!(DurableExecutionHandle::from_report(&report).is_err());
        assert!(intent().handle_from_report(&report).is_err());
    }
    let (client, worker) = peer(missing());
    let observed = client.inspect_durable_agent_task(&intent());
    worker.join().unwrap();
    assert_eq!(observed.unwrap(), missing());
}

#[test]
fn missing_observation_requires_lookup_binding_and_well_typed_policy() {
    for (field, value) in [
        ("schema_version", json!("unknown")),
        ("store_id", json!("c".repeat(32))),
        ("execution_id", json!("c".repeat(32))),
        ("automatic_replay_authorized", Value::Null),
        ("automatic_replay_authorized", json!(true)),
        ("automatic_replay_authorized", json!("false")),
        ("resumable", json!("false")),
        ("resumable", json!(0)),
        ("resumable", json!([])),
        ("execution_started", json!("false")),
        ("execution_started", json!(0)),
        ("execution_started", json!({})),
        ("response", json!(false)),
        ("request_digest", json!(false)),
        ("request_id", json!("")),
    ] {
        let mut report = missing();
        report[field] = value;
        let (client, worker) = peer(report);
        let observed = client.inspect_durable_agent_task(&intent());
        worker.join().unwrap();
        assert!(observed.is_err(), "invalid {field} accepted");
    }
    let mut report = missing();
    report
        .as_object_mut()
        .unwrap()
        .remove("automatic_replay_authorized");
    let (client, worker) = peer(report);
    let observed = client.inspect_durable_agent_task(&intent());
    worker.join().unwrap();
    assert!(observed.is_err());
}
