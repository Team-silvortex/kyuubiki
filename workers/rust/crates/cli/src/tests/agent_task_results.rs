use super::*;

fn identity(n: usize) -> Identity {
    Identity {
        attempt_id: format!("{n:032x}"),
        request_id: format!("request-{n}"),
        task_id: "task".into(),
        task_digest: "a".repeat(64),
        operator_id: "op".into(),
        program_id: "op".into(),
    }
}

#[test]
fn exact_attempt_identity_and_generation_fence_late_and_duplicate_results() {
    let now = Instant::now();
    let mut store = Store::default();
    let id = identity(1);
    store.begin(id.clone(), 1, now);
    assert_eq!(store.lookup(&id, now).0, "pending");
    store.finish(&id, 2, Some(vec![1]), now);
    assert_eq!(store.lookup(&id, now).0, "pending");
    store.finish(&id, 1, Some(vec![1, 2]), now);
    assert_eq!(store.lookup(&id, now).0, "receipt_retained");
    let mut wrong = id.clone();
    wrong.task_digest = "b".repeat(64);
    assert_eq!(store.lookup(&wrong, now).0, "not_retained");
    wrong = id.clone();
    wrong.request_id = "other".into();
    assert_eq!(store.lookup(&wrong, now).0, "not_retained");
    store.begin(id.clone(), 2, now);
    assert_eq!(store.bytes, 0);
    store.finish(&id, 1, Some(vec![9]), now);
    store.finish(&id, 2, Some(vec![8]), now);
    let ambiguous = store.lookup(&id, now);
    assert_eq!(ambiguous.0, "attempt_identity_ambiguous");
    assert!(ambiguous.2.is_none());
}

#[test]
fn expiry_pressure_and_process_restart_never_return_completion_proof() {
    let now = Instant::now();
    let mut store = Store::default();
    for n in 0..=MAX_ENTRIES {
        store.begin(identity(n), 1, now);
    }
    assert_eq!(store.entries.len(), MAX_ENTRIES);
    assert_eq!(store.lookup(&identity(0), now).0, "not_retained");
    store.finish(&identity(0), 1, Some(vec![1]), now);
    assert_eq!(store.bytes, 0);
    assert_eq!(store.lookup(&identity(1), now + TTL).0, "not_retained");
    assert!(store.entries.is_empty());
    assert_eq!(Store::default().lookup(&identity(1), now).0, "not_retained");
}

#[test]
fn reading_a_receipt_never_renews_expiry_or_count_eviction_order() {
    let now = Instant::now();
    let mut store = Store::default();
    store.begin(identity(0), 1, now);
    store.finish(&identity(0), 1, Some(vec![1]), now);
    assert_eq!(
        store
            .lookup(&identity(0), now + TTL - Duration::from_nanos(1))
            .0,
        "receipt_retained"
    );
    assert_eq!(store.lookup(&identity(0), now + TTL).0, "not_retained");
    assert_eq!(store.bytes, 0);

    for n in 0..MAX_ENTRIES {
        store.begin(identity(n), 1, now + TTL);
        store.finish(&identity(n), 1, Some(vec![1]), now + TTL);
    }
    assert_eq!(store.lookup(&identity(0), now + TTL).0, "receipt_retained");
    store.begin(identity(MAX_ENTRIES), 1, now + TTL);
    assert_eq!(store.lookup(&identity(0), now + TTL).0, "not_retained");
    assert_eq!(store.bytes, MAX_ENTRIES - 1);
}

#[test]
fn computation_finishing_after_reservation_expiry_cannot_resurrect_its_result() {
    let now = Instant::now();
    let mut store = Store::default();
    let id = identity(0);
    store.begin(id.clone(), 1, now);
    store.finish(&id, 1, Some(vec![1]), now + TTL);
    let result = store.lookup(&id, now + TTL);
    assert_eq!(result.0, "not_retained");
    assert!(result.1.is_none());
    assert!(result.2.is_none());
    assert!(store.entries.is_empty());
    assert_eq!(store.bytes, 0);
}

#[test]
fn encoded_payload_budget_is_enforced_without_unbounded_serialization() {
    let response = RpcResponse::success("request", json!({"payload":"x".repeat(MAX_ENTRY_BYTES)}));
    assert!(encode_bounded(&response).is_err());
    let now = Instant::now();
    let mut store = Store::default();
    for n in 0..5 {
        store.begin(identity(n), 1, now);
        store.finish(&identity(n), 1, Some(vec![0; MAX_ENTRY_BYTES]), now);
        assert!(store.bytes <= MAX_TOTAL_BYTES);
    }
    assert_eq!(store.lookup(&identity(0), now).0, "not_retained");
    store.begin(identity(6), 1, now);
    store.finish(&identity(6), 1, None, now);
    assert_eq!(store.lookup(&identity(6), now).0, "result_not_retained");
}

#[test]
fn queries_are_exact_and_retention_does_not_touch_execution_or_delivery() {
    let query = json!({"attempt_id":"1".repeat(32),"request_id":"request", "task_id":"task",
        "task_digest":"a".repeat(64), "operator_id":"op", "program_id":"op"});
    let identity: Identity = serde_json::from_value(query.clone()).unwrap();
    assert!(identity.valid());
    let mut wrong = query.clone();
    wrong["host"] = json!("127.0.0.1");
    assert!(serde_json::from_value::<Identity>(wrong).is_err());
    let request = RpcRequest {
        rpc_version: 1,
        id: "read-only".into(),
        method: kyuubiki_protocol::RpcMethod::FetchOperatorTaskResult,
        params: query,
    };
    let crate::transport::AgentReply::Stream(_, response) = handle_fetch(request);
    let result = response.result.unwrap();
    assert_eq!(result["status"], "not_retained");
    assert_eq!(result["automatic_replay_authorized"], false);
    assert_eq!(policy()["storage"], "process_memory_only");
}

#[test]
fn retention_policy_matches_the_exchange_contract_and_rpc_descriptor() {
    let schema: Value = serde_json::from_str(include_str!(
        "../../../../../../schemas/agent-task-result-retention.schema.json"
    ))
    .unwrap();
    let expected = &schema["$defs"]["policy"]["properties"];
    for (key, value) in policy().as_object().unwrap() {
        assert_eq!(value, &expected[key]["const"], "{key}");
    }
    let parsed: kyuubiki_protocol::RpcMethod =
        serde_json::from_value(json!("fetch_operator_task_result")).unwrap();
    assert!(
        kyuubiki_protocol::AgentDescriptor::solver_agent_default()
            .protocol
            .methods
            .contains(&parsed)
    );
}
