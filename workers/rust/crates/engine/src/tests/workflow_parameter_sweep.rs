use crate::workflow_executor::run_transform_operator;

#[test]
fn expands_parameter_sweep_cases_through_sdk_registry() {
    let expanded = run_transform_operator(
        "transform.expand_parameter_sweep",
        serde_json::json!({
            "base": {
                "nodes": [
                    { "id": "n0", "x": 0.0, "y": 0.0 },
                    { "id": "n1", "x": 1.0, "y": 0.0 }
                ],
                "elements": [
                    {
                        "id": "e0",
                        "thickness": 0.01,
                        "youngs_modulus": 70000000000.0
                    }
                ],
                "material": {
                    "density": 2700.0
                }
            },
            "axes": [
                {
                    "label": "thickness",
                    "path": "elements.0.thickness",
                    "values": [0.01, 0.02]
                },
                {
                    "label": "density",
                    "path": "material.density",
                    "values": [2700.0, 7800.0]
                }
            ]
        }),
        serde_json::json!({
            "id_prefix": "material_panel",
            "max_cases": 8,
            "case_metadata": {
                "source": "unit-test"
            }
        }),
    )
    .expect("parameter sweep should expand");

    assert_eq!(expanded["case_count"].as_u64(), Some(4));
    assert_eq!(expanded["axis_count"].as_u64(), Some(2));
    let cases = expanded["cases"]
        .as_array()
        .expect("expanded cases should be an array");
    assert_eq!(cases.len(), 4);
    assert_eq!(cases[0]["id"].as_str(), Some("material_panel_0"));
    assert_eq!(cases[0]["parameters"]["thickness"].as_f64(), Some(0.01));
    assert_eq!(cases[0]["parameters"]["density"].as_f64(), Some(2700.0));
    assert_eq!(
        cases[0]["model"]["elements"][0]["thickness"].as_f64(),
        Some(0.01)
    );
    assert_eq!(
        cases[3]["model"]["material"]["density"].as_f64(),
        Some(7800.0)
    );
    assert_eq!(cases[0]["metadata"]["source"].as_str(), Some("unit-test"));
    assert!(
        cases[3]["label"]
            .as_str()
            .expect("case label should exist")
            .contains("density=7800")
    );
}

#[test]
fn rejects_parameter_sweep_above_configured_case_limit() {
    let error = run_transform_operator(
        "transform.expand_parameter_sweep",
        serde_json::json!({
            "base": { "value": 0 },
            "axes": [
                { "path": "value", "values": [1, 2, 3] },
                { "path": "value", "values": [4, 5, 6] }
            ]
        }),
        serde_json::json!({
            "max_cases": 4
        }),
    )
    .expect_err("oversized parameter sweep should fail");

    assert!(error.contains("above max_cases 4"));
}

#[test]
fn skips_disabled_quality_sweep_expansion_through_sdk_registry() {
    let expanded = run_transform_operator(
        "transform.expand_parameter_sweep",
        serde_json::json!({
            "quality_sweep_expansion_contract": "kyuubiki.quality_sweep_expansion/v1",
            "expansion_enabled": false,
            "reason": "stop",
            "payload": null,
            "config": null
        }),
        serde_json::json!({}),
    )
    .expect("disabled quality sweep should produce an empty sweep result");

    assert_eq!(expanded["case_count"].as_u64(), Some(0));
    assert_eq!(expanded["axis_count"].as_u64(), Some(0));
    assert_eq!(expanded["sweep_enabled"].as_bool(), Some(false));
    assert_eq!(expanded["sweep_action"].as_str(), Some("stop"));
    assert_eq!(expanded["cases"].as_array().map(Vec::len), Some(0));
}

#[test]
fn summarizes_parameter_sweep_case_results_through_sdk_registry() {
    let summarized = run_transform_operator(
        "transform.summarize_parameter_sweep",
        serde_json::json!({
            "cases": [
                {
                    "id": "material_panel_0",
                    "parameters": { "thickness": 0.01, "density": 2700.0 },
                    "metadata": { "round": "seed" },
                    "summary": {
                        "max_stress": 120.0,
                        "mass": 2.7,
                        "note": "light"
                    }
                },
                {
                    "id": "material_panel_1",
                    "parameters": { "thickness": 0.02, "density": 7800.0 },
                    "summary": {
                        "max_stress": 82.0,
                        "mass": 15.6,
                        "note": "heavy"
                    }
                }
            ]
        }),
        serde_json::json!({
            "fields": ["max_stress", "mass"]
        }),
    )
    .expect("parameter sweep summaries should collect");

    assert_eq!(summarized["row_count"].as_u64(), Some(2));
    let rows = summarized["rows"]
        .as_array()
        .expect("summary rows should be an array");
    assert_eq!(rows[0]["case_id"].as_str(), Some("material_panel_0"));
    assert_eq!(rows[0]["parameters"]["density"].as_f64(), Some(2700.0));
    assert_eq!(rows[0]["metadata"]["round"].as_str(), Some("seed"));
    assert_eq!(rows[1]["max_stress"].as_f64(), Some(82.0));
    assert_eq!(
        summarized["numeric_columns"]["max_stress"]["min"].as_f64(),
        Some(82.0)
    );
    assert_eq!(
        summarized["numeric_columns"]["mass"]["mean"].as_f64(),
        Some(9.15)
    );
}

#[test]
fn summarizes_parameter_sweep_can_omit_metadata() {
    let summarized = run_transform_operator(
        "transform.summarize_parameter_sweep",
        serde_json::json!({
            "cases": [
                {
                    "id": "case_a",
                    "parameters": { "thickness": 0.01 },
                    "metadata": { "source_candidate_id": "seed" },
                    "summary": { "mass": 2.7 }
                }
            ]
        }),
        serde_json::json!({
            "fields": ["mass"],
            "include_metadata": false
        }),
    )
    .expect("parameter sweep summary should allow metadata omission");

    let rows = summarized["rows"]
        .as_array()
        .expect("summary rows should be an array");
    assert_eq!(rows[0]["case_id"].as_str(), Some("case_a"));
    assert_eq!(rows[0]["parameters"]["thickness"].as_f64(), Some(0.01));
    assert!(rows[0].get("metadata").is_none());
    assert_eq!(rows[0]["mass"].as_f64(), Some(2.7));
}

#[test]
fn summarizes_parameter_sweep_can_omit_parameters() {
    let summarized = run_transform_operator(
        "transform.summarize_parameter_sweep",
        serde_json::json!({
            "cases": [
                {
                    "id": "case_a",
                    "parameters": { "thickness": 0.01 },
                    "metadata": { "source_candidate_id": "seed" },
                    "summary": { "mass": 2.7 }
                }
            ]
        }),
        serde_json::json!({
            "fields": ["mass"],
            "include_parameters": false
        }),
    )
    .expect("parameter sweep summary should allow parameter omission");

    let rows = summarized["rows"]
        .as_array()
        .expect("summary rows should be an array");
    assert_eq!(rows[0]["case_id"].as_str(), Some("case_a"));
    assert!(rows[0].get("parameters").is_none());
    assert_eq!(
        rows[0]["metadata"]["source_candidate_id"].as_str(),
        Some("seed")
    );
    assert_eq!(rows[0]["mass"].as_f64(), Some(2.7));
}

#[test]
fn joins_distributed_parameter_sweep_results_before_summary() {
    let joined = run_transform_operator(
        "transform.join_parameter_sweep_results",
        serde_json::json!({
            "cases": [
                {
                    "id": "case_a",
                    "parameters": { "viscosity": 0.8 },
                    "model": { "tag": "a" }
                },
                {
                    "id": "case_b",
                    "parameters": { "viscosity": 1.2 },
                    "model": { "tag": "b" }
                }
            ],
            "results": [
                {
                    "case_id": "case_b",
                    "status": "ok",
                    "quality": {
                        "cfd_quality_score": 4.8,
                        "cfd_quality_ready": true
                    }
                },
                {
                    "case_id": "case_a",
                    "status": "ok",
                    "quality": {
                        "cfd_quality_score": 3.1,
                        "cfd_quality_ready": true
                    }
                }
            ]
        }),
        serde_json::json!({
            "summary_field": "quality"
        }),
    )
    .expect("distributed sweep results should join back to cases");

    assert_eq!(joined["case_count"].as_u64(), Some(2));
    assert_eq!(joined["joined_summary_count"].as_u64(), Some(2));
    assert_eq!(joined["missing_summary_count"].as_u64(), Some(0));
    assert_eq!(
        joined["cases"][0]["summary"]["cfd_quality_score"].as_f64(),
        Some(3.1)
    );
    assert_eq!(joined["cases"][1]["result_status"].as_str(), Some("ok"));

    let summarized = run_transform_operator(
        "transform.summarize_parameter_sweep",
        joined,
        serde_json::json!({
            "fields": ["cfd_quality_score"]
        }),
    )
    .expect("joined cases should summarize");
    assert_eq!(summarized["row_count"].as_u64(), Some(2));
    assert_eq!(
        summarized["numeric_columns"]["cfd_quality_score"]["min"].as_f64(),
        Some(3.1)
    );
}

#[test]
fn strict_join_reports_missing_parameter_sweep_results() {
    let error = run_transform_operator(
        "transform.join_parameter_sweep_results",
        serde_json::json!({
            "cases": [
                { "id": "case_a" },
                { "id": "case_b" }
            ],
            "results": [
                {
                    "case_id": "case_a",
                    "summary": { "objective": 1.0 }
                }
            ]
        }),
        serde_json::json!({
            "strict": true
        }),
    )
    .expect_err("strict join should reject missing results");

    assert!(error.contains("missing summaries for 1 case"));
}

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
