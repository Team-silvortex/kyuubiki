use crate::canonical_json;
use serde_json::Value;

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
