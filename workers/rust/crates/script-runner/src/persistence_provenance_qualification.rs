use crate::qualification_support::{
    generated_at_unix_ms, parse_options, portable_output, read_json, repo_path, write_json_compact,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::ffi::OsString;
use std::fs;
use std::path::Path;
use std::process::Command;
use std::time::Instant;

type RunnerResult<T> = Result<T, String>;

const CONTRACT_PATH: &str = "config/architecture/persistence-provenance-qualification.json";
const CONTRACT_SCHEMA: &str = "kyuubiki.persistence-provenance-qualification-contract/v1";
const REPORT_SCHEMA: &str = "kyuubiki.persistence-provenance-qualification-report/v1";
const REPORT_SCHEMA_PATH: &str = "schemas/persistence-provenance-qualification-report.schema.json";
const DEFAULT_OUT: &str = "tmp/persistence-provenance-qualification-report.json";
const EXCERPT_LIMIT: usize = 12_000;

#[derive(Clone, Deserialize)]
struct QualificationContract {
    schema_version: String,
    report_schema: String,
    rounds: usize,
    required_modules: Vec<String>,
    suites: Vec<SuiteSpec>,
}

#[derive(Clone, Deserialize, Serialize)]
struct SuiteSpec {
    id: String,
    modules: Vec<String>,
    program: String,
    cwd: String,
    args: Vec<String>,
    source_files: Vec<String>,
    required_source_text: Vec<String>,
    assertions: Vec<AssertionSpec>,
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
    summary: Summary,
}

#[derive(Deserialize, Serialize)]
struct Platform {
    os: String,
    arch: String,
}

#[derive(Deserialize, Serialize)]
struct SuiteReport {
    id: String,
    modules: Vec<String>,
    program: String,
    cwd: String,
    args: Vec<String>,
    status: String,
    repeatable: bool,
    rounds: Vec<RoundReport>,
}

#[derive(Deserialize, Serialize)]
struct RoundReport {
    round: usize,
    status: String,
    exit_code: i32,
    elapsed_ms: u128,
    output_sha256: String,
    assertions: Vec<AssertionReport>,
    output_excerpt: String,
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
struct Summary {
    module_count: usize,
    suite_count: usize,
    round_count: usize,
    passed_round_count: usize,
    assertion_count: usize,
    acceptance_assertion_count: usize,
    rejection_assertion_count: usize,
    failed_suite_ids: Vec<String>,
}

pub(crate) fn run_check_persistence_provenance_qualification(
    root: &Path,
    args: Vec<OsString>,
) -> RunnerResult<u8> {
    let options = parse_options(args, "persistence provenance qualification")?;
    let contract: QualificationContract = read_json(root, CONTRACT_PATH)?;
    validate_contract(root, &contract)?;

    if options.self_test {
        run_self_test(&contract)?;
        println!("persistence provenance qualification self-test passed");
        return Ok(0);
    }
    if let Some(path) = options.verify_report {
        let report: QualificationReport = read_json(root, &path)?;
        validate_report(&contract, &report)?;
        println!("persistence provenance qualification report passed: {path}");
        return Ok(0);
    }

    let report = execute_qualification(root, &contract)?;
    let out = options.out.as_deref().unwrap_or(DEFAULT_OUT);
    write_json_compact(root, out, &report)?;
    if let Err(error) = validate_report(&contract, &report) {
        eprintln!("persistence provenance qualification failed: {error}");
        eprintln!("failure report written: {out}");
        return Ok(1);
    }
    println!(
        "persistence provenance qualified: {} modules, {} suites, {} assertions",
        report.summary.module_count, report.summary.suite_count, report.summary.assertion_count
    );
    println!("persistence provenance qualification report written: {out}");
    Ok(0)
}

fn validate_contract(root: &Path, contract: &QualificationContract) -> RunnerResult<()> {
    if contract.schema_version != CONTRACT_SCHEMA || contract.report_schema != REPORT_SCHEMA {
        return Err("persistence provenance qualification schemas are invalid".into());
    }
    let schema: Value = read_json(root, REPORT_SCHEMA_PATH)?;
    if schema
        .pointer("/properties/schema_version/const")
        .and_then(Value::as_str)
        != Some(REPORT_SCHEMA)
    {
        return Err("persistence provenance report schema const drifted".into());
    }
    if !(2..=3).contains(&contract.rounds) {
        return Err("persistence provenance qualification requires 2 or 3 rounds".into());
    }
    let expected_modules = [
        "installer-shell",
        "orchestra-control-plane",
        "runtime-installer",
    ]
    .into_iter()
    .collect::<BTreeSet<_>>();
    let modules = contract
        .required_modules
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    if modules != expected_modules || modules.len() != contract.required_modules.len() {
        return Err("persistence provenance required module set drifted".into());
    }
    if contract.suites.len() < 4 {
        return Err("persistence provenance qualification requires at least 4 suites".into());
    }

    let mut suite_ids = BTreeSet::new();
    let mut assertion_ids = BTreeSet::new();
    let mut rejection_boundaries = BTreeSet::new();
    let mut covered_modules = BTreeSet::new();
    let mut acceptance_count = 0usize;
    let mut rejection_count = 0usize;
    for suite in &contract.suites {
        if suite.id.is_empty() || !suite_ids.insert(suite.id.as_str()) {
            return Err(format!(
                "invalid or duplicate qualification suite {}",
                suite.id
            ));
        }
        if !matches!(suite.program.as_str(), "cargo" | "mix" | "node")
            || suite.args.len() < 2
            || suite.args.iter().any(String::is_empty)
        {
            return Err(format!(
                "qualification suite {} command is invalid",
                suite.id
            ));
        }
        if !repo_path(root, &suite.cwd)?.is_dir() {
            return Err(format!(
                "qualification suite {} cwd does not exist",
                suite.id
            ));
        }
        let suite_modules = suite
            .modules
            .iter()
            .map(String::as_str)
            .collect::<BTreeSet<_>>();
        if suite_modules.is_empty()
            || suite_modules.len() != suite.modules.len()
            || !suite_modules.is_subset(&modules)
        {
            return Err(format!(
                "qualification suite {} module set is invalid",
                suite.id
            ));
        }
        covered_modules.extend(suite_modules);
        validate_source_contract(root, suite)?;
        if suite.assertions.len() < 2 {
            return Err(format!(
                "qualification suite {} has too few assertions",
                suite.id
            ));
        }
        for assertion in &suite.assertions {
            if assertion.id.is_empty()
                || assertion.label.is_empty()
                || assertion.boundary.is_empty()
                || !assertion_ids.insert(assertion.id.as_str())
            {
                return Err(format!("invalid or duplicate assertion {}", assertion.id));
            }
            match assertion.outcome.as_str() {
                "acceptance" => acceptance_count += 1,
                "rejection" => {
                    rejection_count += 1;
                    if !rejection_boundaries.insert(assertion.boundary.as_str()) {
                        return Err(format!(
                            "duplicate rejection boundary {}",
                            assertion.boundary
                        ));
                    }
                }
                _ => return Err(format!("assertion {} outcome is invalid", assertion.id)),
            }
        }
    }
    if covered_modules != modules {
        return Err("not every required module has persistence provenance coverage".into());
    }
    if acceptance_count < 7 || rejection_count < 6 {
        return Err(
            "qualification requires at least 7 acceptance and 6 rejection assertions".into(),
        );
    }
    Ok(())
}

fn validate_source_contract(root: &Path, suite: &SuiteSpec) -> RunnerResult<()> {
    let files = suite
        .source_files
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    if files.is_empty() || files.len() != suite.source_files.len() {
        return Err(format!(
            "qualification suite {} source set is invalid",
            suite.id
        ));
    }
    let mut source = String::new();
    for relative in &suite.source_files {
        let path = repo_path(root, relative)?;
        if !path.is_file() {
            return Err(format!("qualification source does not exist: {relative}"));
        }
        source.push_str(
            &fs::read_to_string(&path)
                .map_err(|error| format!("failed to read {relative}: {error}"))?,
        );
        source.push('\n');
    }
    for token in &suite.required_source_text {
        if token.is_empty() || !source.contains(token) {
            return Err(format!(
                "qualification suite {} misses source token {token:?}",
                suite.id
            ));
        }
    }
    Ok(())
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
        let repeatable = rounds.iter().all(|round| round.status == "passed");
        suites.push(SuiteReport {
            id: suite.id.clone(),
            modules: suite.modules.clone(),
            program: suite.program.clone(),
            cwd: suite.cwd.clone(),
            args: suite.args.clone(),
            status: if repeatable { "passed" } else { "failed" }.into(),
            repeatable,
            rounds,
        });
    }
    Ok(build_report(contract, suites, generated_at_unix_ms()?))
}

fn run_suite_round(root: &Path, suite: &SuiteSpec, round: usize) -> RunnerResult<RoundReport> {
    let started = Instant::now();
    let output = Command::new(&suite.program)
        .args(&suite.args)
        .current_dir(repo_path(root, &suite.cwd)?)
        .env("NO_COLOR", "1")
        .output()
        .map_err(|error| format!("failed to execute suite {}: {error}", suite.id))?;
    let rendered = portable_output(root, &output);
    let assertions = suite
        .assertions
        .iter()
        .map(|spec| AssertionReport {
            id: spec.id.clone(),
            label: spec.label.clone(),
            outcome: spec.outcome.clone(),
            boundary: spec.boundary.clone(),
            passed: rendered.contains(&spec.label),
        })
        .collect::<Vec<_>>();
    let exit_code = output.status.code().unwrap_or(-1);
    let passed = exit_code == 0 && assertions.iter().all(|assertion| assertion.passed);
    Ok(RoundReport {
        round,
        status: if passed { "passed" } else { "failed" }.into(),
        exit_code,
        elapsed_ms: started.elapsed().as_millis(),
        output_sha256: output_digest(&rendered),
        assertions,
        output_excerpt: evidence_excerpt(&rendered, suite),
    })
}

fn evidence_excerpt(output: &str, suite: &SuiteSpec) -> String {
    let mut selected = output
        .lines()
        .filter(|line| {
            suite
                .assertions
                .iter()
                .any(|item| line.contains(&item.label))
                || line.contains("test result:")
                || line.contains("tests, 0 failures")
        })
        .collect::<Vec<_>>()
        .join("\n");
    if selected.is_empty() {
        selected = "qualification command produced no matching evidence line".into();
    }
    let mut end = selected.len().min(EXCERPT_LIMIT);
    while !selected.is_char_boundary(end) {
        end -= 1;
    }
    selected.truncate(end);
    selected
}

fn output_digest(output: &str) -> String {
    format!("{:x}", Sha256::digest(output.as_bytes()))
}

fn build_report(
    contract: &QualificationContract,
    suites: Vec<SuiteReport>,
    generated_at_unix_ms: u128,
) -> QualificationReport {
    let failed_suite_ids = suites
        .iter()
        .filter(|suite| !suite.repeatable)
        .map(|suite| suite.id.clone())
        .collect::<Vec<_>>();
    let passed_round_count = suites
        .iter()
        .flat_map(|suite| &suite.rounds)
        .filter(|round| round.status == "passed")
        .count();
    let assertion_count = contract
        .suites
        .iter()
        .map(|suite| suite.assertions.len())
        .sum();
    QualificationReport {
        schema_version: REPORT_SCHEMA.into(),
        generated_at_unix_ms,
        contract_path: CONTRACT_PATH.into(),
        status: if failed_suite_ids.is_empty() {
            "passed"
        } else {
            "failed"
        }
        .into(),
        platform: Platform {
            os: std::env::consts::OS.into(),
            arch: std::env::consts::ARCH.into(),
        },
        rounds: contract.rounds,
        summary: Summary {
            module_count: contract.required_modules.len(),
            suite_count: suites.len(),
            round_count: suites.len() * contract.rounds,
            passed_round_count,
            assertion_count,
            acceptance_assertion_count: count_outcomes(contract, "acceptance"),
            rejection_assertion_count: count_outcomes(contract, "rejection"),
            failed_suite_ids,
        },
        suites,
    }
}

fn count_outcomes(contract: &QualificationContract, outcome: &str) -> usize {
    contract
        .suites
        .iter()
        .flat_map(|suite| &suite.assertions)
        .filter(|assertion| assertion.outcome == outcome)
        .count()
}

fn validate_report(
    contract: &QualificationContract,
    report: &QualificationReport,
) -> RunnerResult<()> {
    if report.schema_version != REPORT_SCHEMA
        || report.contract_path != CONTRACT_PATH
        || report.generated_at_unix_ms == 0
        || report.status != "passed"
        || report.rounds != contract.rounds
        || report.platform.os.is_empty()
        || report.platform.arch.is_empty()
        || report.suites.len() != contract.suites.len()
    {
        return Err("persistence provenance qualification report header is invalid".into());
    }
    for spec in &contract.suites {
        let suite = report
            .suites
            .iter()
            .find(|suite| suite.id == spec.id)
            .ok_or_else(|| format!("qualification report misses suite {}", spec.id))?;
        if suite.modules != spec.modules
            || suite.program != spec.program
            || suite.cwd != spec.cwd
            || suite.args != spec.args
            || suite.status != "passed"
            || !suite.repeatable
            || suite.rounds.len() != contract.rounds
        {
            return Err(format!("qualification report suite {} drifted", spec.id));
        }
        for (index, round) in suite.rounds.iter().enumerate() {
            if round.round != index + 1
                || round.status != "passed"
                || round.exit_code != 0
                || round.output_sha256.len() != 64
                || !round
                    .output_sha256
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
                || round.output_excerpt.is_empty()
                || round.output_excerpt.len() > EXCERPT_LIMIT
                || contains_host_identity(&round.output_excerpt)
                || round.assertions.len() != spec.assertions.len()
            {
                return Err(format!(
                    "qualification report suite {} round drifted",
                    spec.id
                ));
            }
            for assertion_spec in &spec.assertions {
                let assertion = round
                    .assertions
                    .iter()
                    .find(|item| item.id == assertion_spec.id)
                    .ok_or_else(|| {
                        format!("suite {} misses assertion {}", spec.id, assertion_spec.id)
                    })?;
                if assertion.label != assertion_spec.label
                    || assertion.outcome != assertion_spec.outcome
                    || assertion.boundary != assertion_spec.boundary
                    || !assertion.passed
                {
                    return Err(format!(
                        "qualification assertion {} drifted",
                        assertion_spec.id
                    ));
                }
            }
        }
    }
    let expected = build_summary(contract, &report.suites);
    if !summary_matches(&report.summary, &expected) || !report.summary.failed_suite_ids.is_empty() {
        return Err("persistence provenance qualification summary drifted".into());
    }
    Ok(())
}

fn build_summary(contract: &QualificationContract, suites: &[SuiteReport]) -> Summary {
    Summary {
        module_count: contract.required_modules.len(),
        suite_count: suites.len(),
        round_count: suites.len() * contract.rounds,
        passed_round_count: suites
            .iter()
            .flat_map(|suite| &suite.rounds)
            .filter(|round| round.status == "passed")
            .count(),
        assertion_count: contract
            .suites
            .iter()
            .map(|suite| suite.assertions.len())
            .sum(),
        acceptance_assertion_count: count_outcomes(contract, "acceptance"),
        rejection_assertion_count: count_outcomes(contract, "rejection"),
        failed_suite_ids: suites
            .iter()
            .filter(|suite| !suite.repeatable)
            .map(|suite| suite.id.clone())
            .collect(),
    }
}

fn summary_matches(left: &Summary, right: &Summary) -> bool {
    left.module_count == right.module_count
        && left.suite_count == right.suite_count
        && left.round_count == right.round_count
        && left.passed_round_count == right.passed_round_count
        && left.assertion_count == right.assertion_count
        && left.acceptance_assertion_count == right.acceptance_assertion_count
        && left.rejection_assertion_count == right.rejection_assertion_count
        && left.failed_suite_ids == right.failed_suite_ids
}

fn contains_host_identity(text: &str) -> bool {
    text.contains("/Users/")
        || text.contains("/home/")
        || text.contains("\\Users\\")
        || text.contains("@repo/../")
}

fn run_self_test(contract: &QualificationContract) -> RunnerResult<()> {
    let suites = contract
        .suites
        .iter()
        .map(|suite| SuiteReport {
            id: suite.id.clone(),
            modules: suite.modules.clone(),
            program: suite.program.clone(),
            cwd: suite.cwd.clone(),
            args: suite.args.clone(),
            status: "passed".into(),
            repeatable: true,
            rounds: (1..=contract.rounds)
                .map(|round| RoundReport {
                    round,
                    status: "passed".into(),
                    exit_code: 0,
                    elapsed_ms: 1,
                    output_sha256: "0".repeat(64),
                    assertions: suite
                        .assertions
                        .iter()
                        .map(|item| AssertionReport {
                            id: item.id.clone(),
                            label: item.label.clone(),
                            outcome: item.outcome.clone(),
                            boundary: item.boundary.clone(),
                            passed: true,
                        })
                        .collect(),
                    output_excerpt: "self-test evidence".into(),
                })
                .collect(),
        })
        .collect();
    let mut report = build_report(contract, suites, 1);
    validate_report(contract, &report)?;
    report.suites[0].rounds[0].assertions[0].passed = false;
    if validate_report(contract, &report).is_ok() {
        return Err("self-test accepted a failed provenance assertion".into());
    }
    Ok(())
}
