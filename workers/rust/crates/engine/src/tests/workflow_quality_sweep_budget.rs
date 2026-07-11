use crate::{
    workflow_executor::run_transform_operator,
    workflow_quality_sweep_request::build_quality_parameter_sweep_plan,
};

#[test]
fn quality_sweep_plan_reports_axis_budget_truncation() {
    let plan = build_quality_parameter_sweep_plan(
        serde_json::json!({
            "action": "continue",
            "selected_candidate_id": "candidate_budget",
            "request_payload": {
                "search_space": {
                    "model.thickness": [0.01, 0.02],
                    "material.density": [2700.0, 7800.0],
                    "mesh.seed": [0.25, 0.5]
                }
            }
        }),
        serde_json::json!({
            "samples_per_axis": 2,
            "max_axes": 1,
            "max_cases": 8
        }),
    )
    .expect("budgeted quality sweep plan should build");

    assert_eq!(plan["axes"].as_array().map(Vec::len), Some(1));
    assert_eq!(
        plan["sweep_budget"]["status"].as_str(),
        Some("axis_budget_truncated")
    );
    assert_eq!(plan["sweep_budget"]["usable_axis_count"].as_u64(), Some(3));
    assert_eq!(plan["sweep_budget"]["planned_axis_count"].as_u64(), Some(1));
    assert_eq!(
        plan["sweep_budget"]["axis_budget_truncated"].as_bool(),
        Some(true)
    );
    assert_eq!(
        plan["sweep_budget"]["recommendation"].as_str(),
        Some("schedule_followup_axis_batch")
    );
    assert_eq!(
        plan["sweep_budget"]["planned_axis_paths"][0].as_str(),
        Some("material.density")
    );
    assert_eq!(
        plan["sweep_budget"]["case_budget_exceeded"].as_bool(),
        Some(false)
    );
}

#[test]
fn quality_sweep_plan_recommends_axis_reduction_when_case_budget_is_exceeded() {
    let plan = build_quality_parameter_sweep_plan(
        serde_json::json!({
            "action": "continue",
            "request_payload": {
                "search_space": {
                    "model.a": [1, 2, 3],
                    "model.b": [1, 2, 3],
                    "model.c": [1, 2, 3]
                }
            }
        }),
        serde_json::json!({
            "max_cases": 9
        }),
    )
    .expect("case-budgeted quality sweep plan should build");

    assert_eq!(
        plan["sweep_budget"]["status"].as_str(),
        Some("case_budget_exceeded")
    );
    assert_eq!(
        plan["sweep_budget"]["recommendation"].as_str(),
        Some("reduce_axis_count")
    );
    assert_eq!(
        plan["sweep_budget"]["recommended_axis_count"].as_u64(),
        Some(2)
    );
}

#[test]
fn quality_sweep_materialization_marks_case_budget_as_not_ready() {
    let expansion = run_transform_operator(
        "transform.materialize_quality_sweep_expansion",
        serde_json::json!({
            "quality_parameter_sweep_plan_contract": "kyuubiki.quality_parameter_sweep_plan/v1",
            "sweep_enabled": true,
            "case_count_estimate": 27,
            "sweep_budget": {
                "status": "case_budget_exceeded",
                "case_budget_exceeded": true,
                "recommendation": "reduce_axis_count"
            },
            "base": {"model": {"a": 1}},
            "axes": [{
                "path": "model.a",
                "values": [1, 2, 3]
            }]
        }),
        serde_json::json!({}),
    )
    .expect("budget-aware quality sweep expansion should materialize");

    assert_eq!(expansion["expansion_enabled"].as_bool(), Some(true));
    assert_eq!(expansion["expansion_budget_ready"].as_bool(), Some(false));
    assert_eq!(
        expansion["expansion_blocking_reason"].as_str(),
        Some("case_budget_exceeded")
    );
    assert_eq!(
        expansion["payload"]["case_metadata"]["sweep_budget"]["recommendation"].as_str(),
        Some("reduce_axis_count")
    );
}

#[test]
fn quality_sweep_expansion_returns_structured_budget_block() {
    let expanded = run_transform_operator(
        "transform.expand_parameter_sweep",
        serde_json::json!({
            "quality_sweep_expansion_contract": "kyuubiki.quality_sweep_expansion/v1",
            "expansion_enabled": true,
            "expansion_budget_ready": false,
            "expansion_blocking_reason": "case_budget_exceeded",
            "source_candidate_id": "candidate_budget",
            "case_count_estimate": 27,
            "sweep_budget": {
                "status": "case_budget_exceeded",
                "recommendation": "reduce_axis_count",
                "case_budget_exceeded": true
            },
            "payload": {
                "base": {"model": {"a": 1}},
                "axes": [{
                    "path": "model.a",
                    "values": [1, 2, 3]
                }]
            },
            "config": {
                "max_cases": 2
            }
        }),
        serde_json::json!({}),
    )
    .expect("budget-blocked quality sweep should return structured result");

    assert_eq!(expanded["case_count"].as_u64(), Some(0));
    assert_eq!(expanded["cases"].as_array().map(Vec::len), Some(0));
    assert_eq!(expanded["expansion_budget_ready"].as_bool(), Some(false));
    assert_eq!(
        expanded["expansion_blocking_reason"].as_str(),
        Some("case_budget_exceeded")
    );
    assert_eq!(
        expanded["sweep_budget"]["recommendation"].as_str(),
        Some("reduce_axis_count")
    );
}

#[test]
fn quality_lineage_report_carries_sweep_budget_status() {
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
            "plan": {
                "focused_axis_path": "model.thickness",
                "case_count_estimate": 16,
                "sweep_budget": {
                    "status": "case_budget_exceeded",
                    "case_budget_exceeded": true,
                    "case_count_estimate": 16,
                    "max_cases": 8
                }
            },
            "cases": {
                "case_count": 1,
                "cases": [{
                    "id": "case_0",
                    "metadata": {
                        "focused_axis_path": "model.thickness"
                    }
                }]
            }
        }),
        serde_json::json!({}),
    )
    .expect("lineage report should carry sweep budget");

    assert_eq!(
        report["sweep_budget"]["status"].as_str(),
        Some("case_budget_exceeded")
    );
    assert_eq!(
        report["sweep_budget"]["case_budget_exceeded"].as_bool(),
        Some(true)
    );
    assert_eq!(report["lineage_complete"].as_bool(), Some(true));
}
