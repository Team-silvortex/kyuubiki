use crate::workflow_executor::run_transform_operator;

#[test]
fn quality_lineage_report_lists_missing_recovery_fields() {
    let report = run_transform_operator(
        "transform.compose_quality_lineage_report",
        serde_json::json!({
            "request": {
                "selected_candidate_id": "candidate_partial"
            },
            "plan": {
                "focused_axis_path": "model.thickness"
            }
        }),
        serde_json::json!({}),
    )
    .expect("partial quality lineage report should compose");

    assert_eq!(report["lineage_complete"].as_bool(), Some(false));
    assert_eq!(
        report["lineage_missing_fields"].as_array().map(Vec::len),
        Some(3)
    );
    assert!(
        report["lineage_missing_fields"]
            .as_array()
            .expect("missing fields should be an array")
            .iter()
            .any(|field| field.as_str() == Some("seed_metadata"))
    );
    assert!(
        report["lineage_summary"]
            .as_str()
            .expect("lineage summary should exist")
            .contains("missing=3")
    );
}

#[test]
fn quality_lineage_report_treats_budget_block_as_recoverable_complete_state() {
    let report = run_transform_operator(
        "transform.compose_quality_lineage_report",
        serde_json::json!({
            "request": {
                "selected_candidate_id": "candidate_budget",
                "request_payload": {
                    "seed_metadata": {"round": "seed"},
                    "optimization_hint": {
                        "action": "reduce_dominant_term",
                        "focus_field": "model.thickness"
                    }
                }
            },
            "cases": {
                "case_count": 0,
                "expansion_budget_ready": false,
                "expansion_blocking_reason": "case_budget_exceeded",
                "sweep_budget": {
                    "status": "case_budget_exceeded",
                    "recommendation": "reduce_axis_count"
                }
            }
        }),
        serde_json::json!({}),
    )
    .expect("budget-blocked lineage report should compose");

    assert_eq!(report["lineage_complete"].as_bool(), Some(true));
    assert_eq!(
        report["lineage_missing_fields"].as_array().map(Vec::len),
        Some(0)
    );
    assert_eq!(report["expansion_budget_ready"].as_bool(), Some(false));
    assert_eq!(
        report["expansion_blocking_reason"].as_str(),
        Some("case_budget_exceeded")
    );
    assert_eq!(
        report["sweep_budget"]["recommendation"].as_str(),
        Some("reduce_axis_count")
    );
}
