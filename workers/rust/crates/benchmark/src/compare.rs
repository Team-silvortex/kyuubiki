use std::fs;

use crate::{
    config::{BenchmarkConfig, BenchmarkProfile},
    models::{BenchmarkComparison, BenchmarkComparisonCase, BenchmarkReport, BenchmarkResult},
};

pub(crate) fn load_baseline_report(path: &str) -> Result<BenchmarkReport, String> {
    let content = fs::read_to_string(path)
        .map_err(|error| format!("failed to read baseline report '{path}': {error}"))?;
    serde_json::from_str(&content)
        .map_err(|error| format!("failed to parse baseline report '{path}': {error}"))
}

pub(crate) fn write_report(path: &str, kind: &str, payload: &str) -> Result<(), String> {
    fs::write(path, payload).map_err(|error| format!("failed to write {kind} '{path}': {error}"))
}

pub(crate) fn compare_against_baseline(
    current: &BenchmarkReport,
    baseline: &BenchmarkReport,
) -> BenchmarkComparison {
    let peak_rss_comparable = current.rss_scope == baseline.rss_scope;
    let cases = current
        .cases
        .iter()
        .filter_map(|current_case| {
            baseline
                .cases
                .iter()
                .find(|baseline_case| baseline_case.id == current_case.id && baseline_case.ok)
                .map(|baseline_case| BenchmarkComparisonCase {
                    id: current_case.id.clone(),
                    baseline_median_ms: baseline_case.median_ms,
                    median_delta_pct: percent_delta(
                        baseline_case.median_ms,
                        current_case.median_ms,
                    ),
                    peak_rss_delta_pct: percent_delta(
                        baseline_case.peak_rss_kib as f64,
                        current_case.peak_rss_kib as f64,
                    ),
                })
        })
        .collect();

    BenchmarkComparison {
        baseline_generated_at_unix_s: baseline.generated_at_unix_s,
        baseline_rss_scope: baseline.rss_scope.clone(),
        current_rss_scope: current.rss_scope.clone(),
        peak_rss_comparable,
        cases,
    }
}

pub(crate) fn evaluate_regressions(
    config: &BenchmarkConfig,
    current: &BenchmarkReport,
    comparison: &BenchmarkComparison,
) -> Vec<String> {
    let mut failures = Vec::new();

    if config.fail_on_rss_regression_pct.is_some() && !comparison.peak_rss_comparable {
        failures.push(format!(
            "peak RSS scopes differ (baseline={}, current={}); regenerate the baseline with the same case isolation mode",
            comparison.baseline_rss_scope, comparison.current_rss_scope
        ));
    }

    if config.fail_on_median_regression_pct.is_some() || config.fail_on_rss_regression_pct.is_some()
    {
        failures.extend(
            current
                .cases
                .iter()
                .filter(|current_case| {
                    !comparison
                        .cases
                        .iter()
                        .any(|baseline_case| baseline_case.id == current_case.id)
                })
                .map(|current_case| {
                    format!(
                        "{} is missing from the baseline; regression gate cannot evaluate it",
                        current_case.id
                    )
                }),
        );
    }

    for case in &comparison.cases {
        if case.baseline_median_ms < config.min_baseline_median_ms {
            continue;
        }

        if let Some(threshold) = config.fail_on_median_regression_pct
            && case.median_delta_pct > threshold
        {
            failures.push(format!(
                "{} median regression {:+.2}% exceeds threshold +{:.2}%",
                case.id, case.median_delta_pct, threshold
            ));
        }

        if comparison.peak_rss_comparable
            && let Some(threshold) = config.fail_on_rss_regression_pct
            && case.peak_rss_delta_pct > threshold
        {
            failures.push(format!(
                "{} peak RSS regression {:+.2}% exceeds threshold +{:.2}%",
                case.id, case.peak_rss_delta_pct, threshold
            ));
        }
    }

    failures
}

pub(crate) fn render_comparison_report(
    report: &BenchmarkReport,
    comparison: &BenchmarkComparison,
) -> String {
    let mut lines = vec![
        "# Kyuubiki Benchmark Comparison".to_string(),
        String::new(),
        format!("- Profile: `{}`", report.profile.as_str()),
        format!("- Matrix: `{}`", report.matrix),
        format!("- Repeat count: `{}`", report.repeat),
        format!("- Generated at (unix): `{}`", report.generated_at_unix_s),
        format!("- Peak RSS scope: `{}`", report.rss_scope),
        format!(
            "- Baseline generated at (unix): `{}`",
            comparison.baseline_generated_at_unix_s
        ),
        format!(
            "- Baseline peak RSS scope: `{}`",
            comparison.baseline_rss_scope
        ),
        String::new(),
        "| Case | Status | Median ms | Delta | Peak RSS | Delta |".to_string(),
        "| --- | --- | ---: | ---: | ---: | ---: |".to_string(),
    ];

    for result in &report.cases {
        let median_delta = comparison
            .cases
            .iter()
            .find(|case| case.id == result.id)
            .map(|case| format!("{:+.2}%", case.median_delta_pct))
            .unwrap_or_else(|| "n/a".to_string());
        let rss_delta = if comparison.peak_rss_comparable {
            comparison
                .cases
                .iter()
                .find(|case| case.id == result.id)
                .map(|case| format!("{:+.2}%", case.peak_rss_delta_pct))
                .unwrap_or_else(|| "n/a".to_string())
        } else {
            "n/a (scope mismatch)".to_string()
        };

        lines.push(format!(
            "| `{}` | {} | {:.4} | {} | {} MiB | {} |",
            result.id,
            if result.ok { "ok" } else { "fail" },
            result.median_ms,
            median_delta,
            kib_to_mib(result.peak_rss_kib),
            rss_delta
        ));

        if let Some(error) = &result.error {
            lines.push(format!("|  | error | `{error}` |  |  |  |"));
        }

        if !result.memory_stages.is_empty() {
            lines.push(format!(
                "|  | stage rss | `{}` |  |  |  |",
                format_memory_stages(result)
            ));
        }
    }

    lines.push(String::new());
    lines.join("\n")
}

pub(crate) fn print_table(
    results: &[BenchmarkResult],
    repeat: usize,
    profile: BenchmarkProfile,
    matrix: &str,
    rss_scope: &str,
    comparison: Option<&BenchmarkComparison>,
) {
    println!(
        "{}",
        render_table(results, repeat, profile, matrix, rss_scope, comparison)
    );
}

pub(crate) fn render_table(
    results: &[BenchmarkResult],
    repeat: usize,
    profile: BenchmarkProfile,
    matrix: &str,
    rss_scope: &str,
    comparison: Option<&BenchmarkComparison>,
) -> String {
    let mut lines = vec![
        "kyuubiki benchmark suite".to_string(),
        format!("profile: {}", profile.as_str()),
        format!("matrix: {matrix}"),
        format!("repeat count: {repeat}"),
        format!("peak rss scope: {rss_scope}"),
    ];
    if let Some(comparison) = comparison {
        lines.push(format!(
            "baseline compare: {}",
            comparison.baseline_generated_at_unix_s
        ));
    }
    lines.push(String::new());

    let case_width = text_column_width("case", results.iter().map(|result| result.id.as_str()));
    let family_width = text_column_width(
        "family",
        results.iter().map(|result| result.family.as_str()),
    );
    lines.push(format!(
        "{:<case_width$} {:<family_width$} {:<6} {:>6} {:>6} {:>7} {:>10} {:>10} {:>10} {:>10} {:>10} {:>10}",
        "case",
        "family",
        "status",
        "nodes",
        "elems",
        "dofs",
        "min ms",
        "median",
        "mean ms",
        "p95 ms",
        "max ms",
        "peak rss"
    ));

    for result in results {
        lines.push(format!(
            "{:<case_width$} {:<family_width$} {:<6} {:>6} {:>6} {:>7} {:>10.4} {:>10.4} {:>10.4} {:>10.4} {:>10.4} {:>8} MiB",
            result.id,
            result.family,
            if result.ok { "ok" } else { "fail" },
            result.node_count,
            result.element_count,
            result.dof_count,
            result.min_ms,
            result.median_ms,
            result.mean_ms,
            result.p95_ms,
            result.max_ms,
            kib_to_mib(result.peak_rss_kib)
        ));
        if let Some(error) = &result.error {
            lines.push(format!("  error: {error}"));
        }
        if !result.memory_stages.is_empty() {
            lines.push(format!("  stage rss: {}", format_memory_stages(result)));
        }
        if let Some(label) = &result.hotspot_label {
            lines.push(format!(
                "  hotspot: {} {:.3} ms ({:.1}%) | {}",
                label,
                result.hotspot_elapsed_ms.unwrap_or(0.0),
                result.hotspot_share_pct.unwrap_or(0.0),
                result.hotspot_hint.as_deref().unwrap_or("inspect stage")
            ));
        }
        if let Some(iterations) = result.solver_iterations {
            lines.push(format!(
                "  solver: preconditioner={} iterations={} residual={:.3e}",
                result.solver_preconditioner.as_deref().unwrap_or("default"),
                iterations,
                result.solver_residual_norm.unwrap_or(0.0)
            ));
        }
        if let Some(delta) = comparison
            .and_then(|comparison| comparison.cases.iter().find(|case| case.id == result.id))
        {
            let rss_delta = comparison.map_or_else(
                || "n/a".to_string(),
                |comparison| {
                    if comparison.peak_rss_comparable {
                        format!("{:+.2}%", delta.peak_rss_delta_pct)
                    } else {
                        "n/a (scope mismatch)".to_string()
                    }
                },
            );
            lines.push(format!(
                "  vs baseline: median {:+.2}% | peak rss {rss_delta}",
                delta.median_delta_pct
            ));
        }
    }

    lines.join("\n")
}

fn text_column_width<'a>(header: &str, values: impl Iterator<Item = &'a str>) -> usize {
    values.map(str::len).fold(header.len(), usize::max).max(20)
}

pub(crate) fn kib_to_mib(kib: u64) -> u64 {
    kib.div_ceil(1024)
}

fn percent_delta(old: f64, new: f64) -> f64 {
    if old.abs() <= f64::EPSILON {
        0.0
    } else {
        ((new - old) / old) * 100.0
    }
}

fn format_memory_stages(result: &BenchmarkResult) -> String {
    result
        .memory_stages
        .iter()
        .map(|stage| {
            if let Some(elapsed_ms) = stage.elapsed_ms {
                format!(
                    "{}={:.3} ms/{} MiB",
                    stage.label,
                    elapsed_ms,
                    kib_to_mib(stage.rss_kib)
                )
            } else {
                format!("{}={} MiB", stage.label, kib_to_mib(stage.rss_kib))
            }
        })
        .collect::<Vec<_>>()
        .join(", ")
}

#[cfg(test)]
mod tests {
    use super::{
        BenchmarkProfile, compare_against_baseline, evaluate_regressions, load_baseline_report,
        render_table, write_report,
    };
    use crate::{
        config::BenchmarkConfig,
        models::{BenchmarkReport, BenchmarkResult},
    };
    use std::{fs, path::PathBuf};

    #[test]
    fn table_renderer_keeps_status_column_aligned_for_long_case_names() {
        let rendered = render_table(
            &[
                result("short", "frame_2d"),
                result("thermal-plane-triangle-medium", "thermal_plane_triangle_2d"),
            ],
            1,
            BenchmarkProfile::Medium,
            "thermal-structural",
            "shared_process_high_water_mark",
            None,
        );
        let lines = rendered.lines().collect::<Vec<_>>();
        let header = lines
            .iter()
            .find(|line| line.starts_with("case "))
            .expect("table header should render");
        let long_row = lines
            .iter()
            .find(|line| line.starts_with("thermal-plane-triangle-medium"))
            .expect("long case row should render");

        assert_eq!(
            header.find("status"),
            long_row.find("ok"),
            "status column should stay aligned for long case ids"
        );
    }

    #[test]
    fn missing_and_malformed_baselines_fail_with_path_context() {
        let missing = unique_temp_path("missing");
        let malformed = unique_temp_path("malformed");
        fs::write(&malformed, "not-json").expect("malformed fixture should write");

        let missing_error = load_baseline_report(path(&missing)).unwrap_err();
        let malformed_error = load_baseline_report(path(&malformed)).unwrap_err();

        assert!(missing_error.contains(path(&missing)));
        assert!(malformed_error.contains(path(&malformed)));
        let _ = fs::remove_file(malformed);
    }

    #[test]
    fn report_write_failures_return_errors_instead_of_panicking() {
        let directory = unique_temp_path("directory");
        fs::create_dir(&directory).expect("fixture directory should create");

        let error = write_report(path(&directory), "benchmark report", "{}").unwrap_err();

        assert!(error.contains(path(&directory)));
        let _ = fs::remove_dir(directory);
    }

    #[test]
    fn regression_gate_rejects_cases_missing_from_the_baseline() {
        let current = report(vec![result("new-case", "frame_2d")]);
        let baseline = report(Vec::new());
        let comparison = compare_against_baseline(&current, &baseline);
        let config = BenchmarkConfig {
            fail_on_median_regression_pct: Some(5.0),
            ..BenchmarkConfig::default()
        };

        let failures = evaluate_regressions(&config, &current, &comparison);

        assert_eq!(failures.len(), 1);
        assert!(failures[0].contains("new-case is missing"));
    }

    #[test]
    fn rss_gate_rejects_incompatible_measurement_scopes() {
        let mut current = report(vec![result("frame", "frame_2d")]);
        current.rss_scope = "isolated_case_process_high_water_mark".to_string();
        let baseline = report(vec![result("frame", "frame_2d")]);
        let comparison = compare_against_baseline(&current, &baseline);
        let config = BenchmarkConfig {
            fail_on_rss_regression_pct: Some(5.0),
            ..BenchmarkConfig::default()
        };

        let failures = evaluate_regressions(&config, &current, &comparison);

        assert_eq!(failures.len(), 1);
        assert!(failures[0].contains("RSS scopes differ"));
    }

    fn unique_temp_path(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "kyuubiki-benchmark-{label}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock should follow unix epoch")
                .as_nanos()
        ))
    }

    fn path(path: &std::path::Path) -> &str {
        path.to_str().expect("temporary path should be utf-8")
    }

    fn report(cases: Vec<BenchmarkResult>) -> BenchmarkReport {
        BenchmarkReport {
            repeat: 1,
            profile: BenchmarkProfile::Medium,
            matrix: "core".to_string(),
            generated_at_unix_s: 1,
            rss_scope: "shared_process_high_water_mark".to_string(),
            cases,
            preconditioner_comparisons: Vec::new(),
        }
    }

    fn result(id: &str, family: &str) -> BenchmarkResult {
        BenchmarkResult {
            id: id.to_string(),
            family: family.to_string(),
            ok: true,
            error: None,
            repeat: 1,
            min_ms: 1.0,
            median_ms: 1.0,
            mean_ms: 1.0,
            p95_ms: 1.0,
            max_ms: 1.0,
            dof_count: 1,
            node_count: 1,
            element_count: 1,
            history_step_count: None,
            peak_rss_kib: 1024,
            memory_stages: Vec::new(),
            solver_iterations: None,
            solver_matrix_non_zero_count: None,
            solver_residual_norm: None,
            solver_preconditioner: None,
            solver_preconditioner_reason: None,
            hotspot_label: None,
            hotspot_elapsed_ms: None,
            hotspot_share_pct: None,
            hotspot_hint: None,
            max_displacement: 0.0,
            max_stress: 0.0,
        }
    }
}
