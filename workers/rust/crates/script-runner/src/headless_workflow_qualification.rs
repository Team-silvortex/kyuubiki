use crate::qualification_support::{
    generated_at_unix_ms, parse_options, portable_output, read_json, repo_path, write_json,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::ffi::OsString;
use std::fs;
use std::path::Path;
use std::process::Command;
use std::time::Instant;

type RunnerResult<T> = Result<T, String>;

const CONTRACT_PATH: &str = "config/architecture/headless-workflow-qualification.json";
const CONTRACT_SCHEMA: &str = "kyuubiki.headless-workflow-qualification-contract/v1";
const REPORT_SCHEMA: &str = "kyuubiki.headless-workflow-qualification-report/v1";
const DEFAULT_OUT: &str = "tmp/headless-workflow-qualification-report.json";

#[derive(Deserialize)]
struct QualificationContract {
    schema_version: String,
    report_schema: String,
    minimum_total_tests: usize,
    suites: Vec<SuiteContract>,
}

#[derive(Deserialize)]
struct SuiteContract {
    id: String,
    minimum_tests: usize,
    source_files: Vec<String>,
    required_tests: Vec<String>,
}

#[derive(Deserialize, Serialize)]
struct QualificationReport {
    schema_version: String,
    generated_at_unix_ms: u128,
    contract_path: String,
    status: String,
    platform: Platform,
    total_passed: usize,
    suites: Vec<SuiteReport>,
}

#[derive(Deserialize, Serialize)]
struct Platform {
    os: String,
    arch: String,
}

#[derive(Deserialize, Serialize)]
struct SuiteReport {
    id: String,
    command: Vec<String>,
    working_directory: String,
    exit_code: Option<i32>,
    elapsed_ms: u128,
    passed: usize,
    failed: usize,
    ignored: usize,
    required_tests: Vec<CheckResult>,
    output_excerpt: String,
}

#[derive(Deserialize, Serialize)]
struct CheckResult {
    id: String,
    passed: bool,
}

#[derive(Default)]
struct RustSummary {
    passed: usize,
    failed: usize,
    ignored: usize,
}

pub(crate) fn run_check_headless_workflow_qualification(
    root: &Path,
    args: Vec<OsString>,
) -> RunnerResult<u8> {
    let options = parse_options(args, "Headless workflow qualification")?;
    if options.self_test {
        run_self_test()?;
        println!("Headless workflow qualification self-test passed");
        return Ok(0);
    }
    let contract: QualificationContract = read_json(root, CONTRACT_PATH)?;
    validate_contract(root, &contract)?;
    if let Some(path) = options.verify_report {
        let report: QualificationReport = read_json(root, &path)?;
        validate_report(&contract, &report)?;
        println!("Headless workflow qualification report passed: {path}");
        return Ok(0);
    }

    let report = execute_qualification(root, &contract)?;
    let out = options.out.as_deref().unwrap_or(DEFAULT_OUT);
    write_json(root, out, &report)?;
    if let Err(error) = validate_report(&contract, &report) {
        eprintln!("Headless workflow qualification failed: {error}");
        eprintln!("failure report written: {out}");
        return Ok(1);
    }
    println!(
        "Headless workflow qualified: {} tests across {} suites",
        report.total_passed,
        report.suites.len()
    );
    println!("Headless workflow qualification report written: {out}");
    Ok(0)
}

fn validate_contract(root: &Path, contract: &QualificationContract) -> RunnerResult<()> {
    if contract.schema_version != CONTRACT_SCHEMA || contract.report_schema != REPORT_SCHEMA {
        return Err("Headless workflow qualification schemas are invalid".to_string());
    }
    if contract.minimum_total_tests < 230 || contract.suites.len() != 2 {
        return Err("Headless workflow qualification thresholds are too weak".to_string());
    }
    let expected = BTreeSet::from(["headless-sdk-core", "headless-cli-boundaries"]);
    let ids = contract
        .suites
        .iter()
        .map(|suite| suite.id.as_str())
        .collect::<BTreeSet<_>>();
    if ids != expected
        || contract
            .suites
            .iter()
            .map(|suite| suite.minimum_tests)
            .sum::<usize>()
            < contract.minimum_total_tests
    {
        return Err("Headless workflow qualification suite set drifted".to_string());
    }
    for suite in &contract.suites {
        require_unique_nonempty(&suite.source_files, "source file")?;
        require_unique_nonempty(&suite.required_tests, "required test")?;
        let mut source = String::new();
        for path in &suite.source_files {
            if !path.starts_with("workers/rust/") || !path.ends_with(".rs") {
                return Err(format!("invalid Headless workflow source path: {path}"));
            }
            source.push_str(
                &fs::read_to_string(repo_path(root, path)?)
                    .map_err(|error| format!("failed to read {path}: {error}"))?,
            );
        }
        for test in &suite.required_tests {
            let function = test.rsplit("::").next().unwrap_or(test);
            if !source.contains(&format!("fn {function}")) {
                return Err(format!("required Headless workflow test drifted: {test}"));
            }
        }
    }
    Ok(())
}

fn execute_qualification(
    root: &Path,
    contract: &QualificationContract,
) -> RunnerResult<QualificationReport> {
    let suites = contract
        .suites
        .iter()
        .map(|suite| run_suite(root, suite))
        .collect::<RunnerResult<Vec<_>>>()?;
    let total_passed = suites.iter().map(|suite| suite.passed).sum();
    let passed = total_passed >= contract.minimum_total_tests
        && suites.iter().zip(&contract.suites).all(|(report, suite)| {
            report.exit_code == Some(0)
                && report.passed >= suite.minimum_tests
                && report.failed == 0
                && report.required_tests.iter().all(|test| test.passed)
        });
    Ok(QualificationReport {
        schema_version: REPORT_SCHEMA.to_string(),
        generated_at_unix_ms: generated_at_unix_ms()?,
        contract_path: CONTRACT_PATH.to_string(),
        status: if passed { "pass" } else { "fail" }.to_string(),
        platform: Platform {
            os: std::env::consts::OS.to_string(),
            arch: std::env::consts::ARCH.to_string(),
        },
        total_passed,
        suites,
    })
}

fn run_suite(root: &Path, contract: &SuiteContract) -> RunnerResult<SuiteReport> {
    let command = suite_command(&contract.id)?;
    let started = Instant::now();
    let output = Command::new(&command[0])
        .args(&command[1..])
        .current_dir(root.join("workers/rust"))
        .env("NO_COLOR", "1")
        .env("CARGO_TERM_COLOR", "never")
        .output()
        .map_err(|error| format!("failed to execute {}: {error}", contract.id))?;
    let rendered = portable_output(root, &output);
    let summary = parse_rust_summary(&rendered);
    Ok(SuiteReport {
        id: contract.id.clone(),
        command,
        working_directory: "workers/rust".to_string(),
        exit_code: output.status.code(),
        elapsed_ms: started.elapsed().as_millis(),
        passed: summary.passed,
        failed: summary.failed,
        ignored: summary.ignored,
        required_tests: contract
            .required_tests
            .iter()
            .map(|id| CheckResult {
                id: id.clone(),
                passed: rust_test_passed(&rendered, id),
            })
            .collect(),
        output_excerpt: rendered.chars().take(48_000).collect(),
    })
}

fn suite_command(id: &str) -> RunnerResult<Vec<String>> {
    let values = match id {
        "headless-sdk-core" => vec![
            "cargo",
            "test",
            "-p",
            "kyuubiki-headless-sdk",
            "--",
            "--test-threads=1",
        ],
        "headless-cli-boundaries" => vec![
            "cargo",
            "test",
            "-p",
            "kyuubiki-cli",
            "--test",
            "headless_execution_posture",
            "--",
            "--test-threads=1",
        ],
        _ => return Err(format!("unsupported Headless workflow suite: {id}")),
    };
    Ok(values.into_iter().map(str::to_string).collect())
}

fn parse_rust_summary(output: &str) -> RustSummary {
    output
        .lines()
        .filter_map(|line| {
            let fields = line.trim().strip_prefix("test result: ok. ")?;
            let mut summary = RustSummary::default();
            for field in fields.split(';') {
                let mut parts = field.split_whitespace();
                let Some(value) = parts.next().and_then(|value| value.parse::<usize>().ok()) else {
                    continue;
                };
                match parts.next() {
                    Some("passed") => summary.passed = value,
                    Some("failed") => summary.failed = value,
                    Some("ignored") => summary.ignored = value,
                    _ => {}
                }
            }
            Some(summary)
        })
        .max_by_key(|summary| summary.passed)
        .unwrap_or_default()
}

fn rust_test_passed(output: &str, id: &str) -> bool {
    output
        .lines()
        .any(|line| line.trim() == format!("test {id} ... ok"))
}

fn validate_report(
    contract: &QualificationContract,
    report: &QualificationReport,
) -> RunnerResult<()> {
    if report.schema_version != REPORT_SCHEMA
        || report.contract_path != CONTRACT_PATH
        || report.status != "pass"
        || report.generated_at_unix_ms == 0
        || report.platform.os.is_empty()
        || report.platform.arch.is_empty()
        || report.total_passed < contract.minimum_total_tests
        || report.suites.len() != contract.suites.len()
    {
        return Err("Headless workflow qualification report header is invalid".to_string());
    }
    for suite in &contract.suites {
        let Some(result) = report.suites.iter().find(|result| result.id == suite.id) else {
            return Err(format!("Headless workflow suite is missing: {}", suite.id));
        };
        if result.exit_code != Some(0)
            || result.passed < suite.minimum_tests
            || result.failed != 0
            || !exact_passing_checks(&result.required_tests, &suite.required_tests)
        {
            return Err(format!(
                "Headless workflow suite is not qualified: {}",
                suite.id
            ));
        }
    }
    Ok(())
}

fn exact_passing_checks(checks: &[CheckResult], expected: &[String]) -> bool {
    checks.len() == expected.len()
        && expected
            .iter()
            .all(|id| checks.iter().any(|check| check.id == *id && check.passed))
}

fn require_unique_nonempty(values: &[String], label: &str) -> RunnerResult<()> {
    let mut seen = BTreeSet::new();
    for value in values {
        if value.trim().is_empty() || !seen.insert(value.as_str()) {
            return Err(format!("invalid or duplicate {label}: {value}"));
        }
    }
    Ok(())
}

fn run_self_test() -> RunnerResult<()> {
    let output = "test preflight_report::tests::report ... ok\n\
test result: ok. 234 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.0s\n";
    let summary = parse_rust_summary(output);
    if summary.passed != 234
        || summary.failed != 0
        || !rust_test_passed(output, "preflight_report::tests::report")
    {
        return Err("Headless workflow parser self-test failed".to_string());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn parses_qualification_output() {
        super::run_self_test().unwrap();
    }
}
