use crate::{
    OPERATOR_TASK_DIGEST_FIELDS, OperatorTaskDigestError, TASK_DIGEST_FIXED15_V1,
    TASK_DIGEST_TYPED_V2, compute_operator_task_digest, seal_operator_task_digest,
    verify_operator_task_digest,
};
use serde_json::{Value, json};

fn golden() -> Value {
    serde_json::from_str(include_str!(
        "../../../../../../schemas/examples.operator-task-ir.json"
    ))
    .unwrap()
}

#[test]
fn typed_digest_matches_shared_cross_language_vectors_after_json_roundtrip() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../../../../schemas/examples.task-ir-typed-digest.json"
    ))
    .unwrap();
    for case in fixture["cases"].as_array().unwrap() {
        let mut task = json!({"input_artifact":case["value"]});
        seal_operator_task_digest(&mut task, TASK_DIGEST_TYPED_V2).unwrap();
        assert_eq!(
            task["integrity"]["task_digest"], case["task_digest"],
            "{case}"
        );
        let roundtrip: Value = serde_json::from_slice(&serde_json::to_vec(&task).unwrap()).unwrap();
        verify_operator_task_digest(&roundtrip).unwrap();
    }
}

#[test]
fn typed_digest_preserves_shared_generated_binary64_corpus_through_json() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../../../../schemas/examples.task-ir-typed-digest.json"
    ))
    .unwrap();
    let corpus = &fixture["generated_binary64"];
    let mut state = u64::from_str_radix(corpus["seed_hex"].as_str().unwrap(), 16).unwrap();
    let finite_mask = u64::from_str_radix(corpus["finite_mask_hex"].as_str().unwrap(), 16).unwrap();
    let values = (0..corpus["count"].as_u64().unwrap())
        .map(|_| {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            json!(f64::from_bits(state & finite_mask))
        })
        .collect::<Vec<_>>();
    let mut task = json!({"input_artifact":values});
    seal_operator_task_digest(&mut task, TASK_DIGEST_TYPED_V2).unwrap();
    assert_eq!(task["integrity"]["task_digest"], corpus["task_digest"]);
    let decoded: Value = serde_json::from_slice(&serde_json::to_vec(&task).unwrap()).unwrap();
    verify_operator_task_digest(&decoded).unwrap();
}

#[test]
fn typed_digest_covers_every_projected_field_and_distinguishes_numeric_types() {
    let pairs = [
        (json!(1e-30), json!(2e-30)),
        (json!(1e-300), json!(2e-300)),
        (json!(0.0), json!(-0.0)),
        (json!(1), json!(1.0)),
        (json!(0.3333333333333333), json!(0.33333333333333337)),
        (json!(5e-324), json!(1e-323)),
    ];
    for field in OPERATOR_TASK_DIGEST_FIELDS {
        for (a, b) in &pairs {
            let mut task = golden();
            task[*field] = json!({"nested":[{"value":a}]});
            seal_operator_task_digest(&mut task, TASK_DIGEST_TYPED_V2).unwrap();
            let mut changed = task.clone();
            changed[*field] = json!({"nested":[{"value":b}]});
            assert!(
                matches!(
                    verify_operator_task_digest(&changed),
                    Err(OperatorTaskDigestError::Mismatch { .. })
                ),
                "{field}: {a} -> {b}"
            );
        }
    }
}

#[test]
fn typed_digest_encoding_cannot_be_removed_downgraded_or_forged() {
    let mut task = golden();
    seal_operator_task_digest(&mut task, TASK_DIGEST_TYPED_V2).unwrap();
    for replacement in [
        json!(TASK_DIGEST_FIXED15_V1),
        json!("future"),
        Value::Null,
        json!(false),
        json!([]),
        json!({}),
    ] {
        let mut changed = task.clone();
        changed["integrity"]["task_digest_encoding"] = replacement;
        assert!(verify_operator_task_digest(&changed).is_err());
    }
    let mut changed = task.clone();
    changed["integrity"]
        .as_object_mut()
        .unwrap()
        .remove("task_digest_encoding");
    assert!(verify_operator_task_digest(&changed).is_err());
    for algorithm in [json!("sha512"), Value::Null, json!({})] {
        let mut changed = task.clone();
        changed["integrity"]["digest_algorithm"] = algorithm;
        assert!(verify_operator_task_digest(&changed).is_err());
    }
}

#[test]
fn typed_digest_projection_order_and_legacy_defaults_are_stable() {
    let mut task = golden();
    let old_digest = compute_operator_task_digest(&task).unwrap();
    verify_operator_task_digest(&task).unwrap();
    seal_operator_task_digest(&mut task, TASK_DIGEST_FIXED15_V1).unwrap();
    assert_eq!(compute_operator_task_digest(&task).unwrap(), old_digest);
    seal_operator_task_digest(&mut task, TASK_DIGEST_TYPED_V2).unwrap();
    let original = task.clone();
    let mut reversed = serde_json::Map::new();
    for (key, value) in task.as_object().unwrap().iter().rev() {
        reversed.insert(key.clone(), value.clone());
    }
    task = Value::Object(reversed);
    task["annotations"] = json!({"outside_execution_identity":true});
    task["integrity"]["task_digest_fields"] = json!(["config"]);
    verify_operator_task_digest(&task).unwrap();
    seal_operator_task_digest(&mut task, TASK_DIGEST_TYPED_V2).unwrap();
    assert_eq!(
        task["integrity"]["task_digest"],
        original["integrity"]["task_digest"]
    );
    assert_ne!(task["integrity"]["task_digest"], old_digest);
}

#[test]
fn typed_digest_rejects_excessive_depth_and_sealing_failure_is_atomic() {
    let mut value = json!(null);
    for _ in 0..129 {
        value = json!([value]);
    }
    let mut task = json!({"input_artifact":value});
    let original = task.clone();
    assert!(seal_operator_task_digest(&mut task, TASK_DIGEST_TYPED_V2).is_err());
    assert_eq!(task, original);
    let mut task = golden();
    let original = task.clone();
    assert!(seal_operator_task_digest(&mut task, "future").is_err());
    assert_eq!(task, original);
    task["integrity"] = Value::Null;
    assert!(seal_operator_task_digest(&mut task, TASK_DIGEST_TYPED_V2).is_err());
}
#[test]
fn agent_descriptor_explicitly_advertises_typed_digest_verification() {
    let descriptor = crate::AgentDescriptor::solver_agent_default();
    let capability = descriptor
        .capabilities
        .iter()
        .find(|capability| capability.id == crate::TASK_DIGEST_TYPED_V2)
        .expect("typed digest capability");
    assert_eq!(capability.role, "task_digest");
    assert!(
        capability
            .methods
            .contains(&crate::RpcMethod::RunOperatorTaskIr)
    );
    assert!(
        capability
            .methods
            .contains(&crate::RpcMethod::FetchOperatorTaskResult)
    );
}
