use crate::workflow_executor::run_transform_operator;
use serde_json::Value;

#[test]
fn native_quality_composition_matches_shared_contract() {
    check("compose_");
}

#[test]
fn native_quality_ranking_matches_shared_contract() {
    check("rank_");
}

#[test]
fn native_quality_next_round_matches_shared_contract() {
    check("next_");
}

fn check(prefix: &str) {
    let cases: Vec<Value> = serde_json::from_str(include_str!(
        "../../../../../../tests/fixtures/workflow-quality-decision-contract.json"
    ))
    .unwrap();
    let mut failures = Vec::new();
    for case in cases
        .iter()
        .filter(|case| case["id"].as_str().unwrap().starts_with(prefix))
    {
        let result = run_transform_operator(
            case["operator"].as_str().unwrap(),
            case["payload"].clone(),
            case["config"].clone(),
        );
        let matches = if case["error"] == true {
            result.is_err()
        } else {
            result
                .as_ref()
                .is_ok_and(|actual| contains(actual, &case["expected"]))
        };
        if !matches {
            failures.push(format!("{}: {result:?}", case["id"]));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

fn contains(actual: &Value, expected: &Value) -> bool {
    match expected {
        Value::Object(fields) => fields
            .iter()
            .all(|(key, value)| actual.get(key).is_some_and(|got| contains(got, value))),
        Value::Array(items) => actual.as_array().is_some_and(|values| {
            values.len() == items.len() && values.iter().zip(items).all(|(a, b)| contains(a, b))
        }),
        Value::Number(_) => actual.as_f64() == expected.as_f64(),
        _ => actual == expected,
    }
}
