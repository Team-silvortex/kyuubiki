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
