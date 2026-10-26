use kyuubiki_headless_sdk::{DurableExecutionIndexPage, SolverRpcClient, SolverRpcLimits};
use serde_json::{Value, json};
use std::{
    io::{Read, Write},
    net::TcpListener,
    thread,
    time::{Duration, Instant},
};

fn id(value: u8) -> String {
    format!("{value:032x}")
}

fn entry(value: u8) -> Value {
    json!({"schema_version":"kyuubiki.agent-durable-execution/v1",
        "execution_id":id(value), "request_digest":"c".repeat(64),
        "request_id":"original-request", "process_instance_id":"original-process",
        "method":"solve_modal_frame_2d"})
}

fn report(ids: &[u8], next: Option<u8>) -> Value {
    json!({"schema_version":"kyuubiki.agent-durable-execution-index/v1",
        "store_id":"b".repeat(32), "automatic_replay_authorized":false,
        "executions":ids.iter().copied().map(entry).collect::<Vec<_>>(),
        "next_cursor":next.map(id)})
}

fn parse(report: &Value, after: Option<&str>, limit: usize) -> bool {
    DurableExecutionIndexPage::from_report(report, &"b".repeat(32), after, limit).is_ok()
}

#[test]
fn durable_index_accepts_bounded_pages_without_inventing_terminal_state() {
    assert!(parse(&report(&[], None), None, 1));
    assert!(parse(&report(&[1], None), None, 1));
    assert!(parse(&report(&[1, 2], Some(2)), None, 2));
    assert!(parse(&report(&[1, 3], Some(3)), None, 2));
    assert!(parse(&report(&[3], None), Some(&id(2)), 2));
    assert!(parse(
        &report(&(1..=64).collect::<Vec<_>>(), None),
        None,
        64
    ));
    let page =
        DurableExecutionIndexPage::from_report(&report(&[1], None), &"b".repeat(32), None, 1)
            .unwrap();
    assert_eq!(page.executions[0].execution_id, id(1));
    assert_eq!(page.executions[0].request_digest, "c".repeat(64));
    let retained = serde_json::to_value(&page.executions[0]).unwrap();
    assert!(kyuubiki_headless_sdk::DurableExecutionHandle::from_report(&retained).is_err());
    assert!(retained.get("status").is_none());
    assert!(retained.get("response").is_none());
}

#[test]
fn durable_index_rejects_cursor_cycles_duplicates_and_invalid_continuations() {
    for bad in [
        report(&[], Some(1)),
        report(&[1], Some(1)),
        report(&[1, 1], None),
        report(&[2, 1], None),
        report(&[1, 2, 3], None),
        report(&[1, 2], Some(1)),
        report(&[1, 2], Some(3)),
    ] {
        assert!(!parse(&bad, None, 2), "{bad}");
    }
    for bad in [
        report(&[1, 2], None),
        report(&[2], None),
        report(&[1], None),
    ] {
        assert!(!parse(&bad, Some(&id(2)), 2), "{bad}");
    }
    let mut bad = report(&[1], None);
    bad.as_object_mut().unwrap().remove("next_cursor");
    assert!(!parse(&bad, None, 1));
    for cursor in [json!(false), json!(1), json!(""), json!("A".repeat(32))] {
        let mut bad = report(&[1], None);
        bad["next_cursor"] = cursor;
        assert!(!parse(&bad, None, 1));
    }
}

#[test]
fn durable_index_rejects_foreign_ambiguous_or_malformed_admission_metadata() {
    for (field, value) in [
        ("schema_version", json!("unknown")),
        ("store_id", json!("c".repeat(32))),
        ("automatic_replay_authorized", json!(true)),
        ("automatic_replay_authorized", Value::Null),
        ("executions", Value::Null),
        ("status", json!("completed")),
        ("response", json!({"result":42})),
    ] {
        let mut bad = report(&[1], None);
        bad[field] = value;
        assert!(!parse(&bad, None, 1), "{field}");
    }
    for (field, value) in [
        ("schema_version", json!("unknown")),
        ("execution_id", json!("a")),
        ("execution_id", json!("A".repeat(32))),
        ("request_digest", json!("c".repeat(63))),
        ("request_digest", json!("C".repeat(64))),
        ("request_id", json!(" ")),
        ("request_id", json!("x".repeat(257))),
        ("process_instance_id", json!("bad\nidentity")),
        ("method", Value::Null),
        ("method", json!("")),
        ("status", json!("completed")),
        ("response", json!({"result":42})),
        ("store_id", json!("b".repeat(32))),
    ] {
        let mut bad = report(&[1], None);
        bad["executions"][0][field] = value;
        assert!(!parse(&bad, None, 1), "{field}");
    }
    for field in [
        "schema_version",
        "execution_id",
        "request_digest",
        "request_id",
        "process_instance_id",
        "method",
    ] {
        let mut bad = report(&[1], None);
        bad["executions"][0].as_object_mut().unwrap().remove(field);
        assert!(!parse(&bad, None, 1), "missing {field}");
    }
}

#[test]
fn invalid_index_query_is_rejected_before_any_connection() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let client = SolverRpcClient::new("127.0.0.1", listener.local_addr().unwrap().port());
    for (store, after, limit) in [
        ("b".repeat(31), None, 1),
        ("B".repeat(32), None, 1),
        ("b".repeat(32), Some("bad"), 1),
        ("b".repeat(32), None, 0),
        ("b".repeat(32), None, 65),
        ("b".repeat(32), None, usize::MAX),
    ] {
        assert!(
            client
                .list_durable_agent_executions(&store, after, limit)
                .is_err()
        );
        assert_eq!(
            listener.accept().unwrap_err().kind(),
            std::io::ErrorKind::WouldBlock
        );
    }
}

#[test]
fn durable_index_rpc_is_one_pinned_read_without_cursor_following_or_replay() {
    for valid in [true, false] {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let server = listener.try_clone().unwrap();
        let worker = thread::spawn(move || {
            let deadline = Instant::now() + Duration::from_secs(3);
            let mut stream = loop {
                match server.accept() {
                    Ok((stream, _)) => break stream,
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        assert!(Instant::now() < deadline, "expected one index read");
                        thread::sleep(Duration::from_millis(1));
                    }
                    Err(e) => panic!("fixture accept: {e}"),
                }
            };
            stream.set_nonblocking(false).unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(2)))
                .unwrap();
            stream
                .set_write_timeout(Some(Duration::from_secs(2)))
                .unwrap();
            let mut header = [0; 4];
            stream.read_exact(&mut header).unwrap();
            let size = u32::from_be_bytes(header) as usize;
            assert!(size < 65536);
            let mut bytes = vec![0; size];
            stream.read_exact(&mut bytes).unwrap();
            let request: Value = serde_json::from_slice(&bytes).unwrap();
            assert_eq!(request["method"], "list_durable_agent_executions");
            assert_eq!(
                request["params"],
                json!({"expected_store_id":"b".repeat(32), "after":id(1), "limit":1})
            );
            let mut result = report(&[2], Some(2));
            if !valid {
                result["store_id"] = json!("c".repeat(32));
            }
            let bytes = serde_json::to_vec(
                &json!({"rpc_version":1, "id":request["id"], "ok":true, "result":result}),
            )
            .unwrap();
            stream
                .write_all(&(bytes.len() as u32).to_be_bytes())
                .unwrap();
            stream.write_all(&bytes).unwrap();
        });
        let client = SolverRpcClient::new("127.0.0.1", listener.local_addr().unwrap().port())
            .with_limits(SolverRpcLimits {
                io_timeout: Duration::from_secs(1),
                call_timeout: Duration::from_secs(1),
                ..Default::default()
            })
            .unwrap();
        let page = client.list_durable_agent_executions(&"b".repeat(32), Some(&id(1)), 1);
        worker.join().unwrap();
        assert_eq!(page.is_ok(), valid);
        if let Ok(page) = page {
            assert_eq!(page.next_cursor, Some(id(2)));
        }
        assert_eq!(
            listener.accept().unwrap_err().kind(),
            std::io::ErrorKind::WouldBlock
        );
    }
}
