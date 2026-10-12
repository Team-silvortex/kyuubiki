use crate::workflow_executor::run_transform_operator;
use serde_json::Value;

pub(super) fn fixtures() -> Vec<Value> {
    serde_json::from_str(include_str!(
        "../../../../../../tests/fixtures/workflow-quality-sweep-expansion-admission.json"
    ))
    .unwrap()
}

#[test]
fn shared_expansion_admission_preserves_quality_evidence_before_case_generation() {
    let mut failures = Vec::new();
    for case in fixtures() {
        let payload =
            serde_json::from_slice(&serde_json::to_vec(&case["payload"]).unwrap()).unwrap();
        let result = run_transform_operator(
            "transform.expand_parameter_sweep",
            payload,
            case["config"].clone(),
        );
        let valid = if case["error"] == true {
            result.is_err()
        } else {
            result.as_ref().is_ok_and(|value| {
                case["expected"]
                    .as_object()
                    .unwrap()
                    .iter()
                    .all(|(key, expected)| value.get(key) == Some(expected))
            })
        };
        if !valid {
            failures.push(format!("{}: {result:?}", case["id"]));
        }
    }
    assert!(
        failures.is_empty(),
        "{} failures:\n{}",
        failures.len(),
        failures.join("\n")
    );
}
