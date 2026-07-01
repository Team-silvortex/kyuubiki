use crate::{
    workflow_executor::run_transform_operator,
    workflow_guard_transforms::{benchmark_transport_pair, evaluate_transport_guard},
};

fn approx_eq(left: Option<f64>, right: f64) {
    let value = left.expect("expected numeric benchmark value");
    assert!(
        (value - right).abs() < 1.0e-9,
        "left={value}, right={right}"
    );
}

#[test]
fn evaluates_transport_guard_as_warn_and_block() {
    let guard = evaluate_transport_guard(
        serde_json::json!({
            "transport_peclet_peak": 250.0,
            "transport_total_flux_peak_magnitude": 3.2
        }),
        serde_json::json!({
            "rules": [
                { "field": "transport_peclet_peak", "comparison": "gt", "threshold": 200.0, "severity": "warn", "label": "peclet_limit" },
                { "field": "transport_total_flux_peak_magnitude", "comparison": "gt", "threshold": 3.0, "severity": "block", "label": "flux_limit" }
            ]
        }),
    )
    .expect("transport guard should evaluate");

    assert_eq!(guard["guard_status"].as_str(), Some("block"));
    assert_eq!(guard["guard_passed"].as_bool(), Some(false));
    assert_eq!(guard["guard_warn_count"].as_u64(), Some(1));
    assert_eq!(guard["guard_block_count"].as_u64(), Some(1));
    assert_eq!(guard["guard_trigger_count"].as_u64(), Some(2));
    assert_eq!(
        guard["guard_recommendation"].as_str(),
        Some("hold_and_review")
    );
}

#[test]
fn benchmarks_transport_pair_by_flux_and_peclet() {
    let benchmark = benchmark_transport_pair(
        serde_json::json!({
            "left": {
                "transport_total_flux_peak_magnitude": 2.0,
                "transport_peclet_peak": 120.0,
                "transport_concentration_span": 0.72
            },
            "right": {
                "transport_total_flux_peak_magnitude": 3.5,
                "transport_peclet_peak": 160.0,
                "transport_concentration_span": 0.66
            }
        }),
        serde_json::json!({
            "left_label": "stable_candidate",
            "right_label": "aggressive_candidate",
            "criteria": [
                { "field": "transport_total_flux_peak_magnitude", "goal": "min", "weight": 2.0 },
                { "field": "transport_peclet_peak", "goal": "min", "weight": 1.0 },
                { "field": "transport_concentration_span", "goal": "max", "weight": 1.0 }
            ]
        }),
    )
    .expect("transport benchmark should succeed");

    approx_eq(benchmark["stable_candidate_score"].as_f64(), 4.0);
    approx_eq(benchmark["aggressive_candidate_score"].as_f64(), 0.0);
    assert_eq!(
        benchmark["benchmark_winner"].as_str(),
        Some("stable_candidate")
    );
    assert_eq!(benchmark["benchmark_criteria_count"].as_u64(), Some(3));
    assert_eq!(benchmark["benchmark_left_win_count"].as_u64(), Some(3));
    assert_eq!(benchmark["benchmark_right_win_count"].as_u64(), Some(0));
}

#[test]
fn runs_transport_guard_and_benchmark_through_transform_executor() {
    let guard = run_transform_operator(
        "transform.evaluate_transport_guard",
        serde_json::json!({
            "transport_peclet_peak": 88.0,
            "transport_total_flux_peak_magnitude": 1.9
        }),
        serde_json::json!({
            "rules": [
                { "field": "transport_peclet_peak", "comparison": "gt", "threshold": 200.0, "severity": "warn" },
                { "field": "transport_total_flux_peak_magnitude", "comparison": "gt", "threshold": 3.0, "severity": "block" }
            ]
        }),
    )
    .expect("transport guard should run through executor");

    assert_eq!(guard["guard_status"].as_str(), Some("pass"));
    assert_eq!(guard["guard_passed"].as_bool(), Some(true));

    let benchmark = run_transform_operator(
        "transform.benchmark_transport_pair",
        serde_json::json!({
            "left": { "transport_peclet_peak": 90.0 },
            "right": { "transport_peclet_peak": 140.0 }
        }),
        serde_json::json!({
            "criteria": [
                { "field": "transport_peclet_peak", "goal": "min", "weight": 2.0 }
            ]
        }),
    )
    .expect("transport benchmark should run through executor");

    approx_eq(benchmark["left_score"].as_f64(), 2.0);
    approx_eq(benchmark["right_score"].as_f64(), 0.0);
    assert_eq!(benchmark["benchmark_winner"].as_str(), Some("left"));
}
