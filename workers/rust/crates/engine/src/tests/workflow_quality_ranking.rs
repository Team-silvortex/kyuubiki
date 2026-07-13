use crate::{
    workflow_executor::run_transform_operator,
    workflow_quality_objective::{prepare_quality_next_round_request, rank_quality_candidates},
};

fn approx_eq(left: Option<f64>, right: f64) {
    let value = left.expect("expected numeric objective value");
    assert!(
        (value - right).abs() < 1.0e-9,
        "left={value}, right={right}"
    );
}

#[test]
fn ranks_quality_candidates_by_readiness_and_score() {
    let ranking = rank_quality_candidates(
        serde_json::json!({
            "candidates": {
                "candidate_a": {
                    "qualities": {
                        "thermal": {
                            "thermal_quality_score": 2.0,
                            "thermal_quality_ready": true,
                            "thermal_quality_missing_metric_count": 0
                        },
                        "cfd": {
                            "cfd_quality_score": 5.0,
                            "cfd_quality_ready": true,
                            "cfd_quality_missing_metric_count": 0
                        }
                    }
                },
                "candidate_b": {
                    "qualities": {
                        "thermal": {
                            "thermal_quality_score": 1.0,
                            "thermal_quality_ready": true,
                            "thermal_quality_missing_metric_count": 0,
                            "thermal_quality_dominant_term": {
                                "field": "thermal_flux_peak_magnitude",
                                "status": "ok",
                                "penalty": 1.0
                            },
                            "thermal_quality_blocking_terms": []
                        },
                        "cfd": {
                            "cfd_quality_score": 1.5,
                            "cfd_quality_ready": true,
                            "cfd_quality_missing_metric_count": 0
                        }
                    }
                },
                "candidate_blocked": {
                    "qualities": {
                        "thermal": {
                            "thermal_quality_score": 0.5,
                            "thermal_quality_ready": false,
                            "thermal_quality_missing_metric_count": 1
                        }
                    }
                }
            }
        }),
        serde_json::json!({
            "objective": {
                "weights": {"cfd": 2.0},
                "not_ready_penalty": 20.0
            }
        }),
    )
    .expect("quality candidates should rank");

    assert_eq!(
        ranking["quality_candidate_ranking_contract"].as_str(),
        Some("kyuubiki.quality_candidate_ranking/v1")
    );
    assert_eq!(ranking["candidate_count"].as_u64(), Some(3));
    assert_eq!(ranking["ready_candidate_count"].as_u64(), Some(2));
    assert_eq!(ranking["best_candidate_id"].as_str(), Some("candidate_b"));
    assert_eq!(ranking["best_candidate_ready"].as_bool(), Some(true));
    assert_eq!(ranking["ranking"][0]["rank"].as_u64(), Some(1));
    assert_eq!(
        ranking["ranking"][0]["dominant_term"]["domain"].as_str(),
        Some("cfd")
    );
    assert_eq!(
        ranking["ranking"][0]["blocking_terms"]
            .as_array()
            .map(Vec::len),
        Some(0)
    );
}

#[test]
fn runs_quality_candidate_ranking_through_transform_executor() {
    let ranking = run_transform_operator(
        "transform.rank_quality_candidates",
        serde_json::json!({
            "candidate_a": {
                "thermal": {
                    "thermal_quality_score": 4.0,
                    "thermal_quality_ready": true,
                    "thermal_quality_missing_metric_count": 0
                }
            },
            "candidate_b": {
                "thermal": {
                    "thermal_quality_score": 2.0,
                    "thermal_quality_ready": true,
                    "thermal_quality_missing_metric_count": 0
                }
            }
        }),
        serde_json::json!({}),
    )
    .expect("quality candidate ranking should run through executor");

    assert_eq!(ranking["best_candidate_id"].as_str(), Some("candidate_b"));
    approx_eq(ranking["best_candidate_score"].as_f64(), 2.0);
}

#[test]
fn prepares_quality_next_round_request_from_ranking() {
    let request = prepare_quality_next_round_request(
        serde_json::json!({
            "quality_candidate_ranking_contract": "kyuubiki.quality_candidate_ranking/v1",
            "ranking": [{
                "rank": 1,
                "candidate_id": "candidate_b",
                "score": 2.5,
                "ready": true,
                "metadata": {
                    "source_candidate_id": "seed_a",
                    "focused_axis_path": "model.thickness"
                },
                "dominant_term": {
                    "domain": "thermal",
                    "dominant_term": {"field": "thermal_temperature_max"}
                },
                "blocking_terms": [],
                "objective": {"composite_quality_score": 2.5}
            }]
        }),
        serde_json::json!({
            "target_score": 2.0,
            "max_candidates": 12,
            "search_space": {"thickness_mm": [1.0, 4.0]}
        }),
    )
    .expect("quality next round request should build");

    assert_eq!(
        request["quality_next_round_contract"].as_str(),
        Some("kyuubiki.quality_next_round_request/v1")
    );
    assert_eq!(request["action"].as_str(), Some("continue"));
    assert_eq!(
        request["selected_candidate_id"].as_str(),
        Some("candidate_b")
    );
    assert_eq!(
        request["selected_candidate_metadata"]["focused_axis_path"].as_str(),
        Some("model.thickness")
    );
    assert_eq!(
        request["request_payload"]["seed_metadata"]["source_candidate_id"].as_str(),
        Some("seed_a")
    );
    assert_eq!(
        request["selected_dominant_term"]["domain"].as_str(),
        Some("thermal")
    );
    assert_eq!(
        request["selected_iteration_hint"]["action"].as_str(),
        Some("reduce_dominant_term")
    );
    assert_eq!(
        request["request_payload"]["optimization_hint"]["focus_field"].as_str(),
        Some("thermal_temperature_max")
    );
    approx_eq(request["request_payload"]["max_candidates"].as_f64(), 12.0);
}

#[test]
fn prepares_quality_next_round_request_with_blocking_iteration_hint() {
    let request = prepare_quality_next_round_request(
        serde_json::json!({
            "ranking": [{
                "rank": 1,
                "candidate_id": "candidate_blocked",
                "score": 20.0,
                "ready": false,
                "dominant_term": {
                    "domain": "electrostatic",
                    "source": "electrostatic",
                    "dominant_term": {"field": "electrostatic_field_peak_magnitude"}
                },
                "blocking_terms": [{
                    "domain": "electrostatic",
                    "source": "electrostatic",
                    "source_blocking_terms": [{
                        "field": "electrostatic_peak_energy_density",
                        "status": "missing"
                    }]
                }]
            }]
        }),
        serde_json::json!({
            "require_ready": true,
            "target_score": 2.0
        }),
    )
    .expect("blocked quality next round request should build");

    assert_eq!(request["action"].as_str(), Some("replan"));
    assert_eq!(
        request["selected_iteration_hint"]["action"].as_str(),
        Some("fix_blocking_term")
    );
    assert_eq!(
        request["selected_iteration_hint"]["focus_field"].as_str(),
        Some("electrostatic_peak_energy_density")
    );
    assert_eq!(
        request["request_payload"]["optimization_hint"]["blocking_count"].as_u64(),
        Some(1)
    );
}

#[test]
fn runs_quality_next_round_request_through_transform_executor() {
    let request = run_transform_operator(
        "transform.prepare_quality_next_round_request",
        serde_json::json!({
            "ranking": [{
                "rank": 1,
                "candidate_id": "candidate_ready",
                "score": 1.5,
                "ready": true
            }]
        }),
        serde_json::json!({"target_score": 2.0}),
    )
    .expect("quality next round request should run through executor");

    assert_eq!(request["action"].as_str(), Some("stop"));
    assert_eq!(
        request["selected_candidate_id"].as_str(),
        Some("candidate_ready")
    );
}
