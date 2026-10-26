use super::*;

fn nested(depth: usize) -> Value {
    (0..depth).fold(json!("leaf"), |value, n| {
        if n % 2 == 0 {
            json!([value])
        } else {
            json!({"child":value})
        }
    })
}

fn response_with_depth(depth: usize, failure: bool) -> RpcResponse {
    if failure {
        let mut response = RpcResponse::error("request", "worker_failed", "worker failed");
        response.error.as_mut().unwrap().details = Some(nested(depth));
        response
    } else {
        RpcResponse::success("request", nested(depth))
    }
}

#[test]
fn serializable_but_unreadable_results_are_not_retained() {
    for failure in [false, true] {
        let store = Arc::new(Mutex::new(Store::default()));
        let response = response_with_depth(140, failure);
        let original = serde_json::to_vec(&response).unwrap();
        assert!(original.len() < MAX_ENTRY_BYTES);
        assert!(serde_json::from_slice::<Value>(&original).is_err());
        let owner = reservation(&store, identity(1), 1);
        finish(Some(owner), &response);
        let (status, generation, encoded) =
            store.lock().unwrap().lookup(&identity(1), Instant::now());
        assert_eq!(status, "result_not_retained", "failure={failure}");
        assert_eq!(generation, Some(1));
        assert!(encoded.is_none());
        assert_eq!(store.lock().unwrap().bytes, 0);
        assert_eq!(serde_json::to_vec(&response).unwrap(), original);
    }
}

#[test]
fn retention_reserves_depth_for_the_fetch_reply_envelope() {
    for failure in [false, true] {
        let mut found_boundary = false;
        for depth in 120..130 {
            let response = response_with_depth(depth, failure);
            let original = serde_json::to_vec(&response).unwrap();
            let wrapped = RpcResponse::success("fetch", json!({"response":response}));
            let wire = serde_json::to_vec(&wrapped).unwrap();
            if serde_json::from_slice::<Value>(&original).is_ok()
                && serde_json::from_slice::<Value>(&wire).is_err()
            {
                found_boundary = true;
                assert!(
                    encode_bounded(&response).is_err(),
                    "depth={depth}, failure={failure}"
                );
            }
        }
        assert!(found_boundary);
    }
}

#[test]
fn non_object_receipts_cannot_claim_retention() {
    for response in [
        Value::Null,
        json!([]),
        json!(true),
        json!("receipt"),
        json!(1),
    ] {
        let store = Arc::new(Mutex::new(Store::default()));
        let owner = reservation(&store, identity(1), 1);
        finish(Some(owner), &response);
        let (status, _, encoded) = store.lock().unwrap().lookup(&identity(1), Instant::now());
        assert_eq!(status, "result_not_retained", "{response}");
        assert!(encoded.is_none());
    }
}

fn fetch(cache: &Mutex<Store>, id: &Identity) -> RpcResponse {
    fetch_response(
        RpcRequest {
            rpc_version: 1,
            id: "fetch".into(),
            method: kyuubiki_protocol::RpcMethod::FetchOperatorTaskResult,
            params: json!({
                "attempt_id":id.attempt_id, "request_id":id.request_id,
                "task_id":id.task_id, "task_digest":id.task_digest,
                "operator_id":id.operator_id, "program_id":id.program_id
            }),
        },
        cache,
    )
}

#[test]
fn complete_fetch_reply_is_readable_at_both_success_and_failure_depth_boundaries() {
    for failure in [false, true] {
        let mut accepted = 0;
        let mut rejected = 0;
        for depth in 120..140 {
            let cache = Arc::new(Mutex::new(Store::default()));
            let id = identity(1);
            let response = response_with_depth(depth, failure);
            let minimal = RpcResponse::success("fetch", json!({"response":response}));
            let minimal_wire = serde_json::to_vec(&minimal).unwrap();
            let readable = serde_json::from_slice::<Value>(&minimal_wire).is_ok();
            finish(Some(reservation(&cache, id.clone(), 1)), &response);
            let reply = fetch(&cache, &id);
            assert!(reply.ok);
            let wire = serde_json::to_vec(&reply).unwrap();
            let parsed: RpcResponse = serde_json::from_slice(&wire).unwrap();
            assert_eq!(parsed, reply);
            let parsed: Value = serde_json::from_slice(&wire).unwrap();
            let result = &parsed["result"];
            assert_eq!(result["automatic_replay_authorized"], false);
            if readable {
                accepted += 1;
                assert_eq!(result["status"], "receipt_retained", "depth={depth}");
                assert_eq!(result["response"], serde_json::to_value(response).unwrap());
            } else {
                rejected += 1;
                assert_eq!(result["status"], "result_not_retained", "depth={depth}");
                assert!(result["response"].is_null());
            }
        }
        assert!(accepted > 0 && rejected > 0);
    }
}

#[test]
fn unexpected_cache_decode_failure_never_returns_a_retained_null_success() {
    let deep = serde_json::to_vec(&response_with_depth(140, false)).unwrap();
    for (status, encoded) in [
        ("receipt_retained", Some(b"{".to_vec())),
        ("receipt_retained", Some(b"null".to_vec())),
        ("receipt_retained", Some(b"[]".to_vec())),
        ("receipt_retained", Some(deep)),
        ("receipt_retained", None),
        ("pending", Some(b"{}".to_vec())),
        ("result_not_retained", Some(b"{}".to_vec())),
    ] {
        let cache = Arc::new(Mutex::new(Store::default()));
        let id = identity(1);
        // Fault injection below the private, checked writer, not a claim that
        // normal callers can mutate the immutable response bytes in memory.
        {
            let mut store = cache.lock().unwrap();
            store.begin(id.clone(), 1, Instant::now());
            store.finish(&id, 1, encoded, Instant::now());
            store.entries[0].status = status;
        }
        let peer = RpcResponse::success("request-2", json!({"value":42}));
        finish(Some(reservation(&cache, identity(2), 2)), &peer);
        let before = {
            let store = cache.lock().unwrap();
            (
                store.bytes,
                store.entries[0].created,
                store.entries[0].response.clone(),
            )
        };
        for _ in 0..2 {
            let reply = fetch(&cache, &id);
            assert!(!reply.ok);
            assert!(reply.result.is_none());
            assert_eq!(
                reply.error.unwrap().code,
                "task_result_retention_unavailable"
            );
            let healthy = fetch(&cache, &identity(2)).result.unwrap();
            assert_eq!(healthy["status"], "receipt_retained");
            assert_eq!(healthy["response"], serde_json::to_value(&peer).unwrap());
        }
        let store = cache.lock().unwrap();
        assert_eq!(store.bytes, before.0);
        assert_eq!(store.entries[0].created, before.1);
        assert_eq!(store.entries[0].response, before.2);
        assert_eq!(store.entries[0].status, status);
    }
}

#[test]
fn readable_payloads_preserve_float_bits_nulls_extensions_and_failure_details() {
    let numbers = [
        -0.0,
        f64::from_bits(1),
        f64::MIN_POSITIVE,
        f64::MAX,
        -f64::MAX,
    ];
    let payload = json!({
        "numbers":numbers, "nullable":null, "extension":{"items":[null, [], {}]},
        "text":"escaped \"value\"\\\n\u{2603}"
    });
    let mut failure = RpcResponse::error("request", "worker_failed", "worker failed");
    failure.error.as_mut().unwrap().details = Some(payload.clone());
    for response in [RpcResponse::success("request", payload), failure] {
        let cache = Arc::new(Mutex::new(Store::default()));
        finish(Some(reservation(&cache, identity(1), 1)), &response);
        let reply = fetch(&cache, &identity(1));
        let wire = serde_json::to_vec(&reply).unwrap();
        let decoded: Value = serde_json::from_slice(&wire).unwrap();
        let recovered = &decoded["result"]["response"];
        assert_eq!(*recovered, serde_json::to_value(&response).unwrap());
        let payload = if response.ok {
            &recovered["result"]
        } else {
            &recovered["error"]["details"]
        };
        for (index, number) in numbers.iter().enumerate() {
            assert_eq!(
                payload["numbers"][index].as_f64().unwrap().to_bits(),
                number.to_bits()
            );
        }
        assert_eq!(decoded["result"]["automatic_replay_authorized"], false);
    }
}

#[test]
fn shallow_receipts_keep_the_exact_encoded_byte_budget() {
    let mut response = RpcResponse::success("request", json!({"payload":""}));
    let overhead = serde_json::to_vec(&response).unwrap().len();
    response.result.as_mut().unwrap()["payload"] = json!("x".repeat(MAX_ENTRY_BYTES - overhead));
    let encoded = encode_bounded(&response).unwrap();
    assert_eq!(encoded.len(), MAX_ENTRY_BYTES);
    assert_eq!(
        decode_response(&encoded).unwrap(),
        serde_json::to_value(&response).unwrap()
    );
    response.result.as_mut().unwrap()["payload"] =
        json!("x".repeat(MAX_ENTRY_BYTES - overhead + 1));
    assert!(encode_bounded(&response).is_err());
}

#[test]
fn ordinary_unavailable_states_remain_successful_unknown_queries() {
    let cache = Arc::new(Mutex::new(Store::default()));
    let owner = reservation(&cache, identity(1), 1);
    for (id, status) in [(identity(1), "pending"), (identity(2), "not_retained")] {
        let reply = fetch(&cache, &id);
        assert!(reply.ok);
        let result = reply.result.unwrap();
        assert_eq!(result["status"], status);
        assert!(result["response"].is_null());
    }
    drop(owner);
    assert_eq!(
        fetch(&cache, &identity(1)).result.unwrap()["status"],
        "result_not_retained"
    );
}
