use crate::{
    workflow_executor::run_transform_operator,
    workflow_guard_transforms::{benchmark_structural_pair, evaluate_structural_guard},
};

fn approx_eq(left: Option<f64>, right: f64) {
    let value = left.expect("expected numeric benchmark value");
    assert!(
        (value - right).abs() < 1.0e-9,
        "left={value}, right={right}"
    );
}

#[test]
fn evaluates_structural_guard_with_contact_and_stress_rules() {
    let guard = evaluate_structural_guard(
        serde_json::json!({
            "max_displacement": 0.018,
            "max_stress": 265.0,
            "max_contact_force": 42.0,
            "active_contact_count": 1
        }),
        serde_json::json!({
            "rules": [
                { "field": "max_displacement", "comparison": "gt", "threshold": 0.02, "severity": "warn", "label": "serviceability" },
                { "field": "max_stress", "comparison": "gt", "threshold": 250.0, "severity": "block", "label": "stress_limit" },
                { "field": "max_contact_force", "comparison": "gt", "threshold": 50.0, "severity": "warn", "label": "contact_force" }
            ]
        }),
    )
    .expect("structural guard should evaluate");

    assert_eq!(guard["guard_status"].as_str(), Some("block"));
    assert_eq!(guard["guard_passed"].as_bool(), Some(false));
    assert_eq!(guard["guard_warn_count"].as_u64(), Some(0));
    assert_eq!(guard["guard_block_count"].as_u64(), Some(1));
    assert_eq!(guard["guard_trigger_count"].as_u64(), Some(1));
    assert_eq!(
        guard["guard_recommendation"].as_str(),
        Some("hold_and_review")
    );
}

#[test]
fn benchmarks_structural_pair_across_serviceability_and_mass() {
    let benchmark = benchmark_structural_pair(
        serde_json::json!({
            "left": {
                "max_displacement": 0.012,
                "max_stress": 180.0,
                "mass": 14.0,
                "stiffness_margin": 1.4
            },
            "right": {
                "max_displacement": 0.016,
                "max_stress": 165.0,
                "mass": 11.0,
                "stiffness_margin": 1.1
            }
        }),
        serde_json::json!({
            "left_label": "stiff_candidate",
            "right_label": "light_candidate",
            "criteria": [
                { "field": "max_displacement", "goal": "min", "weight": 2.0 },
                { "field": "max_stress", "goal": "min", "weight": 1.0 },
                { "field": "mass", "goal": "min", "weight": 2.0 },
                { "field": "stiffness_margin", "goal": "max", "weight": 1.0 }
            ]
        }),
    )
    .expect("structural benchmark should succeed");

    approx_eq(benchmark["stiff_candidate_score"].as_f64(), 3.0);
    approx_eq(benchmark["light_candidate_score"].as_f64(), 3.0);
    assert_eq!(benchmark["benchmark_winner"].as_str(), Some("tie"));
    assert_eq!(benchmark["benchmark_criteria_count"].as_u64(), Some(4));
    assert_eq!(benchmark["benchmark_left_win_count"].as_u64(), Some(2));
    assert_eq!(benchmark["benchmark_right_win_count"].as_u64(), Some(2));
}

#[test]
fn runs_structural_guard_and_benchmark_through_transform_executor() {
    let guard = run_transform_operator(
        "transform.evaluate_structural_guard",
        serde_json::json!({
            "max_displacement": 0.009,
            "max_stress": 120.0
        }),
        serde_json::json!({
            "rules": [
                { "field": "max_displacement", "comparison": "gt", "threshold": 0.02, "severity": "warn" },
                { "field": "max_stress", "comparison": "gt", "threshold": 250.0, "severity": "block" }
            ]
        }),
    )
    .expect("structural guard should run through executor");

    assert_eq!(guard["guard_status"].as_str(), Some("pass"));
    assert_eq!(guard["guard_passed"].as_bool(), Some(true));

    let benchmark = run_transform_operator(
        "transform.benchmark_structural_pair",
        serde_json::json!({
            "left": { "max_displacement": 0.008 },
            "right": { "max_displacement": 0.011 }
        }),
        serde_json::json!({
            "criteria": [
                { "field": "max_displacement", "goal": "min", "weight": 2.0 }
            ]
        }),
    )
    .expect("structural benchmark should run through executor");

    approx_eq(benchmark["left_score"].as_f64(), 2.0);
    approx_eq(benchmark["right_score"].as_f64(), 0.0);
    assert_eq!(benchmark["benchmark_winner"].as_str(), Some("left"));
}
