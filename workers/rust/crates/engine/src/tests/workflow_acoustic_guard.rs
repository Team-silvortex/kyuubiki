use crate::{
    acoustic_quality::score_acoustic_quality,
    workflow_executor::run_transform_operator,
    workflow_guard_transforms::{benchmark_acoustic_pair, evaluate_acoustic_guard},
};

fn approx_eq(left: Option<f64>, right: f64) {
    let value = left.expect("expected numeric benchmark value");
    assert!(
        (value - right).abs() < 1.0e-9,
        "left={value}, right={right}"
    );
}

#[test]
fn evaluates_acoustic_guard_with_spl_and_intensity_rules() {
    let guard = evaluate_acoustic_guard(
        serde_json::json!({
            "max_sound_pressure_level_db": 94.0,
            "max_acoustic_intensity": 0.36,
            "total_damping_loss": 0.08
        }),
        serde_json::json!({
            "rules": [
                { "field": "max_sound_pressure_level_db", "comparison": "gt", "threshold": 90.0, "severity": "warn", "label": "spl_limit" },
                { "field": "max_acoustic_intensity", "comparison": "gt", "threshold": 0.5, "severity": "block", "label": "intensity_limit" }
            ]
        }),
    )
    .expect("acoustic guard should evaluate");

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
fn benchmarks_acoustic_pair_by_spl_intensity_and_damping() {
    let benchmark = benchmark_acoustic_pair(
        serde_json::json!({
            "left": {
                "max_sound_pressure_level_db": 88.0,
                "max_acoustic_intensity": 0.26,
                "total_damping_loss": 0.12
            },
            "right": {
                "max_sound_pressure_level_db": 92.0,
                "max_acoustic_intensity": 0.21,
                "total_damping_loss": 0.18
            }
        }),
        serde_json::json!({
            "left_label": "quiet_candidate",
            "right_label": "damped_candidate",
            "criteria": [
                { "field": "max_sound_pressure_level_db", "goal": "min", "weight": 2.0 },
                { "field": "max_acoustic_intensity", "goal": "min", "weight": 1.0 },
                { "field": "total_damping_loss", "goal": "max", "weight": 1.0 }
            ]
        }),
    )
    .expect("acoustic benchmark should succeed");

    approx_eq(benchmark["quiet_candidate_score"].as_f64(), 2.0);
    approx_eq(benchmark["damped_candidate_score"].as_f64(), 2.0);
    assert_eq!(benchmark["benchmark_winner"].as_str(), Some("tie"));
    assert_eq!(benchmark["benchmark_criteria_count"].as_u64(), Some(3));
    assert_eq!(benchmark["benchmark_left_win_count"].as_u64(), Some(1));
    assert_eq!(benchmark["benchmark_right_win_count"].as_u64(), Some(2));
}

#[test]
fn runs_acoustic_guard_and_benchmark_through_transform_executor() {
    let guard = run_transform_operator(
        "transform.evaluate_acoustic_guard",
        serde_json::json!({
            "max_sound_pressure_level_db": 78.0
        }),
        serde_json::json!({
            "rules": [
                { "field": "max_sound_pressure_level_db", "comparison": "gt", "threshold": 90.0, "severity": "warn" }
            ]
        }),
    )
    .expect("acoustic guard should run through executor");

    assert_eq!(guard["guard_status"].as_str(), Some("pass"));
    assert_eq!(guard["guard_passed"].as_bool(), Some(true));

    let benchmark = run_transform_operator(
        "transform.benchmark_acoustic_pair",
        serde_json::json!({
            "left": { "max_sound_pressure_level_db": 82.0 },
            "right": { "max_sound_pressure_level_db": 86.0 }
        }),
        serde_json::json!({
            "criteria": [
                { "field": "max_sound_pressure_level_db", "goal": "min", "weight": 2.0 }
            ]
        }),
    )
    .expect("acoustic benchmark should run through executor");

    approx_eq(benchmark["left_score"].as_f64(), 2.0);
    approx_eq(benchmark["right_score"].as_f64(), 0.0);
    assert_eq!(benchmark["benchmark_winner"].as_str(), Some("left"));
}

#[test]
fn scores_acoustic_quality_with_spl_intensity_and_damping_terms() {
    let quality = score_acoustic_quality(
        serde_json::json!({
            "max_sound_pressure_level_db": 80.0,
            "max_acoustic_intensity": 0.2,
            "max_pressure_amplitude": 0.5,
            "total_damping_loss": 0.2
        }),
        serde_json::json!({
            "targets": {
                "max_sound_pressure_level_db": 85.0,
                "max_acoustic_intensity": 0.25,
                "max_pressure_amplitude": 1.0,
                "total_damping_loss": 0.1
            },
            "max_ready_score": 7.0
        }),
    )
    .expect("acoustic quality should score");

    assert_eq!(
        quality["acoustic_quality_contract"].as_str(),
        Some("kyuubiki.acoustic_quality_score/v1")
    );
    assert_eq!(quality["acoustic_quality_ready"].as_bool(), Some(true));
    assert_eq!(quality["acoustic_quality_grade"].as_str(), Some("review"));
    assert_eq!(
        quality["acoustic_quality_missing_metric_count"].as_u64(),
        Some(0)
    );
    approx_eq(
        quality["acoustic_quality_score"].as_f64(),
        5.423529411764706,
    );
}

#[test]
fn blocks_acoustic_quality_when_required_metrics_are_missing() {
    let quality = score_acoustic_quality(
        serde_json::json!({
            "max_sound_pressure_level_db": 80.0
        }),
        serde_json::json!({}),
    )
    .expect("acoustic quality should report missing metrics");

    assert_eq!(quality["acoustic_quality_ready"].as_bool(), Some(false));
    assert_eq!(quality["acoustic_quality_grade"].as_str(), Some("block"));
    assert_eq!(
        quality["acoustic_quality_missing_metric_count"].as_u64(),
        Some(3)
    );
}

#[test]
fn runs_acoustic_quality_through_transform_executor() {
    let quality = run_transform_operator(
        "transform.score_acoustic_quality",
        serde_json::json!({
            "max_sound_pressure_level_db": 70.0,
            "max_acoustic_intensity": 0.1,
            "max_pressure_amplitude": 0.25,
            "total_damping_loss": 0.4
        }),
        serde_json::json!({
            "max_ready_score": 7.0
        }),
    )
    .expect("acoustic quality should run through executor");

    assert_eq!(quality["acoustic_quality_ready"].as_bool(), Some(true));
    assert_eq!(quality["acoustic_quality_grade"].as_str(), Some("good"));
    assert_eq!(quality["acoustic_quality_term_count"].as_u64(), Some(4));
}
