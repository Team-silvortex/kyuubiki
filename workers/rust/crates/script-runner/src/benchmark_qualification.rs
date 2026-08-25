use crate::direct_mesh_benchmark_compare::run_compare_direct_mesh_benchmark;
use crate::qualification_support::{
    generated_at_unix_ms, parse_options, read_json, repo_path, write_json_compact,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsString;
use std::fs;
use std::path::Path;
use std::process::Command;
use std::time::Instant;

mod validation;

use validation::{validate_contract, validate_report};

type RunnerResult<T> = Result<T, String>;

const CONTRACT_PATH: &str = "config/architecture/benchmark-qualification.json";
const CONTRACT_SCHEMA: &str = "kyuubiki.benchmark-qualification-contract/v2";
const REPORT_SCHEMA: &str = "kyuubiki.benchmark-qualification-report/v2";
const RETAINED_REPORT_SCHEMA: &str = "kyuubiki.benchmark-qualification-report/v1";
const REPORT_SCHEMA_PATH: &str = "schemas/benchmark-qualification-report.schema.json";
const DEFAULT_OUT: &str = "tmp/benchmark-qualification-report.json";
const DIRECT_MESH_COMPARE_OUT: &str = "tmp/benchmark-qualification/direct-mesh-self-compare.json";
const ROUTE: &str = "kyuubiki-script-runner benchmark-release";
const LIMITATIONS: [&str; 2] = [
    "Current-line release runs anchor the current benchmark and native command path; retained 500k and 1M scale results were produced by prior remote Linux runs.",
    "Qualification proves repeatable execution, scale coverage, evidence ingestion, and resolved failure accounting; it is not a hardware-independent performance guarantee.",
];

#[derive(Clone, Deserialize)]
struct QualificationContract {
    schema_version: String,
    report_schema: String,
    required_modules: Vec<String>,
    retained_scale_report: String,
    coverage_manifest: String,
    direct_mesh_baseline: String,
    source_files: Vec<String>,
    current_runs: Vec<CurrentRunSpec>,
    profile_requirements: Vec<ProfileRequirement>,
    one_million_node_threshold: usize,
    min_retained_runs: usize,
    min_resolved_failures: usize,
    min_direct_mesh_repeats: usize,
}

#[derive(Clone, Deserialize, Serialize)]
struct CurrentRunSpec {
    id: String,
    profile: String,
    report_profile: String,
    matrix: String,
    case_id: String,
    repeat: usize,
    min_node_count: usize,
}

#[derive(Clone, Deserialize)]
struct ProfileRequirement {
    profile: String,
    expected_case_count: usize,
    require_scale_qualified: bool,
}

#[derive(Deserialize)]
struct CoverageManifest {
    schema_version: String,
    targets: Vec<CoverageTarget>,
}

#[derive(Deserialize)]
struct CoverageTarget {
    matrix: String,
    profile: String,
    expected_cases: Vec<String>,
}

#[derive(Deserialize)]
struct BenchmarkOutput {
    repeat: usize,
    profile: String,
    matrix: String,
    cases: Vec<BenchmarkCaseOutput>,
}

#[derive(Deserialize)]
struct BenchmarkCaseOutput {
    id: String,
    family: String,
    ok: bool,
    error: Option<String>,
    repeat: usize,
    min_ms: f64,
    median_ms: f64,
    mean_ms: f64,
    p95_ms: f64,
    max_ms: f64,
    dof_count: usize,
    node_count: usize,
    element_count: usize,
    peak_rss_kib: u64,
}

#[derive(Deserialize, Serialize)]
struct QualificationReport {
    schema_version: String,
    generated_at_unix_ms: u128,
    contract_path: String,
    status: String,
    platform: Platform,
    source_tree_sha256: String,
    current_runs: Vec<CurrentRunEvidence>,
    scale_archive: ScaleArchiveEvidence,
    direct_mesh: DirectMeshEvidence,
    summary: QualificationSummary,
    limitations: Vec<String>,
}

#[derive(Deserialize, Serialize)]
struct Platform {
    os: String,
    arch: String,
}

#[derive(Deserialize, Serialize)]
struct CurrentRunEvidence {
    id: String,
    route: String,
    args: Vec<String>,
    status: String,
    launch_elapsed_ms: u128,
    stdout_sha256: String,
    repeat: usize,
    profile: String,
    matrix: String,
    case: CurrentCaseEvidence,
}

#[derive(Deserialize, Serialize)]
struct CurrentCaseEvidence {
    id: String,
    family: String,
    ok: bool,
    repeat: usize,
    min_ms: f64,
    median_ms: f64,
    mean_ms: f64,
    p95_ms: f64,
    max_ms: f64,
    dof_count: usize,
    node_count: usize,
    element_count: usize,
    peak_rss_kib: u64,
}

#[derive(Deserialize, Serialize)]
struct ScaleArchiveEvidence {
    schema_version: String,
    source_index_sha256: String,
    #[serde(default)]
    retained_report_path: String,
    #[serde(default)]
    retained_report_sha256: String,
    gate_status: String,
    retained_run_count: usize,
    failed_run_count: usize,
    resolved_failure_count: usize,
    unresolved_failure_count: usize,
    profiles: Vec<ProfileEvidence>,
    one_million_cases: Vec<ScaleCaseEvidence>,
}

#[derive(Deserialize)]
struct RetainedScaleReport {
    schema_version: String,
    status: String,
    scale_archive: ScaleArchiveEvidence,
}

#[derive(Deserialize, Serialize)]
struct ProfileEvidence {
    profile: String,
    expected_case_count: usize,
    covered_case_count: usize,
    missing_case_count: usize,
    scale_qualified_covered_case_count: usize,
    below_scale_threshold_case_count: usize,
}

#[derive(Deserialize, Serialize)]
struct ScaleCaseEvidence {
    matrix: String,
    case_id: String,
    node_count: usize,
    element_count: usize,
    dof_count: usize,
    source_slug: String,
    run_case_count: usize,
    run_repeat: usize,
    run_total_median_ms: f64,
    run_peak_rss_mib: f64,
}

#[derive(Deserialize, Serialize)]
struct DirectMeshEvidence {
    baseline_path: String,
    baseline_sha256: String,
    repeat: usize,
    run_count: usize,
    subtest_sample_count: usize,
    elapsed_mean_s: f64,
    peak_rss_mean_kib: f64,
    comparator_status: String,
    comparison_sha256: String,
}

#[derive(Deserialize, Serialize)]
struct QualificationSummary {
    module_count: usize,
    current_run_count: usize,
    current_repeat_count: usize,
    five_hundred_k_case_count: usize,
    one_million_case_count: usize,
    one_million_matrix_count: usize,
    resolved_failure_count: usize,
    direct_mesh_repeat_count: usize,
}

pub(crate) fn run_check_benchmark_qualification(
    root: &Path,
    args: Vec<OsString>,
) -> RunnerResult<u8> {
    let options = parse_options(args, "benchmark qualification")?;
    let contract: QualificationContract = read_json(root, CONTRACT_PATH)?;
    let manifest: CoverageManifest = read_json(root, &contract.coverage_manifest)?;
    validate_contract(root, &contract, &manifest)?;

    if options.self_test {
        run_self_test(root, &contract, &manifest)?;
        println!("benchmark qualification self-test passed");
        return Ok(0);
    }
    if let Some(path) = options.verify_report {
        let report: QualificationReport = read_json(root, &path)?;
        validate_report(root, &contract, &manifest, &report)?;
        println!("benchmark qualification report passed: {path}");
        return Ok(0);
    }

    let report = execute_qualification(root, &contract, &manifest)?;
    let out = options.out.as_deref().unwrap_or(DEFAULT_OUT);
    write_json_compact(root, out, &report)?;
    validate_report(root, &contract, &manifest, &report)?;
    println!(
        "benchmark qualified: {} current runs, {} 500k cases, {} 1M cases",
        report.summary.current_run_count,
        report.summary.five_hundred_k_case_count,
        report.summary.one_million_case_count
    );
    println!("benchmark qualification report written: {out}");
    Ok(0)
}

fn execute_qualification(
    root: &Path,
    contract: &QualificationContract,
    manifest: &CoverageManifest,
) -> RunnerResult<QualificationReport> {
    let current_runs = contract
        .current_runs
        .iter()
        .map(|spec| run_current_case(root, spec))
        .collect::<RunnerResult<Vec<_>>>()?;
    let scale_archive = collect_scale_archive(root, contract, manifest)?;
    let direct_mesh = collect_direct_mesh(root, contract)?;
    let summary = build_summary(contract, &current_runs, &scale_archive, &direct_mesh);
    Ok(QualificationReport {
        schema_version: REPORT_SCHEMA.into(),
        generated_at_unix_ms: generated_at_unix_ms()?,
        contract_path: CONTRACT_PATH.into(),
        status: "passed".into(),
        platform: Platform {
            os: std::env::consts::OS.into(),
            arch: std::env::consts::ARCH.into(),
        },
        source_tree_sha256: source_tree_digest(root, &contract.source_files)?,
        current_runs,
        scale_archive,
        direct_mesh,
        summary,
        limitations: LIMITATIONS.iter().map(|value| (*value).into()).collect(),
    })
}

fn run_current_case(root: &Path, spec: &CurrentRunSpec) -> RunnerResult<CurrentRunEvidence> {
    let args = benchmark_args(spec);
    let started = Instant::now();
    let output = Command::new(
        std::env::current_exe().map_err(|error| format!("cannot locate native runner: {error}"))?,
    )
    .arg("benchmark-release")
    .args(&args)
    .current_dir(root)
    .env("NO_COLOR", "1")
    .output()
    .map_err(|error| format!("failed to launch benchmark {}: {error}", spec.id))?;
    if !output.status.success() {
        return Err(format!(
            "benchmark {} failed: {}",
            spec.id,
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    let benchmark: BenchmarkOutput = serde_json::from_slice(&output.stdout)
        .map_err(|error| format!("benchmark {} emitted invalid JSON: {error}", spec.id))?;
    if benchmark.cases.len() != 1 {
        return Err(format!("benchmark {} must emit exactly one case", spec.id));
    }
    let case = &benchmark.cases[0];
    if benchmark.repeat != spec.repeat
        || benchmark.profile != spec.report_profile
        || benchmark.matrix != spec.matrix
        || case.id != spec.case_id
        || case.repeat != spec.repeat
        || !case.ok
        || case.error.is_some()
        || case.node_count < spec.min_node_count
    {
        return Err(format!("benchmark {} result drifted", spec.id));
    }
    Ok(CurrentRunEvidence {
        id: spec.id.clone(),
        route: ROUTE.into(),
        args,
        status: "passed".into(),
        launch_elapsed_ms: started.elapsed().as_millis(),
        stdout_sha256: sha256_bytes(&output.stdout),
        repeat: benchmark.repeat,
        profile: benchmark.profile,
        matrix: benchmark.matrix,
        case: CurrentCaseEvidence {
            id: case.id.clone(),
            family: case.family.clone(),
            ok: case.ok,
            repeat: case.repeat,
            min_ms: case.min_ms,
            median_ms: case.median_ms,
            mean_ms: case.mean_ms,
            p95_ms: case.p95_ms,
            max_ms: case.max_ms,
            dof_count: case.dof_count,
            node_count: case.node_count,
            element_count: case.element_count,
            peak_rss_kib: case.peak_rss_kib,
        },
    })
}

fn benchmark_args(spec: &CurrentRunSpec) -> Vec<String> {
    vec![
        "--profile".into(),
        spec.profile.clone(),
        "--matrix".into(),
        spec.matrix.clone(),
        "--case".into(),
        spec.case_id.clone(),
        "--repeat".into(),
        spec.repeat.to_string(),
        "--format".into(),
        "json".into(),
    ]
}

fn collect_scale_archive(
    root: &Path,
    contract: &QualificationContract,
    manifest: &CoverageManifest,
) -> RunnerResult<ScaleArchiveEvidence> {
    let retained: RetainedScaleReport = read_json(root, &contract.retained_scale_report)?;
    if retained.schema_version != RETAINED_REPORT_SCHEMA || retained.status != "passed" {
        return Err("retained benchmark scale report header drifted".into());
    }
    let mut archive = retained.scale_archive;
    archive.retained_report_path = contract.retained_scale_report.clone();
    archive.retained_report_sha256 = sha256_file(root, &contract.retained_scale_report)?;
    validation::validate_scale_archive(root, contract, manifest, &archive)?;
    Ok(archive)
}

fn collect_direct_mesh(
    root: &Path,
    contract: &QualificationContract,
) -> RunnerResult<DirectMeshEvidence> {
    let baseline: Value = read_json(root, &contract.direct_mesh_baseline)?;
    let args = [
        "--current",
        contract.direct_mesh_baseline.as_str(),
        "--baseline",
        contract.direct_mesh_baseline.as_str(),
        "--json-out",
        DIRECT_MESH_COMPARE_OUT,
        "--fail-on-elapsed-regression-pct",
        "0",
        "--fail-on-rss-regression-pct",
        "0",
    ]
    .into_iter()
    .map(OsString::from)
    .collect();
    if run_compare_direct_mesh_benchmark(root, args)? != 0 {
        return Err("direct mesh comparator self-check failed".into());
    }
    let comparison: Value = read_json(root, DIRECT_MESH_COMPARE_OUT)?;
    let runs = array_at(&baseline, "/runs")?;
    let subtest_sample_count = runs
        .iter()
        .map(|run| run["subtests"].as_array().map(Vec::len).unwrap_or(0))
        .sum();
    Ok(DirectMeshEvidence {
        baseline_path: contract.direct_mesh_baseline.clone(),
        baseline_sha256: sha256_file(root, &contract.direct_mesh_baseline)?,
        repeat: usize_at(&baseline, "/source/repeat")?,
        run_count: runs.len(),
        subtest_sample_count,
        elapsed_mean_s: number_at(&baseline, "/aggregate/elapsed_s/mean")?,
        peak_rss_mean_kib: number_at(&baseline, "/aggregate/max_rss_kib/mean")?,
        comparator_status: if comparison["ok"].as_bool() == Some(true) {
            "passed"
        } else {
            "failed"
        }
        .into(),
        comparison_sha256: sha256_file(root, DIRECT_MESH_COMPARE_OUT)?,
    })
}

fn expected_one_million_cases(
    manifest: &CoverageManifest,
) -> RunnerResult<BTreeMap<String, Vec<String>>> {
    let mut expected = BTreeMap::new();
    for target in manifest
        .targets
        .iter()
        .filter(|target| target.profile == "one_million")
    {
        if target.matrix.is_empty()
            || target.expected_cases.is_empty()
            || expected
                .insert(target.matrix.clone(), target.expected_cases.clone())
                .is_some()
        {
            return Err("one-million benchmark coverage target drifted".into());
        }
    }
    Ok(expected)
}

fn build_summary(
    contract: &QualificationContract,
    current_runs: &[CurrentRunEvidence],
    scale: &ScaleArchiveEvidence,
    direct_mesh: &DirectMeshEvidence,
) -> QualificationSummary {
    QualificationSummary {
        module_count: contract.required_modules.len(),
        current_run_count: current_runs.len(),
        current_repeat_count: current_runs.iter().map(|run| run.repeat).sum(),
        five_hundred_k_case_count: scale
            .profiles
            .iter()
            .find(|item| item.profile == "five_hundred_k")
            .map(|item| item.covered_case_count)
            .unwrap_or(0),
        one_million_case_count: scale.one_million_cases.len(),
        one_million_matrix_count: scale
            .one_million_cases
            .iter()
            .map(|item| item.matrix.as_str())
            .collect::<BTreeSet<_>>()
            .len(),
        resolved_failure_count: scale.resolved_failure_count,
        direct_mesh_repeat_count: direct_mesh.repeat,
    }
}

fn run_self_test(
    root: &Path,
    contract: &QualificationContract,
    manifest: &CoverageManifest,
) -> RunnerResult<()> {
    let current_runs = contract
        .current_runs
        .iter()
        .map(|spec| CurrentRunEvidence {
            id: spec.id.clone(),
            route: ROUTE.into(),
            args: benchmark_args(spec),
            status: "passed".into(),
            launch_elapsed_ms: 1,
            stdout_sha256: "0".repeat(64),
            repeat: spec.repeat,
            profile: spec.report_profile.clone(),
            matrix: spec.matrix.clone(),
            case: CurrentCaseEvidence {
                id: spec.case_id.clone(),
                family: "self_test".into(),
                ok: true,
                repeat: spec.repeat,
                min_ms: 1.0,
                median_ms: 1.0,
                mean_ms: 1.0,
                p95_ms: 1.0,
                max_ms: 1.0,
                dof_count: spec.min_node_count,
                node_count: spec.min_node_count,
                element_count: spec.min_node_count,
                peak_rss_kib: 1,
            },
        })
        .collect::<Vec<_>>();
    let profiles = contract
        .profile_requirements
        .iter()
        .map(|item| ProfileEvidence {
            profile: item.profile.clone(),
            expected_case_count: item.expected_case_count,
            covered_case_count: item.expected_case_count,
            missing_case_count: 0,
            scale_qualified_covered_case_count: if item.require_scale_qualified {
                item.expected_case_count
            } else {
                0
            },
            below_scale_threshold_case_count: 0,
        })
        .collect();
    let one_million_cases = expected_one_million_cases(manifest)?
        .into_iter()
        .flat_map(|(matrix, cases)| {
            cases
                .into_iter()
                .map(move |case_id| (matrix.clone(), case_id))
        })
        .map(|(matrix, case_id)| ScaleCaseEvidence {
            matrix,
            case_id,
            node_count: contract.one_million_node_threshold,
            element_count: 1,
            dof_count: 1,
            source_slug: "self-test".into(),
            run_case_count: 1,
            run_repeat: 1,
            run_total_median_ms: 1.0,
            run_peak_rss_mib: 1.0,
        })
        .collect();
    let scale_archive = ScaleArchiveEvidence {
        schema_version: "kyuubiki.benchmark-profile-index/v1".into(),
        source_index_sha256: "0".repeat(64),
        retained_report_path: contract.retained_scale_report.clone(),
        retained_report_sha256: sha256_file(root, &contract.retained_scale_report)?,
        gate_status: "pass".into(),
        retained_run_count: contract.min_retained_runs,
        failed_run_count: contract.min_resolved_failures,
        resolved_failure_count: contract.min_resolved_failures,
        unresolved_failure_count: 0,
        profiles,
        one_million_cases,
    };
    let direct_mesh = DirectMeshEvidence {
        baseline_path: contract.direct_mesh_baseline.clone(),
        baseline_sha256: sha256_file(root, &contract.direct_mesh_baseline)?,
        repeat: contract.min_direct_mesh_repeats,
        run_count: contract.min_direct_mesh_repeats,
        subtest_sample_count: contract.min_direct_mesh_repeats * 2,
        elapsed_mean_s: 1.0,
        peak_rss_mean_kib: 1.0,
        comparator_status: "passed".into(),
        comparison_sha256: "0".repeat(64),
    };
    let summary = build_summary(contract, &current_runs, &scale_archive, &direct_mesh);
    let mut report = QualificationReport {
        schema_version: REPORT_SCHEMA.into(),
        generated_at_unix_ms: 1,
        contract_path: CONTRACT_PATH.into(),
        status: "passed".into(),
        platform: Platform {
            os: "self-test".into(),
            arch: "self-test".into(),
        },
        source_tree_sha256: source_tree_digest(root, &contract.source_files)?,
        current_runs,
        scale_archive,
        direct_mesh,
        summary,
        limitations: LIMITATIONS.iter().map(|value| (*value).into()).collect(),
    };
    validate_report(root, contract, manifest, &report)?;
    let retained_report_sha256 = report.scale_archive.retained_report_sha256.clone();
    report.scale_archive.retained_report_sha256 = "0".repeat(64);
    if validate_report(root, contract, manifest, &report).is_ok() {
        return Err("self-test accepted a tampered retained scale report digest".into());
    }
    report.scale_archive.retained_report_sha256 = retained_report_sha256;
    report.scale_archive.one_million_cases[0].node_count -= 1;
    if validate_report(root, contract, manifest, &report).is_ok() {
        return Err("self-test accepted a below-threshold 1M case".into());
    }
    Ok(())
}

fn source_tree_digest(root: &Path, paths: &[String]) -> RunnerResult<String> {
    let mut paths = paths.to_vec();
    paths.sort();
    let mut hasher = Sha256::new();
    for relative in paths {
        hasher.update(relative.as_bytes());
        hasher.update([0]);
        hasher.update(
            fs::read(repo_path(root, &relative)?)
                .map_err(|error| format!("failed to hash {relative}: {error}"))?,
        );
        hasher.update([0]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

fn sha256_file(root: &Path, relative: &str) -> RunnerResult<String> {
    fs::read(repo_path(root, relative)?)
        .map(|bytes| sha256_bytes(&bytes))
        .map_err(|error| format!("failed to hash {relative}: {error}"))
}

fn sha256_bytes(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn is_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}

fn array_at<'a>(value: &'a Value, pointer: &str) -> RunnerResult<&'a Vec<Value>> {
    value
        .pointer(pointer)
        .and_then(Value::as_array)
        .ok_or_else(|| format!("missing array at {pointer}"))
}

fn usize_at(value: &Value, pointer: &str) -> RunnerResult<usize> {
    value
        .pointer(pointer)
        .and_then(Value::as_u64)
        .and_then(|value| usize::try_from(value).ok())
        .ok_or_else(|| format!("missing integer at {pointer}"))
}

fn number_at(value: &Value, pointer: &str) -> RunnerResult<f64> {
    value
        .pointer(pointer)
        .and_then(Value::as_f64)
        .filter(|value| value.is_finite())
        .ok_or_else(|| format!("missing number at {pointer}"))
}
