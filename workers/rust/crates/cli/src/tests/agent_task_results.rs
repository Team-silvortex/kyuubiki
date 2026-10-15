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

fn reservation(store: &Arc<Mutex<Store>>, id: Identity, generation: u64) -> Reservation {
    store
        .lock()
        .unwrap()
        .begin(id.clone(), generation, Instant::now());
    Reservation {
        identity: id,
        generation,
        store: Arc::clone(store),
        finished: false,
    }
}

#[test]
fn abandoned_reservation_clears_pending_without_inventing_a_receipt() {
    let store = Arc::new(Mutex::new(Store::default()));
    let id = identity(1);
    let owner = reservation(&store, id.clone(), 1);
    drop(owner);
    let (status, generation, response) = store.lock().unwrap().lookup(&id, Instant::now());
    assert_eq!(status, "result_not_retained");
    assert_eq!(generation, Some(1));
    assert!(response.is_none());
    assert_eq!(store.lock().unwrap().bytes, 0);
}

#[test]
fn unwind_before_response_cleans_up_only_its_own_pending_reservation() {
    let store = Arc::new(Mutex::new(Store::default()));
    let other = reservation(&store, identity(2), 2);
    let unwound = std::panic::catch_unwind(|| {
        let _owner = reservation(&store, identity(1), 1);
        panic!("before a task result exists");
    });
    assert!(unwound.is_err());
    assert_eq!(
        store.lock().unwrap().lookup(&identity(1), Instant::now()).0,
        "result_not_retained"
    );
    assert_eq!(
        store.lock().unwrap().lookup(&identity(2), Instant::now()).0,
        "pending"
    );
    drop(other);
}

#[test]
fn abandoned_owner_cannot_change_replaced_ambiguous_expired_or_terminal_entries() {
    for scenario in [
        "generation",
        "identity",
        "ambiguous",
        "expired",
        "evicted",
        "terminal",
    ] {
        let store = Arc::new(Mutex::new(Store::default()));
        let id = identity(1);
        let owner = reservation(&store, id.clone(), 1);
        let expected = {
            let mut store = store.lock().unwrap();
            match scenario {
                "generation" => store.entries[0].generation = 2,
                "identity" => store.entries[0].identity.task_digest = "b".repeat(64),
                "ambiguous" => store.begin(id.clone(), 2, Instant::now()),
                "expired" => store.entries[0].created = Instant::now() - TTL,
                "evicted" => store.remove_first(),
                "terminal" => store.finish(&id, 1, Some(vec![1, 2]), Instant::now()),
                _ => unreachable!(),
            }
            store.prune(Instant::now());
            (
                store.entries.front().map(|e| {
                    (
                        e.identity.clone(),
                        e.generation,
                        e.status,
                        e.response.clone(),
                    )
                }),
                store.bytes,
            )
        };
        drop(owner);
        let store = store.lock().unwrap();
        let actual = (
            store.entries.front().map(|e| {
                (
                    e.identity.clone(),
                    e.generation,
                    e.status,
                    e.response.clone(),
                )
            }),
            store.bytes,
        );
        assert_eq!(actual, expected, "{scenario}");
    }
}

#[test]
fn committed_json_receipts_keep_the_same_bounds_and_survive_reservation_drop() {
    let store = Arc::new(Mutex::new(Store::default()));
    for (n, response) in [
        RpcResponse::success("request", json!({"value":1})),
        RpcResponse::error("request", "failed", "worker failed"),
        RpcResponse::success("request", json!({"payload":"x".repeat(MAX_ENTRY_BYTES)})),
    ]
    .into_iter()
    .enumerate()
    {
        let owner = reservation(&store, identity(n), n as u64);
        let committed = serde_json::to_value(response).unwrap();
        finish(Some(owner), &committed);
        let (status, _, encoded) = store.lock().unwrap().lookup(&identity(n), Instant::now());
        if n == 2 {
            assert_eq!(status, "result_not_retained");
            assert!(encoded.is_none());
        } else {
            assert_eq!(status, "receipt_retained");
            assert_eq!(
                serde_json::from_slice::<Value>(&encoded.unwrap()).unwrap(),
                committed
            );
        }
    }
}
