use crate::workflow_executor::run_transform_operator;
use serde_json::{Value, json};

#[test]
fn keyed_results_never_fall_back_to_another_cases_array_position() {
    let joined = join(
        cases(),
        json!([{"case_id": "b", "summary": {"mass": 2}}]),
        Value::Null,
    )
    .unwrap();
    assert_eq!(joined["joined_summary_count"], 1);
    assert_eq!(joined["missing_case_ids"], json!(["a"]));
    assert_eq!(joined["join_complete"], false);
    assert!(joined["cases"][0].get("summary").is_none());
    assert_eq!(joined["cases"][1]["summary"]["mass"], 2);
    assert!(
        join(
            cases(),
            json!([{"case_id": "b", "summary": {"mass": 2}}]),
            json!({"strict": true})
        )
        .is_err()
    );
}

#[test]
fn unmatched_result_ids_are_reported_instead_of_borrowed() {
    let joined = join(
        json!([{"id": "a"}]),
        json!([
            {"case_id": "a", "summary": {"mass": 1}}, {"case_id": "other", "summary": {"mass": 2}}
        ]),
        Value::Null,
    )
    .unwrap();
    assert_eq!(joined["join_complete"], false);
    assert_eq!(joined["unmatched_result_ids"], json!(["other"]));
    assert_eq!(joined["unmatched_result_count"], 1);
    assert!(
        join(
            json!([{"id": "a"}]),
            json!([{"case_id": "other", "summary": {"mass": 0}}]),
            json!({"strict": true})
        )
        .is_err()
    );
}

#[test]
fn duplicate_cases_and_generated_id_collisions_are_rejected() {
    for cases in [
        json!([{"id": "a"}, {"id": "a"}]),
        json!([{}, {"id": "case_0"}]),
    ] {
        let error = join(cases, json!([]), Value::Null).unwrap_err();
        assert!(error.contains("duplicate"), "{error}");
    }
}

#[test]
fn result_aliases_must_agree_and_be_unique() {
    for results in [
        json!([{"case_id": "a", "summary": {}}, {"id": "a", "summary": {}}]),
        json!([{"case_id": "a", "id": "b", "summary": {}}]),
        json!([{"caseId": "a", "case_id": "b", "summary": {}}]),
    ] {
        assert!(join(cases(), results, Value::Null).is_err());
    }
    let joined = join(
        cases(),
        json!([
            {"case_id": "b", "caseId": "b", "summary": {"mass": 2}},
            {"id": "a", "summary": {"mass": 1}}
        ]),
        json!({"strict": true}),
    )
    .unwrap();
    assert_eq!(joined["cases"][0]["summary"]["mass"], 1);
    assert_eq!(joined["cases"][1]["summary"]["mass"], 2);
    assert_eq!(joined["join_complete"], true);
}

#[test]
fn malformed_supplied_ids_cannot_turn_into_positional_results() {
    for id in [Value::Null, json!(1), json!(""), json!(" \t "), json!([])] {
        assert!(join(json!([{"id": id}]), json!([]), Value::Null).is_err());
        for field in ["id", "case_id", "caseId"] {
            assert!(
                join(
                    json!([{"id": "a"}]),
                    json!([{field: id, "summary": {"mass": 1}}]),
                    Value::Null
                )
                .is_err()
            );
        }
    }
}

#[test]
fn positional_join_requires_an_entire_unlabelled_equal_length_batch() {
    assert!(
        join(
            cases(),
            json!([{"case_id": "a", "summary": {}}, {"mass": 2}]),
            Value::Null
        )
        .is_err()
    );
    assert!(join(cases(), json!([{"mass": 1}]), Value::Null).is_err());
    assert!(
        join(
            cases(),
            json!([{"mass": 1}, {"mass": 2}, {"mass": 3}]),
            Value::Null
        )
        .is_err()
    );
    let joined = join(
        cases(),
        json!([{"mass": 1}, {"summary": {"mass": 2}}]),
        json!({"strict": true}),
    )
    .unwrap();
    assert_eq!(joined["matching_mode"], "position");
    assert_eq!(joined["cases"][0]["summary"]["mass"], 1);
    assert_eq!(joined["cases"][1]["summary"]["mass"], 2);
}

#[test]
fn missing_results_remove_stale_summaries_and_result_aliases() {
    let joined = join(
        json!([{"id": "a", "summary": {"mass": 0}, "result": {"mass": 0}}]),
        json!([]),
        Value::Null,
    )
    .unwrap();
    assert!(joined["cases"][0].get("summary").is_none());
    assert!(joined["cases"][0].get("result").is_none());
    assert_eq!(joined["cases"][0]["result_status"], "missing");
    assert!(
        run_transform_operator("transform.summarize_parameter_sweep", joined, Value::Null).is_err()
    );
}

#[test]
fn failed_or_error_bearing_results_never_become_usable_summaries() {
    for result in [
        json!({"case_id": "a", "status": "failed", "summary": {"mass": 0}}),
        json!({"case_id": "a", "status": "pending", "summary": {"mass": 0}}),
        json!({"case_id": "a", "status": "ok", "error": "solver failed", "summary": {"mass": 0}}),
        json!({"case_id": "a", "status": "ok"}),
    ] {
        let joined = join(
            json!([{"id": "a", "summary": {"mass": -1}}]),
            json!([result]),
            Value::Null,
        )
        .unwrap();
        assert_eq!(joined["joined_summary_count"], 0);
        assert_eq!(joined["rejected_result_count"], 1);
        assert_eq!(joined["missing_case_ids"], json!(["a"]));
        assert_eq!(joined["join_complete"], false);
        assert_eq!(joined["rejected_results"][0]["case_id"], "a");
        assert!(
            joined["rejected_results"][0]["reason"]
                .as_str()
                .unwrap()
                .len()
                > 3
        );
        assert!(joined["cases"][0].get("summary").is_none());
    }
}

#[test]
fn malformed_selected_summaries_do_not_fall_back_to_other_payloads() {
    for summary in [Value::Null, json!([]), json!("bad"), json!(1)] {
        let joined = join(
            json!([{"id": "a"}]),
            json!([{"case_id": "a", "summary": summary, "result": {"mass": 0}}]),
            Value::Null,
        )
        .unwrap();
        assert_eq!(joined["rejected_result_count"], 1);
        assert_eq!(joined["joined_summary_count"], 0);
    }
}

#[test]
fn custom_join_fields_do_not_overwrite_identity_or_reuse_old_default_summaries() {
    for field in [
        "id",
        "case_id",
        "caseId",
        "model",
        "parameters",
        "metadata",
        "result_status",
        "result_error",
    ] {
        assert!(join(cases(), json!([]), json!({"output_field": field})).is_err());
    }
    let joined = join(
        json!([{"id": "a", "summary": {"mass": 0}, "result": {"mass": 0}}]),
        json!([{"case_id": "a", "quality": {"mass": 5}}]),
        json!({"summary_field": "quality", "output_field": "quality_result", "strict": true}),
    )
    .unwrap();
    assert_eq!(joined["cases"][0]["quality_result"]["mass"], 5);
    assert!(joined["cases"][0].get("summary").is_none());
    assert!(joined["cases"][0].get("result").is_none());
    assert!(
        join(
            json!([{"id": "a"}]),
            json!([{"case_id": "a", "summary": {"mass": 1}}]),
            json!({"summary_field": "quality", "strict": true})
        )
        .is_err()
    );
}

#[test]
fn malformed_join_configuration_does_not_silently_disable_strictness() {
    for config in [
        json!([]),
        json!("default"),
        json!({"strict": "true"}),
        json!({"strict": null}),
        json!({"summary_field": null}),
        json!({"output_field": ""}),
        json!({"summary_field": 1}),
    ] {
        assert!(join(cases(), json!([]), config).is_err());
    }
}

fn cases() -> Value {
    json!([{"id": "a"}, {"id": "b"}])
}

#[test]
fn custom_join_output_field_flows_into_summary_without_guessing() {
    let joined = join(
        json!([{"id": "a", "old_quality": {"mass": 0}}]),
        json!([{"case_id": "a", "summary": {"mass": 5}}]),
        json!({"output_field": "new_quality"}),
    )
    .unwrap();
    assert_eq!(joined["joined_summary_field"], "new_quality");
    let summary = run_transform_operator(
        "transform.summarize_parameter_sweep",
        joined.clone(),
        Value::Null,
    )
    .unwrap();
    assert_eq!(summary["rows"][0]["mass"], 5);
    assert!(
        run_transform_operator(
            "transform.summarize_parameter_sweep",
            joined,
            json!({"summary_field": "old_quality"})
        )
        .is_err()
    );
}

#[test]
fn successful_retry_replaces_old_failures_and_preserves_case_metadata() {
    let joined = join(
        json!([{"id": "a", "summary": {"mass": 0}, "result_status": "failed",
        "result_error": "old failure", "parameters": {"k": 2}, "metadata": {"round": 1}}]),
        json!([{"case_id": "a", "status": "ok", "error": null, "summary": {"mass": 5}}]),
        json!({"strict": true}),
    )
    .unwrap();
    assert_eq!(joined["cases"][0]["result_status"], "ok");
    assert!(joined["cases"][0].get("result_error").is_none());
    let summary =
        run_transform_operator("transform.summarize_parameter_sweep", joined, Value::Null).unwrap();
    assert_eq!(summary["rows"][0]["mass"], 5);
    assert_eq!(summary["rows"][0]["parameters"]["k"], 2);
    assert_eq!(summary["rows"][0]["metadata"]["round"], 1);
}

#[test]
fn large_shuffled_results_keep_one_to_one_case_identity() {
    let cases: Vec<Value> = (0..4096)
        .map(|index| json!({"id": format!("c{index}"), "parameters": {"index": index}}))
        .collect();
    let results: Vec<Value> = (0..4096)
        .rev()
        .map(|index| json!({"case_id": format!("c{index}"), "summary": {"mass": index}}))
        .collect();
    let joined = join(
        Value::Array(cases),
        Value::Array(results),
        json!({"strict": true}),
    )
    .unwrap();
    assert_eq!(joined["joined_summary_count"], 4096);
    for (index, case) in joined["cases"].as_array().unwrap().iter().enumerate() {
        assert_eq!(case["id"], format!("c{index}"));
        assert_eq!(case["summary"]["mass"], index);
        assert_eq!(case["parameters"]["index"], index);
    }
}

#[test]
fn declared_join_case_count_cannot_hide_removed_cases() {
    for count in [json!(2), Value::Null, json!("1")] {
        assert!(run_transform_operator("transform.join_parameter_sweep_results", json!({
            "case_count": count, "cases": [{"id": "a"}], "results": [{"case_id": "a", "summary": {"mass": 1}}]
        }), Value::Null).is_err());
    }
}

#[test]
fn incomplete_join_stops_the_research_scoring_and_replanning_chain() {
    use super::workflow_parameter_sweep_graph_fixtures::{
        parameter_sweep_result_scoring_graph, sweep_result_inputs,
    };
    use kyuubiki_protocol::WorkflowGraphRunRequest;

    for recover in [false, true] {
        let mut graph = parameter_sweep_result_scoring_graph(0.0);
        graph
            .nodes
            .iter_mut()
            .find(|node| node.id == "join_results")
            .unwrap()
            .config = Some(json!({"strict": false}));
        graph
            .nodes
            .iter_mut()
            .find(|node| node.id == "summarize_results")
            .unwrap()
            .config = Some(if recover {
            json!({"fields": ["max_stress", "mass"], "on_error": "skip"})
        } else {
            json!({"fields": ["max_stress", "mass"]})
        });
        let mut inputs = sweep_result_inputs();
        inputs
            .get_mut("agent_results")
            .unwrap()
            .as_array_mut()
            .unwrap()
            .remove(0);
        inputs.get_mut("sweep_cases").unwrap()[0]["summary"] = json!({"max_stress": 0, "mass": 0});
        let result = crate::run_workflow_graph(WorkflowGraphRunRequest {
            graph,
            input_artifacts: inputs,
        });
        if !recover {
            let error =
                result.expect_err("missing keyed evidence must not become a best candidate");
            assert!(error.contains("join_complete"), "{error}");
            continue;
        }
        let run = result.expect("explicit recovery should preserve join diagnostics");
        assert_eq!(run.failed_nodes, vec!["summarize_results"]);
        for id in [
            "score_results",
            "rank_quality_candidates",
            "prepare_next_round",
            "expand_next_cases",
            "next_cases_output",
        ] {
            assert!(run.skipped_nodes.iter().any(|node| node == id), "{id}");
            assert!(!run.completed_nodes.iter().any(|node| node == id), "{id}");
        }
        assert!(!run.artifacts.contains_key("score_results.scored"));
        assert!(!run.artifacts.contains_key("expand_next_cases.cases"));
        let joined = &run.artifacts["join_results.joined"];
        assert_eq!(joined["missing_case_ids"], json!(["material_panel_0"]));
        assert!(joined["cases"][0].get("summary").is_none());
        assert_eq!(joined["cases"][1]["summary"]["mass"], 7.8);
    }
}

fn join(cases: Value, results: Value, config: Value) -> Result<Value, String> {
    run_transform_operator(
        "transform.join_parameter_sweep_results",
        json!({"cases": cases, "results": results}),
        config,
    )
}
