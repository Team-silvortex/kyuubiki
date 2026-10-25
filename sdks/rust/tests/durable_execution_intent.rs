use kyuubiki_headless_sdk::{DurableExecutionIntent, SolverRpcClient};
use serde_json::{Value, json};

fn prepare(params: &Value) -> DurableExecutionIntent {
    DurableExecutionIntent::new(
        &"b".repeat(32),
        &"a".repeat(32),
        "solve_modal_frame_2d",
        params,
    )
    .unwrap()
}

#[test]
fn standalone_sdk_matches_agent_protocol_digest_vectors_after_json_readback() {
    let vectors: Vec<Value> =
        serde_json::from_str(include_str!("../fixtures/durable-execution-digests.json")).unwrap();
    for case in vectors {
        let method = case["method"].as_str().unwrap();
        let params: Value =
            serde_json::from_slice(&serde_json::to_vec(&case["params"]).unwrap()).unwrap();
        let intent =
            DurableExecutionIntent::new(&"b".repeat(32), &"a".repeat(32), method, &params).unwrap();
        assert_eq!(intent.request_digest, case["digest"], "{}", case["name"]);
        let retained: DurableExecutionIntent =
            serde_json::from_slice(&serde_json::to_vec(&intent).unwrap()).unwrap();
        retained.validate_input(&params).unwrap();
        assert_eq!(retained, intent);
    }
}

#[test]
fn changed_input_and_invalid_intent_fail_before_network_access() {
    let client = SolverRpcClient::new("invalid.invalid", 1);
    let params = json!({"x":1.0});
    let intent = prepare(&params);
    for changed in [
        json!({"x":1}),
        json!({"x":1.0000000000000002}),
        json!({"x":0.0}),
    ] {
        assert!(
            client
                .submit_prepared_durable_agent_task(&intent, changed)
                .unwrap_err()
                .to_string()
                .contains("input changed")
        );
    }
    for (field, value) in [
        ("schema_version", "unknown"),
        ("method", "ping"),
        ("store_id", "../store"),
        ("execution_id", "invalid"),
        ("request_digest", "bad"),
    ] {
        let mut document = serde_json::to_value(&intent).unwrap();
        document[field] = json!(value);
        let invalid: DurableExecutionIntent = serde_json::from_value(document).unwrap();
        let error = client
            .inspect_durable_agent_task(&invalid)
            .unwrap_err()
            .to_string();
        assert!(error.contains("intent") || error.contains("hex"), "{error}");
    }
}

#[test]
fn digest_guards_recursion_and_task_ir_execute_mode() {
    let mut deep = json!(null);
    for _ in 0..130 {
        deep = json!({"x":deep});
    }
    assert!(
        DurableExecutionIntent::new(&"b".repeat(32), &"a".repeat(32), "solve_bar_1d", &deep)
            .unwrap_err()
            .to_string()
            .contains("depth")
    );
    for params in [json!({}), json!({"mode":"plan"}), json!([])] {
        assert!(
            DurableExecutionIntent::new(
                &"b".repeat(32),
                &"a".repeat(32),
                "run_operator_task_ir",
                &params
            )
            .is_err()
        );
    }
}
