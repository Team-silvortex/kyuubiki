use crate::workflow_executor::run_transform_operator;
use serde_json::{Value, json};

fn row(id: &str, score: f64, ready: bool) -> Value {
    json!({"case_id":id, "objective_score":score, "objective_feasible":ready})
}
fn map(payload: Value, config: Value) -> Result<Value, String> {
    run_transform_operator(
        "transform.map_parameter_sweep_scores_to_quality_candidates",
        payload,
        config,
    )
}

#[test]
fn candidate_mapping_requires_explicit_unique_unambiguous_identity() {
    for rows in [
        json!([row("a", -1.0, true), row("a", -2.0, true)]),
        json!([{"objective_score":-1, "objective_feasible":true}]),
        json!([{"case_id":null, "objective_score":-1,"objective_feasible":true}]),
        json!([{"case_id":"a","id":"b","objective_score":-1,"objective_feasible":true}]),
    ] {
        assert!(
            map(json!({"scored_rows":rows}), Value::Null).is_err(),
            "{rows}"
        );
    }
}

#[test]
fn candidate_mapping_never_defaults_missing_or_malformed_feasibility_to_ready() {
    for flag in [None, Some(Value::Null), Some(json!("true")), Some(json!(1))] {
        let mut candidate = row("a", -1.0, true);
        candidate
            .as_object_mut()
            .unwrap()
            .remove("objective_feasible");
        if let Some(flag) = flag {
            candidate["objective_feasible"] = flag;
        }
        assert!(map(json!({"scored_rows":[candidate]}), Value::Null).is_err());
    }
}

#[test]
fn candidate_mapping_rejects_incomplete_or_failed_evidence() {
    for extra in [
        json!({"summary_complete":false}),
        json!({"join_complete":false}),
        json!({"missing_field_count":1}),
        json!({"scored_count":2}),
        json!({"feasible_count":0}),
        json!({"selection_ready":false}),
    ] {
        let mut payload = extra;
        payload["scored_rows"] = json!([row("a", -1.0, true)]);
        assert!(map(payload, Value::Null).is_err());
    }
    for extra in [
        json!({"result_status":"failed"}),
        json!({"result_error":"bad"}),
        json!({"converged":false}),
    ] {
        let mut candidate = row("a", -1.0, true);
        candidate
            .as_object_mut()
            .unwrap()
            .extend(extra.as_object().unwrap().clone());
        assert!(map(json!({"scored_rows":[candidate]}), Value::Null).is_err());
    }
}

#[test]
fn candidate_mapping_configuration_does_not_silently_fall_back() {
    for config in [
        json!([]),
        json!({"quality_domain":null}),
        json!({"quality_domain":" "}),
        json!({"include_source_row":"false"}),
        json!({"score_reference":null}),
    ] {
        assert!(map(json!({"scored_rows":[row("a", -1.0, true)]}), config).is_err());
    }
}

#[test]
fn candidate_mapping_rejects_wrong_or_infeasible_claimed_best() {
    for best in [
        Value::Null,
        json!({"case_id":"other"}),
        json!({"case_id":"b"}),
        json!({"case_id":"a", "objective_score":-99}),
    ] {
        assert!(
            map(
                json!({"best":best,"scored_rows":[row("a", -1.0, true),row("b", 1e15, false)]}),
                json!({"score_reference":1e15})
            )
            .is_err(),
            "{best}"
        );
    }
}

#[test]
fn positive_objectives_require_an_explicit_fixed_quality_reference() {
    let payload = json!({"scored_rows":[row("a", 10.0, true),row("b", 5.0, true)]});
    assert!(map(payload.clone(), Value::Null).is_err());
    let mapped = map(payload, json!({"score_reference":20})).unwrap();
    assert_eq!(
        mapped["candidates"]["a"]["qualities"]["parameter_sweep"]["parameter_sweep_quality_score"],
        10.0
    );
    assert_eq!(
        mapped["candidates"]["b"]["qualities"]["parameter_sweep"]["parameter_sweep_quality_score"],
        15.0
    );
    assert_eq!(mapped["quality_score_mapping"]["score_reference"], 20.0);
    let ranked =
        run_transform_operator("transform.rank_quality_candidates", mapped, Value::Null).unwrap();
    assert_eq!(ranked["best_candidate_id"], "a");
    let request = run_transform_operator(
        "transform.prepare_quality_next_round_request",
        ranked,
        json!({"target_score":3}),
    )
    .unwrap();
    assert_eq!(request["action"], "continue");
}

#[test]
fn fixed_reference_does_not_clamp_or_overflow_quality_cost() {
    for (score, reference) in [(11.0, 10.0), (-1e308, 1e308)] {
        assert!(
            map(
                json!({"scored_rows":[row("a",score,true)]}),
                json!({"score_reference":reference})
            )
            .is_err()
        );
    }
}

#[test]
fn candidate_mapping_preserves_all_blockers_not_just_largest_objective() {
    let scored = run_transform_operator("transform.score_parameter_sweep", json!({"rows":[{"case_id":"a","mass":1,"stress":200,"heat":150}]}),
        json!({"objectives":[{"field":"mass","weight":100},{"field":"stress","weight":0,"max_allowed":100},{"field":"heat","weight":0,"max_allowed":100}]})).unwrap();
    let mapped = map(scored, Value::Null).unwrap();
    let quality = &mapped["candidates"]["a"]["qualities"]["parameter_sweep"];
    let fields: Vec<_> = quality["parameter_sweep_quality_blocking_terms"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["field"].as_str().unwrap())
        .collect();
    assert_eq!(fields, ["stress", "heat"]);
    assert_eq!(
        quality["parameter_sweep_quality_dominant_term"]["field"],
        "stress"
    );
    assert_eq!(mapped["source_best_case_id"], Value::Null);
}

#[test]
fn candidate_mapping_validates_present_breakdown_and_aggregate_consistency() {
    for breakdown in [
        Value::Null,
        json!({}),
        json!([{}]),
        json!([{"field":"mass","goal":"min","value":1,"score":null}]),
        json!([{"field":"mass","goal":"min","value":1,"score":-99}]),
        json!([{"field":"mass","goal":"min","value":1,"score":-1,"feasible":false}]),
    ] {
        let mut candidate = row("a", -1.0, true);
        candidate["objective_breakdown"] = breakdown;
        assert!(map(json!({"scored_rows":[candidate]}), Value::Null).is_err());
    }
    let scored = run_transform_operator(
        "transform.score_parameter_sweep",
        json!({"rows":[{"case_id":"a","mass":1.25}]}),
        json!({"objectives":[{"field":"mass"}]}),
    )
    .unwrap();
    let exchanged: Value = serde_json::from_slice(&serde_json::to_vec(&scored).unwrap()).unwrap();
    assert!(map(exchanged.clone(), Value::Null).is_ok());
    for inconsistent in [Value::Null, json!("1.25"), json!(2.5)] {
        let mut damaged = exchanged.clone();
        damaged.as_object_mut().unwrap().remove("best");
        damaged["scored_rows"][0]["mass"] = inconsistent;
        assert!(map(damaged, Value::Null).is_err());
    }
}

#[test]
fn all_infeasible_candidates_never_stop_the_quality_round() {
    let mapped = map(
        json!({"best":null,"scored_rows":[row("a",-1e12,false)]}),
        Value::Null,
    )
    .unwrap();
    assert_eq!(mapped["source_best_case_id"], Value::Null);
    let ranked =
        run_transform_operator("transform.rank_quality_candidates", mapped, Value::Null).unwrap();
    assert_eq!(ranked["ready_candidate_count"], 0);
    let request = run_transform_operator(
        "transform.prepare_quality_next_round_request",
        ranked,
        json!({"target_score":2e12}),
    )
    .unwrap();
    assert_eq!(request["action"], "replan");
}

#[test]
fn candidate_mapping_accepts_scoring_json_exchange_with_fractional_weights() {
    for index in 1..=100 {
        let scored = run_transform_operator(
            "transform.score_parameter_sweep",
            json!({"rows":[{"case_id":"a","mass":index as f64 * 0.07,"stress":index as f64 / 3.0}]}),
            json!({"objectives":[{"field":"mass","weight":0.13},{"field":"stress","weight":0.017,"max_allowed":20}]}),
        )
        .unwrap();
        let exchanged = serde_json::from_slice(&serde_json::to_vec(&scored).unwrap()).unwrap();
        map(exchanged, Value::Null).unwrap_or_else(|error| panic!("case {index}: {error}"));
    }
}
