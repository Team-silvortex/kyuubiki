use crate::{
    workflow_executor::run_transform_operator,
    workflow_guard_transforms::{benchmark_electrostatic_pair, evaluate_electrostatic_guard},
};

fn approx_eq(left: Option<f64>, right: f64) {
    let value = left.expect("expected numeric benchmark value");
    assert!(
        (value - right).abs() < 1.0e-9,
        "left={value}, right={right}"
    );
}

#[test]
fn evaluates_electrostatic_guard_as_warn() {
    let guard = evaluate_electrostatic_guard(
        serde_json::json!({
            "electrostatic_field_peak_magnitude": 12.5,
            "electrostatic_peak_energy_density": 0.42
        }),
        serde_json::json!({
            "rules": [
                { "field": "electrostatic_field_peak_magnitude", "comparison": "gt", "threshold": 10.0, "severity": "warn", "label": "field_limit" },
                { "field": "electrostatic_peak_energy_density", "comparison": "gt", "threshold": 0.8, "severity": "block", "label": "energy_limit" }
            ]
        }),
    )
    .expect("electrostatic guard should evaluate");

    assert_eq!(guard["guard_status"].as_str(), Some("warn"));
    assert_eq!(guard["guard_passed"].as_bool(), Some(false));
    assert_eq!(guard["guard_warn_count"].as_u64(), Some(1));
    assert_eq!(guard["guard_block_count"].as_u64(), Some(0));
    assert_eq!(
        guard["guard_recommendation"].as_str(),
        Some("review_before_continue")
    );
}

#[test]
fn benchmarks_electrostatic_pair_by_field_and_energy() {
    let benchmark = benchmark_electrostatic_pair(
        serde_json::json!({
            "left": {
                "electrostatic_field_peak_magnitude": 8.0,
                "electrostatic_peak_energy_density": 0.31,
                "electrostatic_potential_span": 4.4
            },
            "right": {
                "electrostatic_field_peak_magnitude": 9.5,
                "electrostatic_peak_energy_density": 0.27,
                "electrostatic_potential_span": 5.1
            }
        }),
        serde_json::json!({
            "left_label": "insulated_candidate",
            "right_label": "high_gradient_candidate",
            "criteria": [
                { "field": "electrostatic_field_peak_magnitude", "goal": "min", "weight": 2.0 },
                { "field": "electrostatic_peak_energy_density", "goal": "min", "weight": 1.0 },
                { "field": "electrostatic_potential_span", "goal": "max", "weight": 1.0 }
            ]
        }),
    )
    .expect("electrostatic benchmark should succeed");

    approx_eq(benchmark["insulated_candidate_score"].as_f64(), 2.0);
    approx_eq(benchmark["high_gradient_candidate_score"].as_f64(), 2.0);
    assert_eq!(benchmark["benchmark_winner"].as_str(), Some("tie"));
    assert_eq!(benchmark["benchmark_left_win_count"].as_u64(), Some(1));
    assert_eq!(benchmark["benchmark_right_win_count"].as_u64(), Some(2));
}

#[test]
fn runs_electrostatic_guard_and_benchmark_through_transform_executor() {
    let guard = run_transform_operator(
        "transform.evaluate_electrostatic_guard",
        serde_json::json!({
            "electrostatic_field_peak_magnitude": 7.2
        }),
        serde_json::json!({
            "rules": [
                { "field": "electrostatic_field_peak_magnitude", "comparison": "gt", "threshold": 10.0, "severity": "warn" }
            ]
        }),
    )
    .expect("electrostatic guard should run through executor");

    assert_eq!(guard["guard_status"].as_str(), Some("pass"));
    assert_eq!(guard["guard_passed"].as_bool(), Some(true));

    let benchmark = run_transform_operator(
        "transform.benchmark_electrostatic_pair",
        serde_json::json!({
            "left": { "electrostatic_field_peak_magnitude": 8.0 },
            "right": { "electrostatic_field_peak_magnitude": 11.0 }
        }),
        serde_json::json!({
            "criteria": [
                { "field": "electrostatic_field_peak_magnitude", "goal": "min", "weight": 2.0 }
            ]
        }),
    )
    .expect("electrostatic benchmark should run through executor");

    approx_eq(benchmark["left_score"].as_f64(), 2.0);
    approx_eq(benchmark["right_score"].as_f64(), 0.0);
    assert_eq!(benchmark["benchmark_winner"].as_str(), Some("left"));
}
