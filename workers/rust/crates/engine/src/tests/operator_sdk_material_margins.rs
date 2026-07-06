use crate::workflow_executor::run_transform_operator;

#[test]
fn evaluates_material_margins_from_summary_limits() {
    let summary = run_transform_operator(
        "transform.evaluate_material_margins",
        serde_json::json!({
            "max_stress": 180.0,
            "max_temperature": 95.0,
            "max_electric_field": 4.5
        }),
        serde_json::json!({
            "limits": {
                "max_stress": { "limit": 240.0, "direction": "max" },
                "max_temperature": 120.0,
                "max_electric_field": { "limit": 4.0, "direction": "max" }
            }
        }),
    )
    .expect("material margin operator should run");

    assert_eq!(summary["material_constraint_count"].as_u64(), Some(3));
    assert_eq!(summary["material_violation_count"].as_u64(), Some(1));
    assert_eq!(summary["material_status"].as_str(), Some("fail"));
    assert_eq!(
        summary["material_critical_metric"].as_str(),
        Some("max_electric_field")
    );
    assert_eq!(summary["material_failure_index"].as_f64(), Some(1.125));
    assert_eq!(
        summary["material_max_stress_safety_factor"].as_f64(),
        Some(1.3333333333333333)
    );
}

#[test]
fn supports_abs_and_nested_summary_fields() {
    let summary = run_transform_operator(
        "transform.evaluate_material_margins",
        serde_json::json!({
            "summary": {
                "response": {
                    "pressure_ratio": 0.7
                },
                "peak_displacement": -0.012
            }
        }),
        serde_json::json!({
            "output_prefix": "candidate",
            "limits": {
                "response.pressure_ratio": { "limit": 0.8, "direction": "min" },
                "peak_displacement": { "limit": 0.01, "direction": "abs" }
            }
        }),
    )
    .expect("nested material margin operator should run");

    assert_eq!(summary["candidate_status"].as_str(), Some("fail"));
    assert_eq!(
        summary["candidate_critical_metric"].as_str(),
        Some("peak_displacement")
    );
    assert_eq!(summary["candidate_failure_index"].as_f64(), Some(1.2));
    assert_eq!(summary["candidate_violation_count"].as_u64(), Some(2));
}

#[test]
fn ranks_material_candidates_by_feasibility_and_safety_factor() {
    let ranking = run_transform_operator(
        "transform.rank_material_candidates",
        serde_json::json!({
            "candidates": {
                "aluminum_fast": {
                    "material_status": "pass",
                    "material_violation_count": 0,
                    "material_failure_index": 0.82,
                    "material_safety_factor": 1.2195121951,
                    "material_critical_metric": "max_stress"
                },
                "titanium_safe": {
                    "material_status": "pass",
                    "material_violation_count": 0,
                    "material_failure_index": 0.55,
                    "material_safety_factor": 1.8181818182,
                    "material_critical_metric": "max_temperature"
                },
                "polymer_hot": {
                    "material_status": "fail",
                    "material_violation_count": 1,
                    "material_failure_index": 1.4,
                    "material_safety_factor": 0.7142857143,
                    "material_critical_metric": "max_temperature"
                }
            }
        }),
        serde_json::Value::Null,
    )
    .expect("material candidate ranking should run");

    assert_eq!(
        ranking["material_best_candidate_id"].as_str(),
        Some("titanium_safe")
    );
    assert_eq!(ranking["material_feasible_count"].as_u64(), Some(2));
    assert_eq!(ranking["material_candidate_count"].as_u64(), Some(3));
    assert_eq!(
        ranking["material_failure_reasons"]["max_temperature"].as_u64(),
        Some(1)
    );
    assert_eq!(
        ranking["material_rankings"][0]["candidate_id"].as_str(),
        Some("titanium_safe")
    );
}

#[test]
fn ranks_material_candidates_with_partial_summaries() {
    let ranking = run_transform_operator(
        "transform.rank_material_candidates",
        serde_json::json!({
            "baseline": {
                "material_status": "pass",
                "material_violation_count": 0,
                "material_failure_index": 0.9,
                "material_safety_factor": 1.1111111111,
                "material_critical_metric": "max_stress"
            },
            "incomplete": {
                "material_status": "unknown"
            }
        }),
        serde_json::json!({ "include_best_summary": false }),
    )
    .expect("partial material candidates should still rank");

    assert_eq!(
        ranking["material_best_candidate_id"].as_str(),
        Some("baseline")
    );
    assert_eq!(ranking["material_candidate_count"].as_u64(), Some(2));
    assert_eq!(ranking["material_feasible_count"].as_u64(), Some(1));
    assert_eq!(
        ranking["material_failure_reasons"]["unknown"].as_u64(),
        Some(1)
    );
    assert!(ranking.get("material_best_summary").is_none());
    assert!(
        ranking["material_rankings"][1]["failure_index"]
            .as_f64()
            .is_some_and(f64::is_finite)
    );
}

#[test]
fn extracts_material_pareto_frontier_for_multi_objective_candidates() {
    let frontier = run_transform_operator(
        "transform.extract_material_pareto_frontier",
        serde_json::json!({
            "rows": [
                {
                    "candidate_id": "light_hot",
                    "mass": 1.0,
                    "max_temperature": 130.0,
                    "material_status": "pass"
                },
                {
                    "candidate_id": "balanced",
                    "mass": 1.4,
                    "max_temperature": 95.0,
                    "material_status": "pass"
                },
                {
                    "candidate_id": "dominated",
                    "mass": 1.8,
                    "max_temperature": 120.0,
                    "material_status": "pass"
                },
                {
                    "candidate_id": "unsafe",
                    "mass": 0.8,
                    "max_temperature": 90.0,
                    "material_status": "fail"
                }
            ]
        }),
        serde_json::json!({
            "objectives": [
                { "field": "mass", "goal": "min", "weight": 1.0 },
                { "field": "max_temperature", "goal": "min", "weight": 0.01 }
            ]
        }),
    )
    .expect("pareto frontier operator should run");

    assert_eq!(
        frontier["material_pareto_candidate_count"].as_u64(),
        Some(4)
    );
    assert_eq!(frontier["material_pareto_feasible_count"].as_u64(), Some(3));
    assert_eq!(frontier["material_pareto_frontier_count"].as_u64(), Some(2));
    let ids = frontier["material_pareto_frontier"]
        .as_array()
        .expect("frontier should be an array")
        .iter()
        .filter_map(|entry| entry["candidate_id"].as_str())
        .collect::<Vec<_>>();
    assert!(ids.contains(&"light_hot"));
    assert!(ids.contains(&"balanced"));
    assert_eq!(
        frontier["material_pareto_dominated"][0]["candidate_id"].as_str(),
        Some("dominated")
    );
    assert!(
        frontier["material_pareto_dominated"]
            .as_array()
            .expect("dominated should be an array")
            .iter()
            .any(|entry| entry["candidate_id"].as_str() == Some("unsafe")
                && entry["dominated_by"].as_str() == Some("infeasible"))
    );
}

#[test]
fn composes_material_study_envelope_from_multiphysics_summaries() {
    let envelope = run_transform_operator(
        "transform.compose_material_study_envelope",
        serde_json::json!({
            "candidate_id": "alloy_a",
            "summaries": {
                "thermal": {
                    "max_temperature": 96.0,
                    "max_heat_flux": 18.0
                },
                "structural": {
                    "max_stress": 280.0,
                    "max_displacement": 0.006
                },
                "electrostatic": {
                    "max_electric_field": 3.0
                }
            }
        }),
        serde_json::Value::Null,
    )
    .expect("material study envelope should compose");

    assert_eq!(
        envelope["material_envelope_contract"].as_str(),
        Some("kyuubiki.material_study_envelope/v1")
    );
    assert_eq!(
        envelope["material_envelope_candidate_id"].as_str(),
        Some("alloy_a")
    );
    assert_eq!(envelope["material_envelope_domain_count"].as_u64(), Some(3));
    assert_eq!(envelope["material_envelope_metric_count"].as_u64(), Some(5));
    assert_eq!(envelope["material_envelope_status"].as_str(), Some("fail"));
    assert_eq!(
        envelope["material_envelope_critical_metric"].as_str(),
        Some("structural.stress")
    );
    assert!(
        envelope["material_envelope_failure_index"]
            .as_f64()
            .is_some_and(|value| (value - 1.12).abs() < 1.0e-12)
    );
}

#[test]
fn composes_material_study_envelope_with_visible_metric_config() {
    let envelope = run_transform_operator(
        "transform.compose_material_study_envelope",
        serde_json::json!({
            "candidate_id": "foam_b",
            "thermal": { "surface": { "temperature": 88.0 } },
            "transport": { "species_flux": 0.3 }
        }),
        serde_json::json!({
            "output_prefix": "study",
            "metrics": [
                {
                    "source": "thermal",
                    "field": "surface.temperature",
                    "alias": "surface_temperature",
                    "limit": 100.0,
                    "direction": "max",
                    "weight": 2.0
                },
                {
                    "source": "transport",
                    "field": "species_flux",
                    "alias": "species_flux",
                    "limit": 0.5,
                    "direction": "abs",
                    "weight": 1.0
                }
            ]
        }),
    )
    .expect("configured material study envelope should compose");

    assert_eq!(envelope["study_status"].as_str(), Some("pass"));
    assert_eq!(envelope["study_metric_count"].as_u64(), Some(2));
    assert_eq!(
        envelope["study_critical_metric"].as_str(),
        Some("thermal.surface_temperature")
    );
    assert!(
        envelope["study_score"]
            .as_f64()
            .is_some_and(|value| (value - 2.36).abs() < 1.0e-12)
    );
}

#[test]
fn composes_material_study_envelope_batch_from_sweep_rows() {
    let batch = run_transform_operator(
        "transform.compose_material_study_envelope",
        serde_json::json!({
            "rows": [
                {
                    "case_id": "cool_stiff",
                    "summaries": {
                        "thermal": { "max_temperature": 82.0 },
                        "structural": { "max_stress": 160.0 }
                    }
                },
                {
                    "case_id": "hot_light",
                    "summaries": {
                        "thermal": { "max_temperature": 140.0 },
                        "structural": { "max_stress": 120.0 }
                    }
                }
            ]
        }),
        serde_json::Value::Null,
    )
    .expect("material envelope batch should compose");

    assert_eq!(
        batch["material_envelope_batch_contract"].as_str(),
        Some("kyuubiki.material_study_envelope_batch/v1")
    );
    assert_eq!(
        batch["material_envelope_best_candidate_id"].as_str(),
        Some("cool_stiff")
    );
    assert_eq!(batch["material_envelope_candidate_count"].as_u64(), Some(2));
    assert_eq!(
        batch["candidates"]["hot_light"]["material_envelope_status"].as_str(),
        Some("fail")
    );
    assert_eq!(
        batch["candidates"]["cool_stiff"]["material_envelope_candidate_id"].as_str(),
        Some("cool_stiff")
    );
}

#[test]
fn material_envelope_batch_feeds_ranking_and_pareto_chain() {
    let batch = run_transform_operator(
        "transform.compose_material_study_envelope",
        serde_json::json!({
            "rows": [
                {
                    "case_id": "cool_stiff",
                    "summaries": {
                        "thermal": { "max_temperature": 82.0 },
                        "structural": { "max_stress": 160.0 }
                    }
                },
                {
                    "case_id": "warm_safe",
                    "summaries": {
                        "thermal": { "max_temperature": 90.0 },
                        "structural": { "max_stress": 120.0 }
                    }
                },
                {
                    "case_id": "hot_light",
                    "summaries": {
                        "thermal": { "max_temperature": 140.0 },
                        "structural": { "max_stress": 110.0 }
                    }
                }
            ]
        }),
        serde_json::Value::Null,
    )
    .expect("material envelope batch should compose");

    let ranking = run_transform_operator(
        "transform.rank_material_candidates",
        batch.clone(),
        serde_json::json!({
            "margin_prefix": "material_envelope",
            "include_best_summary": false
        }),
    )
    .expect("envelope candidates should rank");
    assert_eq!(
        ranking["material_best_candidate_id"].as_str(),
        Some("cool_stiff")
    );
    assert_eq!(ranking["material_feasible_count"].as_u64(), Some(2));
    assert_eq!(
        ranking["material_failure_reasons"]["thermal.temperature"].as_u64(),
        Some(1)
    );

    let pareto = run_transform_operator(
        "transform.extract_material_pareto_frontier",
        batch,
        serde_json::json!({
            "feasible_field": "material_envelope_status",
            "objectives": [
                { "field": "material_envelope_score", "goal": "min" },
                { "field": "material_envelope_safety_factor", "goal": "max" }
            ]
        }),
    )
    .expect("envelope candidates should feed pareto");
    assert_eq!(pareto["material_pareto_candidate_count"].as_u64(), Some(3));
    assert_eq!(pareto["material_pareto_feasible_count"].as_u64(), Some(2));
    assert!(
        pareto["material_pareto_frontier"]
            .as_array()
            .expect("frontier array")
            .iter()
            .any(|entry| entry["candidate_id"].as_str() == Some("cool_stiff"))
    );
    assert!(
        pareto["material_pareto_dominated"]
            .as_array()
            .expect("dominated array")
            .iter()
            .any(|entry| entry["candidate_id"].as_str() == Some("hot_light")
                && entry["dominated_by"].as_str() == Some("infeasible"))
    );
}
