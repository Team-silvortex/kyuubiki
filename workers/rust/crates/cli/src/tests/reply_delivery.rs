use super::*;
use crate::agent_reply_writer::{ReplyWriter, SharedReplyWriter};
use crate::{agent_watchdog, rpc::handle_request};
use kyuubiki_protocol::{RPC_VERSION, RpcMethod, RpcRequest};
use serde_json::json;
use std::net::{Shutdown, TcpListener};

fn pair() -> (TcpStream, SharedReplyWriter) {
    let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
    let client = TcpStream::connect(listener.local_addr().unwrap()).unwrap();
    let server = listener.accept().unwrap().0;
    (
        client,
        Arc::new(ReplyWriter::new(server, Duration::from_secs(2))),
    )
}

fn solve(id: &str, writer: SharedReplyWriter) -> AgentReply {
    handle_request(
        RpcRequest {
            rpc_version: RPC_VERSION,
            id: id.into(),
            method: RpcMethod::SolveBar1d,
            params: json!({"length":1.0,"area":2.0,"youngs_modulus":1000.0,
                "elements":2,"tip_force":20.0}),
        },
        Some(writer),
    )
}

fn active(id: &str) -> bool {
    agent_watchdog::snapshot()
        .active_executions
        .iter()
        .any(|entry| entry.request_id == id)
}

#[test]
fn solver_remains_active_until_final_reply_is_written() {
    let id = "reply-delivery-pending";
    let (_client, writer) = pair();
    let reply = solve(id, writer.clone());
    assert!(active(id), "result exists, but has not been delivered yet");
    write_agent_reply(&writer, reply).unwrap();
    assert!(!active(id));
}

#[test]
fn failed_reply_delivery_is_not_recorded_as_a_completed_execution() {
    let id = "reply-delivery-failed";
    let (_client, writer) = pair();
    let reply = solve(id, writer.clone());
    writer
        .stream
        .lock()
        .unwrap()
        .shutdown(Shutdown::Both)
        .unwrap();
    assert!(write_agent_reply(&writer, reply).is_err());
    let snapshot = agent_watchdog::snapshot();
    let failure = snapshot
        .recent_failures
        .iter()
        .find(|entry| entry.request_id == id)
        .expect("transport failure must retain the execution identity");
    assert_eq!(failure.reason_code, "result_delivery_failed");
    assert_eq!(failure.method, "solve_bar_1d");
    assert!(!active(id));
}

#[test]
fn abandoned_response_releases_its_lease_and_retains_its_request_identity() {
    let id = "reply-delivery-abandoned";
    let (_client, writer) = pair();
    let reply = solve(id, writer.clone());
    assert!(active(id));
    drop(reply);
    drop(writer);
    assert!(!active(id));
    assert!(
        agent_watchdog::snapshot()
            .recent_failures
            .iter()
            .any(|failure| {
                failure.request_id == id && failure.reason_code == "result_delivery_aborted"
            })
    );
}

#[test]
fn operator_task_ir_also_stays_active_until_its_reply_is_written() {
    let id = "reply-delivery-task-ir";
    let (_client, writer) = pair();
    let task: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../../schemas/examples.operator-task-ir.json"
    ))
    .unwrap();
    let reply = handle_request(
        RpcRequest {
            rpc_version: RPC_VERSION,
            id: id.into(),
            method: RpcMethod::RunOperatorTaskIr,
            params: json!({"task_ir":task}),
        },
        Some(writer.clone()),
    );
    let AgentReply::Stream(_, response) = &reply;
    assert!(response.ok, "{response:?}");
    assert!(active(id));
    write_agent_reply(&writer, reply).unwrap();
    assert!(!active(id));
}

#[test]
fn failed_execution_keeps_its_original_cause_when_reply_delivery_also_fails() {
    let id = "reply-delivery-original-cause";
    let (_client, writer) = pair();
    let reply = handle_request(
        RpcRequest {
            rpc_version: RPC_VERSION,
            id: id.into(),
            method: RpcMethod::SolveBar1d,
            params: json!({"invalid":true}),
        },
        Some(writer.clone()),
    );
    writer
        .stream
        .lock()
        .unwrap()
        .shutdown(Shutdown::Both)
        .unwrap();
    assert!(write_agent_reply(&writer, reply).is_err());
    let snapshot = agent_watchdog::snapshot();
    let failures: Vec<_> = snapshot
        .recent_failures
        .iter()
        .filter(|failure| failure.request_id == id)
        .collect();
    assert_eq!(failures.len(), 1);
    assert_eq!(failures[0].reason_code, "invalid_params");
}

#[test]
fn expired_delivery_deadline_writes_no_bytes() {
    let (mut client, writer) = pair();
    let mut stream = writer.stream.lock().unwrap();
    let error = write_until(&mut stream, b"no partial frame", Instant::now()).unwrap_err();
    assert_eq!(error.kind(), std::io::ErrorKind::TimedOut);
    stream.shutdown(Shutdown::Write).unwrap();
    let mut received = vec![];
    client.read_to_end(&mut received).unwrap();
    assert!(received.is_empty());
}

#[test]
fn failed_frame_closes_transport_before_any_later_response() {
    let (mut client, writer) = pair();
    let heartbeat = json!({"type":"heartbeat"});
    assert!(write_json_frame(&writer, &heartbeat, Instant::now()).is_err());
    let later = json!({"ok":true,"result":"must not follow a failed frame"});
    assert!(
        write_json_frame(&writer, &later, Instant::now() + Duration::from_secs(2)).is_err(),
        "a failed heartbeat frame must make its connection unusable"
    );
    client
        .set_read_timeout(Some(Duration::from_secs(2)))
        .unwrap();
    let mut received = vec![];
    client.read_to_end(&mut received).unwrap();
    assert!(received.is_empty());
}

#[test]
fn heartbeat_shutdown_wakes_a_parked_thread_instead_of_waiting_for_its_interval() {
    let running = Arc::new(AtomicBool::new(true));
    let thread_running = running.clone();
    let (ready_tx, ready_rx) = std::sync::mpsc::channel();
    let join_handle = thread::spawn(move || {
        ready_tx.send(()).unwrap();
        thread::park_timeout(Duration::from_secs(60));
        assert!(!thread_running.load(Ordering::SeqCst));
    });
    ready_rx.recv_timeout(Duration::from_secs(5)).unwrap();
    let handle = HeartbeatHandle {
        running,
        join_handle: Some(join_handle),
    };
    let (stopped_tx, stopped_rx) = std::sync::mpsc::channel();
    let stopper = thread::spawn(move || {
        handle.stop();
        stopped_tx.send(()).unwrap();
    });
    stopped_rx
        .recv_timeout(Duration::from_secs(5))
        .expect("heartbeat stop must unpark");
    stopper.join().unwrap();
}
