use crate::{canonical_json, compute_operator_task_digest, verify_operator_task_digest};
use serde_json::{Value, json};

#[test]
fn task_ir_shared_float_fixture_preserves_fixed_15_rounding_and_signed_zero() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../../../../schemas/examples.task-ir-canonical-floats.json"
    ))
    .unwrap();
    for case in fixture["cases"].as_array().unwrap() {
        assert_eq!(
            canonical_json(&case["value"]),
            case["canonical"].as_str().unwrap(),
            "{case}"
        );
    }
}

#[test]
fn task_ir_legacy_digest_is_not_lossless_numeric_identity() {
    let task: Value = serde_json::from_str(include_str!(
        "../../../../../../schemas/examples.operator-task-ir.json"
    ))
    .unwrap();
    // Keep this limitation explicit until a versioned cross-authoring digest replaces v1.
    for field in ["input_artifact", "config"] {
        for (a, b) in [(1e-30, 2e-30), (1e-300, 2e-300)] {
            let mut original = task.clone();
            original[field] = json!({"nested":[{"value":a}]});
            let digest = compute_operator_task_digest(&original).unwrap();
            original["integrity"]["task_digest"] = json!(digest);
            let mut changed = original.clone();
            changed[field] = json!({"nested":[{"value":b}]});
            assert_ne!(original[field], changed[field]);
            assert_eq!(
                compute_operator_task_digest(&original).unwrap(),
                compute_operator_task_digest(&changed).unwrap()
            );
            assert!(verify_operator_task_digest(&changed).is_ok());
        }
    }
}
