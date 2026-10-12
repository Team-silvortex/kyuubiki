use crate::workflow_executor::run_transform_operator;
use serde_json::Value;

pub(super) fn fixtures() -> Vec<Value> {
    serde_json::from_str(include_str!(
        "../../../../../../tests/fixtures/workflow-quality-sweep-plan-admission.json"
    ))
    .unwrap()
}

#[test]
fn shared_direct_plan_admission_rejects_incoherent_evidence_and_counts() {
    let mut failures = Vec::new();
    for case in fixtures() {
        let payload =
            serde_json::from_slice(&serde_json::to_vec(&case["payload"]).unwrap()).unwrap();
        let result = run_transform_operator(
            "transform.materialize_quality_sweep_expansion",
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
