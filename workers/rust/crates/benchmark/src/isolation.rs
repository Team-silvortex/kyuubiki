use std::path::Path;
use std::process::{Command, Stdio};

use crate::config::BenchmarkConfig;
use crate::models::{BenchmarkReport, BenchmarkResult, ISOLATED_CASE_RSS_SCOPE};
use crate::runner_preconditioner::preconditioner_comparisons;
use crate::runner_util::unix_timestamp;

const COMPARISON_PRECONDITIONERS: [&str; 3] = ["jacobi", "symmetric-gauss-seidel", "ic0"];

pub(crate) fn build_isolated_report(
    case_ids: &[String],
    config: &BenchmarkConfig,
) -> Result<BenchmarkReport, String> {
    let executable = std::env::current_exe()
        .map_err(|error| format!("failed to locate benchmark executable: {error}"))?;
    Ok(build_isolated_report_with(
        case_ids,
        config,
        |case_id, preconditioner| run_child(&executable, case_id, preconditioner, config),
    ))
}

fn build_isolated_report_with(
    case_ids: &[String],
    config: &BenchmarkConfig,
    mut run_child: impl FnMut(&str, &str) -> Result<BenchmarkResult, String>,
) -> BenchmarkReport {
    let compare_preconditioners =
        matches!(config.solver_preconditioner.as_str(), "all" | "compare");
    let mut cases = Vec::new();

    for case_id in case_ids {
        if compare_preconditioners {
            let mut first = child_result(
                case_id,
                COMPARISON_PRECONDITIONERS[0],
                config.repeat,
                &mut run_child,
            );
            if first.solver_preconditioner.is_none() {
                cases.push(first);
                continue;
            }
            tag_preconditioner_result(&mut first);
            cases.push(first);
            for preconditioner in &COMPARISON_PRECONDITIONERS[1..] {
                let mut result =
                    child_result(case_id, preconditioner, config.repeat, &mut run_child);
                tag_preconditioner_result(&mut result);
                cases.push(result);
            }
        } else {
            cases.push(child_result(
                case_id,
                &config.solver_preconditioner,
                config.repeat,
                &mut run_child,
            ));
        }
    }

    BenchmarkReport {
        repeat: config.repeat,
        profile: config.profile,
        matrix: config.matrix.clone(),
        generated_at_unix_s: unix_timestamp(),
        rss_scope: ISOLATED_CASE_RSS_SCOPE.to_string(),
        preconditioner_comparisons: preconditioner_comparisons(&cases),
        cases,
    }
}

fn child_result(
    case_id: &str,
    preconditioner: &str,
    repeat: usize,
    run_child: &mut impl FnMut(&str, &str) -> Result<BenchmarkResult, String>,
) -> BenchmarkResult {
    run_child(case_id, preconditioner)
        .unwrap_or_else(|error| failed_child_result(case_id, preconditioner, repeat, error))
}

fn tag_preconditioner_result(result: &mut BenchmarkResult) {
    let preconditioner = result.solver_preconditioner.as_deref().unwrap_or("unknown");
    result.id = format!("{}#{preconditioner}", result.id);
}

fn run_child(
    executable: &Path,
    case_id: &str,
    preconditioner: &str,
    config: &BenchmarkConfig,
) -> Result<BenchmarkResult, String> {
    let mut command = Command::new(executable);
    command
        .args(child_args(case_id, preconditioner, config))
        .stdout(Stdio::piped());
    if config.progress {
        command.stderr(Stdio::inherit());
    } else {
        command.stderr(Stdio::piped());
    }

    let output = command
        .output()
        .map_err(|error| format!("failed to start isolated benchmark child: {error}"))?;
    let report = serde_json::from_slice::<BenchmarkReport>(&output.stdout).map_err(|error| {
        let status = output.status.code().map_or_else(
            || "terminated by signal".to_string(),
            |code| format!("exit {code}"),
        );
        let stderr = String::from_utf8_lossy(&output.stderr);
        let detail = stderr.trim();
        if detail.is_empty() {
            format!("isolated benchmark child {status}; invalid JSON report: {error}")
        } else {
            format!("isolated benchmark child {status}: {detail}; invalid JSON report: {error}")
        }
    })?;
    let mut cases = report.cases.into_iter();
    let result = cases
        .next()
        .ok_or_else(|| "isolated benchmark child returned no case result".to_string())?;
    if cases.next().is_some() {
        return Err("isolated benchmark child returned multiple case results".to_string());
    }
    if result.id != case_id {
        return Err(format!(
            "isolated benchmark child returned '{}' for requested case '{case_id}'",
            result.id
        ));
    }
    Ok(result)
}

fn child_args(case_id: &str, preconditioner: &str, config: &BenchmarkConfig) -> Vec<String> {
    let mut args = vec![
        "--matrix".to_string(),
        config.matrix.clone(),
        "--profile".to_string(),
        config.profile.as_str().to_string(),
        "--case-exact".to_string(),
        case_id.to_string(),
        "--repeat".to_string(),
        config.repeat.to_string(),
        "--format".to_string(),
        "json".to_string(),
        "--solver-preconditioner".to_string(),
        preconditioner.to_string(),
        "--case-isolation".to_string(),
        "in-process".to_string(),
    ];
    if config.progress {
        args.push("--progress".to_string());
    }
    args
}

fn failed_child_result(
    case_id: &str,
    preconditioner: &str,
    repeat: usize,
    error: String,
) -> BenchmarkResult {
    BenchmarkResult {
        id: case_id.to_string(),
        family: "isolated_child".to_string(),
        ok: false,
        error: Some(error),
        repeat,
        min_ms: 0.0,
        median_ms: 0.0,
        mean_ms: 0.0,
        p95_ms: 0.0,
        max_ms: 0.0,
        dof_count: 0,
        node_count: 0,
        element_count: 0,
        history_step_count: None,
        peak_rss_kib: 0,
        memory_stages: Vec::new(),
        solver_iterations: None,
        solver_matrix_non_zero_count: None,
        solver_residual_norm: None,
        solver_preconditioner: Some(preconditioner.to_string()),
        solver_preconditioner_reason: Some("isolated-child-failure".to_string()),
        hotspot_label: None,
        hotspot_elapsed_ms: None,
        hotspot_share_pct: None,
        hotspot_hint: None,
        max_displacement: 0.0,
        max_stress: 0.0,
    }
}

#[cfg(test)]
mod tests {
    use super::{build_isolated_report_with, child_args, failed_child_result};
    use crate::config::BenchmarkConfig;

    #[test]
    fn comparison_mode_runs_each_supported_preconditioner_in_isolation() {
        let config = BenchmarkConfig {
            solver_preconditioner: "all".to_string(),
            ..BenchmarkConfig::default()
        };
        let mut calls = Vec::new();
        let report = build_isolated_report_with(
            &["frame-2d-10k".to_string()],
            &config,
            |case_id, preconditioner| {
                calls.push(preconditioner.to_string());
                Ok(successful_result(case_id, Some(preconditioner)))
            },
        );

        assert_eq!(calls, ["jacobi", "symmetric-gauss-seidel", "ic0"]);
        assert_eq!(report.cases.len(), 3);
        assert!(report.cases.iter().all(|result| result.id.contains('#')));
    }

    #[test]
    fn comparison_mode_only_runs_non_configurable_cases_once() {
        let config = BenchmarkConfig {
            solver_preconditioner: "all".to_string(),
            ..BenchmarkConfig::default()
        };
        let mut calls = 0;
        let report =
            build_isolated_report_with(&["stokes-flow-10k".to_string()], &config, |case_id, _| {
                calls += 1;
                Ok(successful_result(case_id, None))
            });

        assert_eq!(calls, 1);
        assert_eq!(report.cases[0].id, "stokes-flow-10k");
    }

    #[test]
    fn child_command_uses_exact_selection_and_disables_recursion() {
        let config = BenchmarkConfig::default();
        let args = child_args("frame-2d-10k", "ic0", &config);

        assert!(
            args.windows(2)
                .any(|pair| pair == ["--case-exact", "frame-2d-10k"])
        );
        assert!(
            args.windows(2)
                .any(|pair| pair == ["--case-isolation", "in-process"])
        );
    }

    fn successful_result(
        case_id: &str,
        preconditioner: Option<&str>,
    ) -> crate::models::BenchmarkResult {
        let mut result =
            failed_child_result(case_id, preconditioner.unwrap_or("none"), 1, String::new());
        result.ok = true;
        result.error = None;
        result.median_ms = 1.0;
        result.solver_preconditioner = preconditioner.map(str::to_string);
        result
    }
}
