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

const CONTRACT_PATH: &str = "config/architecture/desktop-ui-validation.json";
const CONTRACT_SCHEMA: &str = "kyuubiki.desktop-ui-validation-contract/v1";
const REPORT_SCHEMA: &str = "kyuubiki.desktop-ui-validation-report/v1";
const DEFAULT_OUT: &str = "tmp/desktop-ui-validation-report.json";

#[derive(Debug, Deserialize)]
struct ValidationContract {
    schema_version: String,
    report_schema: String,
    test_concurrency: usize,
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

pub(crate) fn run_check_desktop_ui_validation(
    root: &Path,
    args: Vec<OsString>,
) -> RunnerResult<u8> {
    let options = parse_options(args, "desktop UI validation")?;
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

fn validate_contract(root: &Path, contract: &ValidationContract) -> RunnerResult<()> {
    if contract.schema_version != CONTRACT_SCHEMA || contract.report_schema != REPORT_SCHEMA {
        return Err("desktop UI validation schema contract is invalid".to_string());
    }
    if !(1..=4).contains(&contract.test_concurrency) {
        return Err("desktop UI test concurrency must be between 1 and 4".to_string());
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

fn test_command(contract: &ValidationContract) -> Vec<String> {
    [
        "node".to_string(),
        "--test".to_string(),
        "--test-reporter=tap".to_string(),
        format!("--test-concurrency={}", contract.test_concurrency),
    ]
    .into_iter()
    .chain(contract.test_files.iter().cloned())
    .collect()
}

fn execute_suite(root: &Path, contract: &ValidationContract) -> RunnerResult<ValidationReport> {
    let command = test_command(contract);
    let started = Instant::now();
    let output = Command::new(&command[0])
        .args(&command[1..])
        .current_dir(root)
        .env("NO_COLOR", "1")
        .output()
        .map_err(|error| format!("failed to execute desktop UI validation: {error}"))?;
    let rendered = portable_output(root, &output);
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
        generated_at_unix_ms: generated_at_unix_ms()?,
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
        output_excerpt: retain_output_excerpt(&rendered, &contract.required_assertions),
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
    output
        .lines()
        .any(|line| passed_assertion_line(line, label))
}

fn passed_assertion_line(line: &str, label: &str) -> bool {
    line.trim()
        .strip_prefix("ok ")
        .and_then(|line| line.split_once(" - "))
        .is_some_and(|(number, actual)| number.parse::<usize>().is_ok() && actual == label)
}

fn retain_output_excerpt(output: &str, requirements: &[AssertionRequirement]) -> String {
    // Keep actual TAP proof even when a large suite places it beyond the log prefix.
    let proof = requirements
        .iter()
        .filter_map(|required| {
            output
                .lines()
                .find(|line| passed_assertion_line(line, &required.label))
                .map(str::trim)
        })
        .collect::<Vec<_>>()
        .join("\n");
    let lines = output.lines().collect::<Vec<_>>();
    let failures = lines
        .iter()
        .enumerate()
        .filter(|(_, line)| line.trim().starts_with("not ok "))
        .flat_map(|(index, _)| lines[index..].iter().take(20).copied())
        .collect::<Vec<_>>()
        .join("\n");
    let failures = failures.chars().take(3_000).collect::<String>();
    let budget =
        12_000_usize.saturating_sub(proof.chars().count() + failures.chars().count() + 150);
    let prefix = output.chars().take(budget / 2).collect::<String>();
    let suffix = output.chars().rev().take(budget / 2).collect::<Vec<_>>();
    format!(
        "Required assertion TAP lines (verbatim):\n{proof}\n\nFailure excerpts:\n{failures}\n\nOutput prefix:\n{prefix}\n\nOutput suffix:\n{}",
        suffix.into_iter().rev().collect::<String>()
    )
}

fn validate_report(contract: &ValidationContract, report: &ValidationReport) -> RunnerResult<()> {
    if report.schema_version != REPORT_SCHEMA
        || report.contract_path != CONTRACT_PATH
        || report.generated_at_unix_ms == 0
    {
        return Err("desktop UI validation report header is invalid".to_string());
    }
    if report.status != "pass" || report.exit_code != Some(0) {
        return Err(format!(
            "desktop UI suite did not pass: {}/{} passed, {} failed, {} cancelled, exit {:?}",
            report.summary.passed,
            report.summary.tests,
            report.summary.failed,
            report.summary.cancelled,
            report.exit_code
        ));
    }
    let expected_command = test_command(contract);
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
        if !assertion.passed || !assertion_passed(&report.output_excerpt, &requirement.label) {
            return Err(format!("assertion {} did not pass", requirement.id));
        }
    }
    if report.assertions.len() != contract.required_assertions.len() {
        return Err("desktop UI report contains an unexpected assertion set".to_string());
    }
    Ok(())
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
    use super::*;

    #[test]
    fn parses_qualification_tap() {
        super::run_self_test().unwrap();
    }

    #[test]
    fn required_proof_survives_large_log_compaction() {
        let label = "late recovery assertion";
        let output = format!(
            "{}\n    ok 218 - {label}\n{}",
            "noise\n".repeat(8_000),
            "tail\n".repeat(8_000)
        );
        let requirements = vec![AssertionRequirement {
            id: "recovery".into(),
            label: label.into(),
        }];
        let excerpt = retain_output_excerpt(&output, &requirements);
        assert!(assertion_passed(&excerpt, label));
        assert!(excerpt.chars().count() <= 12_000);
    }

    #[test]
    fn assertion_names_must_match_complete_tap_pass_lines() {
        assert!(assertion_passed("  ok 8 - recovery", "recovery"));
        for output in [
            "not ok 8 - recovery",
            "# Subtest: recovery",
            "ok 8 - recovery helper",
            "ok x - recovery",
        ] {
            assert!(!assertion_passed(output, "recovery"), "{output}");
        }
    }

    #[test]
    fn log_compaction_cannot_manufacture_missing_or_failed_proof() {
        let requirements = vec![AssertionRequirement {
            id: "recovery".into(),
            label: "recovery".into(),
        }];
        let excerpt =
            retain_output_excerpt("# Subtest: recovery\nnot ok 8 - recovery\n", &requirements);
        assert!(!assertion_passed(&excerpt, "recovery"));
        assert!(excerpt.contains("not ok 8 - recovery"));
    }

    #[test]
    fn cleanup_failures_are_retained_from_the_middle_of_a_large_suite() {
        let output = format!(
            "{}\nnot ok 219 - cleanup\n  error: 'hook timed out'\n{}",
            "prefix\n".repeat(8_000),
            "suffix\n".repeat(8_000)
        );
        let excerpt = retain_output_excerpt(&output, &[]);
        assert!(excerpt.contains("not ok 219 - cleanup"));
        assert!(excerpt.contains("hook timed out"));
        assert!(excerpt.chars().count() <= 12_000);
    }

    #[test]
    fn test_concurrency_is_bounded_and_bound_to_the_report_command() {
        let mut contract = ValidationContract {
            schema_version: CONTRACT_SCHEMA.into(),
            report_schema: REPORT_SCHEMA.into(),
            test_concurrency: 2,
            test_files: vec!["fixture.test.mjs".into()],
            minimum_test_count: 40,
            shell_minimum_actions: vec![],
            required_assertions: vec![],
        };
        assert_eq!(test_command(&contract)[3], "--test-concurrency=2");
        for concurrency in [0, 5] {
            contract.test_concurrency = concurrency;
            assert!(
                validate_contract(Path::new("."), &contract)
                    .unwrap_err()
                    .contains("concurrency")
            );
        }
    }
}
