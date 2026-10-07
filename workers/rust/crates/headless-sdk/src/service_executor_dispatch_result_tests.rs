use super::*;

fn prepared() -> Value {
    json!({"task_id":"task", "task_digest":"a".repeat(64), "operator_id":"op", "program_id":"op"})
}
fn receipt() -> Value {
    json!({"schema_version":"kyuubiki.operator-task-dispatch-result/v1", "attempt_id":"1".repeat(32),
        "task_id":"task", "task_digest":"a".repeat(64), "automatic_replay_authorized":false,
        "publication_performed":false, "status":"not_retained", "outcome":"unknown", "completion":null})
}

#[test]
fn original_result_requires_explicit_fields_and_rejects_uncontracted_authority() {
    let original = receipt();
    for field in original.as_object().unwrap().keys() {
        let mut missing = original.clone();
        missing.as_object_mut().unwrap().remove(field);
        assert!(
            validate(&missing, &prepared(), &"1".repeat(32)).is_err(),
            "{field}"
        );
    }
    for field in [
        "execution_terminal_confirmed",
        "retryable",
        "agent_acknowledgement",
        "result",
    ] {
        let mut extra = original.clone();
        extra[field] = json!(true);
        assert!(
            validate(&extra, &prepared(), &"1".repeat(32)).is_err(),
            "{field}"
        );
    }
}

#[test]
fn every_missing_receipt_status_stays_unknown_with_valid_optional_identity_only() {
    for status in [
        "no_retained_dispatch",
        "not_dispatched",
        "original_endpoint_not_configured",
        "original_endpoint_unreachable",
        "agent_receipt_invalid",
        "pending",
        "not_retained",
        "result_not_retained",
        "attempt_identity_ambiguous",
    ] {
        let mut original = receipt();
        original["status"] = json!(status);
        validate(&original, &prepared(), &"1".repeat(32)).unwrap();
        for (key, value) in [
            ("generation", json!(0)),
            ("generation", json!(1.5)),
            ("generation", Value::Null),
            ("request_id", json!("")),
            ("request_id", json!("x".repeat(257))),
            ("process_instance_id", json!("unavailable")),
        ] {
            let mut invalid = original.clone();
            invalid[key] = value;
            assert!(
                validate(&invalid, &prepared(), &"1".repeat(32)).is_err(),
                "{status}/{key}"
            );
        }
        original["generation"] = json!(1);
        original["request_id"] = json!("retained-request");
        original["process_instance_id"] = json!("agent-boot");
        validate(&original, &prepared(), &"1".repeat(32)).unwrap();
    }
}

#[test]
fn unknown_is_never_completion_or_replay_permission() {
    let prepared = prepared();
    let receipt = receipt();
    validate(&receipt, &prepared, &"1".repeat(32)).unwrap();
    for (key, value) in [
        ("automatic_replay_authorized", json!(true)),
        ("publication_performed", json!(true)),
        ("outcome", json!("executed")),
        ("completion", json!({"result":42})),
        ("attempt_id", json!("2".repeat(32))),
        ("status", json!("success")),
    ] {
        let mut bad = receipt.clone();
        bad[key] = value;
        assert!(validate(&bad, &prepared, &"1".repeat(32)).is_err(), "{key}");
    }
}

#[test]
fn recovered_result_uses_the_existing_task_completion_gate() {
    let prepared = prepared();
    let mut receipt = receipt();
    receipt["status"] = json!("receipt_recovered");
    receipt["outcome"] = json!("executed");
    receipt["request_id"] = json!("request");
    receipt["process_instance_id"] = json!("agent-boot");
    receipt["generation"] = json!(1);
    let mut completion = prepared.clone();
    completion["status"] = json!("executed");
    completion["execution_readiness"] = json!({"status":"executed", "ready_to_dispatch":true});
    let mut agent = prepared.clone();
    agent["operator_task_ir_status"] = json!("executed");
    agent["execution_readiness"] = completion["execution_readiness"].clone();
    agent["result"] = json!({"stress":1.0});
    completion["result"] = agent;
    receipt["completion"] = completion;
    validate(&receipt, &prepared, &"1".repeat(32)).unwrap();
    let mut wrong = receipt.clone();
    wrong["completion"]["task_digest"] = json!("b".repeat(64));
    assert!(validate(&wrong, &prepared, &"1".repeat(32)).is_err());
    wrong = receipt.clone();
    wrong["completion"]["execution_readiness"]["status"] = json!("blocked");
    assert!(validate(&wrong, &prepared, &"1".repeat(32)).is_err());
    wrong = receipt.clone();
    wrong["generation"] = json!(0);
    assert!(validate(&wrong, &prepared, &"1".repeat(32)).is_err());
    wrong = receipt.clone();
    wrong["completion"]["result"]["program_id"] = json!("other");
    assert!(validate(&wrong, &prepared, &"1".repeat(32)).is_err());
    wrong = receipt.clone();
    wrong["completion"]["result"] = json!({"unverified_value":42});
    assert!(validate(&wrong, &prepared, &"1".repeat(32)).is_err());
}

#[test]
fn malformed_attempt_and_task_fail_before_network_access() {
    assert!(
        fetch("http://127.0.0.1:1", None, &json!({}), "bad")
            .unwrap_err()
            .message
            .contains("attempt identity")
    );
    assert!(fetch("http://127.0.0.1:1", None, &json!({}), &"1".repeat(32)).is_err());
}
