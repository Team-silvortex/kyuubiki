use super::*;
use crate::executor_outcome_tests::golden_task;
use crate::{HeadlessExecutor, ServiceHeadlessExecutor, prepare_operator_task_payload};
use serde_json::json;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::time::Duration;

fn prepared() -> Value {
    prepare_operator_task_payload(&json!({"task":golden_task()})).unwrap()
}

fn completed() -> Value {
    let mut receipt = prepared();
    receipt["status"] = json!("executed");
    receipt["result"] = json!({"value":1.0/3.0,"negative_zero":-0.0,"items":[1,2,3]});
    receipt
}

fn blocked() -> Value {
    let mut receipt = prepared();
    receipt["status"] = json!("verified_pending_execution");
    receipt["execution_readiness"] = json!({"status":"blocked", "ready_to_dispatch":false,
        "required_action":"attach_operator_package_runtime", "current_stage":"fetch_package"});
    receipt["package_fetch_request"] = json!({"request_status":"blocked_runtime_not_attached"});
    receipt
}

fn failed() -> Value {
    let mut receipt = prepared();
    receipt["status"] = json!("failed");
    receipt["error_code"] = json!("operator_task_solver_execution_failed");
    receipt["execution_readiness"] = json!({"status":"blocked","ready_to_dispatch":false,
        "current_stage":"dispatch_engine_solver","blocking_stage":"dispatch_engine_solver",
        "blocking_reason":"operator_task_solver_execution_failed","blocking_owner":"agent_runtime",
        "required_action":"inspect_engine_solver_failure"});
    receipt["failure_receipt"] = json!({
        "schema_version":"kyuubiki.agent-operator-task-failure/v1","failure_owner":"agent_runtime",
        "failure_stage":"dispatch_engine_solver","reason_code":"operator_task_solver_execution_failed",
        "message":"invalid model","task_id":receipt["task_id"],"task_digest":receipt["task_digest"],
        "operator_id":receipt["operator_id"],"recovery":{"retryable":false,
            "required_action":"inspect_engine_solver_failure","safe_to_continue_other_tasks":true}
    });
    receipt
}

#[test]
fn http_native_failure_retains_engine_recovery_and_task_lineage() {
    let receipt = failed();
    let outcome = round_trip(&receipt).unwrap();
    assert_eq!(outcome.status, "failed");
    assert_eq!(outcome.result, receipt);
}

#[test]
fn native_failure_summary_is_typed_bound_and_survives_report_compaction() {
    let mut receipt = failed();
    receipt["failure_receipt"]["message"] = json!("x".repeat(4096));
    let mut step = crate::HeadlessExecutionStepReport {
        index: 4,
        action: "operator_task_execute".into(),
        risk: crate::HeadlessRisk::Normal,
        status: "failed".into(),
        payload: json!({"task":golden_task()}),
        result_preview: crate::run::compact_report_value(&receipt),
        requires_confirmation: false,
    };
    let summary = crate::execution_observability::summarize_execution(&[step.clone()]);
    let failure = summary.failure.unwrap();
    assert_eq!(failure.step_index, 4);
    assert_eq!(failure.message.len(), 4096);
    assert_eq!(failure.recommended_action, "inspect_engine_solver_failure");
    assert_eq!(failure.retry_strategy, "none");
    assert!(!failure.retryable);
    assert_eq!(
        step.result_preview["failure_receipt"],
        receipt["failure_receipt"]
    );
    step.result_preview["failure_receipt"]["task_id"] = json!("other-task");
    assert!(
        crate::execution_observability::summarize_execution(&[step])
            .failure
            .is_none()
    );
}

#[test]
fn http_native_failure_identity_recovery_and_state_must_be_consistent() {
    let mut cases = Vec::new();
    for field in [
        "task_id",
        "task_digest",
        "operator_id",
        "program_id",
        "schema_version",
        "failure_owner",
        "reason_code",
    ] {
        let mut receipt = failed();
        receipt["failure_receipt"][field] = json!("rejected-private-value");
        cases.push(receipt);
    }
    for patch in [
        json!({"failure_stage":""}),
        json!({"message":null}),
        json!({"message":"x".repeat(4097)}),
        json!({"recovery":null}),
        json!({"recovery":{"retryable":"false","required_action":"inspect_engine_solver_failure","safe_to_continue_other_tasks":true}}),
        json!({"recovery":{"retryable":false,"required_action":null,"safe_to_continue_other_tasks":true}}),
        json!({"recovery":{"retryable":false,"required_action":"inspect_engine_solver_failure","safe_to_continue_other_tasks":1}}),
    ] {
        let mut receipt = failed();
        receipt["failure_receipt"]
            .as_object_mut()
            .unwrap()
            .extend(patch.as_object().unwrap().clone());
        cases.push(receipt);
    }
    let mut receipt = failed();
    receipt["status"] = json!("executed");
    receipt["execution_readiness"] = json!({"status":"executed","ready_to_dispatch":true});
    receipt["result"] = json!({"value":1});
    cases.push(receipt);
    let mut receipt = failed();
    receipt["execution_readiness"]["required_action"] = json!("rejected-private-value");
    cases.push(receipt);
    let mut receipt = failed();
    receipt["result"] = failed();
    receipt["result"]["operator_task_ir_status"] = json!("failed");
    receipt["result"]["failure_receipt"]["message"] = json!("different failure");
    cases.push(receipt);
    for receipt in cases {
        let error =
            round_trip(&receipt).expect_err("unbound failures must not become recovery advice");
        assert!(
            error
                .message
                .starts_with("operator task execution receipt invalid:")
        );
        assert!(!error.message.contains("rejected-private-value"));
    }
}

fn round_trip(receipt: &Value) -> Result<HeadlessExecutorOutcome, HeadlessExecutorError> {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let body = serde_json::to_vec(receipt).unwrap();
    let server = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(3)))
            .unwrap();
        let request = read_request(&mut stream);
        let split = request
            .windows(4)
            .position(|bytes| bytes == b"\r\n\r\n")
            .unwrap()
            + 4;
        assert!(request.starts_with(b"POST /api/v1/operator-tasks/execute HTTP/1.1\r\n"));
        let payload: Value = serde_json::from_slice(&request[split..]).unwrap();
        assert_eq!(payload["task"], golden_task());
        let header = format!(
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        );
        stream.write_all(header.as_bytes()).unwrap();
        // Exercise actual HTTP framing, not direct JSON function invocation.
        for chunk in body.chunks(37) {
            stream.write_all(chunk).unwrap();
        }
    });
    let result = ServiceHeadlessExecutor::new(&format!("http://{address}")).execute_step(
        "operator_task_execute",
        1,
        &json!({"task":golden_task()}),
    );
    server.join().unwrap();
    result
}

fn read_request(stream: &mut TcpStream) -> Vec<u8> {
    let mut bytes = Vec::new();
    loop {
        let mut buffer = [0; 1024];
        let count = stream.read(&mut buffer).unwrap();
        assert!(count > 0, "request ended before its body");
        bytes.extend_from_slice(&buffer[..count]);
        assert!(bytes.len() < 64 * 1024);
        if let Some(split) = bytes.windows(4).position(|part| part == b"\r\n\r\n") {
            let header = std::str::from_utf8(&bytes[..split]).unwrap();
            let length = header
                .lines()
                .find_map(|line| {
                    line.strip_prefix("Content-Length: ")
                        .map(|value| value.parse::<usize>().unwrap())
                })
                .unwrap();
            if bytes.len() >= split + 4 + length {
                return bytes;
            }
        }
    }
}

#[test]
fn http_task_readiness_is_blocked_instead_of_execution_success() {
    let receipt = blocked();
    let outcome = round_trip(&receipt).unwrap();
    assert_eq!(outcome.status, "blocked");
    assert_eq!(outcome.result, receipt);
    assert_eq!(
        outcome.result["execution_readiness"]["required_action"],
        "attach_operator_package_runtime"
    );
}

#[test]
fn http_completed_receipt_keeps_result_bits_and_task_lineage() {
    let receipt = completed();
    let outcome = round_trip(&receipt).unwrap();
    assert_eq!(outcome.status, "executed");
    assert_eq!(outcome.result, receipt);
    assert_eq!(
        outcome.result["result"]["negative_zero"]
            .as_f64()
            .unwrap()
            .to_bits(),
        (-0.0_f64).to_bits()
    );
}

#[test]
fn wrapped_agent_readiness_cannot_be_promoted_by_outer_dispatch_success() {
    for state in [
        "verified_pending_engine_execution",
        "blocked",
        "ready_for_package_resolution",
        "executed",
    ] {
        let mut agent = if state == "executed" {
            completed()
        } else {
            blocked()
        };
        agent.as_object_mut().unwrap().remove("status");
        agent["operator_task_ir_status"] = json!(state);
        let mut outer = completed();
        outer["result"] = agent.clone();
        let outcome = round_trip(&outer).unwrap();
        assert_eq!(
            outcome.status,
            if state == "executed" {
                "executed"
            } else {
                "blocked"
            }
        );
        assert_eq!(outcome.result["result"], agent);
    }
}

#[test]
fn http_stale_or_malformed_completion_receipts_fail_closed() {
    let mut cases = Vec::new();
    for field in ["task_id", "task_digest", "operator_id", "program_id"] {
        for value in [Value::Null, json!("stale"), json!(3)] {
            let mut receipt = completed();
            receipt[field] = value;
            cases.push(receipt);
        }
        let mut receipt = completed();
        receipt.as_object_mut().unwrap().remove(field);
        cases.push(receipt);
    }
    for state in [Value::Null, json!("queued"), json!(true)] {
        let mut receipt = completed();
        receipt["status"] = state;
        cases.push(receipt);
    }
    for key in ["result", "status"] {
        let mut receipt = completed();
        receipt.as_object_mut().unwrap().remove(key);
        cases.push(receipt);
    }
    for patch in [
        json!({"result":null}),
        json!({"blocked_stage":"fetch_package"}),
        json!({"execution_readiness":null}),
        json!({"execution_readiness":{"status":"mystery"}}),
        json!({"execution_readiness":{"status":"executed","ready_to_dispatch":false}}),
        json!({"execution_readiness":{"status":"executed","ready_to_dispatch":true,"blocking_stage":"fetch_package"}}),
        json!({"execution_readiness":{"status":"executed","ready_to_dispatch":true,"blocking_reason":"package_unavailable"}}),
        json!({"execution_readiness":{"status":"executed","ready_to_dispatch":true,"required_action":"attach_runtime"}}),
        json!({"status":"blocked","execution_readiness":{"status":"executed","ready_to_dispatch":true}}),
        json!({"status":"failed","execution_readiness":{"status":"executed","ready_to_dispatch":true}}),
        json!({"ok":false}),
        json!({"ok":"true"}),
        json!({"error":"solver failed"}),
        json!({"validation_receipt":{"digest_verified":false}}),
        json!({"provenance_receipt":{"task_digest":"stale"}}),
        json!({"provenance_receipt":null}),
    ] {
        let mut receipt = completed();
        receipt
            .as_object_mut()
            .unwrap()
            .extend(patch.as_object().unwrap().clone());
        cases.push(receipt);
    }
    for receipt in cases {
        let error =
            round_trip(&receipt).expect_err("unverified execution must not advance research");
        assert!(
            error
                .message
                .starts_with("operator task execution receipt invalid:")
        );
        assert!(!error.message.contains("stale"));
    }
    assert_eq!(round_trip(&completed()).unwrap().status, "executed");
}

#[test]
fn nested_agent_identity_and_validation_receipts_are_checked_independently() {
    for field in ["task_id", "task_digest", "operator_id", "program_id"] {
        let mut outer = completed();
        let mut agent = completed();
        agent.as_object_mut().unwrap().remove("status");
        agent["operator_task_ir_status"] = json!("executed");
        agent[field] = json!("other-task");
        outer["result"] = agent;
        assert!(round_trip(&outer).is_err());
    }
    let mut outer = blocked();
    outer["result"] = completed();
    outer["result"]["operator_task_ir_status"] = json!("executed");
    assert!(round_trip(&outer).is_err());
}

#[test]
fn valid_noncompletion_states_are_preserved_without_counting_computation() {
    for (state, expected) in [
        ("verified_pending_execution", "blocked"),
        ("blocked", "blocked"),
        ("ready_for_package_resolution", "blocked"),
        ("failed", "failed"),
        ("error", "failed"),
        ("cancelled", "failed"),
    ] {
        let mut receipt = prepared();
        receipt["status"] = json!(state);
        let outcome = round_trip(&receipt).unwrap();
        assert_eq!(outcome.status, expected);
        assert_eq!(outcome.result, receipt);
    }
}

#[test]
fn service_task_preflight_rejects_digest_tamper_without_opening_a_socket() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let mut executor =
        ServiceHeadlessExecutor::new(&format!("http://{}", listener.local_addr().unwrap()));
    let mut task = golden_task();
    task["input_artifact"]["x"] = json!(2);
    let error = executor
        .execute_step("operator_task_execute", 1, &json!({"task":task}))
        .unwrap_err();
    assert!(error.message.contains("digest mismatch"));
    assert_eq!(
        listener.accept().unwrap_err().kind(),
        std::io::ErrorKind::WouldBlock
    );
}
