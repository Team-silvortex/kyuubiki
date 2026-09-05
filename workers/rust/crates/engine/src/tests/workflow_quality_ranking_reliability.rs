use crate::workflow_quality_objective::{
    prepare_quality_next_round_request, rank_quality_candidates,
};
use serde_json::{Value, json};

#[test]
fn failed_candidate_evaluations_remain_visible_in_partial_rankings() {
    for candidates in [
        json!({"good": candidate(0.0), "broken": {}, "null_candidate": null}),
        json!([{"id": "good", "qualities": candidate(0.0)["qualities"]},
               {"id": "broken"}, null]),
    ] {
        let ranking =
            rank_quality_candidates(json!({"candidates": candidates}), Value::Null).unwrap();
        assert_eq!(ranking["input_candidate_count"], 3);
        assert_eq!(ranking["candidate_count"], 1);
        assert_eq!(ranking["rejected_candidate_count"], 2);
        assert_eq!(ranking["ranking_complete"], false);
        assert_eq!(ranking["best_candidate_id"], "good");
        let rejected = ranking["rejected_candidates"].as_array().unwrap();
        assert_eq!(rejected.len(), 2);
        assert!(
            rejected
                .iter()
                .all(|entry| !entry["error"].as_str().unwrap().is_empty())
        );
    }
}

#[test]
fn partial_ranking_cannot_stop_research_after_a_good_remaining_score() {
    let ranking = rank_quality_candidates(
        json!({"candidates": {"good": candidate(0.0), "broken": {}}}),
        Value::Null,
    )
    .unwrap();
    let request =
        prepare_quality_next_round_request(ranking.clone(), json!({"target_score": 3.0})).unwrap();
    assert_eq!(request["action"], "replan");
    assert_eq!(
        request["source_rejected_candidates"],
        ranking["rejected_candidates"]
    );
    assert_eq!(
        request["selected_iteration_hint"]["action"],
        "repair_rejected_candidates"
    );
}

#[test]
fn all_rejected_candidates_retain_a_named_failure_reason() {
    let error = rank_quality_candidates(json!({"candidates": {"broken": {}}}), Value::Null)
        .expect_err("no candidate can be selected");
    assert!(
        error.contains("broken") && error.contains("quality"),
        "{error}"
    );
}

#[test]
fn malformed_or_ambiguous_candidate_collections_cannot_be_reinterpreted() {
    for value in [Value::Null, json!(1), json!("candidates")] {
        let error = rank_quality_candidates(
            json!({"candidates": value, "hidden": candidate(0.0)}),
            Value::Null,
        )
        .expect_err("invalid candidates must not fall back to sibling fields");
        assert!(error.contains("candidates"), "{error}");
    }
    let mut first = candidate(1.0);
    first["id"] = json!("same");
    let error = rank_quality_candidates(json!({"candidates": [first.clone(), first]}), Value::Null)
        .expect_err("duplicate IDs make the selected seed ambiguous");
    assert!(
        error.contains("same") && error.contains("duplicate"),
        "{error}"
    );
}

#[test]
fn invalid_selected_scores_never_default_to_zero() {
    for value in [Value::Null, json!("0"), json!(-1.0), json!(false)] {
        let error = prepare_quality_next_round_request(
            json!({"ranking": [{"candidate_id": "bad", "score": value, "ready": true}]}),
            Value::Null,
        )
        .expect_err("invalid scores must not stop research");
        assert!(error.contains("score"), "{error}");
    }
    assert!(
        prepare_quality_next_round_request(
            json!({"ranking": [{"candidate_id": "bad", "ready": true}]}),
            Value::Null,
        )
        .is_err()
    );
}

#[test]
fn an_unready_candidate_cannot_stop_even_when_readiness_is_optional() {
    for (require_ready, action) in [(false, "continue"), (true, "replan")] {
        let request = prepare_quality_next_round_request(
            json!({"ranking": [{"candidate_id": "pending", "score": 0.0, "ready": false}]}),
            json!({"require_ready": require_ready}),
        )
        .unwrap();
        assert_eq!(request["action"], action);
    }
}

#[test]
fn valid_complete_rankings_preserve_selection_and_stop_behavior() {
    let ranking = rank_quality_candidates(
        json!({"candidates": {"first": candidate(0.0), "second": candidate(1.0)}}),
        Value::Null,
    )
    .unwrap();
    assert_eq!(ranking["ranking_complete"], true);
    assert_eq!(ranking["rejected_candidates"], json!([]));
    let request = prepare_quality_next_round_request(ranking, Value::Null).unwrap();
    assert_eq!(request["selected_candidate_id"], "first");
    assert_eq!(request["action"], "stop");
}

#[test]
fn an_explicit_bad_summary_cannot_disappear_from_quality_coverage() {
    let mut bad = candidate(0.0);
    bad["qualities"]["transport"] = Value::Null;
    let ranking = rank_quality_candidates(
        json!({"candidates": {"good": candidate(1.0), "partial": bad}}),
        Value::Null,
    )
    .unwrap();
    assert_eq!(ranking["candidate_count"], 1);
    assert_eq!(ranking["rejected_candidates"][0]["candidate_id"], "partial");
    assert!(
        ranking["rejected_candidates"][0]["error"]
            .as_str()
            .unwrap()
            .contains("transport")
    );
}

#[test]
fn contradictory_ranking_completeness_metadata_is_rejected() {
    let ranking = rank_quality_candidates(
        json!({"candidates": {"good": candidate(0.0), "broken": {}}}),
        Value::Null,
    )
    .unwrap();
    for (field, value) in [
        ("ranking_complete", json!(true)),
        ("ranking_complete", Value::Null),
        ("rejected_candidate_count", json!(0)),
        ("rejected_candidates", Value::Null),
        ("candidate_count", json!(9)),
        ("input_candidate_count", json!(1)),
    ] {
        let mut malformed = ranking.clone();
        malformed[field] = value;
        assert!(
            prepare_quality_next_round_request(malformed, Value::Null).is_err(),
            "{field}"
        );
    }
}

#[test]
fn explicit_blocking_terms_override_a_claimed_ready_flag() {
    let mut blocked = candidate(0.0);
    blocked["qualities"]["thermal"]["thermal_quality_blocking_terms"] =
        json!([{"field": "temperature", "status": "missing"}]);
    let ranking =
        rank_quality_candidates(json!({"candidates": {"blocked": blocked}}), Value::Null).unwrap();
    assert_eq!(ranking["best_candidate_ready"], false);
    let request = prepare_quality_next_round_request(ranking, Value::Null).unwrap();
    assert_eq!(request["action"], "replan");
    let error = prepare_quality_next_round_request(
        json!({"ranking": [{"candidate_id": "bad", "score": 0.0, "ready": true, "blocking_terms": null}]}),
        Value::Null,
    ).expect_err("malformed blockers cannot be treated as an empty list");
    assert!(error.contains("blocking_terms"), "{error}");
}

fn candidate(score: f64) -> Value {
    json!({"qualities": {"thermal": {"thermal_quality_score": score, "thermal_quality_ready": true}}})
}
