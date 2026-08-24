use crate::qualification_support::{
    combined_output, generated_at_unix_ms, parse_options, portable_output, read_json, repo_path,
    write_json,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::ffi::OsString;
use std::fs;
use std::path::Path;
use std::process::Command;
use std::time::Instant;

type RunnerResult<T> = Result<T, String>;

const CONTRACT_PATH: &str = "config/architecture/desktop-deployment-update-qualification.json";
const CONTRACT_SCHEMA: &str = "kyuubiki.desktop-deployment-update-qualification-contract/v1";
const REPORT_SCHEMA: &str = "kyuubiki.desktop-deployment-update-qualification-report/v1";
const DEFAULT_OUT: &str = "tmp/desktop-deployment-update-qualification-report.json";

#[derive(Deserialize)]
struct QualificationContract {
    schema_version: String,
    report_schema: String,
    minimum_installer_tests: usize,
    minimum_browser_tests: usize,
    browser_test_files: Vec<String>,
    required_installer_tests: Vec<String>,
    required_browser_assertions: Vec<String>,
    contract_checks: Vec<String>,
}

#[derive(Deserialize, Serialize)]
struct QualificationReport {
    schema_version: String,
    generated_at_unix_ms: u128,
    contract_path: String,
    status: String,
    platform: Platform,
    installer_tests: InstallerTestReport,
    browser_tests: BrowserTestReport,
    contract_checks: Vec<CommandReport>,
}

#[derive(Deserialize, Serialize)]
struct Platform {
    os: String,
    arch: String,
}

#[derive(Deserialize, Serialize)]
struct InstallerTestReport {
    command: Vec<String>,
    exit_code: Option<i32>,
    elapsed_ms: u128,
    passed: usize,
    failed: usize,
    ignored: usize,
    required_tests: Vec<CheckResult>,
    output_excerpt: String,
}

#[derive(Deserialize, Serialize)]
struct BrowserTestReport {
    command: Vec<String>,
    exit_code: Option<i32>,
    elapsed_ms: u128,
    tests: usize,
    passed: usize,
    failed: usize,
    cancelled: usize,
    skipped: usize,
    todo: usize,
    assertions: Vec<CheckResult>,
    output_excerpt: String,
}

#[derive(Clone, Deserialize, Serialize)]
struct CheckResult {
    id: String,
    passed: bool,
}

#[derive(Deserialize, Serialize)]
struct CommandReport {
    command: String,
    status: String,
    exit_code: Option<i32>,
    elapsed_ms: u128,
    output: String,
}

#[derive(Default)]
struct RustSummary {
    passed: usize,
    failed: usize,
    ignored: usize,
}

#[derive(Default)]
struct TapSummary {
    tests: usize,
    passed: usize,
    failed: usize,
    cancelled: usize,
    skipped: usize,
    todo: usize,
}

pub(crate) fn run_check_desktop_deployment_update_qualification(
    root: &Path,
    args: Vec<OsString>,
) -> RunnerResult<u8> {
    let options = parse_options(args, "desktop deployment/update qualification")?;
    if options.self_test {
        run_self_test()?;
        println!("desktop deployment/update qualification self-test passed");
        return Ok(0);
    }
    let contract: QualificationContract = read_json(root, CONTRACT_PATH)?;
    validate_contract(root, &contract)?;
    if let Some(path) = options.verify_report {
        let report: QualificationReport = read_json(root, &path)?;
        validate_report(&contract, &report)?;
        println!("desktop deployment/update qualification report passed: {path}");
        return Ok(0);
    }

    let report = execute_qualification(root, &contract)?;
    let out = options.out.as_deref().unwrap_or(DEFAULT_OUT);
    write_json(root, out, &report)?;
    if let Err(error) = validate_report(&contract, &report) {
        eprintln!("desktop deployment/update qualification failed: {error}");
        eprintln!("failure report written: {out}");
        return Ok(1);
    }
    println!(
        "desktop deployment/update qualified: {} installer tests, {} browser tests",
        report.installer_tests.passed, report.browser_tests.passed
    );
    println!("desktop deployment/update qualification report written: {out}");
    Ok(0)
}

fn validate_contract(root: &Path, contract: &QualificationContract) -> RunnerResult<()> {
    if contract.schema_version != CONTRACT_SCHEMA || contract.report_schema != REPORT_SCHEMA {
        return Err("desktop deployment/update qualification schemas are invalid".to_string());
    }
    if contract.minimum_installer_tests < 50 || contract.minimum_browser_tests < 5 {
        return Err("desktop deployment/update qualification thresholds are too weak".to_string());
    }
    require_unique_nonempty(&contract.browser_test_files, "browser test file")?;
    require_unique_nonempty(&contract.required_installer_tests, "installer test")?;
    require_unique_nonempty(&contract.required_browser_assertions, "browser assertion")?;
    require_unique_nonempty(&contract.contract_checks, "contract check")?;
    if contract.contract_checks
        != [
            "check-install-update-disk-hygiene",
            "check-gui-runtime-capability-contract",
        ]
    {
        return Err("desktop deployment/update contract check set drifted".to_string());
    }

    let installer_tests = fs::read_to_string(repo_path(
        root,
        "workers/rust/crates/installer/src/tests/update_delivery.rs",
    )?)
    .map_err(|error| format!("failed to read update delivery tests: {error}"))?;
    for test in &contract.required_installer_tests {
        let short = test.rsplit("::").next().unwrap_or(test);
        if !installer_tests.contains(&format!("fn {short}")) {
            return Err(format!("required Installer test drifted: {test}"));
        }
    }

    let mut browser_source = String::new();
    for path in &contract.browser_test_files {
        if !path.starts_with("tests/integration/") || !path.ends_with(".test.mjs") {
            return Err(format!("invalid browser qualification path: {path}"));
        }
        browser_source.push_str(
            &fs::read_to_string(repo_path(root, path)?)
                .map_err(|error| format!("failed to read {path}: {error}"))?,
        );
    }
    for assertion in &contract.required_browser_assertions {
        let generated_fragment = assertion
            .split_once(' ')
            .map(|(_, remainder)| remainder)
            .unwrap_or(assertion);
        if !browser_source.contains(assertion) && !browser_source.contains(generated_fragment) {
            return Err(format!("required browser assertion drifted: {assertion}"));
        }
    }
    Ok(())
}

fn execute_qualification(
    root: &Path,
    contract: &QualificationContract,
) -> RunnerResult<QualificationReport> {
    let installer_tests = run_installer_tests(root, contract)?;
    let browser_tests = run_browser_tests(root, contract)?;
    let contract_checks = contract
        .contract_checks
        .iter()
        .map(|command| run_runner_command(command))
        .collect::<RunnerResult<Vec<_>>>()?;
    let passed = installer_tests.exit_code == Some(0)
        && installer_tests.passed >= contract.minimum_installer_tests
        && installer_tests.failed == 0
        && installer_tests
            .required_tests
            .iter()
            .all(|test| test.passed)
        && browser_tests.exit_code == Some(0)
        && browser_tests.tests >= contract.minimum_browser_tests
        && browser_tests.tests == browser_tests.passed
        && browser_tests.failed
            + browser_tests.cancelled
            + browser_tests.skipped
            + browser_tests.todo
            == 0
        && browser_tests
            .assertions
            .iter()
            .all(|assertion| assertion.passed)
        && contract_checks.iter().all(|check| check.status == "pass");
    Ok(QualificationReport {
        schema_version: REPORT_SCHEMA.to_string(),
        generated_at_unix_ms: generated_at_unix_ms()?,
        contract_path: CONTRACT_PATH.to_string(),
        status: if passed { "pass" } else { "fail" }.to_string(),
        platform: Platform {
            os: std::env::consts::OS.to_string(),
            arch: std::env::consts::ARCH.to_string(),
        },
        installer_tests,
        browser_tests,
        contract_checks,
    })
}

fn run_installer_tests(
    root: &Path,
    contract: &QualificationContract,
) -> RunnerResult<InstallerTestReport> {
    let command = [
        "cargo",
        "test",
        "-p",
        "kyuubiki-installer",
        "--",
        "--test-threads=1",
    ]
    .into_iter()
    .map(str::to_string)
    .collect::<Vec<_>>();
    let started = Instant::now();
    let output = Command::new(&command[0])
        .args(&command[1..])
        .current_dir(root.join("workers/rust"))
        .env("NO_COLOR", "1")
        .output()
        .map_err(|error| format!("failed to execute Installer tests: {error}"))?;
    let rendered = portable_output(root, &output);
    let summary = parse_rust_summary(&rendered);
    Ok(InstallerTestReport {
        command,
        exit_code: output.status.code(),
        elapsed_ms: started.elapsed().as_millis(),
        passed: summary.passed,
        failed: summary.failed,
        ignored: summary.ignored,
        required_tests: contract
            .required_installer_tests
            .iter()
            .map(|id| CheckResult {
                id: id.clone(),
                passed: rust_test_passed(&rendered, id),
            })
            .collect(),
        output_excerpt: rendered.chars().take(32_000).collect(),
    })
}

fn run_browser_tests(
    root: &Path,
    contract: &QualificationContract,
) -> RunnerResult<BrowserTestReport> {
    let command = ["node", "--test", "--test-reporter=tap"]
        .into_iter()
        .map(str::to_string)
        .chain(contract.browser_test_files.iter().cloned())
        .collect::<Vec<_>>();
    let started = Instant::now();
    let output = Command::new(&command[0])
        .args(&command[1..])
        .current_dir(root)
        .env("NO_COLOR", "1")
        .output()
        .map_err(|error| format!("failed to execute deployment/update browser tests: {error}"))?;
    let rendered = portable_output(root, &output);
    let summary = parse_tap_summary(&rendered);
    Ok(BrowserTestReport {
        command,
        exit_code: output.status.code(),
        elapsed_ms: started.elapsed().as_millis(),
        tests: summary.tests,
        passed: summary.passed,
        failed: summary.failed,
        cancelled: summary.cancelled,
        skipped: summary.skipped,
        todo: summary.todo,
        assertions: contract
            .required_browser_assertions
            .iter()
            .map(|id| CheckResult {
                id: id.clone(),
                passed: tap_test_passed(&rendered, id),
            })
            .collect(),
        output_excerpt: rendered.chars().take(24_000).collect(),
    })
}

fn run_runner_command(command: &str) -> RunnerResult<CommandReport> {
    let executable = std::env::current_exe()
        .map_err(|error| format!("failed to resolve script runner: {error}"))?;
    let started = Instant::now();
    let output = Command::new(executable)
        .arg(command)
        .output()
        .map_err(|error| format!("failed to execute {command}: {error}"))?;
    let rendered = combined_output(&output);
    Ok(CommandReport {
        command: command.to_string(),
        status: if output.status.success() {
            "pass"
        } else {
            "fail"
        }
        .to_string(),
        exit_code: output.status.code(),
        elapsed_ms: started.elapsed().as_millis(),
        output: rendered.chars().take(8_000).collect(),
    })
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

fn parse_tap_summary(output: &str) -> TapSummary {
    TapSummary {
        tests: tap_summary_value(output, "tests"),
        passed: tap_summary_value(output, "pass"),
        failed: tap_summary_value(output, "fail"),
        cancelled: tap_summary_value(output, "cancelled"),
        skipped: tap_summary_value(output, "skipped"),
        todo: tap_summary_value(output, "todo"),
    }
}

fn tap_summary_value(output: &str, key: &str) -> usize {
    output
        .lines()
        .rev()
        .find_map(|line| {
            line.trim()
                .strip_prefix(&format!("# {key} "))
                .and_then(|value| value.parse().ok())
        })
        .unwrap_or(0)
}

fn rust_test_passed(output: &str, id: &str) -> bool {
    output
        .lines()
        .any(|line| line.trim() == format!("test {id} ... ok"))
}

fn tap_test_passed(output: &str, label: &str) -> bool {
    output.lines().any(|line| {
        let line = line.trim();
        line.starts_with("ok ") && line.contains(label)
    })
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
    {
        return Err("desktop deployment/update report header is invalid".to_string());
    }
    if report.installer_tests.exit_code != Some(0)
        || report.installer_tests.passed < contract.minimum_installer_tests
        || report.installer_tests.failed != 0
        || !exact_passing_checks(
            &report.installer_tests.required_tests,
            &contract.required_installer_tests,
        )
    {
        return Err("Installer update test evidence is not qualified".to_string());
    }
    if report.browser_tests.exit_code != Some(0)
        || report.browser_tests.tests < contract.minimum_browser_tests
        || report.browser_tests.tests != report.browser_tests.passed
        || report.browser_tests.failed
            + report.browser_tests.cancelled
            + report.browser_tests.skipped
            + report.browser_tests.todo
            != 0
        || !exact_passing_checks(
            &report.browser_tests.assertions,
            &contract.required_browser_assertions,
        )
    {
        return Err("desktop deployment/update browser evidence is not qualified".to_string());
    }
    if report.contract_checks.len() != contract.contract_checks.len() {
        return Err("desktop deployment/update contract check count drifted".to_string());
    }
    for required in &contract.contract_checks {
        if !report.contract_checks.iter().any(|check| {
            check.command == *required && check.status == "pass" && check.exit_code == Some(0)
        }) {
            return Err(format!("contract check did not pass: {required}"));
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
    let rust = "test tests::update_delivery::tamper ... ok\n\
test result: ok. 60 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.0s\n";
    let tap = "ok 1 - update closure\n# tests 8\n# pass 8\n# fail 0\n# cancelled 0\n# skipped 0\n# todo 0\n";
    let rust_summary = parse_rust_summary(rust);
    let tap_summary = parse_tap_summary(tap);
    if rust_summary.passed != 60
        || !rust_test_passed(rust, "tests::update_delivery::tamper")
        || tap_summary.tests != 8
        || tap_summary.passed != 8
        || !tap_test_passed(tap, "update closure")
    {
        return Err("desktop deployment/update parser self-test failed".to_string());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn parses_qualification_outputs() {
        super::run_self_test().unwrap();
    }
}
