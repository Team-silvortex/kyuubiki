use crate::workflow_executor::run_transform_operator;
use serde_json::json;

#[test]
fn quality_lineage_report_exposes_coupled_readiness_repair_plan() {
    let report = run_transform_operator(
        "transform.compose_quality_lineage_report",
        json!({
            "request": {
                "selected_candidate_id": "candidate_coupled_blocked",
                "selected_candidate_ready": true,
                "request_payload": {
                    "seed_metadata": {"round": "coupled-readiness"},
                    "optimization_hint": {
                        "action": "fix_coupled_readiness",
                        "focus_domain": "electrostatic",
                        "focus_source": "coupled_readiness",
                        "blocking_count": 2
                    },
                    "coupled_readiness": {
                        "coupled_readiness_contract": "kyuubiki.coupled_readiness/v1",
                        "coupled_readiness_state": "block",
                        "coupled_readiness_recommendation": "hold_and_repair_inputs",
                        "coupled_readiness_blocking_domains": ["electrostatic"],
                        "coupled_readiness_required_missing": ["thermo"],
                        "coupled_readiness_warning_domains": []
                    }
                }
            },
            "plan": {
                "focused_axis_path": "electrostatic.voltage",
                "case_count_estimate": 2
            },
            "cases": {
                "case_count": 2,
                "cases": [{
                    "id": "quality_candidate_0",
                    "metadata": {
                        "source_candidate_id": "candidate_coupled_blocked",
                        "focused_axis_path": "electrostatic.voltage"
                    }
                }]
            }
        }),
        json!({}),
    )
    .expect("coupled readiness lineage report should compose");

    assert_eq!(report["lineage_complete"].as_bool(), Some(true));
    assert_eq!(
        report["coupled_readiness"]["coupled_readiness_state"].as_str(),
        Some("block")
    );
    assert_eq!(
        report["repair_plan"]["repair_action"].as_str(),
        Some("fix_coupled_readiness")
    );
    assert_eq!(
        report["repair_plan"]["coupled_blocking_domains"][0].as_str(),
        Some("electrostatic")
    );
    assert_eq!(
        report["repair_plan"]["coupled_required_missing"][0].as_str(),
        Some("thermo")
    );
    assert_eq!(
        report["repair_plan"]["recommended_next_step"].as_str(),
        Some("repair_coupled_domain_inputs_before_next_sweep")
    );
}

#[test]
fn quality_lineage_report_exposes_coupled_readiness_review_plan() {
    let report = run_transform_operator(
        "transform.compose_quality_lineage_report",
        json!({
            "request": {
                "selected_candidate_id": "candidate_coupled_warning",
                "selected_candidate_ready": true,
                "request_payload": {
                    "seed_metadata": {"round": "coupled-review"},
                    "optimization_hint": {
                        "action": "review_coupled_readiness",
                        "focus_domain": "thermal",
                        "focus_source": "coupled_readiness",
                        "warning_count": 1
                    },
                    "coupled_readiness": {
                        "coupled_readiness_contract": "kyuubiki.coupled_readiness/v1",
                        "coupled_readiness_state": "warn",
                        "coupled_readiness_recommendation": "review_before_next_round",
                        "coupled_readiness_blocking_domains": [],
                        "coupled_readiness_required_missing": [],
                        "coupled_readiness_warning_domains": ["thermal"]
                    }
                }
            },
            "plan": {
                "focused_axis_path": "thermal.temperature",
                "case_count_estimate": 2
            },
            "cases": {
                "case_count": 2,
                "cases": [{
                    "id": "quality_candidate_0",
                    "metadata": {
                        "source_candidate_id": "candidate_coupled_warning",
                        "focused_axis_path": "thermal.temperature"
                    }
                }]
            }
        }),
        json!({}),
    )
    .expect("coupled readiness review lineage report should compose");

    assert_eq!(report["lineage_complete"].as_bool(), Some(true));
    assert_eq!(
        report["repair_plan"]["repair_action"].as_str(),
        Some("review_coupled_readiness")
    );
    assert_eq!(
        report["repair_plan"]["coupled_warning_domains"][0].as_str(),
        Some("thermal")
    );
    assert_eq!(
        report["repair_plan"]["recommended_next_step"].as_str(),
        Some("review_coupled_domain_warnings_before_next_sweep")
    );
}
