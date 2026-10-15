use super::*;

fn invalid_responses() -> Vec<(&'static str, RpcResponse)> {
    let success = || RpcResponse::success("request-one", json!({"value": 2}));
    let failure = || RpcResponse::error("request-one", "solve_failed", "solver failed");
    let mut version = success();
    version.rpc_version = 0;
    let mut missing_result = success();
    missing_result.result = None;
    let mut both = success();
    both.error = failure().error;
    let mut missing_error = failure();
    missing_error.error = None;
    let mut failed_result = failure();
    failed_result.result = Some(json!({"value": 2}));
    vec![
        ("version", version),
        (
            "null_result",
            RpcResponse::success("request-one", Value::Null),
        ),
        ("missing_result", missing_result),
        ("success_and_error", both),
        ("missing_error", missing_error),
        ("failed_result", failed_result),
        (
            "empty_code",
            RpcResponse::error("request-one", " ", "solver failed"),
        ),
        (
            "empty_message",
            RpcResponse::error("request-one", "solve_failed", "\t"),
        ),
    ]
}

#[test]
fn null_success_is_rejected_before_the_restart_reader_loses_its_result() {
    let scratch = Scratch::new();
    let mut store = Store::open(&scratch.0).unwrap();
    let record = admit(&mut store, &task('a'));
    let response = RpcResponse::success("request-one", Value::Null);
    assert!(response.result.is_some());
    let decoded: RpcResponse =
        serde_json::from_slice(&serde_json::to_vec(&response).unwrap()).unwrap();
    assert!(decoded.result.is_none());
    let report = store.finish(&record, response).unwrap();
    assert_response_rejected(&report, "invalid_response");
}

#[test]
fn invalid_response_state_is_never_committed_as_success_or_poisoned_storage() {
    for (case, response) in invalid_responses() {
        let scratch = Scratch::new();
        let mut store = Store::open(&scratch.0).unwrap();
        let task = task('a');
        let record = admit(&mut store, &task);
        let report = store.finish(&record, response).unwrap();
        assert_eq!(report["status"], "failed", "{case}: {report}");
        assert_response_rejected(&report, "invalid_response");
        assert!(!store.write_failed, "{case}");
        assert_eq!(store.lookup(&task.execution_id).unwrap(), report, "{case}");
        drop(store);
        let mut store = Store::open(&scratch.0).unwrap();
        assert_eq!(store.lookup(&task.execution_id).unwrap(), report, "{case}");
        assert!(matches!(
            store.admit_with(&task, "new-process", "repeat", || -> Result<(), String> {
                panic!("an invalid response must not authorize replay");
            }),
            Ok((Admission::Existing(_), None))
        ));
        admit(&mut store, &super::task('b'));
    }
}

#[test]
fn legacy_invalid_receipts_remain_inspect_errors_not_terminal_or_replayable() {
    for (case, response) in invalid_responses() {
        let scratch = Scratch::new();
        let mut store = Store::open(&scratch.0).unwrap();
        let task = task('a');
        let record = admit(&mut store, &task);
        let name = format!("{}.receipt", record.execution_id);
        io::write(
            &scratch.0,
            &name,
            &Receipt { record, response },
            MAX_RECEIPT_BYTES,
        )
        .unwrap();
        let original = fs::read(scratch.0.join(&name)).unwrap();
        drop(store);
        let mut store = Store::open(&scratch.0).unwrap();
        assert!(store.lookup(&task.execution_id).is_err(), "{case}");
        assert!(
            store
                .admit_with(&task, "process-two", "repeat", || -> Result<(), String> {
                    panic!("an invalid retained receipt must not invoke the engine");
                })
                .is_err(),
            "{case}"
        );
        assert_eq!(fs::read(scratch.0.join(&name)).unwrap(), original, "{case}");
        assert_eq!(
            store
                .list(&kyuubiki_protocol::DurableAgentExecutionList {
                    expected_store_id: None,
                    limit: 1,
                    after: None,
                })
                .unwrap()["executions"][0]["execution_id"],
            task.execution_id
        );
        let next = admit(&mut store, &super::task('b'));
        assert_eq!(
            store
                .finish(
                    &next,
                    RpcResponse::success("request-one", json!({"value":3}))
                )
                .unwrap()["status"],
            "completed"
        );
    }
}

#[test]
fn valid_response_contract_preserves_nested_nulls_and_non_object_results() {
    let mut responses = vec![RpcResponse::error_with_details(
        "request-one",
        "solve_failed",
        "solver failed",
        json!({"checkpoint":null}),
    )];
    for value in [
        json!({"optional":null}),
        json!([null, 2]),
        json!(false),
        json!(0),
        json!(""),
    ] {
        responses.push(RpcResponse::success("request-one", value));
    }
    for response in responses {
        let scratch = Scratch::new();
        let mut store = Store::open(&scratch.0).unwrap();
        let record = admit(&mut store, &task('a'));
        let expected = serde_json::to_value(&response).unwrap();
        let report = store.finish(&record, response).unwrap();
        assert_eq!(report["response"], expected);
        drop(store);
        let store = Store::open(&scratch.0).unwrap();
        assert_eq!(store.lookup(&record.execution_id).unwrap(), report);
    }
}

#[test]
fn invalid_response_replacement_write_failure_stays_unknown() {
    let scratch = Scratch::new();
    let mut store = Store::open(&scratch.0).unwrap();
    let record = admit(&mut store, &task('a'));
    let pending = scratch
        .0
        .join(format!("{}.receipt.next", record.execution_id));
    fs::create_dir(&pending).unwrap();
    assert!(
        store
            .finish(&record, RpcResponse::success("request-one", Value::Null))
            .is_err()
    );
    assert!(store.write_failed);
    let report = store.lookup(&record.execution_id).unwrap();
    assert_eq!(report["status"], "outcome_unknown");
    assert!(report["response"].is_null());
    assert!(store.admit(&task('b'), "process", "request-two").is_err());
    fs::remove_dir(pending).unwrap();
    drop(store);
    let mut store = Store::open(&scratch.0).unwrap();
    assert_eq!(store.lookup(&record.execution_id).unwrap(), report);
    assert!(matches!(
        store.admit(&task('a'), "new-process", "repeat").unwrap(),
        Admission::Existing(_)
    ));
}

#[test]
fn wrong_response_identity_never_becomes_a_compact_terminal_failure() {
    let scratch = Scratch::new();
    let mut store = Store::open(&scratch.0).unwrap();
    let record = admit(&mut store, &task('a'));
    assert!(
        store
            .finish(
                &record,
                RpcResponse::success("another-request", Value::Null)
            )
            .is_err()
    );
    assert!(
        !scratch
            .0
            .join(format!("{}.receipt", record.execution_id))
            .exists()
    );
    assert_eq!(
        store.lookup(&record.execution_id).unwrap()["status"],
        "running"
    );
    let report = store
        .finish(
            &record,
            RpcResponse::success("request-one", json!({"value":2})),
        )
        .unwrap();
    assert_eq!(report["status"], "completed");
}
