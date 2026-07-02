use crate::workflow_executor::run_transform_operator;

#[test]
fn scores_parameter_sweep_rows_with_objective_limits() {
    let scored = run_transform_operator(
        "transform.score_parameter_sweep",
        serde_json::json!({
            "rows": [
                {
                    "case_id": "thin_light",
                    "parameters": { "thickness": 0.01 },
                    "max_stress": 140.0,
                    "mass": 2.2
                },
                {
                    "case_id": "balanced",
                    "parameters": { "thickness": 0.02 },
                    "max_stress": 88.0,
                    "mass": 4.8
                },
                {
                    "case_id": "heavy_safe",
                    "parameters": { "thickness": 0.03 },
                    "max_stress": 62.0,
                    "mass": 8.9
                }
            ]
        }),
        serde_json::json!({
            "objectives": [
                {
                    "field": "mass",
                    "goal": "min",
                    "weight": 1.0
                },
                {
                    "field": "max_stress",
                    "goal": "min",
                    "weight": 0.02,
                    "max_allowed": 100.0
                }
            ]
        }),
    )
    .expect("parameter sweep scoring should succeed");

    assert_eq!(scored["scored_count"].as_u64(), Some(3));
    assert_eq!(scored["best"]["case_id"].as_str(), Some("balanced"));
    assert_eq!(scored["best"]["objective_feasible"].as_bool(), Some(true));
    let scored_rows = scored["scored_rows"]
        .as_array()
        .expect("scored rows should be an array");
    assert_eq!(scored_rows[2]["case_id"].as_str(), Some("thin_light"));
    assert_eq!(scored_rows[2]["objective_feasible"].as_bool(), Some(false));
}

#[test]
fn maps_scored_parameter_sweep_rows_to_quality_candidates() {
    let candidates = run_transform_operator(
        "transform.map_parameter_sweep_scores_to_quality_candidates",
        serde_json::json!({
            "best": {
                "case_id": "balanced"
            },
            "scored_rows": [
                {
                    "case_id": "balanced",
                    "parameters": { "thickness": 0.02 },
                    "objective_score": -5.0,
                    "objective_feasible": true
                },
                {
                    "case_id": "thin_light",
                    "parameters": { "thickness": 0.01 },
                    "objective_score": -1000000000002.0,
                    "objective_feasible": false
                }
            ]
        }),
        serde_json::json!({
            "quality_domain": "material_sweep"
        }),
    )
    .expect("scored sweep rows should map to quality candidates");

    assert_eq!(candidates["candidate_count"].as_u64(), Some(2));
    assert_eq!(candidates["source_best_case_id"].as_str(), Some("balanced"));
    assert_eq!(
        candidates["candidates"]["balanced"]["qualities"]["material_sweep"]
            ["material_sweep_quality_ready"]
            .as_bool(),
        Some(true)
    );
    assert_eq!(
        candidates["candidates"]["thin_light"]["qualities"]["material_sweep"]
            ["material_sweep_quality_ready"]
            .as_bool(),
        Some(false)
    );
}

#[test]
fn maps_scored_parameter_sweep_rows_without_source_rows() {
    let candidates = run_transform_operator(
        "transform.map_parameter_sweep_scores_to_quality_candidates",
        serde_json::json!({
            "best": {
                "case_id": "balanced"
            },
            "scored_rows": [
                {
                    "case_id": "balanced",
                    "parameters": { "thickness": 0.02 },
                    "objective_score": -5.0,
                    "objective_feasible": true,
                    "metadata": { "source_candidate_id": "seed" }
                }
            ]
        }),
        serde_json::json!({
            "quality_domain": "material_sweep",
            "include_source_row": false
        }),
    )
    .expect("scored sweep rows should map without source rows");

    let balanced = &candidates["candidates"]["balanced"];
    assert_eq!(balanced["parameters"]["thickness"].as_f64(), Some(0.02));
    assert!(balanced.get("source_row").is_none());
    assert_eq!(
        balanced["qualities"]["material_sweep"]["material_sweep_quality_ready"].as_bool(),
        Some(true)
    );
}

#[test]
fn score_parameter_sweep_reports_missing_numeric_objective_fields() {
    let error = run_transform_operator(
        "transform.score_parameter_sweep",
        serde_json::json!({
            "rows": [
                {
                    "case_id": "candidate_without_mass"
                }
            ]
        }),
        serde_json::json!({
            "objectives": [
                {
                    "field": "mass",
                    "goal": "min"
                }
            ]
        }),
    )
    .expect_err("missing numeric objective field should fail");

    assert!(error.contains("missing numeric field mass"));
}

#[test]
fn score_parameter_sweep_rejects_unsupported_objective_goals() {
    let error = run_transform_operator(
        "transform.score_parameter_sweep",
        serde_json::json!({
            "rows": [
                {
                    "case_id": "candidate",
                    "mass": 1.0
                }
            ]
        }),
        serde_json::json!({
            "objectives": [
                {
                    "field": "mass",
                    "goal": "median"
                }
            ]
        }),
    )
    .expect_err("unsupported objective goal should fail");

    assert!(error.contains("unsupported objective goal: median"));
}
