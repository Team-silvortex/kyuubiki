use crate::{
    magnetostatic_quality::score_magnetostatic_quality,
    workflow_executor::run_transform_operator,
    workflow_guard_transforms::{benchmark_magnetostatic_pair, evaluate_magnetostatic_guard},
};

fn approx_eq(left: Option<f64>, right: f64) {
    let value = left.expect("expected numeric benchmark value");
    assert!(
        (value - right).abs() < 1.0e-9,
        "left={value}, right={right}"
    );
}

#[test]
fn evaluates_magnetostatic_guard_as_block() {
    let guard = evaluate_magnetostatic_guard(
        serde_json::json!({
            "magnetostatic_field_peak_magnitude": 13.0,
            "total_stored_energy": 9.5
        }),
        serde_json::json!({
            "rules": [
                { "field": "magnetostatic_field_peak_magnitude", "threshold": 12.0, "severity": "block", "label": "h_peak" },
                { "field": "total_stored_energy", "threshold": 20.0, "severity": "warn" }
            ]
        }),
    )
    .expect("magnetostatic guard should succeed");

    assert_eq!(guard["guard_status"].as_str(), Some("block"));
    assert_eq!(guard["guard_block_count"].as_u64(), Some(1));
    assert_eq!(guard["guard_trigger_count"].as_u64(), Some(1));
}

#[test]
fn benchmarks_magnetostatic_pair_by_field_and_energy() {
    let benchmark = benchmark_magnetostatic_pair(
        serde_json::json!({
            "left": {
                "magnetostatic_field_peak_magnitude": 11.0,
                "total_stored_energy": 7.0
            },
            "right": {
                "magnetostatic_field_peak_magnitude": 13.0,
                "total_stored_energy": 9.5
            }
        }),
        serde_json::json!({
            "left_label": "candidate_a",
            "right_label": "candidate_b",
            "criteria": [
                { "field": "magnetostatic_field_peak_magnitude", "goal": "min", "weight": 2.0 },
                { "field": "total_stored_energy", "goal": "min", "weight": 1.0 }
            ]
        }),
    )
    .expect("magnetostatic benchmark should succeed");

    assert_eq!(benchmark["benchmark_winner"].as_str(), Some("candidate_a"));
    approx_eq(benchmark["candidate_a_score"].as_f64(), 3.0);
    approx_eq(benchmark["candidate_b_score"].as_f64(), 0.0);
}

#[test]
fn scores_magnetostatic_quality_with_field_flux_energy_and_current_terms() {
    let quality = score_magnetostatic_quality(
        serde_json::json!({
            "magnetostatic_field_peak_magnitude": 9.0,
            "magnetostatic_flux_peak_magnitude": 12.0,
            "magnetostatic_energy_density_peak": 4.0,
            "magnetostatic_current_density_sum": 5.0
        }),
        serde_json::json!({
            "targets": {
                "magnetostatic_field_peak_magnitude": 12.0,
                "magnetostatic_flux_peak_magnitude": 16.0,
                "magnetostatic_energy_density_peak": 8.0,
                "magnetostatic_current_density_sum": 10.0
            },
            "max_ready_score": 8.0
        }),
    )
    .expect("magnetostatic quality should score");

    assert_eq!(
        quality["magnetostatic_quality_contract"].as_str(),
        Some("kyuubiki.magnetostatic_quality_score/v1")
    );
    assert_eq!(quality["magnetostatic_quality_ready"].as_bool(), Some(true));
    assert_eq!(
        quality["magnetostatic_quality_grade"].as_str(),
        Some("good")
    );
    assert_eq!(
        quality["magnetostatic_quality_missing_metric_count"].as_u64(),
        Some(0)
    );
    approx_eq(quality["magnetostatic_quality_score"].as_f64(), 5.25);
}

#[test]
fn blocks_magnetostatic_quality_when_required_metrics_are_missing() {
    let quality = score_magnetostatic_quality(
        serde_json::json!({
            "magnetostatic_field_peak_magnitude": 8.0
        }),
        serde_json::json!({}),
    )
    .expect("magnetostatic quality should report missing metrics");

    assert_eq!(
        quality["magnetostatic_quality_ready"].as_bool(),
        Some(false)
    );
    assert_eq!(
        quality["magnetostatic_quality_grade"].as_str(),
        Some("block")
    );
    assert_eq!(
        quality["magnetostatic_quality_missing_metric_count"].as_u64(),
        Some(3)
    );
}

#[test]
fn runs_magnetostatic_quality_through_transform_executor() {
    let quality = run_transform_operator(
        "transform.score_magnetostatic_quality",
        serde_json::json!({
            "magnetostatic_field_peak_magnitude": 2.0,
            "magnetostatic_flux_peak_magnitude": 3.0,
            "magnetostatic_energy_density_peak": 1.0,
            "magnetostatic_current_density_sum": 1.5
        }),
        serde_json::json!({
            "max_ready_score": 8.0
        }),
    )
    .expect("magnetostatic quality should run through executor");

    assert_eq!(quality["magnetostatic_quality_ready"].as_bool(), Some(true));
    assert_eq!(
        quality["magnetostatic_quality_grade"].as_str(),
        Some("excellent")
    );
    assert_eq!(
        quality["magnetostatic_quality_term_count"].as_u64(),
        Some(4)
    );
}

#[test]
fn runs_magnetostatic_guard_and_benchmark_through_transform_executor() {
    let guard = run_transform_operator(
        "transform.evaluate_magnetostatic_guard",
        serde_json::json!({
            "magnetostatic_field_peak_magnitude": 8.0
        }),
        serde_json::json!({
            "rules": [
                { "field": "magnetostatic_field_peak_magnitude", "threshold": 12.0, "severity": "warn" }
            ]
        }),
    )
    .expect("magnetostatic guard should run through executor");

    assert_eq!(guard["guard_status"].as_str(), Some("pass"));
    assert_eq!(guard["guard_passed"].as_bool(), Some(true));

    let benchmark = run_transform_operator(
        "transform.benchmark_magnetostatic_pair",
        serde_json::json!({
            "left": { "magnetostatic_field_peak_magnitude": 8.0 },
            "right": { "magnetostatic_field_peak_magnitude": 10.0 }
        }),
        serde_json::json!({
            "criteria": [
                { "field": "magnetostatic_field_peak_magnitude", "goal": "min", "weight": 2.0 }
            ]
        }),
    )
    .expect("magnetostatic benchmark should run through executor");

    approx_eq(benchmark["left_score"].as_f64(), 2.0);
    approx_eq(benchmark["right_score"].as_f64(), 0.0);
    assert_eq!(benchmark["benchmark_winner"].as_str(), Some("left"));
}
