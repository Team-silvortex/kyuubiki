use crate::{
    workflow_quality_sweep_plan::materialize_quality_sweep_expansion,
    workflow_quality_sweep_request::build_quality_parameter_sweep_plan,
};

#[test]
fn carries_validation_repair_strategy_into_quality_sweep_plan() {
    let plan = build_quality_parameter_sweep_plan(
        serde_json::json!({
            "action": "replan",
            "selected_candidate_id": "candidate_validation_blocked",
            "request_payload": {
                "optimization_hint": {
                    "action": "fix_validation_failure",
                    "focus_domain": "validation",
                    "focus_source": "cross_check",
                    "focus_field": "max_temperature"
                },
                "search_space": {
                    "model.max_temperature": {"values": [98.0, 100.0]},
                    "material.density": [2700.0, 7800.0]
                }
            }
        }),
        serde_json::json!({
            "max_axes": 1
        }),
    )
    .expect("validation repair quality sweep plan should build");

    assert_eq!(
        plan["repair_strategy"].as_str(),
        Some("rerun_validation_focused_sweep")
    );
    assert_eq!(
        plan["repair_focus"]["field"].as_str(),
        Some("max_temperature")
    );
    assert_eq!(plan["repair_focus"]["source"].as_str(), Some("cross_check"));
    assert_eq!(
        plan["focused_axis_path"].as_str(),
        Some("model.max_temperature")
    );
}

#[test]
fn materialized_quality_sweep_preserves_validation_repair_metadata() {
    let expansion = materialize_quality_sweep_expansion(
        serde_json::json!({
            "quality_parameter_sweep_plan_contract": "kyuubiki.quality_parameter_sweep_plan/v1",
            "sweep_enabled": true,
            "source_candidate_id": "candidate_validation_blocked",
            "optimization_hint": {
                "action": "fix_validation_failure",
                "focus_field": "max_temperature"
            },
            "focused_axis_path": "model.max_temperature",
            "repair_strategy": "rerun_validation_focused_sweep",
            "repair_focus": {
                "field": "max_temperature",
                "source": "cross_check",
                "domain": "validation"
            },
            "base": {"model": {"max_temperature": 100.0}},
            "axes": [{
                "path": "model.max_temperature",
                "values": [98.0, 100.0]
            }]
        }),
        serde_json::json!({}),
    )
    .expect("validation repair sweep expansion should materialize");

    assert_eq!(
        expansion["payload"]["case_metadata"]["repair_strategy"].as_str(),
        Some("rerun_validation_focused_sweep")
    );
    assert_eq!(
        expansion["payload"]["case_metadata"]["repair_focus"]["source"].as_str(),
        Some("cross_check")
    );
    assert_eq!(
        expansion["payload"]["case_metadata"]["focused_axis_path"].as_str(),
        Some("model.max_temperature")
    );
}
