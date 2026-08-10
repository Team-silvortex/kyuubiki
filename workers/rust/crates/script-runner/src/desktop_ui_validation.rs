use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

type RunnerResult<T> = Result<T, String>;

const CONTRACT_PATH: &str = "config/architecture/desktop-ui-validation.json";
const CONTRACT_SCHEMA: &str = "kyuubiki.desktop-ui-validation-contract/v1";
const REPORT_SCHEMA: &str = "kyuubiki.desktop-ui-validation-report/v1";
const DEFAULT_OUT: &str = "tmp/desktop-ui-validation-report.json";

#[derive(Debug, Deserialize)]
struct ValidationContract {
    schema_version: String,
    report_schema: String,
    test_files: Vec<String>,
    minimum_test_count: usize,
    shell_minimum_actions: Vec<ShellRequirement>,
    required_assertions: Vec<AssertionRequirement>,
}

#[derive(Debug, Deserialize)]
struct ShellRequirement {
    id: String,
    minimum_actions: usize,
}

#[derive(Debug, Deserialize)]
struct AssertionRequirement {
    id: String,
    label: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct ValidationReport {
    schema_version: String,
    generated_at_unix_ms: u128,
    contract_path: String,
    status: String,
    platform: Platform,
    command: Vec<String>,
    exit_code: Option<i32>,
    elapsed_ms: u128,
    summary: TestSummary,
    shells: Vec<ShellResult>,
    assertions: Vec<AssertionResult>,
    output_excerpt: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct Platform {
    os: String,
    arch: String,
}

#[derive(Debug, Default, Deserialize, Serialize)]
struct TestSummary {
    tests: usize,
    passed: usize,
    failed: usize,
    cancelled: usize,
    skipped: usize,
    todo: usize,
    failed_actions: usize,
    missing_actions: usize,
}

#[derive(Debug, Deserialize, Serialize)]
struct ShellResult {
    id: String,
    actions: usize,
    blocked: usize,
}

#[derive(Debug, Deserialize, Serialize)]
struct AssertionResult {
    id: String,
    label: String,
    passed: bool,
}

#[derive(Default)]
struct Options {
    out: Option<String>,
    verify_report: Option<String>,
    self_test: bool,
}

pub(crate) fn run_check_desktop_ui_validation(
    root: &Path,
    args: Vec<OsString>,
) -> RunnerResult<u8> {
    let options = parse_options(args)?;
    if options.self_test {
        run_self_test()?;
        println!("desktop UI validation self-test passed");
        return Ok(0);
    }

    let contract: ValidationContract = read_json(root, CONTRACT_PATH)?;
    validate_contract(root, &contract)?;
    if let Some(path) = options.verify_report {
        let report: ValidationReport = read_json(root, &path)?;
        validate_report(&contract, &report)?;
        println!("desktop UI validation report passed: {path}");
        return Ok(0);
    }

    let report = execute_suite(root, &contract)?;
    let output_path = options.out.as_deref().unwrap_or(DEFAULT_OUT);
    write_json(root, output_path, &report)?;
    if let Err(error) = validate_report(&contract, &report) {
        eprintln!("desktop UI validation failed: {error}");
        eprintln!("failure report written: {output_path}");
        return Ok(1);
    }
    println!(
        "desktop UI validation qualified: {}/{} tests, {} shell(s)",
        report.summary.passed,
        report.summary.tests,
        report.shells.len()
    );
    println!("desktop UI validation report written: {output_path}");
    Ok(0)
}

fn parse_options(args: Vec<OsString>) -> RunnerResult<Options> {
    let mut options = Options::default();
    let mut iter = args.into_iter();
    while let Some(arg) = iter.next() {
        match arg.to_string_lossy().as_ref() {
            "--out" => options.out = Some(required_path(&mut iter, "--out")?),
            "--verify-report" => {
                options.verify_report = Some(required_path(&mut iter, "--verify-report")?)
            }
            "--self-test" => options.self_test = true,
            other => return Err(format!("unknown desktop UI validation argument: {other}")),
        }
    }
    if options.out.is_some() && options.verify_report.is_some() {
        return Err("--out and --verify-report cannot be combined".to_string());
    }
    Ok(options)
}

fn required_path(iter: &mut impl Iterator<Item = OsString>, flag: &str) -> RunnerResult<String> {
    iter.next()
        .map(|value| value.to_string_lossy().to_string())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("{flag} requires a repository-relative path"))
}

fn validate_contract(root: &Path, contract: &ValidationContract) -> RunnerResult<()> {
    if contract.schema_version != CONTRACT_SCHEMA || contract.report_schema != REPORT_SCHEMA {
        return Err("desktop UI validation schema contract is invalid".to_string());
    }
    if contract.minimum_test_count < 10 || contract.test_files.len() < 3 {
        return Err("desktop UI validation thresholds are too weak".to_string());
    }
    let mut test_files = BTreeSet::new();
    let mut combined_sources = String::new();
    for relative in &contract.test_files {
        if !relative.starts_with("tests/integration/")
            || !relative.ends_with(".test.mjs")
            || !test_files.insert(relative)
        {
            return Err(format!(
                "invalid or duplicate desktop UI test file: {relative}"
            ));
        }
        combined_sources.push_str(
            &fs::read_to_string(repo_path(root, relative)?)
                .map_err(|error| format!("failed to read {relative}: {error}"))?,
        );
    }
    let shell_ids = contract
        .shell_minimum_actions
        .iter()
        .map(|entry| entry.id.as_str())
        .collect::<BTreeSet<_>>();
    if shell_ids != BTreeSet::from(["hub", "installer", "workbench"])
        || contract
            .shell_minimum_actions
            .iter()
            .any(|entry| entry.minimum_actions == 0)
    {
        return Err("desktop UI validation must cover all three shells".to_string());
    }
    let mut assertion_ids = BTreeSet::new();
    for assertion in &contract.required_assertions {
        if assertion.id.is_empty()
            || assertion.label.is_empty()
            || !assertion_ids.insert(assertion.id.as_str())
            || !combined_sources.contains(&assertion.label)
        {
            return Err(format!("invalid or stale UI assertion: {}", assertion.id));
        }
    }
    Ok(())
}

fn execute_suite(root: &Path, contract: &ValidationContract) -> RunnerResult<ValidationReport> {
    let command = std::iter::once("node".to_string())
        .chain(
            ["--test", "--test-reporter=tap"]
                .into_iter()
                .map(str::to_string),
        )
        .chain(contract.test_files.iter().cloned())
        .collect::<Vec<_>>();
    let started = Instant::now();
    let output = Command::new(&command[0])
        .args(&command[1..])
        .current_dir(root)
        .env("NO_COLOR", "1")
        .output()
        .map_err(|error| format!("failed to execute desktop UI validation: {error}"))?;
    let rendered = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let summary = parse_summary(&rendered);
    let shells = contract
        .shell_minimum_actions
        .iter()
        .filter_map(|requirement| parse_shell_result(&rendered, &requirement.id))
        .collect::<Vec<_>>();
    let assertions = contract
        .required_assertions
        .iter()
        .map(|required| AssertionResult {
            id: required.id.clone(),
            label: required.label.clone(),
            passed: assertion_passed(&rendered, &required.label),
        })
        .collect::<Vec<_>>();
    let passed = output.status.success()
        && summary.tests >= contract.minimum_test_count
        && summary.tests == summary.passed
        && summary.failed + summary.cancelled + summary.skipped + summary.todo == 0
        && shells.len() == contract.shell_minimum_actions.len()
        && assertions.iter().all(|assertion| assertion.passed);
    Ok(ValidationReport {
        schema_version: REPORT_SCHEMA.to_string(),
        generated_at_unix_ms: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|error| format!("system clock before epoch: {error}"))?
            .as_millis(),
        contract_path: CONTRACT_PATH.to_string(),
        status: if passed { "pass" } else { "fail" }.to_string(),
        platform: Platform {
            os: std::env::consts::OS.to_string(),
            arch: std::env::consts::ARCH.to_string(),
        },
        command,
        exit_code: output.status.code(),
        elapsed_ms: started.elapsed().as_millis(),
        summary,
        shells,
        assertions,
        output_excerpt: rendered.chars().take(12_000).collect(),
    })
}

fn parse_summary(output: &str) -> TestSummary {
    TestSummary {
        tests: summary_value(output, "tests"),
        passed: summary_value(output, "pass"),
        failed: summary_value(output, "fail"),
        cancelled: summary_value(output, "cancelled"),
        skipped: summary_value(output, "skipped"),
        todo: summary_value(output, "todo"),
        failed_actions: 0,
        missing_actions: 0,
    }
}

fn summary_value(output: &str, key: &str) -> usize {
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

fn parse_shell_result(output: &str, shell: &str) -> Option<ShellResult> {
    let prefix = format!("# {shell}: ");
    output.lines().find_map(|line| {
        let content = line.trim().strip_prefix(&prefix)?;
        let mut parts = content.split_whitespace();
        let actions = parts.next()?.parse().ok()?;
        if parts.next()? != "actions," {
            return None;
        }
        let blocked = parts.next()?.parse().ok()?;
        Some(ShellResult {
            id: shell.to_string(),
            actions,
            blocked,
        })
    })
}

fn assertion_passed(output: &str, label: &str) -> bool {
    output.lines().any(|line| {
        let line = line.trim();
        line.starts_with("ok ") && line.contains(label)
    })
}

fn validate_report(contract: &ValidationContract, report: &ValidationReport) -> RunnerResult<()> {
    if report.schema_version != REPORT_SCHEMA
        || report.contract_path != CONTRACT_PATH
        || report.status != "pass"
        || report.generated_at_unix_ms == 0
        || report.exit_code != Some(0)
    {
        return Err("desktop UI validation report header is invalid".to_string());
    }
    let expected_command = std::iter::once("node".to_string())
        .chain(
            ["--test", "--test-reporter=tap"]
                .into_iter()
                .map(str::to_string),
        )
        .chain(contract.test_files.iter().cloned())
        .collect::<Vec<_>>();
    if report.command != expected_command
        || report.summary.tests < contract.minimum_test_count
        || report.summary.tests != report.summary.passed
        || report.summary.failed
            + report.summary.cancelled
            + report.summary.skipped
            + report.summary.todo
            + report.summary.failed_actions
            + report.summary.missing_actions
            != 0
    {
        return Err("desktop UI validation report summary is not qualified".to_string());
    }
    for requirement in &contract.shell_minimum_actions {
        let shell = report
            .shells
            .iter()
            .find(|entry| entry.id == requirement.id)
            .ok_or_else(|| format!("report misses {} shell evidence", requirement.id))?;
        if shell.actions < requirement.minimum_actions || shell.blocked > shell.actions {
            return Err(format!(
                "{} shell action coverage is too weak",
                requirement.id
            ));
        }
    }
    if report.shells.len() != contract.shell_minimum_actions.len() {
        return Err("desktop UI report contains an unexpected shell set".to_string());
    }
    for requirement in &contract.required_assertions {
        let assertion = report
            .assertions
            .iter()
            .find(|entry| entry.id == requirement.id && entry.label == requirement.label)
            .ok_or_else(|| format!("report misses assertion {}", requirement.id))?;
        if !assertion.passed || !report.output_excerpt.contains(&requirement.label) {
            return Err(format!("assertion {} did not pass", requirement.id));
        }
    }
    if report.assertions.len() != contract.required_assertions.len() {
        return Err("desktop UI report contains an unexpected assertion set".to_string());
    }
    Ok(())
}

fn repo_path(root: &Path, relative: &str) -> RunnerResult<PathBuf> {
    let path = Path::new(relative);
    if relative.is_empty()
        || path.is_absolute()
        || path
            .components()
            .any(|component| component.as_os_str() == "..")
    {
        return Err(format!("path escapes repository: {relative}"));
    }
    Ok(root.join(path))
}

fn read_json<T: serde::de::DeserializeOwned>(root: &Path, relative: &str) -> RunnerResult<T> {
    let text = fs::read_to_string(repo_path(root, relative)?)
        .map_err(|error| format!("failed to read {relative}: {error}"))?;
    serde_json::from_str(&text).map_err(|error| format!("invalid JSON {relative}: {error}"))
}

fn write_json(root: &Path, relative: &str, report: &ValidationReport) -> RunnerResult<()> {
    let path = repo_path(root, relative)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
    }
    let rendered = serde_json::to_string_pretty(report)
        .map_err(|error| format!("failed to encode desktop UI report: {error}"))?;
    fs::write(&path, format!("{rendered}\n"))
        .map_err(|error| format!("failed to write {}: {error}", path.display()))
}

fn run_self_test() -> RunnerResult<()> {
    let tap = "ok 1 - desktop shell layout keeps operational workspaces dominant\n\
# hub: 43 actions, 0 explicitly blocked by preconditions\n\
# tests 20\n# pass 20\n# fail 0\n# cancelled 0\n# skipped 0\n# todo 0\n";
    let summary = parse_summary(tap);
    if summary.tests != 20 || summary.passed != 20 || summary.failed != 0 {
        return Err("desktop UI TAP summary parser self-test failed".to_string());
    }
    let shell = parse_shell_result(tap, "hub")
        .ok_or_else(|| "desktop UI shell parser self-test failed".to_string())?;
    if shell.actions != 43
        || shell.blocked != 0
        || !assertion_passed(
            tap,
            "desktop shell layout keeps operational workspaces dominant",
        )
    {
        return Err("desktop UI qualification parser self-test failed".to_string());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn parses_qualification_tap() {
        super::run_self_test().unwrap();
    }
}
