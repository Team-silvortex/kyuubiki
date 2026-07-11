use crate::workflow_quality_objective::prepare_quality_next_round_request;

#[test]
fn replans_next_round_for_validation_block_without_require_ready() {
    let request = prepare_quality_next_round_request(
        serde_json::json!({
            "ranking": [{
                "rank": 1,
                "candidate_id": "candidate_validation_blocked",
                "score": 1.0,
                "ready": false,
                "dominant_term": {
                    "domain": "validation",
                    "source": "cross_check",
                    "dominant_term": {"field": "validation_max_relative_error"}
                },
                "blocking_terms": [{
                    "domain": "validation",
                    "source": "cross_check",
                    "source_blocking_terms": [{
                        "field": "max_temperature",
                        "status": "tolerance_failed",
                        "relative_error": 0.03
                    }]
                }]
            }]
        }),
        serde_json::json!({
            "target_score": 2.0
        }),
    )
    .expect("validation-blocked next round request should build");

    assert_eq!(request["action"].as_str(), Some("replan"));
    assert_eq!(
        request["selected_iteration_hint"]["action"].as_str(),
        Some("fix_validation_failure")
    );
    assert_eq!(
        request["request_payload"]["optimization_hint"]["focus_domain"].as_str(),
        Some("validation")
    );
    assert_eq!(
        request["request_payload"]["optimization_hint"]["focus_field"].as_str(),
        Some("max_temperature")
    );
}
