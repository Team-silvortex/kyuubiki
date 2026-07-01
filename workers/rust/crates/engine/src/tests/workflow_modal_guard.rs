use crate::{
    modal_quality::score_modal_quality,
    workflow_executor::run_transform_operator,
    workflow_guard_transforms::{benchmark_modal_pair, evaluate_modal_guard},
};

fn approx_eq(left: Option<f64>, right: f64) {
    let value = left.expect("expected numeric benchmark value");
    assert!(
        (value - right).abs() < 1.0e-9,
        "left={value}, right={right}"
    );
}

#[test]
fn evaluates_modal_guard_with_frequency_band_rules() {
    let guard = evaluate_modal_guard(
        serde_json::json!({
            "min_frequency_hz": 18.0,
            "max_frequency_hz": 220.0,
            "total_mass": 42.0,
            "mode_1_participation_norm": 1.7
        }),
        serde_json::json!({
            "rules": [
                { "field": "min_frequency_hz", "comparison": "lt", "threshold": 20.0, "severity": "block", "label": "first_mode_floor" },
                { "field": "max_frequency_hz", "comparison": "gt", "threshold": 260.0, "severity": "warn", "label": "upper_band" }
            ]
        }),
    )
    .expect("modal guard should evaluate");

    assert_eq!(guard["guard_status"].as_str(), Some("block"));
    assert_eq!(guard["guard_passed"].as_bool(), Some(false));
    assert_eq!(guard["guard_block_count"].as_u64(), Some(1));
    assert_eq!(guard["guard_warn_count"].as_u64(), Some(0));
    assert_eq!(
        guard["guard_recommendation"].as_str(),
        Some("hold_and_review")
    );
}

#[test]
fn benchmarks_modal_pair_by_frequency_mass_and_participation() {
    let benchmark = benchmark_modal_pair(
        serde_json::json!({
            "left": {
                "min_frequency_hz": 28.0,
                "total_mass": 15.0,
                "mode_1_participation_norm": 1.2
            },
            "right": {
                "min_frequency_hz": 24.0,
                "total_mass": 11.0,
                "mode_1_participation_norm": 1.6
            }
        }),
        serde_json::json!({
            "left_label": "stiff_modal_candidate",
            "right_label": "light_modal_candidate",
            "criteria": [
                { "field": "min_frequency_hz", "goal": "max", "weight": 2.0 },
                { "field": "total_mass", "goal": "min", "weight": 2.0 },
                { "field": "mode_1_participation_norm", "goal": "min", "weight": 1.0 }
            ]
        }),
    )
    .expect("modal benchmark should succeed");

    approx_eq(benchmark["stiff_modal_candidate_score"].as_f64(), 3.0);
    approx_eq(benchmark["light_modal_candidate_score"].as_f64(), 2.0);
    assert_eq!(
        benchmark["benchmark_winner"].as_str(),
        Some("stiff_modal_candidate")
    );
    assert_eq!(benchmark["benchmark_criteria_count"].as_u64(), Some(3));
    assert_eq!(benchmark["benchmark_left_win_count"].as_u64(), Some(2));
    assert_eq!(benchmark["benchmark_right_win_count"].as_u64(), Some(1));
}

#[test]
fn runs_modal_guard_and_benchmark_through_transform_executor() {
    let guard = run_transform_operator(
        "transform.evaluate_modal_guard",
        serde_json::json!({
            "min_frequency_hz": 35.0
        }),
        serde_json::json!({
            "rules": [
                { "field": "min_frequency_hz", "comparison": "lt", "threshold": 20.0, "severity": "block" }
            ]
        }),
    )
    .expect("modal guard should run through executor");

    assert_eq!(guard["guard_status"].as_str(), Some("pass"));
    assert_eq!(guard["guard_passed"].as_bool(), Some(true));

    let benchmark = run_transform_operator(
        "transform.benchmark_modal_pair",
        serde_json::json!({
            "left": { "min_frequency_hz": 32.0 },
            "right": { "min_frequency_hz": 26.0 }
        }),
        serde_json::json!({
            "criteria": [
                { "field": "min_frequency_hz", "goal": "max", "weight": 2.0 }
            ]
        }),
    )
    .expect("modal benchmark should run through executor");

    approx_eq(benchmark["left_score"].as_f64(), 2.0);
    approx_eq(benchmark["right_score"].as_f64(), 0.0);
    assert_eq!(benchmark["benchmark_winner"].as_str(), Some("left"));
}

#[test]
fn scores_modal_quality_with_frequency_and_mass_penalties() {
    let quality = score_modal_quality(
        serde_json::json!({
            "min_frequency_hz": 32.0,
            "max_frequency_hz": 180.0,
            "total_mass": 18.0,
            "mode_1_participation_norm": 1.3
        }),
        serde_json::json!({
            "targets": {
                "min_frequency_hz": 20.0,
                "total_mass": 25.0,
                "mode_1_participation_norm": 2.0,
                "frequency_span_hz": 250.0
            },
            "max_ready_score": 8.0
        }),
    )
    .expect("modal quality should score");

    assert_eq!(quality["modal_quality_ready"].as_bool(), Some(true));
    assert_eq!(
        quality["modal_quality_missing_metric_count"].as_u64(),
        Some(0)
    );
    assert_eq!(quality["modal_quality_term_count"].as_u64(), Some(4));
    assert_eq!(quality["modal_quality_grade"].as_str(), Some("good"));
    let terms = quality["modal_quality_terms"]
        .as_array()
        .expect("quality terms should be an array");
    assert_eq!(terms[0]["goal"].as_str(), Some("max"));
    assert_eq!(terms[3]["field"].as_str(), Some("frequency_span_hz"));
    approx_eq(terms[3]["value"].as_f64(), 148.0);
}

#[test]
fn blocks_modal_quality_when_required_metrics_are_missing() {
    let quality = score_modal_quality(
        serde_json::json!({
            "min_frequency_hz": 12.0
        }),
        serde_json::json!({}),
    )
    .expect("modal quality should still return missing terms");

    assert_eq!(quality["modal_quality_ready"].as_bool(), Some(false));
    assert_eq!(
        quality["modal_quality_missing_metric_count"].as_u64(),
        Some(3)
    );
    assert_eq!(quality["modal_quality_grade"].as_str(), Some("block"));
}

#[test]
fn runs_modal_quality_through_transform_executor() {
    let quality = run_transform_operator(
        "transform.score_modal_quality",
        serde_json::json!({
            "min_frequency_hz": 28.0,
            "max_frequency_hz": 120.0,
            "total_mass": 16.0,
            "mode_1_participation_norm": 1.1
        }),
        serde_json::json!({
            "max_ready_score": 8.0
        }),
    )
    .expect("modal quality should run through executor");

    assert_eq!(
        quality["modal_quality_contract"].as_str(),
        Some("kyuubiki.modal_quality_score/v1")
    );
    assert_eq!(quality["modal_quality_ready"].as_bool(), Some(true));
}
