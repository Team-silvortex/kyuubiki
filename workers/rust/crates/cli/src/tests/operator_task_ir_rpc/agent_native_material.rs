use super::*;

#[test]
fn operator_task_runtime_executes_agent_native_material_thermal_shock() {
    let result = run_operator_task_ir(&serde_json::json!({
        "mode": OPERATOR_TASK_MODE_EXECUTE,
        "task_ir": thermal_shock_operator_task_ir()
    }))
    .expect("agent-native builtin should execute");

    assert_eq!(result["operator_task_ir_status"], "executed");
    assert_eq!(
        result["execution_runtime_status"],
        "agent_native_builtin_executed"
    );
    assert!(result["blocked_stage"].is_null());
    assert!(result["package_fetch_request"].is_null());
    assert_eq!(result["execution_plan"][2]["status"], "skipped");
    assert_eq!(result["execution_plan"][4]["stage"], "dispatch_entrypoint");
    assert_eq!(result["execution_plan"][4]["status"], "complete");
    assert_eq!(
        result["result"]["material_thermal_shock_candidate_count"],
        2
    );
    assert_eq!(result["result"]["material_thermal_shock_pass_count"], 1);
    assert_eq!(
        result["result"]["material_thermal_shock_best_candidate_id"],
        "alloy"
    );
    assert_eq!(
        result["result"]["material_thermal_shock_assessments"][1]["thermal_shock_status"],
        "fail"
    );
}

#[test]
fn operator_task_runtime_executes_agent_native_material_margins() {
    let task = transform_operator_task_ir(
        "agent-native-material-margins",
        "transform.evaluate_material_margins",
        "material_margin",
        serde_json::json!({
            "max_stress": 225.0,
            "max_temperature": 80.0
        }),
        serde_json::json!({
            "limits": {
                "max_stress": { "limit": 200.0 },
                "max_temperature": { "limit": 120.0 }
            }
        }),
    );

    let result = run_operator_task_ir(&serde_json::json!({
        "mode": OPERATOR_TASK_MODE_EXECUTE,
        "task_ir": task
    }))
    .expect("agent-native material margins should execute");

    assert_eq!(result["operator_task_ir_status"], "executed");
    assert_eq!(result["result"]["material_constraint_count"], 2);
    assert_eq!(result["result"]["material_violation_count"], 1);
    assert_eq!(result["result"]["material_status"], "fail");
    assert_eq!(result["result"]["material_critical_metric"], "max_stress");
    assert_eq!(result["result"]["material_failure_index"], 1.125);
}

#[test]
fn operator_task_runtime_executes_agent_native_material_ranking() {
    let task = transform_operator_task_ir(
        "agent-native-material-ranking",
        "transform.rank_material_candidates",
        "material_candidate_rank",
        serde_json::json!({
            "candidates": {
                "aluminum": {
                    "material_status": "pass",
                    "material_violation_count": 0,
                    "material_failure_index": 0.82,
                    "material_safety_factor": 1.21,
                    "material_critical_metric": "max_stress"
                },
                "titanium": {
                    "material_status": "pass",
                    "material_violation_count": 0,
                    "material_failure_index": 0.55,
                    "material_safety_factor": 1.81,
                    "material_critical_metric": "max_temperature"
                },
                "polymer": {
                    "material_status": "fail",
                    "material_violation_count": 1,
                    "material_failure_index": 1.4,
                    "material_safety_factor": 0.71,
                    "material_critical_metric": "max_temperature"
                }
            }
        }),
        serde_json::json!({}),
    );

    let result = run_operator_task_ir(&serde_json::json!({
        "mode": OPERATOR_TASK_MODE_EXECUTE,
        "task_ir": task
    }))
    .expect("agent-native material ranking should execute");

    assert_eq!(result["operator_task_ir_status"], "executed");
    assert_eq!(result["result"]["material_candidate_count"], 3);
    assert_eq!(result["result"]["material_feasible_count"], 2);
    assert_eq!(result["result"]["material_best_candidate_id"], "titanium");
    assert_eq!(
        result["result"]["material_failure_reasons"]["max_temperature"],
        1
    );
    assert_eq!(
        result["result"]["material_rankings"][0]["candidate_id"],
        "titanium"
    );
}

#[test]
fn operator_task_runtime_executes_agent_native_material_scoring() {
    let task = transform_operator_task_ir(
        "agent-native-material-scoring",
        "transform.score_material_candidates",
        "material_candidate_score",
        serde_json::json!({
            "candidates": {
                "aluminum": {
                    "mass": 1.8,
                    "cost": 4.0,
                    "material_safety_factor": 1.3,
                    "material_status": "pass"
                },
                "titanium": {
                    "mass": 2.4,
                    "cost": 10.0,
                    "material_safety_factor": 2.0,
                    "material_status": "pass"
                },
                "polymer": {
                    "mass": 1.0,
                    "cost": 2.0,
                    "material_safety_factor": 0.7,
                    "material_status": "fail"
                }
            }
        }),
        serde_json::json!({
            "criteria": [
                { "field": "mass", "goal": "min", "weight": 0.35 },
                { "field": "cost", "goal": "min", "weight": 0.15 },
                { "field": "material_safety_factor", "goal": "max", "weight": 0.5 }
            ]
        }),
    );

    let result = run_operator_task_ir(&serde_json::json!({
        "mode": OPERATOR_TASK_MODE_EXECUTE,
        "task_ir": task
    }))
    .expect("agent-native material scoring should execute");

    assert_eq!(result["operator_task_ir_status"], "executed");
    assert_eq!(result["result"]["material_score_candidate_count"], 3);
    assert_eq!(result["result"]["material_score_feasible_count"], 2);
    assert_eq!(
        result["result"]["material_score_best_candidate_id"],
        "titanium"
    );
    assert!(
        result["result"]["material_score_best_score"]
            .as_f64()
            .expect("best score should be numeric")
            > 0.0
    );
    assert_eq!(
        result["result"]["material_score_rankings"][0]["candidate_id"],
        "titanium"
    );
    assert_eq!(
        result["result"]["material_score_rankings"][1]["candidate_id"],
        "aluminum"
    );
    assert_eq!(
        result["result"]["material_score_rankings"][2]["candidate_id"],
        "polymer"
    );
    assert_eq!(
        result["result"]["material_score_rankings"][0]["criteria_breakdown"]
            .as_array()
            .expect("criteria breakdown should be an array")
            .len(),
        3
    );
    assert_eq!(
        result["result"]["material_score_ranges"]["mass"]["min"],
        1.0
    );
    assert_eq!(
        result["result"]["material_score_ranges"]["mass"]["max"],
        2.4
    );
    assert_eq!(
        result["result"]["material_score_ranges"]["cost"]["min"],
        2.0
    );
    assert_eq!(
        result["result"]["material_score_ranges"]["material_safety_factor"]["max"],
        2.0
    );
    assert_eq!(
        result["result"]["material_score_policy"]["total_weight"],
        1.0
    );
    assert_eq!(
        result["result"]["material_score_policy"]["infeasible_penalty"],
        1.0
    );
    assert_eq!(
        result["result"]["material_score_policy"]["status_field"],
        "material_status"
    );
}

#[test]
fn operator_task_runtime_applies_custom_material_score_policy() {
    let task = transform_operator_task_ir(
        "agent-native-material-scoring-custom-policy",
        "transform.score_material_candidates",
        "material_candidate_score",
        serde_json::json!({
            "candidates": {
                "baseline": {
                    "score_metric": 0.8,
                    "review_status": "approved"
                },
                "experimental": {
                    "score_metric": 1.0,
                    "review_status": "rejected"
                }
            }
        }),
        serde_json::json!({
            "status_field": "review_status",
            "feasible_status": "approved",
            "infeasible_penalty": 1.2,
            "criteria": [
                { "field": "score_metric", "goal": "max", "weight": 2.0 }
            ]
        }),
    );

    let result = run_operator_task_ir(&serde_json::json!({
        "mode": OPERATOR_TASK_MODE_EXECUTE,
        "task_ir": task
    }))
    .expect("custom material score policy should execute");

    assert_eq!(
        result["result"]["material_score_policy"]["status_field"],
        "review_status"
    );
    assert_eq!(
        result["result"]["material_score_policy"]["feasible_status"],
        "approved"
    );
    assert_eq!(
        result["result"]["material_score_policy"]["infeasible_penalty"],
        1.2
    );
    assert_eq!(
        result["result"]["material_score_policy"]["total_weight"],
        2.0
    );
    assert_eq!(
        result["result"]["material_score_rankings"][0]["candidate_id"],
        "baseline"
    );
    assert_eq!(
        result["result"]["material_score_rankings"][1]["candidate_id"],
        "experimental"
    );
    assert_eq!(
        result["result"]["material_score_rankings"][1]["feasible"],
        false
    );
    assert_eq!(
        result["result"]["material_score_rankings"][1]["weighted_score"],
        1.0
    );
    let final_score = result["result"]["material_score_rankings"][1]["final_score"]
        .as_f64()
        .expect("final score should be numeric");
    assert!((final_score - -0.2).abs() < 1.0e-9);
}

fn assert_material_score_config_error(
    task_id: &str,
    config: serde_json::Value,
    expected_message: &str,
) {
    let task = transform_operator_task_ir(
        task_id,
        "transform.score_material_candidates",
        "material_candidate_score",
        serde_json::json!({
            "candidates": {
                "aluminum": {
                    "mass": 1.8,
                    "material_safety_factor": 1.3,
                    "material_status": "pass"
                }
            }
        }),
        config,
    );

    let error = run_operator_task_ir(&serde_json::json!({
        "mode": OPERATOR_TASK_MODE_EXECUTE,
        "task_ir": task
    }))
    .expect_err("invalid material score config should fail execution");

    assert_eq!(error.code, "operator_task_execution_failed");
    assert_eq!(error.message, expected_message);
}

#[test]
fn operator_task_runtime_reports_missing_material_score_criteria() {
    assert_material_score_config_error(
        "agent-native-material-scoring-missing-criteria",
        serde_json::json!({}),
        "missing_material_score_criteria",
    );
}

#[test]
fn operator_task_runtime_reports_missing_material_score_candidates() {
    let task = transform_operator_task_ir(
        "agent-native-material-scoring-missing-candidates",
        "transform.score_material_candidates",
        "material_candidate_score",
        serde_json::json!({ "candidates": {} }),
        serde_json::json!({
            "criteria": [
                { "field": "mass", "goal": "min", "weight": 1.0 }
            ]
        }),
    );

    let error = run_operator_task_ir(&serde_json::json!({
        "mode": OPERATOR_TASK_MODE_EXECUTE,
        "task_ir": task
    }))
    .expect_err("missing candidates should fail execution");

    assert_eq!(error.code, "operator_task_execution_failed");
    assert_eq!(error.message, "missing_material_candidates");
}

#[test]
fn operator_task_runtime_reports_invalid_material_score_goal() {
    assert_material_score_config_error(
        "agent-native-material-scoring-invalid-goal",
        serde_json::json!({
            "criteria": [
                { "field": "mass", "goal": "near", "weight": 1.0 }
            ]
        }),
        "invalid_material_score_criterion",
    );
}

#[test]
fn operator_task_runtime_reports_invalid_material_score_weight() {
    assert_material_score_config_error(
        "agent-native-material-scoring-invalid-weight",
        serde_json::json!({
            "criteria": [
                { "field": "mass", "goal": "min", "weight": 0.0 }
            ]
        }),
        "invalid_material_score_criterion",
    );
}

#[test]
fn operator_task_runtime_reports_invalid_material_score_policy() {
    assert_material_score_config_error(
        "agent-native-material-scoring-invalid-policy",
        serde_json::json!({
            "infeasible_penalty": -0.1,
            "criteria": [
                { "field": "mass", "goal": "min", "weight": 1.0 }
            ]
        }),
        "invalid_material_score_policy",
    );
}

#[test]
fn operator_task_runtime_reports_missing_material_score_metric() {
    let task = transform_operator_task_ir(
        "agent-native-material-scoring-missing-metric",
        "transform.score_material_candidates",
        "material_candidate_score",
        serde_json::json!({
            "candidates": {
                "aluminum": {
                    "mass": 1.8,
                    "material_status": "pass"
                }
            }
        }),
        serde_json::json!({
            "criteria": [
                { "field": "mass", "goal": "min", "weight": 0.35 },
                { "field": "material_safety_factor", "goal": "max", "weight": 0.65 }
            ]
        }),
    );

    let error = run_operator_task_ir(&serde_json::json!({
        "mode": OPERATOR_TASK_MODE_EXECUTE,
        "task_ir": task
    }))
    .expect_err("missing score metric should fail execution");

    assert_eq!(error.code, "operator_task_execution_failed");
    assert_eq!(error.message, "missing_material_score_metric");
}

#[test]
fn operator_task_runtime_preflights_agent_native_material_thermal_shock_without_execution() {
    let result = run_operator_task_ir(&serde_json::json!({
        "mode": OPERATOR_TASK_MODE_PREFLIGHT,
        "task_ir": thermal_shock_operator_task_ir()
    }))
    .expect("preflight should not dispatch agent-native builtin");

    assert_eq!(
        result["operator_task_ir_status"],
        OPERATOR_TASK_STATUS_VERIFIED_PENDING
    );
    assert!(result.get("result").is_none());
    assert_eq!(
        result["package_fetch_request"]["request_status"],
        "blocked_runtime_not_attached"
    );
}
