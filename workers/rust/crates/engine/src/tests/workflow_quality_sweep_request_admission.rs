use crate::workflow_executor::run_transform_operator;
use serde_json::{Value, json};

pub(super) fn fixtures() -> Vec<Value> {
    serde_json::from_str(include_str!(
        "../../../../../../tests/fixtures/workflow-quality-sweep-request-admission.json"
    ))
    .unwrap()
}

#[test]
fn shared_sweep_request_admission_rejects_masked_and_conflicting_intent() {
    let mut failures = Vec::new();
    for case in fixtures() {
        let input = serde_json::from_slice(&serde_json::to_vec(&case["payload"]).unwrap()).unwrap();
        let result = run_transform_operator(
            "transform.build_quality_parameter_sweep_plan",
            input,
            json!({"search_space":{"temperature":[1,2]}}),
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

#[test]
fn sweep_report_alias_matrix_retains_evidence_after_json_exchange() {
    let reports = [
        Value::Null,
        json!({"coupled_readiness_state":"ready"}),
        json!({"coupled_readiness_state":"warn"}),
        json!({"coupled_readiness_state":"block"}),
        json!({"coupled_readiness_ready":false}),
    ];
    for a in 0..5 {
        for b in 0..5 {
            for c in 0..5 {
                for d in 0..5 {
                    let distinct = [a, b, c, d]
                        .into_iter()
                        .filter(|i| *i != 0)
                        .collect::<std::collections::BTreeSet<_>>();
                    let selected = distinct.first().copied().unwrap_or(0);
                    let hint = match selected {
                        2 => json!({"action":"review_coupled_readiness"}),
                        3 | 4 => json!({"action":"fix_coupled_readiness"}),
                        _ => Value::Null,
                    };
                    let payload = json!({"action":if selected >= 3 {"replan"} else {"continue"},
                    "selected_coupled_readiness":reports[b],"coupled_readiness":reports[c],
                    "selected_iteration_hint":hint,"request_payload":{"coupled_readiness":reports[a],
                    "seed_objective":{"coupled_readiness":reports[d]},
                    "optimization_hint":hint,"search_space":{"temperature":[1,2]}}});
                    let payload =
                        serde_json::from_slice(&serde_json::to_vec(&payload).unwrap()).unwrap();
                    let result = run_transform_operator(
                        "transform.build_quality_parameter_sweep_plan",
                        payload,
                        json!({}),
                    );
                    if distinct.len() > 1 {
                        assert!(result.is_err(), "{a},{b},{c},{d}");
                    } else {
                        let plan = result.unwrap();
                        assert_eq!(plan["coupled_readiness"], reports[selected]);
                        let materialized = run_transform_operator(
                            "transform.materialize_quality_sweep_expansion",
                            plan,
                            json!({}),
                        )
                        .unwrap();
                        let cases = run_transform_operator(
                            "transform.expand_parameter_sweep",
                            materialized,
                            json!({}),
                        )
                        .unwrap();
                        assert_eq!(cases["case_count"], 2);
                        for case in cases["cases"].as_array().unwrap() {
                            assert_eq!(case["metadata"]["coupled_readiness"], reports[selected]);
                        }
                    }
                }
            }
        }
    }
}
