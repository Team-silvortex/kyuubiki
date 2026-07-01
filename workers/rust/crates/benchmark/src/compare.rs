use std::fs;

use crate::{
    config::{BenchmarkConfig, BenchmarkProfile},
    models::{BenchmarkComparison, BenchmarkComparisonCase, BenchmarkReport, BenchmarkResult},
};

pub(crate) fn load_baseline_report(path: &str) -> Result<BenchmarkReport, String> {
    let content = fs::read_to_string(path).map_err(|error| error.to_string())?;
    serde_json::from_str(&content).map_err(|error| error.to_string())
}

pub(crate) fn compare_against_baseline(
    current: &BenchmarkReport,
    baseline: &BenchmarkReport,
) -> BenchmarkComparison {
    let cases = current
        .cases
        .iter()
        .filter_map(|current_case| {
            baseline
                .cases
                .iter()
                .find(|baseline_case| baseline_case.id == current_case.id)
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
        cases,
    }
}

pub(crate) fn evaluate_regressions(
    config: &BenchmarkConfig,
    comparison: &BenchmarkComparison,
) -> Vec<String> {
    let mut failures = Vec::new();

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

        if let Some(threshold) = config.fail_on_rss_regression_pct
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
        format!(
            "- Baseline generated at (unix): `{}`",
            comparison.baseline_generated_at_unix_s
        ),
        String::new(),
        "| Case | Status | Median ms | Delta | Peak RSS | Delta |".to_string(),
        "| --- | --- | ---: | ---: | ---: | ---: |".to_string(),
    ];

    for result in &report.cases {
        let (median_delta, rss_delta) = comparison
            .cases
            .iter()
            .find(|case| case.id == result.id)
            .map(|case| {
                (
                    format!("{:+.2}%", case.median_delta_pct),
                    format!("{:+.2}%", case.peak_rss_delta_pct),
                )
            })
            .unwrap_or_else(|| ("n/a".to_string(), "n/a".to_string()));

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
            lines.push(format!("|  | error | `{}` |  |  |  |", error));
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
    comparison: Option<&BenchmarkComparison>,
) {
    println!("kyuubiki benchmark suite");
    println!("profile: {}", profile.as_str());
    println!("matrix: {matrix}");
    println!("repeat count: {repeat}");
    if let Some(comparison) = comparison {
        println!(
            "baseline compare: {}",
            comparison.baseline_generated_at_unix_s
        );
    }
    println!();
    println!(
        "{:<22} {:<20} {:<6} {:>6} {:>6} {:>7} {:>10} {:>10} {:>10} {:>10} {:>10} {:>10}",
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
    );

    for result in results {
        println!(
            "{:<22} {:<20} {:<6} {:>6} {:>6} {:>7} {:>10.4} {:>10.4} {:>10.4} {:>10.4} {:>10.4} {:>8} MiB",
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
        );
        if let Some(error) = &result.error {
            println!("  error: {error}");
        }
        if !result.memory_stages.is_empty() {
            println!("  stage rss: {}", format_memory_stages(result));
        }
        if let Some(iterations) = result.solver_iterations {
            println!(
                "  solver: preconditioner={} iterations={} residual={:.3e}",
                result.solver_preconditioner.as_deref().unwrap_or("default"),
                iterations,
                result.solver_residual_norm.unwrap_or(0.0)
            );
        }
        if let Some(delta) = comparison
            .and_then(|comparison| comparison.cases.iter().find(|case| case.id == result.id))
        {
            println!(
                "  vs baseline: median {:+.2}% | peak rss {:+.2}%",
                delta.median_delta_pct, delta.peak_rss_delta_pct
            );
        }
    }
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
