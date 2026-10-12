use crate::workflow_executor::run_transform_operator;
use serde_json::{Value, json};

#[test]
fn native_collection_obeys_shared_service_contract_fixtures() {
    let cases: Vec<Value> = serde_json::from_str(include_str!(
        "../../../../../../tests/fixtures/workflow-sweep-collection-contract.json"
    ))
    .unwrap();
    let mut failures = Vec::new();
    for case in cases {
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
                .is_ok_and(|value| contains(value, &case["expected"]))
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

#[test]
fn fresh_join_clears_prior_custom_summary_without_changing_the_input() {
    for received in [false, true] {
        let payload = json!({"joined_summary_field":"old_quality", "cases":[{
            "id":"a", "old_quality":{"mass":0}, "summary":{"mass":0}, "result":{"mass":0},
            "result_error":"old failure", "metadata":{"round":1}, "parameters":{"x":2}
        }], "results":if received {json!([{"case_id":"a","summary":{"mass":5}}])} else {json!([])}});
        let joined = run_transform_operator(
            "transform.join_parameter_sweep_results",
            payload.clone(),
            json!({"output_field":"fresh"}),
        )
        .unwrap();
        let case = &joined["cases"][0];
        for field in ["old_quality", "summary", "result", "result_error"] {
            assert!(case.get(field).is_none(), "{field}: {joined}");
        }
        assert_eq!(case["metadata"], payload["cases"][0]["metadata"]);
        assert_eq!(case["parameters"], payload["cases"][0]["parameters"]);
        assert_eq!(payload["cases"][0]["old_quality"]["mass"], 0);
        let summary =
            run_transform_operator("transform.summarize_parameter_sweep", joined, Value::Null);
        if received {
            assert_eq!(summary.unwrap()["rows"][0]["mass"], 5);
        } else {
            assert!(summary.is_err());
        }
    }
}

#[test]
fn native_collection_rejects_every_reserved_metric_even_when_selected_explicitly() {
    for field in [
        "id",
        "case_id",
        "caseId",
        "model",
        "parameters",
        "metadata",
        "result_status",
        "result_error",
        "objective_score",
        "objective_feasible",
        "objective_breakdown",
    ] {
        let payload = json!({"cases":[{"id":"a","summary":{field:1}}]});
        assert!(
            run_transform_operator(
                "transform.summarize_parameter_sweep",
                payload,
                json!({"fields":[field],"include_metadata":false,"include_parameters":false})
            )
            .is_err(),
            "{field}"
        );
    }
}

#[test]
fn nonconverged_join_diagnostics_cannot_reach_summary_or_scoring() {
    let payload = json!({"cases":[{"id":"a"}],"results":[{
        "case_id":"a","summary":{"mass":1,"converged":false,"achieved_load_factor":0.5}
    }]});
    assert!(
        run_transform_operator(
            "transform.join_parameter_sweep_results",
            payload.clone(),
            json!({"strict":true})
        )
        .is_err()
    );
    let joined = run_transform_operator(
        "transform.join_parameter_sweep_results",
        payload,
        Value::Null,
    )
    .unwrap();
    assert!(
        joined["rejected_results"][0]["reason"]
            .as_str()
            .unwrap()
            .contains("nonconverged")
    );
    assert!(
        run_transform_operator(
            "transform.summarize_parameter_sweep",
            joined,
            json!({"fields":["mass"],"fail_on_missing":false})
        )
        .is_err()
    );
}
