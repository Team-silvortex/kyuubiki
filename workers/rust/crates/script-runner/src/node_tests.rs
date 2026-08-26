use std::ffi::OsString;
use std::fs;
use std::io::{self, Write};
use std::path::Path;
use std::process::Command;

use crate::RunnerResult;
use serde_json::json;

const FRONTEND_COVERAGE_SCHEMA: &str = "kyuubiki.frontend-coverage-summary/v1";
const DEFAULT_COVERAGE_LINES: u8 = 50;
const DEFAULT_COVERAGE_BRANCHES: u8 = 60;
const DEFAULT_COVERAGE_FUNCTIONS: u8 = 55;

#[derive(Debug, Eq, PartialEq)]
struct FrontendCoverageOptions {
    out_dir: String,
    filters: Vec<OsString>,
    lines: u8,
    branches: u8,
    functions: u8,
}

#[derive(Debug, PartialEq)]
struct FrontendCoverageMetrics {
    lines: f64,
    branches: f64,
    functions: f64,
}

pub(crate) fn run_node_command(
    paths: &crate::RepoPaths,
    command: &str,
    rest: Vec<OsString>,
) -> Option<RunnerResult<u8>> {
    let result = match command {
        "playground-fem-node-test" => {
            run_node_test(&paths.root, &["apps/web/playground/test/fem.test.mjs"])
        }
        "frontend-typecheck" => run_frontend_typecheck(&paths.frontend, rest),
        "frontend-unit-test" => run_frontend_unit_test(&paths.frontend, &[], rest),
        "frontend-unit-coverage-test" => {
            run_frontend_unit_coverage_test(&paths.root, &paths.frontend, rest)
        }
        "frontend-unit-workflow-test" => {
            run_frontend_unit_test(&paths.frontend, &["workflow"], rest)
        }
        "frontend-ui-layout-check" => {
            run_frontend_check(&paths.frontend, "./scripts/check-ui-layout.mjs")
        }
        "frontend-workflow-search-layout-check" => run_frontend_check(
            &paths.frontend,
            "./scripts/check-workflow-search-layout.mjs",
        ),
        "frontend-workflow-topology-check" => run_frontend_check(
            &paths.frontend,
            "./scripts/check-workflow-topology-regression.mjs",
        ),
        "frontend-workflow-benchmark" => {
            run_frontend_check(&paths.frontend, "./scripts/workflow-benchmark.mjs")
        }
        "hub-gui-compile-ui" => run_hub_gui_compile(&paths.hub_gui),
        "hub-gui-smoke-node-test" => run_hub_gui_smoke(&paths.hub_gui),
        "installer-gui-smoke-node-test" => run_app_smoke(&paths.installer_gui),
        "workbench-gui-smoke-node-test" => run_app_smoke(&paths.workbench_gui),
        "integration-api-node-test" => run_node_test(
            &paths.root,
            &["tests/integration/orchestrator-agent-api-smoke.test.mjs"],
        ),
        "integration-cluster-node-test" => run_node_test(
            &paths.root,
            &["tests/integration/distributed-control-plane-smoke.test.mjs"],
        ),
        "integration-direct-mesh-node-test" => run_node_test(
            &paths.root,
            &["tests/integration/direct-mesh-gui-smoke.test.mjs"],
        ),
        "integration-desktop-gui-node-test" => run_node_test_serial(
            &paths.root,
            &[
                "apps/desktop-shared/test/tauri-bridge.test.mjs",
                "tests/integration/desktop-gui-action-sweep.test.mjs",
                "tests/integration/desktop-gui-capability-closure.test.mjs",
                "tests/integration/desktop-gui-call-chain-contract.test.mjs",
                "tests/integration/desktop-gui-layout-priority.test.mjs",
                "tests/integration/desktop-gui-navigation-closure.test.mjs",
                "tests/integration/desktop-shell-regression.test.mjs",
                "tests/integration/workbench-shell-regression.test.mjs",
            ],
        ),
        "integration-benchmark-profile-index-node-test" => run_node_test(
            &paths.root,
            &["tests/integration/benchmark-profile-index.test.mjs"],
        ),
        "integration-ui-mechanical-node-test" => run_node_test(
            &paths.root,
            &["tests/integration/workbench-ui-mechanical-smoke.test.mjs"],
        ),
        "integration-ui-thermal-node-test" => run_node_test(
            &paths.root,
            &["tests/integration/workbench-ui-thermal-smoke.test.mjs"],
        ),
        "integration-ui-workflow-node-test" => run_node_test(
            &paths.root,
            &["tests/integration/workbench-ui-workflow-invocation.test.mjs"],
        ),
        _ => return None,
    };
    Some(result)
}

pub(crate) fn run_node_script(
    cwd: &Path,
    script_path: &str,
    fixed_args: &[&str],
    rest: Vec<OsString>,
) -> RunnerResult<u8> {
    crate::run_command(
        cwd,
        "node",
        std::iter::once(OsString::from(script_path))
            .chain(fixed_args.iter().map(OsString::from))
            .chain(rest),
    )
}

pub(crate) fn run_node_test(cwd: &Path, test_paths: &[&str]) -> RunnerResult<u8> {
    crate::run_command(
        cwd,
        "node",
        std::iter::once(OsString::from("--test")).chain(test_paths.iter().map(OsString::from)),
    )
}

fn run_node_test_serial(cwd: &Path, test_paths: &[&str]) -> RunnerResult<u8> {
    crate::run_command(
        cwd,
        "node",
        [
            OsString::from("--test"),
            OsString::from("--test-concurrency=1"),
        ]
        .into_iter()
        .chain(test_paths.iter().map(OsString::from)),
    )
}

pub(crate) fn run_hub_gui_compile(hub_gui: &Path) -> RunnerResult<u8> {
    crate::run_command(
        hub_gui,
        "node",
        [OsString::from("./scripts/compile-ui.mjs")],
    )
}

pub(crate) fn run_hub_gui_smoke(hub_gui: &Path) -> RunnerResult<u8> {
    let compile = run_hub_gui_compile(hub_gui)?;
    if compile != 0 {
        return Ok(compile);
    }
    run_node_test(hub_gui, &["./test/smoke.test.mjs"])
}

pub(crate) fn run_app_smoke(app_dir: &Path) -> RunnerResult<u8> {
    run_node_test(app_dir, &["./test/smoke.test.mjs"])
}

pub(crate) fn run_frontend_typecheck(frontend: &Path, rest: Vec<OsString>) -> RunnerResult<u8> {
    run_node_script(frontend, "./scripts/typecheck.mjs", &[], rest)
}

pub(crate) fn run_frontend_unit_test(
    frontend: &Path,
    fixed_args: &[&str],
    rest: Vec<OsString>,
) -> RunnerResult<u8> {
    run_node_script(frontend, "./scripts/test-unit.mjs", fixed_args, rest)
}

pub(crate) fn run_frontend_unit_coverage_test(
    root: &Path,
    frontend: &Path,
    rest: Vec<OsString>,
) -> RunnerResult<u8> {
    let options = parse_frontend_coverage_args(rest)?;
    validate_coverage_out_dir(&options.out_dir)?;
    let absolute_out = root.join(&options.out_dir);
    if absolute_out.exists() {
        fs::remove_dir_all(&absolute_out)
            .map_err(|error| format!("failed to clear {}: {error}", absolute_out.display()))?;
    }
    fs::create_dir_all(&absolute_out)
        .map_err(|error| format!("failed to create {}: {error}", absolute_out.display()))?;
    let output = Command::new("node")
        .arg("./scripts/test-unit.mjs")
        .args(&options.filters)
        .current_dir(frontend)
        .env("NO_COLOR", "1")
        .env("KYUUBIKI_FRONTEND_COVERAGE", "1")
        .env(
            "KYUUBIKI_FRONTEND_COVERAGE_LINES",
            options.lines.to_string(),
        )
        .env(
            "KYUUBIKI_FRONTEND_COVERAGE_BRANCHES",
            options.branches.to_string(),
        )
        .env(
            "KYUUBIKI_FRONTEND_COVERAGE_FUNCTIONS",
            options.functions.to_string(),
        )
        .output()
        .map_err(|error| format!("failed to run frontend coverage tests: {error}"))?;
    io::stdout()
        .write_all(&output.stdout)
        .map_err(|error| format!("failed to relay frontend coverage stdout: {error}"))?;
    io::stderr()
        .write_all(&output.stderr)
        .map_err(|error| format!("failed to relay frontend coverage stderr: {error}"))?;

    let process_code = output.status.code().unwrap_or(1) as u8;
    let combined_output = format!(
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let parsed_metrics = parse_frontend_coverage_summary(&combined_output);
    let summary_error = parsed_metrics.as_ref().err().cloned();
    let code = if process_code == 0 && parsed_metrics.is_err() {
        1
    } else {
        process_code
    };
    let coverage = parsed_metrics.as_ref().ok().map(|metrics| {
        json!({
            "lines": metrics.lines,
            "branches": metrics.branches,
            "functions": metrics.functions,
        })
    });
    let report = json!({
        "schema_version": FRONTEND_COVERAGE_SCHEMA,
        "status": if code == 0 { "passed" } else { "failed" },
        "process_exit_code": process_code,
        "test_filters": options.filters.iter().map(|value| value.to_string_lossy()).collect::<Vec<_>>(),
        "thresholds": {
            "lines": options.lines,
            "branches": options.branches,
            "functions": options.functions,
        },
        "coverage": coverage,
        "summary_error": summary_error,
        "raw_v8_coverage_retained": false,
    });
    let summary_path = absolute_out.join("summary.json");
    fs::write(
        &summary_path,
        format!(
            "{}\n",
            serde_json::to_string_pretty(&report)
                .map_err(|error| format!("failed to encode frontend coverage summary: {error}"))?
        ),
    )
    .map_err(|error| format!("failed to write {}: {error}", summary_path.display()))?;
    println!(
        "frontend coverage summary: {}/summary.json",
        options.out_dir
    );
    if let Some(error) = parsed_metrics.err() {
        eprintln!("frontend coverage summary unavailable: {error}");
    }
    Ok(code)
}

pub(crate) fn run_frontend_check(frontend: &Path, script_path: &str) -> RunnerResult<u8> {
    run_node_script(frontend, script_path, &[], Vec::new())
}

fn parse_frontend_coverage_args(rest: Vec<OsString>) -> RunnerResult<FrontendCoverageOptions> {
    let mut options = FrontendCoverageOptions {
        out_dir: "tmp/coverage/frontend/v8".to_string(),
        filters: Vec::new(),
        lines: DEFAULT_COVERAGE_LINES,
        branches: DEFAULT_COVERAGE_BRANCHES,
        functions: DEFAULT_COVERAGE_FUNCTIONS,
    };
    let mut iter = rest.into_iter();
    while let Some(arg) = iter.next() {
        match arg.to_str() {
            Some("--out-dir") => options.out_dir = next_coverage_arg(&mut iter, "--out-dir")?,
            Some("--lines") => options.lines = next_coverage_threshold(&mut iter, "--lines")?,
            Some("--branches") => {
                options.branches = next_coverage_threshold(&mut iter, "--branches")?
            }
            Some("--functions") => {
                options.functions = next_coverage_threshold(&mut iter, "--functions")?
            }
            Some(value) if value.starts_with("--") => {
                return Err(format!("unknown frontend coverage argument: {value}"));
            }
            _ => options.filters.push(arg),
        }
    }
    Ok(options)
}

fn next_coverage_arg(
    iter: &mut impl Iterator<Item = OsString>,
    flag: &str,
) -> RunnerResult<String> {
    iter.next()
        .ok_or_else(|| format!("{flag} requires a value"))?
        .into_string()
        .map_err(|_| format!("{flag} value is not valid utf-8"))
}

fn next_coverage_threshold(
    iter: &mut impl Iterator<Item = OsString>,
    flag: &str,
) -> RunnerResult<u8> {
    let value = next_coverage_arg(iter, flag)?;
    value
        .parse::<u8>()
        .ok()
        .filter(|value| *value <= 100)
        .ok_or_else(|| format!("{flag} must be an integer from 0 through 100"))
}

fn parse_frontend_coverage_summary(stdout: &str) -> RunnerResult<FrontendCoverageMetrics> {
    let line = stdout
        .lines()
        .rev()
        .find(|line| line.contains("all files") && line.contains('|'))
        .ok_or_else(|| "frontend coverage output misses the all-files summary".to_string())?;
    let columns = line.split('|').map(str::trim).collect::<Vec<_>>();
    if columns.len() < 4 {
        return Err("frontend coverage all-files summary is malformed".to_string());
    }
    let parse = |index: usize, name: &str| {
        columns[index]
            .parse::<f64>()
            .ok()
            .filter(|value| value.is_finite() && (0.0..=100.0).contains(value))
            .ok_or_else(|| format!("frontend coverage {name} percentage is invalid"))
    };
    Ok(FrontendCoverageMetrics {
        lines: parse(1, "lines")?,
        branches: parse(2, "branches")?,
        functions: parse(3, "functions")?,
    })
}

fn validate_coverage_out_dir(out_dir: &str) -> RunnerResult<()> {
    if Path::new(out_dir).is_absolute()
        || out_dir.split('/').any(|part| part == "..")
        || !out_dir.starts_with("tmp/coverage/")
    {
        return Err("frontend coverage out dir must be under tmp/coverage/".to_string());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frontend_coverage_args_reject_unsafe_out_dir() {
        assert!(parse_frontend_coverage_args(vec![OsString::from("workflow")]).is_ok());
        let parsed = parse_frontend_coverage_args(vec![
            OsString::from("--out-dir"),
            OsString::from("tmp/coverage/frontend/v8"),
            OsString::from("--lines"),
            OsString::from("51"),
            OsString::from("workflow"),
        ])
        .unwrap();
        assert_eq!(parsed.out_dir, "tmp/coverage/frontend/v8");
        assert_eq!(parsed.filters, vec![OsString::from("workflow")]);
        assert_eq!(parsed.lines, 51);
        assert_eq!(parsed.branches, DEFAULT_COVERAGE_BRANCHES);
        assert!(
            parse_frontend_coverage_args(vec![OsString::from("--lines"), OsString::from("101")])
                .is_err()
        );
        assert!(parse_frontend_coverage_args(vec![OsString::from("--unknown")]).is_err());
        assert!(validate_coverage_out_dir("/tmp/coverage").is_err());
        assert!(validate_coverage_out_dir("tmp/../coverage").is_err());
        assert!(validate_coverage_out_dir("dist/coverage").is_err());
    }

    #[test]
    fn frontend_coverage_summary_parses_all_files_row() {
        let metrics = parse_frontend_coverage_summary(
            "ℹ file | % line | % branch | % funcs\nℹ all files | 53.67 | 63.48 | 57.83 |\n",
        )
        .unwrap();
        assert_eq!(
            metrics,
            FrontendCoverageMetrics {
                lines: 53.67,
                branches: 63.48,
                functions: 57.83,
            }
        );
        assert!(parse_frontend_coverage_summary("no summary").is_err());
    }
}
