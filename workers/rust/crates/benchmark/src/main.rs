use std::process;

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
mod protocol_cases;
mod runner;
mod runner_hotspot;
mod runner_metrics;
mod runner_preconditioner;
mod runner_progress;
mod runner_shape;
mod runner_structural;
mod runner_util;
mod shape_report;
#[cfg(test)]
mod workflow_payloads;

use catalog::benchmark_cases;
use compare::{
    compare_against_baseline, evaluate_regressions, load_baseline_report, print_table,
    render_comparison_report, write_report,
};
use config::{BenchmarkCommand, BenchmarkConfig, HELP_TEXT, OutputFormat};
use headless_cases::{headless_sdk_cases, is_headless_sdk_matrix};
use models::{BenchmarkReport, select_cases};
use protocol_cases::{is_protocol_matrix, protocol_cases};
use runner::{build_report, build_report_with_progress};
use shape_report::{build_shape_report, print_shape_table};

fn main() {
    if let Err(error) = run() {
        eprintln!("benchmark configuration error: {error}");
        process::exit(2);
    }
}

fn run() -> Result<(), String> {
    let config = match BenchmarkConfig::from_env()? {
        BenchmarkCommand::Help => {
            print!("{HELP_TEXT}");
            return Ok(());
        }
        BenchmarkCommand::Run(config) => config,
    };
    let cases = if is_headless_sdk_matrix(&config.matrix) {
        headless_sdk_cases()
    } else if is_protocol_matrix(&config.matrix) {
        protocol_cases()
    } else {
        benchmark_cases(config.profile, &config.matrix)
    };
    let selected = select_cases(&cases, config.case_filter.as_deref());
    if selected.is_empty() {
        return Err(match config.case_filter.as_deref() {
            Some(filter) => format!(
                "no benchmark case matched '{filter}' in matrix '{}'",
                config.matrix
            ),
            None => format!("benchmark matrix '{}' contains no cases", config.matrix),
        });
    }

    if config.dry_run_shapes {
        let report = build_shape_report(&selected, config.profile, &config.matrix);
        match config.format {
            OutputFormat::Json => {
                let payload = serde_json::to_string_pretty(&report)
                    .map_err(|error| format!("failed to serialize shape report: {error}"))?;
                println!("{payload}");
            }
            OutputFormat::Table => print_shape_table(&report),
        }
        return Ok(());
    }

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

    let comparison = match &config.baseline_compare {
        Some(path) => {
            let baseline = load_baseline_report(path)?;
            Some(compare_against_baseline(&report, &baseline))
        }
        None => None,
    };

    if let (Some(path), Some(comparison)) = (&config.compare_report_out, &comparison) {
        let payload = render_comparison_report(&report, comparison);
        write_report(path, "comparison report", &payload)?;
    }

    match config.format {
        OutputFormat::Json => {
            let payload = serde_json::to_string_pretty(&report)
                .map_err(|error| format!("failed to serialize benchmark report: {error}"))?;
            println!("{payload}");
        }
        OutputFormat::Table => print_table(
            &report.cases,
            config.repeat,
            config.profile,
            &report.matrix,
            comparison.as_ref(),
        ),
    }

    let case_failures = failed_case_messages(&report);
    if !case_failures.is_empty() {
        eprintln!();
        eprintln!("benchmark case execution failed:");
        for failure in case_failures {
            eprintln!("  {failure}");
        }
        process::exit(1);
    }

    if let Some(comparison) = &comparison {
        let failures = evaluate_regressions(&config, &report, comparison);
        if !failures.is_empty() {
            eprintln!();
            eprintln!("benchmark regression gate failed:");
            for failure in failures {
                eprintln!("  {failure}");
            }
            process::exit(1);
        }
    }

    if let Some(path) = &config.baseline_out {
        let payload = serde_json::to_string_pretty(&report)
            .map_err(|error| format!("failed to serialize benchmark report: {error}"))?;
        write_report(path, "baseline report", &payload)?;
    }

    Ok(())
}

fn failed_case_messages(report: &BenchmarkReport) -> Vec<String> {
    report
        .cases
        .iter()
        .filter(|case| !case.ok)
        .map(|case| {
            format!(
                "{}: {}",
                case.id,
                case.error.as_deref().unwrap_or("unknown execution failure")
            )
        })
        .collect()
}

#[cfg(test)]
include!("tests.rs");
