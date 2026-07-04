use std::{fs, process};

mod catalog;
mod catalog_defaults;
mod compare;
mod config;
mod generators;
mod generators_extended;
mod generators_structural;
mod generators_thermal_structural;
mod headless_cases;
mod models;
mod runner;
mod runner_metrics;
mod runner_preconditioner;
mod runner_progress;
mod runner_shape;
mod runner_structural;
mod runner_util;
#[cfg(test)]
mod workflow_payloads;
#[cfg(test)]
include!("tests.rs");

use catalog::benchmark_cases;
use compare::{
    compare_against_baseline, evaluate_regressions, load_baseline_report, print_table,
    render_comparison_report,
};
use config::{BenchmarkConfig, OutputFormat};
use headless_cases::{headless_sdk_cases, is_headless_sdk_matrix};
use models::select_cases;
use runner::{build_report, build_report_with_progress};

fn main() {
    let config = BenchmarkConfig::from_env();
    let cases = if is_headless_sdk_matrix(&config.matrix) {
        headless_sdk_cases()
    } else {
        benchmark_cases(config.profile, &config.matrix)
    };
    let selected = select_cases(&cases, config.case_filter.as_deref());
    let report = if config.progress {
        build_report_with_progress(
            &selected,
            config.repeat,
            config.profile,
            &config.matrix,
            &config.solver_preconditioner,
            true,
        )
    } else {
        build_report(
            &selected,
            config.repeat,
            config.profile,
            &config.matrix,
            &config.solver_preconditioner,
        )
    };

    if let Some(path) = &config.baseline_out {
        let payload = serde_json::to_string_pretty(&report).expect("report should serialize");
        fs::write(path, payload).expect("baseline snapshot should write");
    }

    let comparison = config
        .baseline_compare
        .as_ref()
        .and_then(|path| load_baseline_report(path).ok())
        .map(|baseline| compare_against_baseline(&report, &baseline));

    if let (Some(path), Some(comparison)) = (&config.compare_report_out, &comparison) {
        let payload = render_comparison_report(&report, comparison);
        fs::write(path, payload).expect("comparison report should write");
    }

    match config.format {
        OutputFormat::Json => {
            println!(
                "{}",
                serde_json::to_string_pretty(&report).expect("report should serialize")
            );
        }
        OutputFormat::Table => print_table(
            &report.cases,
            config.repeat,
            config.profile,
            &report.matrix,
            comparison.as_ref(),
        ),
    }

    if let Some(comparison) = &comparison {
        let failures = evaluate_regressions(&config, comparison);
        if !failures.is_empty() {
            eprintln!();
            eprintln!("benchmark regression gate failed:");
            for failure in failures {
                eprintln!("  {failure}");
            }
            process::exit(1);
        }
    }
}
