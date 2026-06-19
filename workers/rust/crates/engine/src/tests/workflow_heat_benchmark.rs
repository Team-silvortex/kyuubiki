use crate::{
    workflow_executor::run_transform_operator,
    workflow_guard_transforms::benchmark_coupled_heat_pair,
};

fn approx_eq(left: Option<f64>, right: f64) {
    let value = left.expect("expected numeric benchmark value");
    assert!(
        (value - right).abs() < 1.0e-9,
        "left={value}, right={right}"
    );
}

#[test]
fn benchmarks_coupled_heat_pair_with_min_and_max_goals() {
    let benchmark = benchmark_coupled_heat_pair(
        serde_json::json!({
            "left": {
                "max_temperature": 80.0,
                "max_stress": 120.0,
                "efficiency": 0.82
            },
            "right": {
                "max_temperature": 92.0,
                "max_stress": 110.0,
                "efficiency": 0.76
            }
        }),
        serde_json::json!({
            "left_label": "baseline",
            "right_label": "candidate",
            "criteria": [
                { "field": "max_temperature", "goal": "min", "weight": 2.0 },
                { "field": "max_stress", "goal": "min", "weight": 1.0 },
                { "field": "efficiency", "goal": "max", "weight": 3.0 }
            ]
        }),
    )
    .expect("benchmark should succeed");

    approx_eq(benchmark["baseline_score"].as_f64(), 5.0);
    approx_eq(benchmark["candidate_score"].as_f64(), 1.0);
    assert_eq!(benchmark["benchmark_winner"].as_str(), Some("baseline"));
    approx_eq(benchmark["benchmark_margin"].as_f64(), 4.0);
    assert_eq!(benchmark["benchmark_criteria_count"].as_u64(), Some(3));
    assert_eq!(benchmark["benchmark_left_win_count"].as_u64(), Some(2));
    assert_eq!(benchmark["benchmark_right_win_count"].as_u64(), Some(1));
    assert_eq!(
        benchmark["benchmark_recommendation"].as_str(),
        Some("prefer_baseline")
    );
}

#[test]
fn benchmarks_coupled_heat_pair_allows_ties() {
    let benchmark = benchmark_coupled_heat_pair(
        serde_json::json!({
            "left": { "max_temperature": 80.0 },
            "right": { "max_temperature": 80.0 }
        }),
        serde_json::json!({
            "criteria": [
                { "field": "max_temperature", "goal": "min", "weight": 2.0 }
            ]
        }),
    )
    .expect("benchmark should succeed");

    approx_eq(benchmark["left_score"].as_f64(), 1.0);
    approx_eq(benchmark["right_score"].as_f64(), 1.0);
    assert_eq!(benchmark["benchmark_winner"].as_str(), Some("tie"));
    assert_eq!(benchmark["benchmark_tie_count"].as_u64(), Some(1));
    assert_eq!(
        benchmark["benchmark_recommendation"].as_str(),
        Some("keep_both_under_review")
    );
}

#[test]
fn runs_heat_benchmark_through_transform_executor() {
    let benchmark = run_transform_operator(
        "transform.benchmark_coupled_heat_pair",
        serde_json::json!({
            "left": { "max_flux": 18.0, "stability": 0.9 },
            "right": { "max_flux": 12.0, "stability": 0.8 }
        }),
        serde_json::json!({
            "criteria": [
                { "left_field": "max_flux", "right_field": "max_flux", "field": "max_flux", "goal": "min", "weight": 2.0 },
                { "left_field": "stability", "right_field": "stability", "field": "stability", "goal": "max", "weight": 1.0 }
            ]
        }),
    )
    .expect("transform benchmark should succeed");

    approx_eq(benchmark["left_score"].as_f64(), 1.0);
    approx_eq(benchmark["right_score"].as_f64(), 2.0);
    assert_eq!(benchmark["benchmark_winner"].as_str(), Some("right"));
    let breakdown = benchmark["benchmark_breakdown"]
        .as_array()
        .expect("expected benchmark breakdown");
    assert_eq!(breakdown.len(), 2);
    assert_eq!(breakdown[0]["field"].as_str(), Some("max_flux"));
}
