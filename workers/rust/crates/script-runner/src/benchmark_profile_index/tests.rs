use super::{
    CoverageTarget, annotate_resolved_failures, coverage_summaries, evaluate_gate,
    matrix_summaries, normalized_failure_kind, report_case_metrics, report_preconditioners,
    solver_strategy_summaries,
};
use serde_json::json;

#[test]
fn gate_warns_on_empty_retained_runs() {
    let gate = evaluate_gate(&[], &[], &[], &[]);
    assert_eq!(gate["status"], "warn");
    assert_eq!(gate["reasons"][0], "no retained benchmark profile runs");
}

#[test]
fn strategy_summary_keeps_the_latest_single_case_result_per_preconditioner() {
    let summaries = solver_strategy_summaries(&[
        json!({
            "slug": "latest-ic0",
            "matrix": "thermal-core",
            "profile": "one_million",
            "case_count": 1,
            "case_ids": ["heat-plane-quad-1m"],
            "solver_preconditioners": ["ic0"],
            "total_median_ms": 23.0,
            "peak_rss_mib": 100.0,
        }),
        json!({
            "slug": "sgs",
            "matrix": "thermal-core",
            "profile": "one_million",
            "case_count": 1,
            "case_ids": ["heat-plane-quad-1m"],
            "solver_preconditioners": ["symmetric-gauss-seidel"],
            "total_median_ms": 21.0,
            "peak_rss_mib": 99.0,
        }),
        json!({
            "slug": "older-ic0",
            "matrix": "thermal-core",
            "profile": "one_million",
            "case_count": 1,
            "case_ids": ["heat-plane-quad-1m"],
            "solver_preconditioners": ["ic0"],
            "total_median_ms": 30.0,
            "peak_rss_mib": 101.0,
        }),
    ]);
    assert_eq!(summaries.len(), 1);
    assert_eq!(summaries[0]["strategies"][0]["slug"], "latest-ic0");
    assert_eq!(summaries[0]["strategies"][1]["slug"], "sgs");
}

#[test]
fn reads_preconditioners_from_legacy_raw_reports() {
    let report = json!({
        "cases": [
            { "solver_preconditioner": "ic0" },
            { "solver_preconditioner": "ic0" },
            { "solver_preconditioner": "symmetric-gauss-seidel" }
        ]
    });
    assert_eq!(
        report_preconditioners(&report),
        vec![
            "ic0".to_string(),
            "ic0".to_string(),
            "symmetric-gauss-seidel".to_string(),
        ]
    );
}

#[test]
fn reads_solver_metrics_from_legacy_raw_reports() {
    let report = json!({
        "cases": [{
            "id": "heat-plane-quad-1m",
            "solver_preconditioner": "ic0",
            "solver_iterations": 1122,
            "solver_residual_norm": 1.0e-9,
        }]
    });
    assert_eq!(report_case_metrics(&report)[0]["solver_iterations"], 1122);
}

#[test]
fn gate_warns_on_failed_remote_runs_without_counting_them_as_coverage() {
    let failures = vec![json!({
        "slug": "truss-1m-timeout",
        "matrix": "mechanical-core",
        "profile": "1m",
        "phase": "remote-execution",
        "timed_out": true,
    })];
    let gate = evaluate_gate(
        &[json!({"case_count": 1, "total_median_ms": 1.0, "peak_rss_mib": 1.0})],
        &failures,
        &[],
        &[],
    );
    assert_eq!(gate["status"], "warn");
    assert!(
        gate["reasons"]
            .as_array()
            .unwrap()
            .iter()
            .any(|reason| reason.as_str().unwrap().contains("truss-1m-timeout"))
    );
}

#[test]
fn normalizes_legacy_timeout_receipts() {
    assert_eq!(
        normalized_failure_kind(&json!({"timed_out": true})),
        "timeout"
    );
    assert_eq!(
        normalized_failure_kind(&json!({"failure_kind": "configuration"})),
        "configuration"
    );
}

#[test]
fn marks_legacy_profile_failures_resolved_by_later_successes() {
    let mut failures = vec![json!({
        "matrix": "compound-core",
        "profile": "1m",
        "case": "compound-surface-panel-1m",
    })];
    let runs = vec![json!({
        "slug": "ic0-success",
        "matrix": "compound-core",
        "profile": "one_million",
        "case_ids": ["compound-surface-panel-1m"],
    })];
    annotate_resolved_failures(&mut failures, &runs);
    assert_eq!(failures[0]["resolved_by_success"], true);
    assert_eq!(failures[0]["resolved_by_slug"], "ic0-success");
}

#[test]
fn coverage_counts_normalized_case_ids() {
    let runs = vec![json!({
        "matrix": "mechanical-core",
        "profile": "four_hundred_k",
        "case_ids": ["axial-bar-400k", "truss-roof-400k#jacobi"],
    })];
    let coverage = coverage_summaries(
        &runs,
        &[CoverageTarget {
            matrix: "mechanical-core".to_string(),
            profile: "four_hundred_k".to_string(),
            expected_cases: vec![
                "axial-bar-400k".to_string(),
                "truss-roof-400k".to_string(),
                "space-frame-400k".to_string(),
            ],
        }],
    );
    assert_eq!(coverage[0]["covered_case_count"], 2);
    assert_eq!(coverage[0]["missing_cases"][0], "space-frame-400k");
}

#[test]
fn matrix_summary_groups_runs() {
    let rows = matrix_summaries(&[
        json!({"matrix": "mechanical-core", "case_count": 1, "total_median_ms": 10.0, "peak_rss_mib": 100.0, "slowest_case": "a"}),
        json!({"matrix": "mechanical-core", "case_count": 2, "total_median_ms": 25.5, "peak_rss_mib": 250.0, "slowest_case": "b"}),
    ]);
    assert_eq!(rows[0]["run_count"], 2);
    assert_eq!(rows[0]["case_count"], 3);
    assert_eq!(rows[0]["slowest_case"], "b");
}
