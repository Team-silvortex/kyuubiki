use crate::{
    workflow_executor::run_transform_operator,
    workflow_quality_objective::build_quality_parameter_sweep_plan,
    workflow_quality_sweep_plan::materialize_quality_sweep_expansion,
};

fn approx_eq(left: Option<f64>, right: f64) {
    let value = left.expect("expected numeric value");
    assert!(
        (value - right).abs() < 1.0e-9,
        "left={value}, right={right}"
    );
}

#[test]
fn builds_quality_parameter_sweep_plan_from_next_round_request() {
    let plan = build_quality_parameter_sweep_plan(
        serde_json::json!({
            "quality_next_round_contract": "kyuubiki.quality_next_round_request/v1",
            "action": "continue",
            "selected_candidate_id": "candidate_b",
            "target_score": 2.0,
            "request_payload": {
                "max_candidates": 12,
                "search_space": {
                    "elements.0.thickness": {"min": 0.01, "max": 0.03},
                    "material.density": [2700.0, 7800.0]
                }
            }
        }),
        serde_json::json!({
            "samples_per_axis": 3,
            "id_prefix": "quality_candidate",
            "base": {"elements": [{"thickness": 0.02}]}
        }),
    )
    .expect("quality parameter sweep plan should build");

    assert_eq!(
        plan["quality_parameter_sweep_plan_contract"].as_str(),
        Some("kyuubiki.quality_parameter_sweep_plan/v1")
    );
    assert_eq!(plan["sweep_enabled"].as_bool(), Some(true));
    assert_eq!(plan["source_candidate_id"].as_str(), Some("candidate_b"));
    assert_eq!(plan["id_prefix"].as_str(), Some("quality_candidate"));
    assert_eq!(plan["case_count_estimate"].as_u64(), Some(6));
    assert_eq!(plan["axes"].as_array().map(Vec::len), Some(2));
}

#[test]
fn runs_quality_parameter_sweep_plan_through_transform_executor() {
    let plan = run_transform_operator(
        "transform.build_quality_parameter_sweep_plan",
        serde_json::json!({
            "action": "continue",
            "selected_candidate_id": "candidate_a",
            "request_payload": {
                "search_space": {
                    "model.thickness": {"values": [0.01, 0.02]}
                }
            }
        }),
        serde_json::json!({"base": {"model": {"thickness": 0.01}}}),
    )
    .expect("quality parameter sweep plan should run through executor");

    assert_eq!(plan["sweep_enabled"].as_bool(), Some(true));
    assert_eq!(plan["case_count_estimate"].as_u64(), Some(2));
    assert_eq!(plan["axes"][0]["path"].as_str(), Some("model.thickness"));
}

#[test]
fn materializes_quality_sweep_expansion_payload() {
    let expansion = materialize_quality_sweep_expansion(
        serde_json::json!({
            "quality_parameter_sweep_plan_contract": "kyuubiki.quality_parameter_sweep_plan/v1",
            "sweep_enabled": true,
            "source_candidate_id": "candidate_b",
            "id_prefix": "quality_candidate",
            "max_cases": 12,
            "case_count_estimate": 2,
            "base": {"model": {"thickness": 0.01}},
            "axes": [{
                "label": "thickness",
                "path": "model.thickness",
                "values": [0.01, 0.02]
            }]
        }),
        serde_json::json!({}),
    )
    .expect("quality sweep expansion should materialize");

    assert_eq!(
        expansion["quality_sweep_expansion_contract"].as_str(),
        Some("kyuubiki.quality_sweep_expansion/v1")
    );
    assert_eq!(expansion["expansion_enabled"].as_bool(), Some(true));
    assert_eq!(
        expansion["payload"]["axes"][0]["path"].as_str(),
        Some("model.thickness")
    );
    assert_eq!(
        expansion["config"]["id_prefix"].as_str(),
        Some("quality_candidate")
    );
    approx_eq(expansion["config"]["max_cases"].as_f64(), 12.0);
}

#[test]
fn runs_quality_sweep_expansion_through_transform_executor() {
    let expansion = run_transform_operator(
        "transform.materialize_quality_sweep_expansion",
        serde_json::json!({
            "sweep_enabled": true,
            "base": {"model": {"thickness": 0.01}},
            "axes": [{"path": "model.thickness", "values": [0.01, 0.02]}]
        }),
        serde_json::json!({"id_prefix": "q"}),
    )
    .expect("quality sweep expansion should run through executor");

    assert_eq!(expansion["expansion_enabled"].as_bool(), Some(true));
    assert_eq!(expansion["config"]["id_prefix"].as_str(), Some("q"));
}

#[test]
fn expands_materialized_quality_sweep_through_parameter_sweep_operator() {
    let expansion = run_transform_operator(
        "transform.materialize_quality_sweep_expansion",
        serde_json::json!({
            "quality_parameter_sweep_plan_contract": "kyuubiki.quality_parameter_sweep_plan/v1",
            "sweep_enabled": true,
            "id_prefix": "quality_candidate",
            "max_cases": 4,
            "base": {
                "elements": [{"thickness": 0.01}],
                "material": {"density": 2700.0}
            },
            "axes": [
                {"path": "elements.0.thickness", "values": [0.01, 0.02]},
                {"path": "material.density", "values": [2700.0, 7800.0]}
            ]
        }),
        serde_json::json!({}),
    )
    .expect("quality sweep expansion should materialize");

    let expanded = run_transform_operator(
        "transform.expand_parameter_sweep",
        expansion,
        serde_json::json!({}),
    )
    .expect("materialized quality sweep should expand directly");

    assert_eq!(expanded["case_count"].as_u64(), Some(4));
    assert_eq!(expanded["axis_count"].as_u64(), Some(2));
    assert_eq!(
        expanded["cases"][0]["id"].as_str(),
        Some("quality_candidate_0")
    );
    assert_eq!(
        expanded["cases"][3]["model"]["material"]["density"].as_f64(),
        Some(7800.0)
    );
}
