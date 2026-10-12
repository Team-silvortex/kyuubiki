use crate::workflow_executor::run_transform_operator;
use serde_json::{Value, json};

#[test]
fn native_sweep_scoring_obeys_shared_service_contract_fixtures() {
    let cases: Vec<Value> = serde_json::from_str(include_str!(
        "../../../../../../tests/fixtures/workflow-sweep-score-contract.json"
    ))
    .unwrap();
    let mut failures = Vec::new();
    for case in cases {
        let result = run_transform_operator(
            "transform.score_parameter_sweep",
            case["payload"].clone(),
            case["config"].clone(),
        );
        let matches = if case["error"] == true {
            result.is_err()
        } else {
            result
                .as_ref()
                .is_ok_and(|result| contains(result, &case["expected"]))
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
        Value::Number(_) => actual.as_f64() == expected.as_f64(),
        Value::Array(items) => actual.as_array().is_some_and(|values| {
            values.len() == items.len() && values.iter().zip(items).all(|(a, b)| contains(a, b))
        }),
        _ => actual == expected,
    }
}

#[test]
fn native_score_breakdown_retains_hard_limit_evidence_even_for_zero_weight() {
    let scored = score(
        json!([{"case_id":"a", "mass":3}]),
        json!([{"field":"mass", "weight":0, "max_allowed":2}]),
    )
    .unwrap();
    let term = &scored["scored_rows"][0]["objective_breakdown"][0];
    assert_eq!(term["feasible"], false);
    assert_eq!(term["max_allowed"], 2.0);
    assert_eq!(term["score"], 0.0);
}

#[test]
fn native_score_validates_all_objectives_before_rows() {
    for objective in [
        json!({"field":""}),
        json!({"field":"mass", "goal":null}),
        json!({"field":"mass", "min_allowed":"0"}),
    ] {
        assert!(score(json!([{"mass":1}]), json!([objective])).is_err());
    }
}

#[test]
fn native_score_normalizes_signed_zero_ties_by_exact_identity() {
    let result = score(
        json!([{"case_id":"z", "mass":-0.0}, {"case_id":"a", "mass":0.0}]),
        json!([{"field":"mass", "goal":"max"}]),
    )
    .unwrap();
    assert_eq!(result["best"]["case_id"], "a");
}

fn score(rows: Value, objectives: Value) -> Result<Value, String> {
    run_transform_operator(
        "transform.score_parameter_sweep",
        json!({"rows":rows}),
        json!({"objectives":objectives}),
    )
}
