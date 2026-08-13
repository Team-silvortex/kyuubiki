use crate::qualification_support::{
    generated_at_unix_ms, parse_options, portable_output, read_json, repo_path, write_json_compact,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::ffi::OsString;
use std::fs;
use std::path::Path;
use std::process::Command;
use std::time::Instant;

type RunnerResult<T> = Result<T, String>;

const CONTRACT_PATH: &str = "config/architecture/workbench-validation-qualification.json";
const CONTRACT_SCHEMA: &str = "kyuubiki.workbench-validation-qualification-contract/v1";
const REPORT_SCHEMA: &str = "kyuubiki.workbench-validation-qualification-report/v1";
const REPORT_SCHEMA_PATH: &str = "schemas/workbench-validation-qualification-report.schema.json";
const DEFAULT_OUT: &str = "tmp/workbench-validation-qualification-report.json";

#[derive(Clone, Deserialize)]
struct QualificationContract {
    schema_version: String,
    report_schema: String,
    rounds: usize,
    required_boundaries: Vec<String>,
    suites: Vec<SuiteSpec>,
}

#[derive(Clone, Deserialize)]
struct SuiteSpec {
    id: String,
    program: String,
    cwd: String,
    args: Vec<String>,
    source_files: Vec<String>,
    required_source_text: Vec<String>,
    minimum_test_count: usize,
    required_assertions: Vec<AssertionSpec>,
}

#[derive(Clone, Deserialize, Serialize)]
struct AssertionSpec {
    id: String,
    label: String,
    outcome: String,
    boundary: String,
}

#[derive(Deserialize, Serialize)]
struct QualificationReport {
    schema_version: String,
    generated_at_unix_ms: u128,
    contract_path: String,
    status: String,
    platform: Platform,
    rounds: usize,
    suites: Vec<SuiteReport>,
    boundaries: Vec<BoundaryReport>,
    summary: QualificationSummary,
}

#[derive(Deserialize, Serialize)]
struct Platform {
    os: String,
    arch: String,
}

#[derive(Deserialize, Serialize)]
struct SuiteReport {
    id: String,
    program: String,
    cwd: String,
    args: Vec<String>,
    rounds: Vec<RoundReport>,
    repeatable: bool,
    stable_semantics: bool,
}

#[derive(Deserialize, Serialize)]
struct RoundReport {
    round: usize,
    status: String,
    exit_code: Option<i32>,
    elapsed_ms: u128,
    semantic_sha256: String,
    summary: TestSummary,
    assertions: Vec<AssertionReport>,
    normalized_output: String,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
struct TestSummary {
    tests: usize,
    passed: usize,
    failed: usize,
    cancelled: usize,
    skipped: usize,
    todo: usize,
}

#[derive(Deserialize, Serialize)]
struct AssertionReport {
    id: String,
    label: String,
    outcome: String,
    boundary: String,
    passed: bool,
}

#[derive(Deserialize, Serialize)]
struct BoundaryReport {
    id: String,
    status: String,
    suite_id: String,
    assertion_id: String,
}

#[derive(Deserialize, Serialize)]
struct QualificationSummary {
    suite_count: usize,
    round_count: usize,
    passed_round_count: usize,
    tests_per_round: usize,
    executed_test_count: usize,
    assertion_count: usize,
    acceptance_assertion_count: usize,
    rejection_boundary_count: usize,
    stable_suite_count: usize,
    failed_suite_ids: Vec<String>,
}

#[derive(Deserialize, Serialize)]
struct SemanticOutput {
    summary: TestSummary,
    test_labels: Vec<String>,
}

pub(crate) fn run_check_workbench_validation_qualification(
    root: &Path,
    args: Vec<OsString>,
) -> RunnerResult<u8> {
    let options = parse_options(args, "Workbench validation qualification")?;
    let contract: QualificationContract = read_json(root, CONTRACT_PATH)?;
    validate_contract(root, &contract)?;

    if options.self_test {
        run_self_test(root, &contract)?;
        println!("Workbench validation qualification self-test passed");
        return Ok(0);
    }
    if let Some(path) = options.verify_report {
        let report: QualificationReport = read_json(root, &path)?;
        validate_report(&contract, &report)?;
        println!("Workbench validation qualification report passed: {path}");
        return Ok(0);
    }

    let report = execute_qualification(root, &contract)?;
    let out = options.out.as_deref().unwrap_or(DEFAULT_OUT);
    write_json_compact(root, out, &report)?;
    if let Err(error) = validate_report(&contract, &report) {
        eprintln!("Workbench validation qualification failed: {error}");
        eprintln!("failure report written: {out}");
        return Ok(1);
    }
    println!(
        "workbench validation qualified: {} tests per round, {} total executions, {} rejection boundaries",
        report.summary.tests_per_round,
        report.summary.executed_test_count,
        report.summary.rejection_boundary_count
    );
    println!("Workbench validation qualification report written: {out}");
    Ok(0)
}

fn validate_contract(root: &Path, contract: &QualificationContract) -> RunnerResult<()> {
    if contract.schema_version != CONTRACT_SCHEMA || contract.report_schema != REPORT_SCHEMA {
        return Err("Workbench validation qualification schemas are invalid".to_string());
    }
    let report_schema: serde_json::Value = read_json(root, REPORT_SCHEMA_PATH)?;
    if report_schema
        .pointer("/properties/schema_version/const")
        .and_then(serde_json::Value::as_str)
        != Some(REPORT_SCHEMA)
    {
        return Err("Workbench validation report schema const drifted".to_string());
    }
    if !(2..=3).contains(&contract.rounds) || contract.suites.len() < 2 {
        return Err(
            "Workbench qualification requires at least two suites over 2 or 3 rounds".into(),
        );
    }
    require_unique_nonempty(&contract.required_boundaries, "required boundary")?;
    if contract.required_boundaries.len() < 4 {
        return Err("Workbench qualification requires at least four rejection boundaries".into());
    }

    let required_boundaries = contract
        .required_boundaries
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    let mut suite_ids = BTreeSet::new();
    let mut assertion_ids = BTreeSet::new();
    let mut rejection_boundaries = BTreeSet::new();
    let mut acceptance_count = 0usize;
    let mut minimum_test_count = 0usize;
    for suite in &contract.suites {
        if suite.id.is_empty() || !suite_ids.insert(suite.id.as_str()) {
            return Err(format!("invalid or duplicate Workbench suite {}", suite.id));
        }
        validate_suite_command(suite)?;
        if !repo_path(root, &suite.cwd)?.is_dir() {
            return Err(format!("Workbench suite {} cwd does not exist", suite.id));
        }
        let source = read_suite_sources(root, suite)?;
        if suite.minimum_test_count == 0 || suite.required_assertions.is_empty() {
            return Err(format!("Workbench suite {} has weak thresholds", suite.id));
        }
        for text in &suite.required_source_text {
            if text.is_empty() || !source.contains(text) {
                return Err(format!(
                    "Workbench suite {} misses source guard {text}",
                    suite.id
                ));
            }
        }
        for assertion in &suite.required_assertions {
            if assertion.id.is_empty()
                || assertion.label.is_empty()
                || assertion.boundary.is_empty()
                || !assertion_ids.insert(assertion.id.as_str())
                || !source.contains(&assertion.label)
            {
                return Err(format!(
                    "invalid or stale Workbench assertion {}",
                    assertion.id
                ));
            }
            match assertion.outcome.as_str() {
                "acceptance" => acceptance_count += 1,
                "rejection" => {
                    if !rejection_boundaries.insert(assertion.boundary.as_str()) {
                        return Err(format!(
                            "duplicate Workbench rejection boundary {}",
                            assertion.boundary
                        ));
                    }
                }
                _ => return Err(format!("invalid assertion outcome for {}", assertion.id)),
            }
        }
        minimum_test_count += suite.minimum_test_count;
    }
    if minimum_test_count < 39 || acceptance_count < 17 {
        return Err("Workbench qualification scope is below the release threshold".into());
    }
    if rejection_boundaries != required_boundaries {
        return Err("Workbench rejection boundaries do not match the contract".into());
    }
    Ok(())
}

fn validate_suite_command(suite: &SuiteSpec) -> RunnerResult<()> {
    if suite.args.is_empty() || suite.args.iter().any(String::is_empty) {
        return Err(format!("Workbench suite {} has no command", suite.id));
    }
    match suite.program.as_str() {
        "self"
            if suite
                .args
                .first()
                .is_some_and(|arg| arg == "frontend-unit-test") =>
        {
            Ok(())
        }
        "node"
            if suite
                .args
                .starts_with(&["--test".to_string(), "--test-reporter=tap".to_string()]) =>
        {
            Ok(())
        }
        "self" => Err(format!(
            "Workbench suite {} must use the native frontend-unit-test command",
            suite.id
        )),
        "node" => Err(format!(
            "Workbench suite {} must use the deterministic TAP runner",
            suite.id
        )),
        _ => Err(format!(
            "Workbench suite {} uses an unsupported program",
            suite.id
        )),
    }
}

fn read_suite_sources(root: &Path, suite: &SuiteSpec) -> RunnerResult<String> {
    if suite.source_files.is_empty() {
        return Err(format!("Workbench suite {} has no source files", suite.id));
    }
    let mut unique = BTreeSet::new();
    let mut source = String::new();
    for relative in &suite.source_files {
        if !unique.insert(relative.as_str())
            || !(relative.starts_with("apps/frontend/test/")
                || relative.starts_with("tests/integration/"))
        {
            return Err(format!("invalid Workbench test source {relative}"));
        }
        source.push_str(
            &fs::read_to_string(repo_path(root, relative)?)
                .map_err(|error| format!("failed to read {relative}: {error}"))?,
        );
        source.push('\n');
    }
    Ok(source)
}

fn execute_qualification(
    root: &Path,
    contract: &QualificationContract,
) -> RunnerResult<QualificationReport> {
    let mut suites = Vec::new();
    for suite in &contract.suites {
        let mut rounds = Vec::new();
        for round in 1..=contract.rounds {
            rounds.push(run_suite_round(root, suite, round)?);
        }
        let repeatable = rounds.iter().all(|round| round.status == "pass");
        let stable_semantics = rounds
            .iter()
            .map(|round| round.semantic_sha256.as_str())
            .collect::<BTreeSet<_>>()
            .len()
            == 1;
        suites.push(SuiteReport {
            id: suite.id.clone(),
            program: suite.program.clone(),
            cwd: suite.cwd.clone(),
            args: suite.args.clone(),
            rounds,
            repeatable,
            stable_semantics,
        });
    }
    build_report(contract, suites, generated_at_unix_ms()?)
}

fn run_suite_round(root: &Path, suite: &SuiteSpec, round: usize) -> RunnerResult<RoundReport> {
    let started = Instant::now();
    let output = match suite.program.as_str() {
        "self" => Command::new(
            std::env::current_exe()
                .map_err(|error| format!("failed to resolve script runner: {error}"))?,
        )
        .args(&suite.args)
        .current_dir(repo_path(root, &suite.cwd)?)
        .env("NO_COLOR", "1")
        .output(),
        "node" => Command::new("node")
            .args(&suite.args)
            .current_dir(repo_path(root, &suite.cwd)?)
            .env("NO_COLOR", "1")
            .output(),
        _ => unreachable!("program validated before execution"),
    }
    .map_err(|error| format!("failed to execute Workbench suite {}: {error}", suite.id))?;
    let rendered = portable_output(root, &output);
    let semantic = parse_semantic_output(&rendered)?;
    let normalized_output = serde_json::to_string(&semantic)
        .map_err(|error| format!("failed to normalize Workbench output: {error}"))?;
    let assertions = suite
        .required_assertions
        .iter()
        .map(|required| AssertionReport {
            id: required.id.clone(),
            label: required.label.clone(),
            outcome: required.outcome.clone(),
            boundary: required.boundary.clone(),
            passed: semantic.test_labels.binary_search(&required.label).is_ok(),
        })
        .collect::<Vec<_>>();
    let passed = output.status.success()
        && semantic.summary.tests >= suite.minimum_test_count
        && semantic.summary.tests == semantic.summary.passed
        && semantic.summary.failed
            + semantic.summary.cancelled
            + semantic.summary.skipped
            + semantic.summary.todo
            == 0
        && assertions.iter().all(|assertion| assertion.passed);
    Ok(RoundReport {
        round,
        status: if passed { "pass" } else { "fail" }.to_string(),
        exit_code: output.status.code(),
        elapsed_ms: started.elapsed().as_millis(),
        semantic_sha256: digest(&normalized_output),
        summary: semantic.summary,
        assertions,
        normalized_output,
    })
}

fn parse_semantic_output(output: &str) -> RunnerResult<SemanticOutput> {
    let summary = TestSummary {
        tests: summary_value(output, "tests"),
        passed: summary_value(output, "pass"),
        failed: summary_value(output, "fail"),
        cancelled: summary_value(output, "cancelled"),
        skipped: summary_value(output, "skipped"),
        todo: summary_value(output, "todo"),
    };
    let mut labels = BTreeSet::new();
    for line in output.lines() {
        let line = line.trim();
        if let Some(label) = line.strip_prefix("✔ ") {
            labels.insert(strip_duration(label).to_string());
        } else if line.starts_with("ok ") {
            if let Some((_, label)) = line.split_once(" - ") {
                labels.insert(label.trim().to_string());
            }
        }
    }
    if summary.tests == 0 || labels.is_empty() {
        return Err("Workbench test output has no parseable test summary or labels".to_string());
    }
    Ok(SemanticOutput {
        summary,
        test_labels: labels.into_iter().collect(),
    })
}

fn summary_value(output: &str, key: &str) -> usize {
    output
        .lines()
        .rev()
        .find_map(|line| {
            let line = line
                .trim()
                .strip_prefix('ℹ')
                .or_else(|| line.trim().strip_prefix('#'))?
                .trim();
            let mut fields = line.split_whitespace();
            (fields.next() == Some(key))
                .then(|| fields.next()?.parse::<usize>().ok())
                .flatten()
        })
        .unwrap_or(0)
}

fn strip_duration(label: &str) -> &str {
    let Some((name, suffix)) = label.rsplit_once(" (") else {
        return label.trim();
    };
    if suffix.ends_with("ms)") || suffix.ends_with("s)") {
        name.trim()
    } else {
        label.trim()
    }
}

fn build_report(
    contract: &QualificationContract,
    suites: Vec<SuiteReport>,
    generated_at_unix_ms: u128,
) -> RunnerResult<QualificationReport> {
    let boundaries = contract
        .required_boundaries
        .iter()
        .map(|boundary| {
            let (suite, assertion) = contract
                .suites
                .iter()
                .find_map(|suite| {
                    suite
                        .required_assertions
                        .iter()
                        .find(|assertion| {
                            assertion.outcome == "rejection" && assertion.boundary == *boundary
                        })
                        .map(|assertion| (suite, assertion))
                })
                .ok_or_else(|| format!("missing Workbench boundary mapping {boundary}"))?;
            let passed = suites
                .iter()
                .find(|report| report.id == suite.id)
                .is_some_and(|report| {
                    report.rounds.iter().all(|round| {
                        round
                            .assertions
                            .iter()
                            .any(|receipt| receipt.id == assertion.id && receipt.passed)
                    })
                });
            Ok(BoundaryReport {
                id: boundary.clone(),
                status: if passed { "pass" } else { "fail" }.to_string(),
                suite_id: suite.id.clone(),
                assertion_id: assertion.id.clone(),
            })
        })
        .collect::<RunnerResult<Vec<_>>>()?;
    let failed_suite_ids = suites
        .iter()
        .filter(|suite| !suite.repeatable || !suite.stable_semantics)
        .map(|suite| suite.id.clone())
        .collect::<Vec<_>>();
    let tests_per_round = suites
        .iter()
        .filter_map(|suite| suite.rounds.first())
        .map(|round| round.summary.tests)
        .sum();
    let executed_test_count = suites
        .iter()
        .flat_map(|suite| &suite.rounds)
        .map(|round| round.summary.tests)
        .sum();
    let passed_round_count = suites
        .iter()
        .flat_map(|suite| &suite.rounds)
        .filter(|round| round.status == "pass")
        .count();
    let assertion_count = contract
        .suites
        .iter()
        .map(|suite| suite.required_assertions.len())
        .sum();
    let acceptance_assertion_count = contract
        .suites
        .iter()
        .flat_map(|suite| &suite.required_assertions)
        .filter(|assertion| assertion.outcome == "acceptance")
        .count();
    let stable_suite_count = suites.iter().filter(|suite| suite.stable_semantics).count();
    let status = if failed_suite_ids.is_empty()
        && boundaries.iter().all(|boundary| boundary.status == "pass")
    {
        "pass"
    } else {
        "fail"
    };
    Ok(QualificationReport {
        schema_version: REPORT_SCHEMA.to_string(),
        generated_at_unix_ms,
        contract_path: CONTRACT_PATH.to_string(),
        status: status.to_string(),
        platform: Platform {
            os: std::env::consts::OS.to_string(),
            arch: std::env::consts::ARCH.to_string(),
        },
        rounds: contract.rounds,
        summary: QualificationSummary {
            suite_count: suites.len(),
            round_count: suites.len() * contract.rounds,
            passed_round_count,
            tests_per_round,
            executed_test_count,
            assertion_count,
            acceptance_assertion_count,
            rejection_boundary_count: boundaries.len(),
            stable_suite_count,
            failed_suite_ids,
        },
        suites,
        boundaries,
    })
}

fn validate_report(
    contract: &QualificationContract,
    report: &QualificationReport,
) -> RunnerResult<()> {
    if report.schema_version != REPORT_SCHEMA
        || report.contract_path != CONTRACT_PATH
        || report.generated_at_unix_ms == 0
        || report.status != "pass"
        || report.rounds != contract.rounds
        || report.platform.os.is_empty()
        || report.platform.arch.is_empty()
        || report.suites.len() != contract.suites.len()
    {
        return Err("Workbench qualification report header or scope is invalid".to_string());
    }
    for spec in &contract.suites {
        let suite = report
            .suites
            .iter()
            .find(|suite| suite.id == spec.id)
            .ok_or_else(|| format!("Workbench report misses suite {}", spec.id))?;
        if suite.program != spec.program
            || suite.cwd != spec.cwd
            || suite.args != spec.args
            || !suite.repeatable
            || !suite.stable_semantics
            || suite.rounds.len() != contract.rounds
        {
            return Err(format!("Workbench report suite {} drifted", spec.id));
        }
        let hashes = suite
            .rounds
            .iter()
            .map(|round| round.semantic_sha256.as_str())
            .collect::<BTreeSet<_>>();
        if hashes.len() != 1 {
            return Err(format!(
                "Workbench suite {} semantics are unstable",
                spec.id
            ));
        }
        for (index, round) in suite.rounds.iter().enumerate() {
            validate_round(spec, round, index + 1)?;
        }
    }
    let expected_boundaries = contract
        .required_boundaries
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    let actual_boundaries = report
        .boundaries
        .iter()
        .map(|boundary| boundary.id.as_str())
        .collect::<BTreeSet<_>>();
    if actual_boundaries != expected_boundaries
        || report.boundaries.iter().any(|boundary| {
            boundary.status != "pass"
                || !contract.suites.iter().any(|suite| {
                    suite.id == boundary.suite_id
                        && suite.required_assertions.iter().any(|assertion| {
                            assertion.id == boundary.assertion_id
                                && assertion.outcome == "rejection"
                                && assertion.boundary == boundary.id
                        })
                })
        })
    {
        return Err("Workbench qualification boundary receipts drifted".to_string());
    }
    validate_summary(contract, report)
}

fn validate_round(spec: &SuiteSpec, round: &RoundReport, index: usize) -> RunnerResult<()> {
    let semantic: SemanticOutput = serde_json::from_str(&round.normalized_output)
        .map_err(|error| format!("invalid normalized Workbench output: {error}"))?;
    if round.round != index
        || round.status != "pass"
        || round.exit_code != Some(0)
        || round.semantic_sha256 != digest(&round.normalized_output)
        || round.summary != semantic.summary
        || round.summary.tests < spec.minimum_test_count
        || round.summary.tests != round.summary.passed
        || round.summary.failed
            + round.summary.cancelled
            + round.summary.skipped
            + round.summary.todo
            != 0
        || round.assertions.len() != spec.required_assertions.len()
    {
        return Err(format!("Workbench suite {} round {index} failed", spec.id));
    }
    for required in &spec.required_assertions {
        if semantic.test_labels.binary_search(&required.label).is_err()
            || !round.assertions.iter().any(|assertion| {
                assertion.id == required.id
                    && assertion.label == required.label
                    && assertion.outcome == required.outcome
                    && assertion.boundary == required.boundary
                    && assertion.passed
            })
        {
            return Err(format!(
                "Workbench suite {} round {index} misses assertion {}",
                spec.id, required.id
            ));
        }
    }
    Ok(())
}

fn validate_summary(
    contract: &QualificationContract,
    report: &QualificationReport,
) -> RunnerResult<()> {
    let expected_round_count = contract.suites.len() * contract.rounds;
    let tests_per_round: usize = report
        .suites
        .iter()
        .filter_map(|suite| suite.rounds.first())
        .map(|round| round.summary.tests)
        .sum();
    let executed_test_count: usize = report
        .suites
        .iter()
        .flat_map(|suite| &suite.rounds)
        .map(|round| round.summary.tests)
        .sum();
    let assertion_count: usize = contract
        .suites
        .iter()
        .map(|suite| suite.required_assertions.len())
        .sum();
    let acceptance_count = contract
        .suites
        .iter()
        .flat_map(|suite| &suite.required_assertions)
        .filter(|assertion| assertion.outcome == "acceptance")
        .count();
    let summary = &report.summary;
    if summary.suite_count != contract.suites.len()
        || summary.round_count != expected_round_count
        || summary.passed_round_count != expected_round_count
        || summary.tests_per_round != tests_per_round
        || summary.tests_per_round < 39
        || summary.executed_test_count != executed_test_count
        || summary.executed_test_count < 78
        || summary.assertion_count != assertion_count
        || summary.assertion_count < 21
        || summary.acceptance_assertion_count != acceptance_count
        || summary.acceptance_assertion_count < 17
        || summary.rejection_boundary_count != contract.required_boundaries.len()
        || summary.stable_suite_count != contract.suites.len()
        || !summary.failed_suite_ids.is_empty()
    {
        return Err("Workbench qualification report summary failed".to_string());
    }
    Ok(())
}

fn run_self_test(root: &Path, contract: &QualificationContract) -> RunnerResult<()> {
    let mut weak = contract.clone();
    weak.rounds = 1;
    if validate_contract(root, &weak).is_ok() {
        return Err("self-test accepted one Workbench qualification round".to_string());
    }
    let mut missing_boundary = contract.clone();
    missing_boundary.required_boundaries.pop();
    if validate_contract(root, &missing_boundary).is_ok() {
        return Err("self-test accepted an unlisted rejection boundary".to_string());
    }
    let pretty = "✔ beta case (2.1ms)\n✔ alpha case (1.0ms)\nℹ tests 2\nℹ pass 2\nℹ fail 0\nℹ cancelled 0\nℹ skipped 0\nℹ todo 0\n";
    let tap = "TAP version 13\nok 1 - alpha case\nok 2 - beta case\n1..2\n# tests 2\n# pass 2\n# fail 0\n# cancelled 0\n# skipped 0\n# todo 0\n";
    let pretty_semantic = parse_semantic_output(pretty)?;
    let tap_semantic = parse_semantic_output(tap)?;
    if pretty_semantic.summary != tap_semantic.summary
        || pretty_semantic.test_labels != tap_semantic.test_labels
    {
        return Err("self-test found output-format semantic drift".to_string());
    }
    let first = serde_json::to_string(&pretty_semantic)
        .map_err(|error| format!("failed to encode self-test output: {error}"))?;
    let second = serde_json::to_string(&parse_semantic_output(
        &pretty.replace("2.1ms", "99.4ms").replace("1.0ms", "0.4ms"),
    )?)
    .map_err(|error| format!("failed to encode self-test output: {error}"))?;
    if digest(&first) != digest(&second) {
        return Err("self-test retained timing noise in semantic hashes".to_string());
    }
    Ok(())
}

fn require_unique_nonempty(values: &[String], label: &str) -> RunnerResult<()> {
    let unique = values
        .iter()
        .filter(|value| !value.is_empty())
        .collect::<BTreeSet<_>>();
    if values.is_empty() || unique.len() != values.len() {
        return Err(format!("{label} values must be non-empty and unique"));
    }
    Ok(())
}

fn digest(value: &str) -> String {
    format!("{:x}", Sha256::digest(value.as_bytes()))
}

#[cfg(test)]
mod tests {
    use super::{parse_semantic_output, strip_duration};

    #[test]
    fn strips_node_test_timing_without_truncating_labels() {
        assert_eq!(strip_duration("case (1.25ms)"), "case");
        assert_eq!(strip_duration("case (detail)"), "case (detail)");
    }

    #[test]
    fn semantic_parser_orders_labels_and_reads_tap_summary() {
        let output = "ok 2 - zeta\nok 1 - alpha\n# tests 2\n# pass 2\n# fail 0\n";
        let semantic = parse_semantic_output(output).expect("parse TAP output");
        assert_eq!(semantic.test_labels, ["alpha", "zeta"]);
        assert_eq!(semantic.summary.tests, 2);
        assert_eq!(semantic.summary.passed, 2);
    }
}
