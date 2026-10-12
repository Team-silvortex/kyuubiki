use crate::workflow_executor::run_transform_operator;
use serde_json::{Value, json};

pub(super) fn fixtures() -> Vec<Value> {
    serde_json::from_str(include_str!(
        "../../../../../../tests/fixtures/workflow-sweep-expansion-counts.json"
    ))
    .unwrap()
}

#[test]
fn count_mirror_placements_accept_integral_floats_but_not_stale_or_null_values() {
    let source = fixtures()
        .into_iter()
        .find(|row| row["id"] == "wrapped_valid")
        .unwrap()["payload"]
        .clone();
    let choices = [
        None,
        Some(json!(2)),
        Some(json!(2.0)),
        Some(Value::Null),
        Some(json!(3)),
    ];
    for a in &choices {
        for b in &choices {
            for c in &choices {
                let mut payload = source.clone();
                for (path, value) in [
                    ("", a),
                    ("/payload", b),
                    ("/payload/case_metadata/sweep_budget", c),
                ] {
                    let target = payload.pointer_mut(path).unwrap().as_object_mut().unwrap();
                    target.remove("case_count_estimate");
                    if let Some(value) = value {
                        target.insert("case_count_estimate".into(), value.clone());
                    }
                }
                let valid = [a, b, c]
                    .iter()
                    .all(|value| value.as_ref().is_none_or(|v| v.as_f64() == Some(2.0)));
                let payload =
                    serde_json::from_slice(&serde_json::to_vec(&payload).unwrap()).unwrap();
                let result =
                    run_transform_operator("transform.expand_parameter_sweep", payload, json!({}));
                assert_eq!(result.is_ok(), valid, "{a:?}/{b:?}/{c:?}: {result:?}");
                if valid {
                    assert_eq!(result.unwrap()["case_count"], 2);
                }
            }
        }
    }
}

#[test]
fn corrupt_expansion_counts_stop_dependents_and_corrected_fresh_runs_recover() {
    use super::workflow_quality_sweep_expansion_recovery::graph;
    use crate::run_workflow_graph;
    use kyuubiki_protocol::WorkflowGraphRunRequest;
    use std::collections::BTreeMap;
    let fixtures = fixtures();
    let good = fixtures
        .iter()
        .find(|row| row["id"] == "wrapped_valid")
        .unwrap()["payload"]
        .clone();
    for id in [
        "raw_axis_stale",
        "wrapped_total_stale",
        "blocked_deferred_with_values",
        "blocked_metadata_budget_stale",
        "stopped_total_stale",
        "metadata_limit_too_small",
        "raw_budget_block",
        "diagnostic_count_conflict",
    ] {
        for policy in ["fail", "skip"] {
            let graph = graph(policy);
            let run = |payload| {
                run_workflow_graph(WorkflowGraphRunRequest {
                    graph: graph.clone(),
                    input_artifacts: BTreeMap::from([("quality_candidates".into(), payload)]),
                })
            };
            let result =
                run(fixtures.iter().find(|row| row["id"] == id).unwrap()["payload"].clone());
            if policy == "fail" {
                assert!(result.is_err(), "{id}");
            } else {
                let rejected = result.unwrap();
                assert_eq!(rejected.failed_nodes, ["expand_cases"], "{id}");
                assert_eq!(rejected.completed_nodes, ["quality_candidates"]);
                assert!(!rejected.artifacts.contains_key("expand_cases.cases"));
            }
            let fresh = run(good.clone()).unwrap();
            assert!(fresh.failed_nodes.is_empty());
            let result = &fresh.artifacts["expand_cases.cases"];
            assert_eq!(result["case_count"], 2);
            for case in result["cases"].as_array().unwrap() {
                assert_eq!(case["metadata"], good["payload"]["case_metadata"]);
            }
        }
    }
}

#[test]
fn generated_deferred_and_concrete_counts_stay_consistent_without_allocating_samples() {
    for limit in [1, 4] {
        let plan = run_transform_operator(
            "transform.build_quality_parameter_sweep_plan",
            json!({"request_payload":{"search_space":{"temperature_delta":[120,160]}}}),
            json!({"max_cases":limit}),
        )
        .unwrap();
        let expansion = run_transform_operator(
            "transform.materialize_quality_sweep_expansion",
            plan,
            json!({"max_cases":limit}),
        )
        .unwrap();
        for path in [
            "/case_count_estimate",
            "/sweep_budget/case_count_estimate",
            "/payload/case_metadata/sweep_budget/case_count_estimate",
            "/payload/axes/0/value_count",
        ] {
            let mut corrupt = expansion.clone();
            *corrupt.pointer_mut(path).unwrap() = json!(3);
            assert!(
                run_transform_operator(
                    "transform.expand_parameter_sweep",
                    corrupt,
                    json!({"max_cases":10})
                )
                .is_err(),
                "{limit}:{path}"
            );
        }
        let result = run_transform_operator(
            "transform.expand_parameter_sweep",
            expansion,
            json!({"max_cases":10}),
        )
        .unwrap();
        assert_eq!(result["case_count"], if limit == 1 { 0 } else { 2 });
    }
    let billion = fixtures()
        .into_iter()
        .find(|row| row["id"] == "deferred_valid")
        .unwrap();
    let body = &billion["payload"]["payload"];
    let plan = json!({"base":body["base"],"axes":body["axes"],"sweep_budget":{"case_budget_exceeded":true},"case_count_estimate":1_000_000_000});
    for values in [Value::Null, json!([]), json!([120, 160])] {
        let mut ambiguous = plan.clone();
        ambiguous["axes"][0]["values"] = values;
        assert!(
            run_transform_operator(
                "transform.materialize_quality_sweep_expansion",
                ambiguous,
                json!({})
            )
            .is_err()
        );
    }
    let expansion = run_transform_operator(
        "transform.materialize_quality_sweep_expansion",
        plan,
        json!({"max_cases":2_000_000_000}),
    )
    .unwrap();
    assert!(expansion["payload"]["axes"][0].get("values").is_none());
    assert_eq!(
        run_transform_operator(
            "transform.expand_parameter_sweep",
            expansion,
            json!({"max_cases":2_000_000_000})
        )
        .unwrap()["case_count"],
        0
    );
}

#[test]
fn shared_expansion_counts_reject_stale_and_ambiguous_declarations() {
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
            failures.push(format!(
                "{}: case_count={:?}, error={:?}",
                case["id"],
                result.as_ref().ok().map(|v| &v["case_count"]),
                result.as_ref().err()
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "{} expectation failures:\n{}",
        failures.len(),
        failures.join("\n")
    );
}
