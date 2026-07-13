use crate::{
    workflow_quality_sweep_plan::materialize_quality_sweep_expansion,
    workflow_quality_sweep_request::build_quality_parameter_sweep_plan,
};
use serde_json::json;

#[test]
fn coupled_readiness_focus_domain_prioritizes_sweep_axis() {
    let plan = build_quality_parameter_sweep_plan(
        json!({
            "action": "replan",
            "selected_candidate_id": "candidate_coupled_blocked",
            "request_payload": {
                "optimization_hint": {
                    "action": "fix_coupled_readiness",
                    "focus_domain": "electrostatic",
                    "focus_source": "coupled_readiness",
                    "blocking_count": 1
                },
                "coupled_readiness": {
                    "coupled_readiness_contract": "kyuubiki.coupled_readiness/v1",
                    "coupled_readiness_state": "block",
                    "coupled_readiness_recommendation": "hold_and_repair_inputs",
                    "coupled_readiness_blocking_domains": ["electrostatic"]
                },
                "search_space": {
                    "thermal.temperature": [290.0, 310.0],
                    "electrostatic.voltage": [3.0, 5.0],
                    "material.density": [2700.0, 7800.0]
                }
            }
        }),
        json!({
            "max_axes": 1,
            "samples_per_axis": 2
        }),
    )
    .expect("coupled readiness repair sweep plan should build");

    assert_eq!(
        plan["repair_strategy"].as_str(),
        Some("repair_coupled_readiness_sweep")
    );
    assert_eq!(
        plan["focused_axis_path"].as_str(),
        Some("electrostatic.voltage")
    );
    assert_eq!(
        plan["repair_focus"]["domain"].as_str(),
        Some("electrostatic")
    );
    assert_eq!(
        plan["repair_focus"]["readiness_recommendation"].as_str(),
        Some("hold_and_repair_inputs")
    );
    assert_eq!(
        plan["coupled_readiness"]["coupled_readiness_state"].as_str(),
        Some("block")
    );
}

#[test]
fn materialized_quality_sweep_preserves_coupled_readiness_metadata() {
    let expansion = materialize_quality_sweep_expansion(
        json!({
            "quality_parameter_sweep_plan_contract": "kyuubiki.quality_parameter_sweep_plan/v1",
            "sweep_enabled": true,
            "source_candidate_id": "candidate_coupled_blocked",
            "optimization_hint": {
                "action": "fix_coupled_readiness",
                "focus_domain": "electrostatic"
            },
            "coupled_readiness": {
                "coupled_readiness_contract": "kyuubiki.coupled_readiness/v1",
                "coupled_readiness_state": "block",
                "coupled_readiness_recommendation": "hold_and_repair_inputs"
            },
            "focused_axis_path": "electrostatic.voltage",
            "repair_strategy": "repair_coupled_readiness_sweep",
            "repair_focus": {
                "domain": "electrostatic",
                "source": "coupled_readiness"
            },
            "base": {"electrostatic": {"voltage": 3.0}},
            "axes": [{
                "path": "electrostatic.voltage",
                "values": [3.0, 5.0]
            }]
        }),
        json!({}),
    )
    .expect("coupled repair sweep expansion should materialize");

    assert_eq!(
        expansion["payload"]["case_metadata"]["coupled_readiness"]
            ["coupled_readiness_recommendation"]
            .as_str(),
        Some("hold_and_repair_inputs")
    );
    assert_eq!(
        expansion["payload"]["case_metadata"]["repair_strategy"].as_str(),
        Some("repair_coupled_readiness_sweep")
    );
    assert_eq!(
        expansion["payload"]["case_metadata"]["focused_axis_path"].as_str(),
        Some("electrostatic.voltage")
    );
}

#[test]
fn coupled_readiness_warning_uses_review_sweep_strategy() {
    let plan = build_quality_parameter_sweep_plan(
        json!({
            "action": "continue",
            "selected_candidate_id": "candidate_coupled_warning",
            "request_payload": {
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
                    "coupled_readiness_warning_domains": ["thermal"]
                },
                "search_space": {
                    "thermal.temperature": [290.0, 310.0],
                    "electrostatic.voltage": [3.0, 5.0],
                    "material.density": [2700.0, 7800.0]
                }
            }
        }),
        json!({
            "max_axes": 1,
            "samples_per_axis": 2
        }),
    )
    .expect("coupled readiness review sweep plan should build");

    assert_eq!(
        plan["repair_strategy"].as_str(),
        Some("review_coupled_readiness_sweep")
    );
    assert_eq!(
        plan["focused_axis_path"].as_str(),
        Some("thermal.temperature")
    );
    assert_eq!(
        plan["repair_focus"]["readiness_recommendation"].as_str(),
        Some("review_before_next_round")
    );
}

#[test]
fn sweep_plan_repair_focus_uses_top_level_coupled_readiness() {
    let plan = build_quality_parameter_sweep_plan(
        json!({
            "action": "replan",
            "selected_candidate_id": "candidate_top_level_readiness",
            "selected_iteration_hint": {
                "action": "fix_coupled_readiness",
                "focus_domain": "electrostatic",
                "focus_source": "coupled_readiness",
                "blocking_count": 1
            },
            "coupled_readiness": {
                "coupled_readiness_contract": "kyuubiki.coupled_readiness/v1",
                "coupled_readiness_state": "block",
                "coupled_readiness_recommendation": "hold_and_repair_inputs"
            },
            "request_payload": {
                "search_space": {
                    "electrostatic.voltage": [3.0, 5.0],
                    "material.density": [2700.0, 7800.0]
                }
            }
        }),
        json!({"max_axes": 1}),
    )
    .expect("top-level coupled readiness should be accepted by sweep plan");

    assert_eq!(
        plan["repair_focus"]["readiness_state"].as_str(),
        Some("block")
    );
    assert_eq!(
        plan["repair_focus"]["readiness_recommendation"].as_str(),
        Some("hold_and_repair_inputs")
    );
    assert_eq!(
        plan["coupled_readiness"]["coupled_readiness_state"].as_str(),
        Some("block")
    );
}
