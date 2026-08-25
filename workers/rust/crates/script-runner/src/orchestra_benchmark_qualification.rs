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

const CONTRACT_PATH: &str = "config/architecture/orchestra-benchmark-qualification.json";
const CONTRACT_SCHEMA_PATH: &str = "schemas/orchestra-benchmark-qualification-contract.schema.json";
const REPORT_SCHEMA_PATH: &str = "schemas/orchestra-benchmark-qualification-report.schema.json";
const CONTRACT_SCHEMA: &str = "kyuubiki.orchestra-benchmark-qualification-contract/v1";
const REPORT_SCHEMA: &str = "kyuubiki.orchestra-benchmark-qualification-report/v1";
const DEFAULT_OUT: &str = "tmp/orchestra-benchmark-qualification-report.json";
const LIMITATIONS: &[&str] = &[
    "Elapsed thresholds are catastrophic-regression guards on the capture host, not hardware-independent performance guarantees.",
    "The qualification covers compact 256, 512, and 1024 pass-through Orchestra graphs with deterministic fake Agent sessions.",
];

#[derive(Clone, Deserialize)]
struct QualificationContract {
    schema_version: String,
    report_schema: String,
    required_module: String,
    rounds: usize,
    runner: RunnerSpec,
    cases: Vec<CaseSpec>,
    source_files: Vec<String>,
}

#[derive(Clone, Deserialize)]
struct RunnerSpec {
    program: String,
    cwd: String,
    environment: String,
    args_prefix: Vec<String>,
}

#[derive(Clone, Deserialize)]
struct CaseSpec {
    pass_through_count: usize,
    expected_completed_nodes: usize,
    response_mode: String,
    max_elapsed_ms: u64,
}

#[derive(Deserialize)]
struct RawReport {
    cases: Vec<RawCase>,
}

#[derive(Deserialize)]
struct RawCase {
    pass_through_count: usize,
    elapsed_ms: u64,
    completed_nodes: usize,
    skipped_nodes: usize,
    response_options: RawResponseOptions,
    performance: RawPerformance,
}

#[derive(Deserialize)]
struct RawResponseOptions {
    response_mode: String,
}

#[derive(Deserialize)]
struct RawPerformance {
    completed_node_count: usize,
    loop_passes: usize,
    total_elapsed_ms: f64,
    scheduler_overhead_ms: f64,
}

#[derive(Clone, Deserialize, Serialize)]
struct QualificationReport {
    schema_version: String,
    generated_at_unix_ms: u128,
    contract_path: String,
    status: String,
    platform: Platform,
    source_tree_sha256: String,
    rounds: Vec<RoundEvidence>,
    summary: Summary,
    limitations: Vec<String>,
}

#[derive(Clone, Deserialize, Serialize)]
struct Platform {
    os: String,
    arch: String,
}

#[derive(Clone, Deserialize, Serialize)]
struct RoundEvidence {
    round: usize,
    program: String,
    cwd: String,
    args: Vec<String>,
    launch_elapsed_ms: u128,
    raw_report_sha256: String,
    semantic_sha256: String,
    cases: Vec<CaseEvidence>,
}

#[derive(Clone, Deserialize, Serialize)]
struct CaseEvidence {
    pass_through_count: usize,
    elapsed_ms: u64,
    completed_nodes: usize,
    skipped_nodes: usize,
    response_mode: String,
    loop_passes: usize,
    total_elapsed_ms: f64,
    scheduler_overhead_ms: f64,
}

#[derive(Clone, Deserialize, PartialEq, Serialize)]
struct Summary {
    round_count: usize,
    case_count: usize,
    largest_pass_through_count: usize,
    max_observed_elapsed_ms: u64,
    stable_semantics: bool,
}

pub(crate) fn run_check_orchestra_benchmark_qualification(
    root: &Path,
    args: Vec<OsString>,
) -> RunnerResult<u8> {
    let options = parse_options(args, "Orchestra benchmark qualification")?;
    let contract: QualificationContract = read_json(root, CONTRACT_PATH)?;
    validate_contract(root, &contract)?;
    if options.self_test {
        validator_self_test(root, &contract)?;
        println!("Orchestra benchmark qualification self-test passed");
        return Ok(0);
    }
    if let Some(path) = options.verify_report {
        let report: QualificationReport = read_json(root, &path)?;
        validate_report(root, &contract, &report)?;
        println!("Orchestra benchmark qualification report passed: {path}");
        return Ok(0);
    }

    let report = capture_report(root, &contract)?;
    validate_report(root, &contract, &report)?;
    let out = options.out.as_deref().unwrap_or(DEFAULT_OUT);
    write_json_compact(root, out, &report)?;
    println!(
        "Orchestra benchmark qualified: {} round(s), {} captured case(s)",
        report.summary.round_count, report.summary.case_count
    );
    println!("Orchestra benchmark qualification report written: {out}");
    Ok(0)
}

fn validate_contract(root: &Path, contract: &QualificationContract) -> RunnerResult<()> {
    if contract.schema_version != CONTRACT_SCHEMA
        || contract.report_schema != REPORT_SCHEMA
        || contract.required_module != "orchestra-control-plane"
        || contract.rounds != 3
    {
        return Err("Orchestra benchmark qualification contract header drifted".to_string());
    }
    for (path, expected) in [
        (CONTRACT_SCHEMA_PATH, CONTRACT_SCHEMA),
        (REPORT_SCHEMA_PATH, REPORT_SCHEMA),
    ] {
        let schema: Value = read_json(root, path)?;
        if schema
            .pointer("/properties/schema_version/const")
            .and_then(Value::as_str)
            != Some(expected)
        {
            return Err(format!("{path} schema const drifted"));
        }
    }
    if contract.runner.program != "mix"
        || contract.runner.cwd != "apps/web"
        || contract.runner.environment != "test"
        || contract.runner.args_prefix
            != ["run", "../../scripts/workflow-large-graph-benchmark.exs"]
    {
        return Err("Orchestra benchmark runner drifted".to_string());
    }
    let expected = [(256, 261), (512, 517), (1024, 1029)];
    if contract.cases.len() != expected.len() {
        return Err("Orchestra benchmark case count drifted".to_string());
    }
    for (spec, (pass_through_count, completed_nodes)) in contract.cases.iter().zip(expected) {
        if spec.pass_through_count != pass_through_count
            || spec.expected_completed_nodes != completed_nodes
            || spec.response_mode != "auto-compact"
            || !(100..=10_000).contains(&spec.max_elapsed_ms)
        {
            return Err(format!(
                "Orchestra benchmark case {} drifted",
                spec.pass_through_count
            ));
        }
    }
    if contract.source_files.len() < 8
        || contract.source_files.iter().collect::<BTreeSet<_>>().len()
            != contract.source_files.len()
    {
        return Err("Orchestra benchmark source set is incomplete".to_string());
    }
    for path in &contract.source_files {
        if !repo_path(root, path)?.is_file() {
            return Err(format!("Orchestra benchmark source is missing: {path}"));
        }
    }
    Ok(())
}

fn capture_report(
    root: &Path,
    contract: &QualificationContract,
) -> RunnerResult<QualificationReport> {
    let run_root = format!(
        "tmp/orchestra-benchmark-qualification-{}",
        std::process::id()
    );
    fs::create_dir_all(repo_path(root, &run_root)?)
        .map_err(|error| format!("failed to create Orchestra benchmark run root: {error}"))?;
    let capture = capture_rounds(root, contract, &run_root);
    let cleanup = fs::remove_dir_all(repo_path(root, &run_root)?)
        .map_err(|error| format!("failed to clean Orchestra benchmark run root: {error}"));
    let rounds = capture?;
    cleanup?;
    let summary = summarize(&rounds);
    Ok(QualificationReport {
        schema_version: REPORT_SCHEMA.to_string(),
        generated_at_unix_ms: generated_at_unix_ms()?,
        contract_path: CONTRACT_PATH.to_string(),
        status: "pass".to_string(),
        platform: Platform {
            os: std::env::consts::OS.to_string(),
            arch: std::env::consts::ARCH.to_string(),
        },
        source_tree_sha256: source_tree_digest(root, &contract.source_files)?,
        rounds,
        summary,
        limitations: LIMITATIONS
            .iter()
            .map(|value| (*value).to_string())
            .collect(),
    })
}

fn capture_rounds(
    root: &Path,
    contract: &QualificationContract,
    run_root: &str,
) -> RunnerResult<Vec<RoundEvidence>> {
    let mut rounds = Vec::new();
    for round in 1..=contract.rounds {
        let report_path = format!("{run_root}/round-{round}.json");
        let output_arg = format!("../../{report_path}");
        let actual_args = actual_args(contract, &output_arg);
        let started = Instant::now();
        let output = Command::new(&contract.runner.program)
            .args(&actual_args)
            .current_dir(repo_path(root, &contract.runner.cwd)?)
            .env("MIX_ENV", &contract.runner.environment)
            .output()
            .map_err(|error| format!("failed to launch Orchestra benchmark: {error}"))?;
        if !output.status.success() {
            return Err(format!(
                "Orchestra benchmark round {round} failed: {}",
                portable_output(root, &output)
            ));
        }
        let raw_bytes = fs::read(repo_path(root, &report_path)?)
            .map_err(|error| format!("failed to read Orchestra benchmark round: {error}"))?;
        let raw: RawReport = serde_json::from_slice(&raw_bytes)
            .map_err(|error| format!("invalid Orchestra benchmark round: {error}"))?;
        let cases = case_evidence(contract, raw)?;
        rounds.push(RoundEvidence {
            round,
            program: contract.runner.program.clone(),
            cwd: contract.runner.cwd.clone(),
            args: retained_args(contract),
            launch_elapsed_ms: started.elapsed().as_millis().max(1),
            raw_report_sha256: digest(&raw_bytes),
            semantic_sha256: semantic_digest(&cases)?,
            cases,
        });
    }
    Ok(rounds)
}

fn actual_args(contract: &QualificationContract, output: &str) -> Vec<String> {
    contract
        .runner
        .args_prefix
        .iter()
        .cloned()
        .chain(["--output".to_string(), output.to_string()])
        .chain(
            contract
                .cases
                .iter()
                .map(|case| case.pass_through_count.to_string()),
        )
        .collect()
}

fn retained_args(contract: &QualificationContract) -> Vec<String> {
    actual_args(contract, "@round-report")
}

fn case_evidence(
    contract: &QualificationContract,
    raw: RawReport,
) -> RunnerResult<Vec<CaseEvidence>> {
    if raw.cases.len() != contract.cases.len() {
        return Err("Orchestra benchmark raw case count drifted".to_string());
    }
    contract
        .cases
        .iter()
        .zip(raw.cases)
        .map(|(spec, case)| {
            let evidence = CaseEvidence {
                pass_through_count: case.pass_through_count,
                elapsed_ms: case.elapsed_ms,
                completed_nodes: case.completed_nodes,
                skipped_nodes: case.skipped_nodes,
                response_mode: case.response_options.response_mode,
                loop_passes: case.performance.loop_passes,
                total_elapsed_ms: case.performance.total_elapsed_ms,
                scheduler_overhead_ms: case.performance.scheduler_overhead_ms,
            };
            validate_case(spec, &evidence, case.performance.completed_node_count)?;
            Ok(evidence)
        })
        .collect()
}

fn validate_case(
    spec: &CaseSpec,
    case: &CaseEvidence,
    performance_completed_nodes: usize,
) -> RunnerResult<()> {
    if case.pass_through_count != spec.pass_through_count
        || case.completed_nodes != spec.expected_completed_nodes
        || performance_completed_nodes != spec.expected_completed_nodes
        || case.skipped_nodes != 0
        || case.response_mode != spec.response_mode
        || case.loop_passes != 1
        || case.elapsed_ms > spec.max_elapsed_ms
        || !valid_metric(case.total_elapsed_ms, spec.max_elapsed_ms)
        || !valid_metric(case.scheduler_overhead_ms, spec.max_elapsed_ms)
    {
        return Err(format!(
            "Orchestra benchmark case {} failed qualification",
            spec.pass_through_count
        ));
    }
    Ok(())
}

fn valid_metric(value: f64, max: u64) -> bool {
    value.is_finite() && value >= 0.0 && value <= max as f64
}

fn validate_report(
    root: &Path,
    contract: &QualificationContract,
    report: &QualificationReport,
) -> RunnerResult<()> {
    if report.schema_version != REPORT_SCHEMA
        || report.contract_path != CONTRACT_PATH
        || report.generated_at_unix_ms == 0
        || report.status != "pass"
        || report.platform.os.is_empty()
        || report.platform.arch.is_empty()
        || report.limitations != LIMITATIONS
    {
        return Err("Orchestra benchmark report header drifted".to_string());
    }
    if report.source_tree_sha256 != source_tree_digest(root, &contract.source_files)? {
        return Err("Orchestra benchmark report source tree drifted".to_string());
    }
    if report.rounds.len() != contract.rounds {
        return Err("Orchestra benchmark report round count drifted".to_string());
    }
    let mut semantics = BTreeSet::new();
    for (index, round) in report.rounds.iter().enumerate() {
        if round.round != index + 1
            || round.program != contract.runner.program
            || round.cwd != contract.runner.cwd
            || round.args != retained_args(contract)
            || round.launch_elapsed_ms == 0
            || !is_digest(&round.raw_report_sha256)
            || round.semantic_sha256 != semantic_digest(&round.cases)?
        {
            return Err(format!("Orchestra benchmark round {} drifted", round.round));
        }
        if round.cases.len() != contract.cases.len() {
            return Err(format!(
                "Orchestra benchmark round {} is incomplete",
                round.round
            ));
        }
        for (spec, case) in contract.cases.iter().zip(&round.cases) {
            validate_case(spec, case, case.completed_nodes)?;
        }
        semantics.insert(round.semantic_sha256.as_str());
    }
    if semantics.len() != 1 || report.summary != summarize(&report.rounds) {
        return Err("Orchestra benchmark semantics or summary drifted".to_string());
    }
    let rendered = serde_json::to_string(report)
        .map_err(|error| format!("failed to inspect Orchestra benchmark report: {error}"))?;
    if rendered.contains("/Users/")
        || rendered.contains("/home/")
        || rendered.contains("\\\\Users\\\\")
    {
        return Err("Orchestra benchmark report leaks a host path".to_string());
    }
    Ok(())
}

fn summarize(rounds: &[RoundEvidence]) -> Summary {
    Summary {
        round_count: rounds.len(),
        case_count: rounds.iter().map(|round| round.cases.len()).sum(),
        largest_pass_through_count: rounds
            .iter()
            .flat_map(|round| &round.cases)
            .map(|case| case.pass_through_count)
            .max()
            .unwrap_or(0),
        max_observed_elapsed_ms: rounds
            .iter()
            .flat_map(|round| &round.cases)
            .map(|case| case.elapsed_ms)
            .max()
            .unwrap_or(0),
        stable_semantics: rounds
            .iter()
            .map(|round| round.semantic_sha256.as_str())
            .collect::<BTreeSet<_>>()
            .len()
            <= 1,
    }
}

fn semantic_digest(cases: &[CaseEvidence]) -> RunnerResult<String> {
    let values = cases
        .iter()
        .map(|case| {
            (
                case.pass_through_count,
                case.completed_nodes,
                case.skipped_nodes,
                case.response_mode.as_str(),
                case.loop_passes,
            )
        })
        .collect::<Vec<_>>();
    serde_json::to_vec(&values)
        .map(|bytes| digest(&bytes))
        .map_err(|error| format!("failed to encode Orchestra benchmark semantics: {error}"))
}

fn source_tree_digest(root: &Path, paths: &[String]) -> RunnerResult<String> {
    let mut ordered = paths.to_vec();
    ordered.sort();
    let mut hasher = Sha256::new();
    for path in ordered {
        let bytes = fs::read(repo_path(root, &path)?).map_err(|error| {
            format!("failed to read Orchestra benchmark source {path}: {error}")
        })?;
        hasher.update(path.len().to_le_bytes());
        hasher.update(path.as_bytes());
        hasher.update(bytes.len().to_le_bytes());
        hasher.update(bytes);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn is_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn validator_self_test(root: &Path, contract: &QualificationContract) -> RunnerResult<()> {
    let cases = contract
        .cases
        .iter()
        .map(|spec| CaseEvidence {
            pass_through_count: spec.pass_through_count,
            elapsed_ms: 1,
            completed_nodes: spec.expected_completed_nodes,
            skipped_nodes: 0,
            response_mode: spec.response_mode.clone(),
            loop_passes: 1,
            total_elapsed_ms: 0.5,
            scheduler_overhead_ms: 0.1,
        })
        .collect::<Vec<_>>();
    let semantic_sha256 = semantic_digest(&cases)?;
    let rounds = (1..=contract.rounds)
        .map(|round| RoundEvidence {
            round,
            program: contract.runner.program.clone(),
            cwd: contract.runner.cwd.clone(),
            args: retained_args(contract),
            launch_elapsed_ms: 1,
            raw_report_sha256: "0".repeat(64),
            semantic_sha256: semantic_sha256.clone(),
            cases: cases.clone(),
        })
        .collect::<Vec<_>>();
    let report = QualificationReport {
        schema_version: REPORT_SCHEMA.to_string(),
        generated_at_unix_ms: 1,
        contract_path: CONTRACT_PATH.to_string(),
        status: "pass".to_string(),
        platform: Platform {
            os: "fixture".to_string(),
            arch: "fixture".to_string(),
        },
        source_tree_sha256: source_tree_digest(root, &contract.source_files)?,
        summary: summarize(&rounds),
        rounds,
        limitations: LIMITATIONS
            .iter()
            .map(|value| (*value).to_string())
            .collect(),
    };
    validate_report(root, contract, &report)?;
    let mut tampered = report.clone();
    tampered.rounds[0].cases[0].completed_nodes += 1;
    tampered.rounds[0].semantic_sha256 = semantic_digest(&tampered.rounds[0].cases)?;
    tampered.summary = summarize(&tampered.rounds);
    if validate_report(root, contract, &tampered).is_ok() {
        return Err("validator self-test accepted a tampered graph result".to_string());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{CaseEvidence, digest, is_digest, semantic_digest};

    #[test]
    fn semantic_digest_ignores_timing_noise() {
        let mut case = CaseEvidence {
            pass_through_count: 256,
            elapsed_ms: 10,
            completed_nodes: 261,
            skipped_nodes: 0,
            response_mode: "auto-compact".to_string(),
            loop_passes: 1,
            total_elapsed_ms: 5.0,
            scheduler_overhead_ms: 1.0,
        };
        let before = semantic_digest(&[case.clone()]).unwrap();
        case.elapsed_ms = 20;
        case.total_elapsed_ms = 9.0;
        assert_eq!(before, semantic_digest(&[case]).unwrap());
        assert_eq!(digest(b"stable").len(), 64);
    }

    #[test]
    fn report_digests_must_use_canonical_lowercase_hex() {
        assert!(is_digest(&"a".repeat(64)));
        assert!(!is_digest(&"A".repeat(64)));
        assert!(!is_digest(&"g".repeat(64)));
        assert!(!is_digest(&"a".repeat(63)));
    }
}
