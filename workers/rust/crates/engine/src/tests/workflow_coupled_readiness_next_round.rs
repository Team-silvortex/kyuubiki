use crate::workflow_quality_objective::prepare_quality_next_round_request;
use serde_json::json;

#[test]
fn coupled_readiness_block_forces_quality_next_round_replan() {
    let request = prepare_quality_next_round_request(
        json!({
            "quality_candidate_ranking_contract": "kyuubiki.quality_candidate_ranking/v1",
            "ranking": [
                {
                    "rank": 1,
                    "candidate_id": "coupled_candidate",
                    "score": 1.4,
                    "ready": true,
                    "objective": {
                        "composite_quality_contract": "kyuubiki.composite_quality_objective/v1",
                        "composite_quality_score": 1.4,
                        "composite_quality_ready": true,
                        "composite_quality_dominant_term": {
                            "domain": "thermal",
                            "source": "thermal_quality",
                            "dominant_term": {
                                "field": "thermal_quality_score"
                            }
                        },
                        "composite_quality_blocking_terms": []
                    },
                    "coupled_readiness": {
                        "coupled_readiness_contract": "kyuubiki.coupled_readiness/v1",
                        "coupled_readiness_ready": false,
                        "coupled_readiness_state": "block",
                        "coupled_readiness_recommendation": "hold_and_repair_inputs",
                        "coupled_readiness_blocking_domains": ["electrostatic"],
                        "coupled_readiness_required_missing": ["thermo"],
                        "coupled_readiness_warning_domains": []
                    }
                }
            ]
        }),
        json!({
            "target_score": 3.0,
            "search_space": {
                "mesh_size": [0.1, 0.2]
            }
        }),
    )
    .expect("coupled readiness should be accepted by next-round request");

    assert_eq!(
        request.get("action").and_then(|value| value.as_str()),
        Some("replan")
    );
    assert_eq!(
        request
            .get("source_coupled_readiness_contract")
            .and_then(|value| value.as_str()),
        Some("kyuubiki.coupled_readiness/v1")
    );
    assert_eq!(
        request
            .get("selected_iteration_hint")
            .and_then(|hint| hint.get("action"))
            .and_then(|value| value.as_str()),
        Some("fix_coupled_readiness")
    );
    assert_eq!(
        request
            .get("selected_iteration_hint")
            .and_then(|hint| hint.get("focus_domain"))
            .and_then(|value| value.as_str()),
        Some("electrostatic")
    );
    assert_eq!(
        request
            .get("request_payload")
            .and_then(|payload| payload.get("coupled_readiness"))
            .and_then(|readiness| readiness.get("coupled_readiness_recommendation"))
            .and_then(|value| value.as_str()),
        Some("hold_and_repair_inputs")
    );
}

#[test]
fn coupled_readiness_warning_keeps_next_round_running_with_review_hint() {
    let request = prepare_quality_next_round_request(
        json!({
            "quality_candidate_ranking_contract": "kyuubiki.quality_candidate_ranking/v1",
            "ranking": [
                {
                    "rank": 1,
                    "candidate_id": "coupled_warning_candidate",
                    "score": 4.0,
                    "ready": true,
                    "objective": {
                        "composite_quality_contract": "kyuubiki.composite_quality_objective/v1",
                        "composite_quality_score": 4.0,
                        "composite_quality_ready": true,
                        "composite_quality_dominant_term": {
                            "domain": "thermal",
                            "source": "thermal_quality"
                        },
                        "composite_quality_blocking_terms": []
                    },
                    "coupled_readiness": {
                        "coupled_readiness_contract": "kyuubiki.coupled_readiness/v1",
                        "coupled_readiness_ready": true,
                        "coupled_readiness_state": "warn",
                        "coupled_readiness_recommendation": "review_before_next_round",
                        "coupled_readiness_blocking_domains": [],
                        "coupled_readiness_required_missing": [],
                        "coupled_readiness_warning_domains": ["thermal"]
                    }
                }
            ]
        }),
        json!({"target_score": 3.0}),
    )
    .expect("warning coupled readiness should produce a next-round request");

    assert_eq!(
        request.get("action").and_then(|value| value.as_str()),
        Some("continue")
    );
    assert_eq!(
        request
            .get("selected_iteration_hint")
            .and_then(|hint| hint.get("action"))
            .and_then(|value| value.as_str()),
        Some("review_coupled_readiness")
    );
    assert_eq!(
        request
            .get("selected_iteration_hint")
            .and_then(|hint| hint.get("warning_domains"))
            .and_then(|domains| domains.as_array())
            .and_then(|domains| domains.first())
            .and_then(|value| value.as_str()),
        Some("thermal")
    );
}

#[test]
fn coupled_readiness_warning_prevents_early_stop_even_when_score_meets_target() {
    let request = prepare_quality_next_round_request(
        json!({
            "quality_candidate_ranking_contract": "kyuubiki.quality_candidate_ranking/v1",
            "ranking": [
                {
                    "rank": 1,
                    "candidate_id": "coupled_warning_good_score",
                    "score": 1.0,
                    "ready": true,
                    "objective": {
                        "composite_quality_contract": "kyuubiki.composite_quality_objective/v1",
                        "composite_quality_score": 1.0,
                        "composite_quality_ready": true,
                        "composite_quality_dominant_term": {
                            "domain": "thermal",
                            "source": "thermal_quality"
                        },
                        "composite_quality_blocking_terms": []
                    },
                    "coupled_readiness": {
                        "coupled_readiness_contract": "kyuubiki.coupled_readiness/v1",
                        "coupled_readiness_ready": true,
                        "coupled_readiness_state": "warn",
                        "coupled_readiness_recommendation": "review_before_next_round",
                        "coupled_readiness_warning_domains": ["thermal"]
                    }
                }
            ]
        }),
        json!({"target_score": 3.0}),
    )
    .expect("warning readiness should keep review path active");

    assert_eq!(
        request.get("action").and_then(|value| value.as_str()),
        Some("continue")
    );
    assert_eq!(
        request
            .get("selected_iteration_hint")
            .and_then(|hint| hint.get("action"))
            .and_then(|value| value.as_str()),
        Some("review_coupled_readiness")
    );
}
