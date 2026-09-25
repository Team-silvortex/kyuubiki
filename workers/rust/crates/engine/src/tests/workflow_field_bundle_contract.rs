use crate::workflow_executor::{run_extract_operator, run_transform_operator};
use serde_json::Value;

fn check(fixtures: &str) {
    let cases: Vec<Value> = serde_json::from_str(fixtures).unwrap();
    for case in cases {
        let id = case["id"].as_str().unwrap();
        let operator = case["operator"].as_str().unwrap();
        let run = if operator.starts_with("extract.") {
            run_extract_operator
        } else {
            run_transform_operator
        };
        let result = run(operator, case["payload"].clone(), case["config"].clone());
        if let Some(expected) = case["error_contains"].as_str() {
            let error = result.unwrap_err();
            assert!(
                error.contains(expected),
                "{id}: {error}, expected {expected}"
            );
        } else {
            let output = result.unwrap_or_else(|error| panic!("{id}: {error}"));
            subset(&output, &case["output"], id);
        }
    }
}

fn subset(actual: &Value, expected: &Value, id: &str) {
    if let Some(fields) = expected.as_object() {
        for (key, value) in fields {
            subset(
                actual
                    .get(key)
                    .unwrap_or_else(|| panic!("{id}: missing {key}")),
                value,
                id,
            );
        }
    } else if let (Some(actual), Some(expected)) = (actual.as_f64(), expected.as_f64()) {
        let scale = actual.abs().max(expected.abs());
        assert!(
            scale == 0.0 || (actual / scale - expected / scale).abs() <= 1e-12,
            "{id}: {actual} != {expected}"
        );
    } else {
        assert_eq!(actual, expected, "{id}");
    }
}

#[test]
fn shared_field_contract_matches_elixir_fixture_expectations() {
    check(include_str!(
        "../../../../../../tests/fixtures/workflow-field-contract.json"
    ));
}

#[test]
fn shared_bundle_contract_matches_elixir_fixture_expectations() {
    check(include_str!(
        "../../../../../../tests/fixtures/workflow-bundle-contract.json"
    ));
}
