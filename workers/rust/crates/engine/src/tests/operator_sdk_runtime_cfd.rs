use crate::workflow_executor::{run_extract_operator, run_transform_operator};

#[test]
fn runs_cfd_diagnostics_operator_through_sdk_registry() {
    let summary = run_extract_operator(
        "extract.stokes_flow_result_diagnostics",
        serde_json::json!({
            "nodes": [
                { "id": "n0", "velocity_magnitude": 0.0, "pressure": 1.0 },
                { "id": "n1", "velocity_magnitude": 2.0, "pressure": -3.0 }
            ],
            "elements": [
                { "id": "f0", "divergence_error": 0.02, "reynolds_number": 5.0, "viscous_dissipation": 0.3 },
                { "id": "f1", "divergence_error": 0.08, "reynolds_number": 12.0, "viscous_dissipation": 0.9 }
            ]
        }),
        serde_json::Value::Null,
    )
    .expect("extract.stokes_flow_result_diagnostics should succeed");

    assert_eq!(summary["diagnostic_domain"].as_str(), Some("fluid"));
    assert_eq!(summary["cfd_velocity_max"].as_f64(), Some(2.0));
    assert_eq!(summary["cfd_divergence_error_peak"].as_f64(), Some(0.08));
    assert_eq!(
        summary["cfd_reynolds_number_peak_element_id"].as_str(),
        Some("f1")
    );
    assert_eq!(summary["cfd_velocity_span"].as_f64(), Some(2.0));
    assert_eq!(summary["cfd_pressure_span"].as_f64(), Some(4.0));
}

#[test]
fn runs_cfd_guard_operator_through_sdk_registry() {
    let guard = run_transform_operator(
        "transform.evaluate_cfd_guard",
        serde_json::json!({ "cfd_divergence_error_peak": 0.08 }),
        serde_json::json!({
            "rules": [
                { "field": "cfd_divergence_error_peak", "threshold": 0.05, "severity": "block" }
            ]
        }),
    )
    .expect("transform.evaluate_cfd_guard should succeed");

    assert_eq!(guard["guard_status"].as_str(), Some("block"));
    assert_eq!(guard["guard_block_count"].as_u64(), Some(1));
}

#[test]
fn runs_cfd_benchmark_operator_through_sdk_registry() {
    let benchmark = run_transform_operator(
        "transform.benchmark_cfd_pair",
        serde_json::json!({
            "left": { "cfd_divergence_error_peak": 0.03, "cfd_reynolds_number_peak": 8.0 },
            "right": { "cfd_divergence_error_peak": 0.08, "cfd_reynolds_number_peak": 12.0 }
        }),
        serde_json::json!({
            "left_label": "candidate_a",
            "right_label": "candidate_b",
            "criteria": [
                { "field": "cfd_divergence_error_peak", "goal": "min", "weight": 2.0 },
                { "field": "cfd_reynolds_number_peak", "goal": "min", "weight": 1.0 }
            ]
        }),
    )
    .expect("transform.benchmark_cfd_pair should succeed");

    assert_eq!(benchmark["benchmark_winner"].as_str(), Some("candidate_a"));
    assert_eq!(benchmark["candidate_a_score"].as_f64(), Some(3.0));
    assert_eq!(benchmark["benchmark_criteria_count"].as_u64(), Some(2));
}

#[test]
fn scores_cfd_quality_objective_through_sdk_registry() {
    let quality = run_transform_operator(
        "transform.score_cfd_quality",
        serde_json::json!({
            "cfd_divergence_error_peak": 0.025,
            "cfd_reynolds_number_peak": 5.0,
            "cfd_viscous_dissipation_total": 0.5,
            "cfd_velocity_min": 0.0,
            "cfd_velocity_max": 2.0,
            "cfd_pressure_span": 10.0
        }),
        serde_json::json!({
            "max_ready_score": 8.0,
            "weights": {
                "cfd_divergence_error_peak": 4.0,
                "cfd_reynolds_number_peak": 2.0,
                "cfd_viscous_dissipation_total": 1.0,
                "cfd_velocity_span": 0.5,
                "cfd_pressure_span": 0.5
            }
        }),
    )
    .expect("transform.score_cfd_quality should succeed");

    assert_eq!(
        quality["cfd_quality_contract"].as_str(),
        Some("kyuubiki.cfd_quality_score/v1")
    );
    assert_eq!(quality["cfd_quality_ready"].as_bool(), Some(true));
    assert_eq!(
        quality["cfd_quality_missing_metric_count"].as_u64(),
        Some(0)
    );
    assert_eq!(quality["cfd_quality_score"].as_f64(), Some(5.0));
    assert_eq!(quality["cfd_quality_grade"].as_str(), Some("good"));
}

#[test]
fn blocks_cfd_quality_when_required_metrics_are_missing() {
    let quality = run_transform_operator(
        "transform.score_cfd_quality",
        serde_json::json!({ "cfd_divergence_error_peak": 0.01 }),
        serde_json::Value::Null,
    )
    .expect("transform.score_cfd_quality should produce a visible blocked score");

    assert_eq!(quality["cfd_quality_ready"].as_bool(), Some(false));
    assert_eq!(quality["cfd_quality_grade"].as_str(), Some("block"));
    assert!(
        quality["cfd_quality_missing_metric_count"]
            .as_u64()
            .unwrap_or_default()
            > 0
    );
}
