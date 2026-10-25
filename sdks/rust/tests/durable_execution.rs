use kyuubiki_headless_sdk::{DurableExecutionHandle, SolverRpcClient};
use serde_json::{Value, json};
use std::{
    io::{Read, Write},
    net::TcpListener,
    thread,
};

fn receipt() -> Value {
    json!({"schema_version":"kyuubiki.agent-durable-execution/v1",
        "store_id":"b".repeat(32), "execution_id":"a".repeat(32),
        "request_digest":"c".repeat(64), "process_instance_id":"process-a", "request_id":"submit-a",
        "method":"solve_modal_frame_2d", "status":"completed", "resumable":false,
        "automatic_replay_authorized":false,
        "response":{"rpc_version":1, "id":"submit-a", "ok":true, "result":{"modes":[1,2,3,4]}}})
}

fn failure() -> Value {
    let mut failed = receipt();
    failed["status"] = json!("failed");
    failed["response"] = json!({"rpc_version":1, "id":"submit-a", "ok":false,
        "result":null, "error":{"code":"cancelled", "message":"original failure",
        "details":{"stage":"solve", "values":[-0.0, 1.0000000000000002]}}});
    failed
}

#[test]
fn retained_terminal_observation_cannot_change_or_be_downgraded() {
    let original = receipt();
    let handle = DurableExecutionHandle::from_report(&original).unwrap();
    handle.validate_followup(&original, &original).unwrap();
    let mut changed = original.clone();
    changed["response"]["result"]["modes"][0] = json!(99);
    assert!(
        handle
            .validate_followup(&original, &changed)
            .unwrap_err()
            .to_string()
            .contains("terminal observation")
    );
    assert!(handle.validate_followup(&original, &failure()).is_err());
    for state in ["running", "outcome_unknown"] {
        let mut newer = original.clone();
        newer["status"] = json!(state);
        newer["response"] = Value::Null;
        assert!(
            handle.validate_followup(&original, &newer).is_err(),
            "{state}"
        );
    }
    assert_eq!(original, receipt());
}

#[test]
fn retained_terminal_failure_keeps_original_cause_and_opaque_details() {
    let original = failure();
    let handle = DurableExecutionHandle::from_report(&original).unwrap();
    handle.validate_followup(&original, &original).unwrap();
    assert!(handle.completed_result(&original).is_err());
    for (field, changed_value) in [
        ("code", json!("different-code")),
        ("message", json!("different-message")),
        ("details", json!({"stage":"later"})),
        ("details", Value::Null),
    ] {
        let mut changed = original.clone();
        changed["response"]["error"][field] = changed_value;
        assert!(
            handle.validate_followup(&original, &changed).is_err(),
            "{field}"
        );
    }
    assert!(handle.validate_followup(&original, &receipt()).is_err());
    let mut without_details = original.clone();
    without_details["response"]["error"]
        .as_object_mut()
        .unwrap()
        .remove("details");
    let mut explicit_null = without_details.clone();
    explicit_null["response"]["error"]["details"] = Value::Null;
    handle
        .validate_followup(&without_details, &explicit_null)
        .unwrap();
}

#[test]
fn terminal_consistency_preserves_numeric_bits_and_normalizes_optional_nulls() {
    let mut original = receipt();
    original["response"]["result"] =
        json!({"value":-0.0, "fine":1.0000000000000002, "count":u64::MAX});
    let handle = DurableExecutionHandle::from_report(&original).unwrap();
    let mut explicit_null = original.clone();
    explicit_null["response"]["error"] = Value::Null;
    explicit_null["response"]["event"] = Value::Null;
    explicit_null["observed_at"] = json!("new optional observation metadata");
    handle.validate_followup(&original, &explicit_null).unwrap();
    for (field, value) in [
        ("value", json!(0.0)),
        ("fine", json!(1.0)),
        ("count", json!(u64::MAX - 1)),
    ] {
        let mut changed = original.clone();
        changed["response"]["result"][field] = value;
        if field == "value" {
            assert_eq!(
                original["response"]["result"], changed["response"]["result"],
                "ordinary JSON equality alone loses the sign of zero"
            );
        }
        assert!(
            handle.validate_followup(&original, &changed).is_err(),
            "{field}"
        );
    }
}

#[test]
fn completed_result_binding_preserves_typed_values_after_json_roundtrip() {
    for value in [
        json!(-0.0),
        json!(0.0),
        json!(f64::from_bits(1)),
        json!(-f64::from_bits(1)),
        json!(f64::MIN_POSITIVE),
        json!(f64::MAX),
        json!(-f64::MAX),
        json!(1.0000000000000002),
        json!(i64::MIN),
        json!(u64::MAX),
        json!(9_007_199_254_740_993_u64),
        json!(false),
        json!("result"),
        json!({}),
        json!([]),
    ] {
        let mut report = receipt();
        report["response"]["result"] = json!({"nested":[value, null]});
        let handle = DurableExecutionHandle::from_report(&report).unwrap();
        let decoded = serde_json::from_slice(
            &serde_json::to_vec_pretty(&report["response"]["result"]).unwrap(),
        )
        .unwrap();
        handle.validate_completed_result(&report, &decoded).unwrap();
    }
    // Scalar result payloads are valid too; the contract is not modal-specific.
    let mut report = receipt();
    report["response"]["result"] = json!(false);
    DurableExecutionHandle::from_report(&report)
        .unwrap()
        .validate_completed_result(&report, &json!(false))
        .unwrap();
}

#[test]
fn completed_result_binding_rejects_numeric_and_structural_substitutions() {
    for (original, changed) in [
        (json!(-0.0), json!(0.0)),
        (json!(1.0000000000000002), json!(1.0)),
        (json!(f64::from_bits(1)), json!(0.0)),
        (
            json!(f64::MAX),
            json!(f64::from_bits(f64::MAX.to_bits() - 1)),
        ),
        (json!(21.0), json!(21)),
        (json!(u64::MAX), json!(u64::MAX - 1)),
        (
            json!(9_007_199_254_740_993_u64),
            json!(9_007_199_254_740_992_u64),
        ),
        (json!(false), json!(0)),
        (json!("1"), json!(1)),
        (json!({"optional":null}), json!({})),
        (json!([1, 2]), json!([2, 1])),
    ] {
        let mut report = receipt();
        report["response"]["result"] = json!({"nested":[original]});
        let handle = DurableExecutionHandle::from_report(&report).unwrap();
        let candidate = json!({"nested":[changed]});
        assert!(
            handle
                .validate_completed_result(&report, &candidate)
                .unwrap_err()
                .to_string()
                .contains("original completed receipt")
        );
        assert!(
            handle
                .validate_completed_result(&report, &Value::Null)
                .is_err()
        );
        handle
            .validate_completed_result(&report, &report["response"]["result"])
            .unwrap();
    }
}

#[test]
fn completed_result_binding_accepts_formatting_but_not_changed_numeric_types() {
    let mut report = receipt();
    report["response"]["result"] =
        serde_json::from_str(r#"{"x":1.0,"nested":{"a":-0.0,"b":"\u0061"}}"#).unwrap();
    let equivalent = serde_json::from_str(r#"{ "nested": {"b":"a", "a":-0e0}, "x":1e0 }"#).unwrap();
    let handle = DurableExecutionHandle::from_report(&report).unwrap();
    handle
        .validate_completed_result(&report, &equivalent)
        .unwrap();
    let mut integer = equivalent;
    integer["x"] = json!(1);
    assert!(handle.validate_completed_result(&report, &integer).is_err());
}

#[test]
fn completed_result_binding_rejects_noncompleted_or_foreign_receipts() {
    let original = receipt();
    let handle = DurableExecutionHandle::from_report(&original).unwrap();
    let result = &original["response"]["result"];
    for state in ["running", "outcome_unknown"] {
        let mut report = original.clone();
        report["status"] = json!(state);
        report["response"] = Value::Null;
        assert!(handle.validate_completed_result(&report, result).is_err());
    }
    let failed = failure();
    match handle
        .validate_completed_result(&failed, result)
        .unwrap_err()
    {
        kyuubiki_headless_sdk::SdkError::Rpc {
            code,
            message,
            request_id,
            details,
        } => {
            assert_eq!(code.as_deref(), Some("cancelled"));
            assert_eq!(message, "original failure");
            assert_eq!(request_id.as_deref(), Some("submit-a"));
            assert_eq!(
                details.as_ref(),
                Some(&failed["response"]["error"]["details"])
            );
        }
        other => panic!("original solver cause was lost: {other}"),
    }
    let mut foreign = original.clone();
    foreign["store_id"] = json!("e".repeat(32));
    assert!(handle.validate_completed_result(&foreign, result).is_err());
    let mut contradictory = original.clone();
    contradictory["response"]["error"] = failed["response"]["error"].clone();
    assert!(
        handle
            .validate_completed_result(&contradictory, result)
            .is_err()
    );
}

#[test]
fn completed_result_binding_bounds_both_retained_and_candidate_depth() {
    let mut deep = json!(0);
    for _ in 0..130 {
        deep = Value::Array(vec![deep]);
    }
    let mut report = receipt();
    let handle = DurableExecutionHandle::from_report(&report).unwrap();
    assert!(
        handle
            .validate_completed_result(&report, &deep)
            .unwrap_err()
            .to_string()
            .contains("depth")
    );
    report["response"]["result"] = deep;
    assert!(
        handle
            .validate_completed_result(&report, &json!(1))
            .unwrap_err()
            .to_string()
            .contains("depth")
    );
}

#[test]
fn nonterminal_observations_do_not_invent_terminal_commitments_or_new_identity() {
    let completed = receipt();
    let handle = DurableExecutionHandle::from_report(&completed).unwrap();
    for state in ["running", "outcome_unknown"] {
        let mut earlier = completed.clone();
        earlier["status"] = json!(state);
        earlier["response"] = Value::Null;
        handle.validate_followup(&earlier, &earlier).unwrap();
        handle.validate_followup(&earlier, &completed).unwrap();
        handle.validate_followup(&earlier, &failure()).unwrap();
        let mut foreign = completed.clone();
        foreign["process_instance_id"] = json!("replacement-process");
        assert!(handle.validate_followup(&earlier, &foreign).is_err());
        assert!(handle.validate_followup(&foreign, &completed).is_err());
        assert!(handle.completed_result(&earlier).is_err());
    }
}

#[test]
fn followup_validation_is_bounded_and_invalid_retained_identity_precedes_network() {
    let mut deep = receipt();
    let mut payload = Value::Null;
    for _ in 0..130 {
        payload = Value::Array(vec![payload]);
    }
    deep["response"]["result"] = payload;
    let handle = DurableExecutionHandle::from_report(&deep).unwrap();
    assert!(
        handle
            .validate_followup(&deep, &deep)
            .unwrap_err()
            .to_string()
            .contains("depth")
    );
    let client = SolverRpcClient::new("invalid.invalid", 1);
    let mut invalid = receipt();
    invalid["response"]["id"] = json!("not-the-submission");
    assert!(
        client
            .fetch_durable_agent_execution_after(&invalid)
            .unwrap_err()
            .to_string()
            .contains("terminal response does not match")
    );
}

#[test]
fn history_aware_fetch_reads_once_without_replaying_or_rewriting_terminals() {
    let original = receipt();
    let mut changed = original.clone();
    changed["response"]["result"]["modes"][0] = json!(99);
    let missing = json!({"schema_version":"kyuubiki.agent-durable-execution/v1",
        "store_id":"b".repeat(32), "execution_id":"a".repeat(32),
        "status":"not_found", "automatic_replay_authorized":false});
    for (retained, observed, accepts) in [
        (original.clone(), original.clone(), true),
        (original.clone(), changed, false),
        (failure(), original.clone(), false),
        (failure(), failure(), true),
        (original, missing, false),
    ] {
        let expected = observed.clone();
        let (client, server) = mock(observed, |request| {
            assert_eq!(request["method"], "fetch_durable_agent_execution");
            assert_eq!(
                request["params"],
                json!({"expected_store_id":"b".repeat(32), "execution_id":"a".repeat(32)})
            );
        });
        let fetched = client.fetch_durable_agent_execution_after(&retained);
        server.join().unwrap();
        assert_eq!(fetched.is_ok(), accepts);
        if accepts {
            assert_eq!(fetched.unwrap(), expected);
        }
    }
}

#[test]
fn history_aware_fetch_bounds_retained_content_before_connecting() {
    use std::time::Duration;
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let client = SolverRpcClient::new("127.0.0.1", listener.local_addr().unwrap().port())
        .with_limits(kyuubiki_headless_sdk::SolverRpcLimits {
            io_timeout: Duration::from_millis(50),
            call_timeout: Duration::from_millis(250),
            ..Default::default()
        })
        .unwrap();
    let mut payload = json!(0);
    for _ in 0..130 {
        payload = Value::Array(vec![payload]);
    }
    for mut retained in [receipt(), failure()] {
        if retained["status"] == "completed" {
            retained["response"]["result"] = payload.clone();
        } else {
            retained["response"]["error"]["details"] = payload.clone();
        }
        let error = client
            .fetch_durable_agent_execution_after(&retained)
            .unwrap_err();
        assert!(
            error.to_string().contains("depth"),
            "local error was hidden: {error}"
        );
        assert_eq!(
            listener.accept().unwrap_err().kind(),
            std::io::ErrorKind::WouldBlock
        );
    }
}

#[test]
fn retained_handle_rejects_changed_ownership_and_contradictory_success() {
    let original = receipt();
    let handle = DurableExecutionHandle::from_report(&original).unwrap();
    let recovered: DurableExecutionHandle =
        serde_json::from_slice(&serde_json::to_vec(&handle).unwrap()).unwrap();
    assert_eq!(
        recovered.completed_result(&original).unwrap(),
        &original["response"]["result"]
    );
    for (pointer, value) in [
        ("/store_id", json!("d".repeat(32))),
        ("/execution_id", json!("d".repeat(32))),
        ("/request_digest", json!("d".repeat(64))),
        ("/request_id", json!("other")),
        ("/process_instance_id", json!("other")),
        ("/method", json!("solve_bar_1d")),
        ("/automatic_replay_authorized", json!(true)),
        ("/resumable", json!(true)),
        ("/status", json!("outcome_unknown")),
        ("/response/id", json!("other")),
        ("/response/ok", json!(false)),
        ("/response/result", Value::Null),
    ] {
        let mut changed = original.clone();
        *changed.pointer_mut(pointer).unwrap() = value;
        assert!(recovered.completed_result(&changed).is_err(), "{pointer}");
    }
}

#[test]
fn unfinished_or_failed_execution_is_never_a_physical_result() {
    let original = receipt();
    let handle = DurableExecutionHandle::from_report(&original).unwrap();
    for status in ["running", "outcome_unknown", "failed"] {
        let mut report = original.clone();
        report["status"] = json!(status);
        report["response"] = if status == "failed" {
            json!({"rpc_version":1, "id":"submit-a", "ok":false, "error":{"code":"cancelled", "message":"cancelled at checkpoint"}})
        } else {
            Value::Null
        };
        handle.validate_report(&report).unwrap();
        assert!(handle.completed_result(&report).is_err());
    }
}

fn mock(
    report: Value,
    check: impl FnOnce(Value) + Send + 'static,
) -> (SolverRpcClient, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut header = [0; 4];
        stream.read_exact(&mut header).unwrap();
        let size = u32::from_be_bytes(header) as usize;
        assert!(size < 65536);
        let mut bytes = vec![0; size];
        stream.read_exact(&mut bytes).unwrap();
        let request: Value = serde_json::from_slice(&bytes).unwrap();
        let response = serde_json::to_vec(
            &json!({"rpc_version":1, "id":request["id"], "ok":true, "result":report}),
        )
        .unwrap();
        check(request);
        stream
            .write_all(&(response.len() as u32).to_be_bytes())
            .unwrap();
        stream.write_all(&response).unwrap();
    });
    (SolverRpcClient::new("127.0.0.1", port), server)
}

#[test]
fn fetch_pins_the_original_store_and_never_submits_a_task() {
    let original = receipt();
    let handle = DurableExecutionHandle::from_report(&original).unwrap();
    let (client, server) = mock(original.clone(), |request| {
        assert_eq!(request["method"], "fetch_durable_agent_execution");
        assert_eq!(
            request["params"],
            json!({"expected_store_id":"b".repeat(32), "execution_id":"a".repeat(32)})
        );
    });
    assert_eq!(
        client.fetch_durable_agent_execution(&handle).unwrap(),
        original
    );
    server.join().unwrap();
}

#[test]
fn missing_receipt_is_observable_but_cannot_recover_a_completed_handle() {
    let missing = json!({"schema_version":"kyuubiki.agent-durable-execution/v1",
        "store_id":"b".repeat(32), "execution_id":"a".repeat(32),
        "status":"not_found", "automatic_replay_authorized":false});
    let (client, server) = mock(missing.clone(), |_| {});
    assert_eq!(
        client
            .inspect_durable_agent_execution(&"b".repeat(32), &"a".repeat(32))
            .unwrap(),
        missing
    );
    server.join().unwrap();
    let (client, server) = mock(missing, |_| {});
    assert!(
        client
            .fetch_durable_agent_execution(
                &DurableExecutionHandle::from_report(&receipt()).unwrap()
            )
            .is_err()
    );
    server.join().unwrap();
}

#[test]
fn invalid_handle_and_invalid_submission_fail_before_connecting() {
    let client = SolverRpcClient::new("invalid.invalid", 1);
    let mut handle = DurableExecutionHandle::from_report(&receipt()).unwrap();
    handle.store_id = "../store".into();
    assert!(
        client
            .fetch_durable_agent_execution(&handle)
            .unwrap_err()
            .to_string()
            .contains("hex")
    );
    assert!(
        client
            .submit_durable_agent_task(
                &"b".repeat(32),
                &"a".repeat(32),
                "submit_durable_agent_task",
                json!({})
            )
            .unwrap_err()
            .to_string()
            .contains("requires")
    );
}

#[test]
fn mismatched_admission_is_not_accepted_as_a_new_execution() {
    let (client, server) = mock(receipt(), |request| {
        assert_eq!(request["method"], "submit_durable_agent_task");
        assert_eq!(request["params"]["expected_store_id"], "b".repeat(32));
        assert_eq!(request["params"]["execution_id"], "d".repeat(32));
    });
    assert!(
        client
            .submit_durable_agent_task(
                &"b".repeat(32),
                &"d".repeat(32),
                "solve_modal_frame_2d",
                json!({})
            )
            .is_err()
    );
    server.join().unwrap();
}

#[test]
fn matching_ids_cannot_hide_a_receipt_for_different_input() {
    let (client, server) = mock(receipt(), |_| {});
    let admitted = client.submit_durable_agent_task(
        &"b".repeat(32),
        &"a".repeat(32),
        "solve_modal_frame_2d",
        json!({"probe":1.0}),
    );
    server.join().unwrap();
    assert!(
        admitted.is_err(),
        "a matching ID must not hide a different request digest"
    );
}

#[test]
fn a_valid_outer_receipt_cannot_hide_an_invalid_inner_response() {
    for (field, value) in [
        ("rpc_version", json!(2)),
        ("rpc_version", Value::Null),
        ("event", json!("heartbeat")),
        ("progress", json!({})),
    ] {
        let mut report = receipt();
        report["response"][field] = value;
        assert!(
            DurableExecutionHandle::from_report(&report).is_err(),
            "inner {field}"
        );
    }
}

#[test]
fn malformed_failure_is_not_a_valid_terminal_receipt() {
    for error in [
        json!({}),
        json!({"code":"failed"}),
        json!({"code":" ","message":"reason"}),
        json!({"code":"failed","message":" "}),
        json!({"code":1,"message":"reason"}),
    ] {
        let mut report = receipt();
        report["status"] = json!("failed");
        report["response"] = json!({"rpc_version":1, "id":"submit-a", "ok":false, "error":error});
        assert!(
            DurableExecutionHandle::from_report(&report).is_err(),
            "{report}"
        );
    }
}

#[test]
fn completed_result_preserves_the_original_solver_failure_instead_of_a_generic_error() {
    let mut report = receipt();
    report["status"] = json!("failed");
    report["response"] = json!({"rpc_version":1,"id":"submit-a","ok":false,
        "error":{"code":"invalid_density","message":"density must be positive"}});
    let handle = DurableExecutionHandle::from_report(&report).unwrap();
    let error = handle.completed_result(&report).unwrap_err();
    assert!(
        matches!(error, kyuubiki_headless_sdk::SdkError::Rpc {code:Some(code),message,request_id:Some(id),..} if code == "invalid_density" && message == "density must be positive" && id == "submit-a")
    );
}

#[test]
fn recovered_failure_details_are_retained_but_never_authorize_replay() {
    let mut report = receipt();
    report["status"] = json!("failed");
    let details = json!({"solver_checkpoint":{"stage":"modal_shape_norm"},
        "hint":"private-diagnostic-marker", "automatic_replay_authorized":true,
        "nested_result":{"status":"completed"}, "small":5e-324, "zero":-0.0});
    report["response"] = json!({"rpc_version":1,"id":"submit-a","ok":false,
        "error":{"code":"solve_failed","message":"original solver cause","details":details}});
    let retained: Value = serde_json::from_slice(&serde_json::to_vec(&report).unwrap()).unwrap();
    let handle = DurableExecutionHandle::from_report(&retained).unwrap();
    let error = handle.completed_result(&retained).unwrap_err();
    assert_eq!(error.to_string(), "solve_failed: original solver cause");
    match error {
        kyuubiki_headless_sdk::SdkError::Rpc {
            request_id: Some(id),
            details: Some(observed),
            ..
        } => {
            assert_eq!(id, "submit-a");
            assert_eq!(observed, details);
            assert_eq!(observed["small"].as_f64().unwrap().to_bits(), 1);
            assert_eq!(
                observed["zero"].as_f64().unwrap().to_bits(),
                (-0.0_f64).to_bits()
            );
        }
        other => panic!("expected original diagnostic failure: {other}"),
    }
    assert_eq!(retained, report);
    let mut wrong = retained;
    wrong["response"]["id"] = json!("another-attempt");
    assert!(matches!(
        handle.completed_result(&wrong),
        Err(kyuubiki_headless_sdk::SdkError::Transport(_))
    ));
}

#[test]
fn missing_response_state_and_control_character_identities_are_not_receipts() {
    for status in ["running", "outcome_unknown"] {
        let mut report = receipt();
        report["status"] = json!(status);
        report.as_object_mut().unwrap().remove("response");
        assert!(DurableExecutionHandle::from_report(&report).is_err());
    }
    for field in ["request_id", "process_instance_id", "method"] {
        for value in [" ", "hidden\nidentity"] {
            let mut report = receipt();
            report[field] = json!(value);
            if field == "request_id" {
                report["response"]["id"] = json!(value);
            }
            assert!(
                DurableExecutionHandle::from_report(&report).is_err(),
                "{field}"
            );
        }
    }
}
