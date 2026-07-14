use super::{CoverageTarget, coverage_summaries, evaluate_gate, matrix_summaries};
use serde_json::json;

#[test]
fn gate_warns_on_empty_retained_runs() {
    let gate = evaluate_gate(&[], &[], &[]);
    assert_eq!(gate["status"], "warn");
    assert_eq!(gate["reasons"][0], "no retained benchmark profile runs");
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
