use crate::RunnerResult;
use crate::qualification_support::{generated_at_unix_ms, read_json, repo_path, write_json};
use kyuubiki_engine::{
    BuiltInOperatorRegistryKind, DynamicOperatorHostSession,
    load_external_operator_packages_with_dynamic_host,
};
use kyuubiki_operator_sdk::{
    OPERATOR_JSON_ABI_SCHEMA_VERSION, OPERATOR_PACKAGE_MANIFEST_FILE,
    current_platform_library_file_name,
};
use kyuubiki_protocol::{OperatorRunContext, OperatorRunRequest};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::ffi::OsString;
use std::fs;
use std::hint::black_box;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Barrier};
use std::thread;
use std::time::Instant;

mod validation;

use validation::{
    load_and_validate_contract, source_tree_digest, validate_report, validator_self_test,
};

const CONTRACT_PATH: &str = "config/architecture/operator-sdk-performance-qualification.json";
const CONTRACT_SCHEMA: &str = "kyuubiki.operator-sdk-performance-qualification-contract/v1";
const REPORT_SCHEMA: &str = "kyuubiki.operator-sdk-performance-qualification/v1";
const QUALIFICATION_ID: &str = "operator-sdk-dynamic-json-abi-performance";
const PACKAGE_ID: &str = "operator.template.summary";
const OPERATOR_ID: &str = "extract.template_summary";
const LIMITATIONS: &[&str] = &[
    "This microbenchmark isolates the in-process dynamic operator host and excludes network, scheduler, and solver time.",
    "Thresholds are regression gates for the current release profile, not cross-hardware ranking claims.",
];

#[derive(Debug, Clone, Deserialize)]
struct QualificationContract {
    schema_version: String,
    qualification_id: String,
    execution_abi: String,
    source_files: Vec<String>,
    measurement: Measurement,
    thresholds: Thresholds,
    report: ReportPolicy,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct Measurement {
    resident_activation_iterations: usize,
    warmup_iterations: usize,
    compact_iterations: usize,
    medium_iterations: usize,
    medium_value_count: usize,
    concurrent_workers: usize,
    concurrent_iterations_per_worker: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct Thresholds {
    cold_activation_max_ns: u64,
    resident_activation_p95_max_ns: u64,
    first_dispatch_max_ns: u64,
    compact_p95_max_ns: u64,
    medium_p95_max_ns: u64,
    concurrent_min_dispatches_per_second: f64,
}

#[derive(Debug, Clone, Deserialize)]
struct ReportPolicy {
    schema_version: String,
    schema_path: String,
    default_output: String,
    retained_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct QualificationReport {
    schema_version: String,
    qualification_id: String,
    generated_at_unix_ms: u128,
    status: String,
    source_tree_sha256: String,
    platform: Platform,
    package: PackageIdentity,
    measurement: Measurement,
    thresholds: Thresholds,
    metrics: Metrics,
    optimization_contract: OptimizationContract,
    limitations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Platform {
    os: String,
    arch: String,
    build_profile: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PackageIdentity {
    package_id: String,
    operator_id: String,
    execution_abi: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Metrics {
    activation: ActivationCase,
    first_dispatch_ns: u64,
    compact: LatencyCase,
    medium: LatencyCase,
    concurrent: ConcurrentCase,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ActivationCase {
    cold_ns: u64,
    resident_iterations: usize,
    resident_p50_ns: u64,
    resident_p95_ns: u64,
    resident_max_ns: u64,
    resident_mean_ns: f64,
    errors: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LatencyCase {
    iterations: usize,
    request_bytes: usize,
    p50_ns: u64,
    p95_ns: u64,
    max_ns: u64,
    mean_ns: f64,
    errors: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ConcurrentCase {
    workers: usize,
    total_dispatches: usize,
    elapsed_ns: u64,
    dispatches_per_second: f64,
    errors: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OptimizationContract {
    host_response_copy_bytes_per_dispatch: usize,
    registry_initializations_per_library: usize,
    response_buffer_release: String,
}

pub(crate) fn run_qualify(root: &Path, args: Vec<OsString>) -> RunnerResult<u8> {
    let options = Options::parse(args)?;
    if options.help {
        println!("usage: kyuubiki-script-runner qualify-operator-sdk-performance [--out path]");
        return Ok(0);
    }
    if cfg!(debug_assertions) {
        return Err("operator SDK performance qualification requires a --release runner".into());
    }
    let contract = load_and_validate_contract(root)?;
    let output = options
        .output
        .unwrap_or_else(|| contract.report.default_output.clone());
    repo_path(root, &output)?;
    build_template(root)?;

    let relative_root = format!("tmp/operator-sdk-performance-{}", std::process::id());
    let work_root = repo_path(root, &relative_root)?;
    fs::create_dir_all(&work_root)
        .map_err(|error| format!("failed to create {}: {error}", work_root.display()))?;
    let result = capture(root, &contract, &output, &work_root);
    let cleanup = fs::remove_dir_all(&work_root)
        .map_err(|error| format!("failed to remove {}: {error}", work_root.display()));
    match (result, cleanup) {
        (Ok(status), Ok(())) => Ok(status),
        (Err(error), Ok(())) => Err(error),
        (Ok(_), Err(error)) | (Err(_), Err(error)) => Err(error),
    }
}

pub(crate) fn run_check(root: &Path, args: Vec<OsString>) -> RunnerResult<u8> {
    let options = CheckOptions::parse(args)?;
    if options.help {
        println!(
            "usage: kyuubiki-script-runner check-operator-sdk-performance-qualification [--self-test] [--verify-report path]"
        );
        return Ok(0);
    }
    let contract = load_and_validate_contract(root)?;
    if options.self_test {
        validator_self_test(root, &contract)?;
        println!("operator SDK performance qualification self-test passed");
        if options.report.is_none() {
            return Ok(0);
        }
    }
    let report_path = options
        .report
        .unwrap_or_else(|| contract.report.retained_path.clone());
    let report: QualificationReport = read_json(root, &report_path)?;
    validate_report(root, &contract, &report)?;
    println!("operator SDK performance qualification report passed: {report_path}");
    Ok(0)
}

fn capture(
    root: &Path,
    contract: &QualificationContract,
    output: &str,
    work_root: &Path,
) -> RunnerResult<u8> {
    prepare_package(root, work_root)?;
    let (session, activation) = measure_activation(
        work_root,
        contract.measurement.resident_activation_iterations,
    )?;

    let compact_request = request((1..=8).map(|value| value as f64).collect());
    let first_request = compact_request.clone();
    let first_at = Instant::now();
    verify_result(&session.run_operator(first_request)?, 8)?;
    let first_dispatch_ns = elapsed_ns(first_at);
    for _ in 0..contract.measurement.warmup_iterations {
        verify_result(&session.run_operator(compact_request.clone())?, 8)?;
    }

    let compact = measure_latency(
        &session,
        &compact_request,
        contract.measurement.compact_iterations,
        8,
    );
    let medium_request = request(
        (0..contract.measurement.medium_value_count)
            .map(|value| value as f64 * 0.25)
            .collect(),
    );
    let medium = measure_latency(
        &session,
        &medium_request,
        contract.measurement.medium_iterations,
        contract.measurement.medium_value_count,
    );
    let concurrent = measure_concurrent(
        Arc::new(session),
        compact_request,
        contract.measurement.concurrent_workers,
        contract.measurement.concurrent_iterations_per_worker,
    )?;

    let metrics = Metrics {
        activation,
        first_dispatch_ns,
        compact,
        medium,
        concurrent,
    };
    let status = if metrics_pass(&metrics, &contract.thresholds) {
        "pass"
    } else {
        "fail"
    };
    let report = QualificationReport {
        schema_version: REPORT_SCHEMA.to_string(),
        qualification_id: QUALIFICATION_ID.to_string(),
        generated_at_unix_ms: generated_at_unix_ms()?,
        status: status.to_string(),
        source_tree_sha256: source_tree_digest(root, &contract.source_files)?,
        platform: Platform {
            os: std::env::consts::OS.to_string(),
            arch: std::env::consts::ARCH.to_string(),
            build_profile: "release".to_string(),
        },
        package: PackageIdentity {
            package_id: PACKAGE_ID.to_string(),
            operator_id: OPERATOR_ID.to_string(),
            execution_abi: OPERATOR_JSON_ABI_SCHEMA_VERSION.to_string(),
        },
        measurement: contract.measurement.clone(),
        thresholds: contract.thresholds.clone(),
        metrics,
        optimization_contract: OptimizationContract {
            host_response_copy_bytes_per_dispatch: 0,
            registry_initializations_per_library: 1,
            response_buffer_release: "same-library-raii".to_string(),
        },
        limitations: LIMITATIONS
            .iter()
            .map(|value| (*value).to_string())
            .collect(),
    };
    write_json(root, output, &report)?;
    validate_report(root, contract, &report)?;
    println!("operator SDK performance qualification passed: {output}");
    Ok(0)
}

fn build_template(root: &Path) -> RunnerResult<()> {
    let status = Command::new("cargo")
        .args([
            "build",
            "--release",
            "--locked",
            "--manifest-path",
            "workers/rust/templates/operator-crate-template/Cargo.toml",
            "--lib",
        ])
        .current_dir(root)
        .status()
        .map_err(|error| format!("failed to build operator template: {error}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!(
            "operator template release build failed with {status}"
        ))
    }
}

fn prepare_package(root: &Path, work_root: &Path) -> RunnerResult<()> {
    let package_root = work_root.join("operator-template-summary");
    fs::create_dir_all(&package_root).map_err(|error| {
        format!(
            "failed to create benchmark package {}: {error}",
            package_root.display()
        )
    })?;
    let library_name = current_platform_library_file_name("kyuubiki_operator_template");
    let source_library = root
        .join("workers/rust/templates/operator-crate-template/target/release")
        .join(&library_name);
    fs::copy(&source_library, package_root.join(&library_name))
        .map_err(|error| format!("failed to stage {}: {error}", source_library.display()))?;
    let manifest_path =
        root.join("workers/rust/templates/operator-crate-template/kyuubiki-operator.json");
    let mut manifest: Value = serde_json::from_str(
        &fs::read_to_string(&manifest_path)
            .map_err(|error| format!("failed to read {}: {error}", manifest_path.display()))?,
    )
    .map_err(|error| format!("invalid template package manifest: {error}"))?;
    manifest["entrypoint"] = Value::String(library_name);
    fs::write(
        package_root.join(OPERATOR_PACKAGE_MANIFEST_FILE),
        format!(
            "{}\n",
            serde_json::to_string_pretty(&manifest)
                .map_err(|error| format!("failed to encode benchmark manifest: {error}"))?
        ),
    )
    .map_err(|error| format!("failed to write benchmark package manifest: {error}"))
}

fn verify_activation(session: &DynamicOperatorHostSession) -> RunnerResult<()> {
    if session.loaded_library_count() != 1
        || session.report().activated_package_summaries.len() != 1
        || session.package_for_operator(OPERATOR_ID).is_none()
        || session.registry().describe(OPERATOR_ID).is_none()
    {
        return Err(
            "benchmark package did not activate exactly one traceable dynamic operator".into(),
        );
    }
    Ok(())
}

fn activate_session(work_root: &Path) -> RunnerResult<DynamicOperatorHostSession> {
    let session = load_external_operator_packages_with_dynamic_host(
        BuiltInOperatorRegistryKind::Extract,
        work_root,
    )
    .map_err(|error| format!("failed to activate benchmark operator package: {error}"))?;
    verify_activation(&session)?;
    Ok(session)
}

fn measure_activation(
    work_root: &Path,
    resident_iterations: usize,
) -> RunnerResult<(DynamicOperatorHostSession, ActivationCase)> {
    let cold_started = Instant::now();
    let session = activate_session(work_root)?;
    let cold_ns = elapsed_ns(cold_started);
    let mut samples = Vec::with_capacity(resident_iterations);
    let mut errors = 0;
    for _ in 0..resident_iterations {
        let started = Instant::now();
        let result = activate_session(work_root);
        samples.push(elapsed_ns(started));
        if result.is_err() {
            errors += 1;
        }
    }
    samples.sort_unstable();
    let activation = ActivationCase {
        cold_ns,
        resident_iterations,
        resident_p50_ns: percentile(&samples, 50),
        resident_p95_ns: percentile(&samples, 95),
        resident_max_ns: samples.last().copied().unwrap_or(1),
        resident_mean_ns: samples.iter().map(|value| *value as f64).sum::<f64>()
            / resident_iterations as f64,
        errors,
    };
    Ok((session, activation))
}

fn request(values: Vec<f64>) -> OperatorRunRequest {
    OperatorRunRequest {
        operator_id: OPERATOR_ID.to_string(),
        input: serde_json::json!({"payload": {"values": values}, "config": {}}),
        context: OperatorRunContext::default(),
    }
}

fn measure_latency(
    session: &DynamicOperatorHostSession,
    request: &OperatorRunRequest,
    iterations: usize,
    expected_count: usize,
) -> LatencyCase {
    let request_bytes = serde_json::to_vec(request).map_or(0, |bytes| bytes.len());
    let mut samples = Vec::with_capacity(iterations);
    let mut errors = 0;
    for _ in 0..iterations {
        let sample = request.clone();
        let started = Instant::now();
        let result = session.run_operator(sample);
        samples.push(elapsed_ns(started));
        match result {
            Ok(result) if verify_result(&result, expected_count).is_ok() => {}
            _ => errors += 1,
        }
    }
    samples.sort_unstable();
    LatencyCase {
        iterations,
        request_bytes,
        p50_ns: percentile(&samples, 50),
        p95_ns: percentile(&samples, 95),
        max_ns: samples.last().copied().unwrap_or(1),
        mean_ns: samples.iter().map(|value| *value as f64).sum::<f64>() / iterations as f64,
        errors,
    }
}

fn measure_concurrent(
    session: Arc<DynamicOperatorHostSession>,
    request: OperatorRunRequest,
    workers: usize,
    iterations_per_worker: usize,
) -> RunnerResult<ConcurrentCase> {
    let barrier = Arc::new(Barrier::new(workers + 1));
    let handles = (0..workers)
        .map(|_| {
            let session = Arc::clone(&session);
            let request = request.clone();
            let barrier = Arc::clone(&barrier);
            thread::spawn(move || {
                barrier.wait();
                (0..iterations_per_worker)
                    .filter(|_| {
                        let result = session.run_operator(request.clone());
                        !matches!(result, Ok(result) if verify_result(&result, 8).is_ok())
                    })
                    .count()
            })
        })
        .collect::<Vec<_>>();
    let started = Instant::now();
    barrier.wait();
    let errors = handles
        .into_iter()
        .map(|handle| {
            handle
                .join()
                .map_err(|_| "benchmark worker panicked".to_string())
        })
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .sum();
    let elapsed_ns = elapsed_ns(started);
    let total_dispatches = workers * iterations_per_worker;
    Ok(ConcurrentCase {
        workers,
        total_dispatches,
        elapsed_ns,
        dispatches_per_second: total_dispatches as f64 * 1_000_000_000.0 / elapsed_ns as f64,
        errors,
    })
}

fn verify_result(
    result: &kyuubiki_protocol::OperatorRunResult,
    expected_count: usize,
) -> RunnerResult<()> {
    black_box(result);
    if result.summary["count"].as_u64() == Some(expected_count as u64) {
        Ok(())
    } else {
        Err("benchmark operator returned an invalid summary".to_string())
    }
}

fn metrics_pass(metrics: &Metrics, thresholds: &Thresholds) -> bool {
    metrics.activation.cold_ns <= thresholds.cold_activation_max_ns
        && metrics.activation.resident_p95_ns <= thresholds.resident_activation_p95_max_ns
        && metrics.first_dispatch_ns <= thresholds.first_dispatch_max_ns
        && metrics.compact.p95_ns <= thresholds.compact_p95_max_ns
        && metrics.medium.p95_ns <= thresholds.medium_p95_max_ns
        && metrics.concurrent.dispatches_per_second
            >= thresholds.concurrent_min_dispatches_per_second
        && metrics.activation.errors
            + metrics.compact.errors
            + metrics.medium.errors
            + metrics.concurrent.errors
            == 0
}

fn percentile(samples: &[u64], percentile: usize) -> u64 {
    let index = ((samples.len() * percentile).div_ceil(100)).saturating_sub(1);
    samples.get(index).copied().unwrap_or(1)
}

fn elapsed_ns(started: Instant) -> u64 {
    u64::try_from(started.elapsed().as_nanos())
        .unwrap_or(u64::MAX)
        .max(1)
}

#[derive(Default)]
struct Options {
    help: bool,
    output: Option<String>,
}

impl Options {
    fn parse(args: Vec<OsString>) -> RunnerResult<Self> {
        let mut options = Self::default();
        let mut args = args.into_iter();
        while let Some(arg) = args.next() {
            match arg.to_string_lossy().as_ref() {
                "--help" | "-h" => options.help = true,
                "--out" => options.output = Some(relative_arg(&mut args, "--out")?),
                other => return Err(format!("unknown operator SDK benchmark argument: {other}")),
            }
        }
        Ok(options)
    }
}

#[derive(Default)]
struct CheckOptions {
    help: bool,
    self_test: bool,
    report: Option<String>,
}

impl CheckOptions {
    fn parse(args: Vec<OsString>) -> RunnerResult<Self> {
        let mut options = Self::default();
        let mut args = args.into_iter();
        while let Some(arg) = args.next() {
            match arg.to_string_lossy().as_ref() {
                "--help" | "-h" => options.help = true,
                "--self-test" => options.self_test = true,
                "--verify-report" => {
                    options.report = Some(relative_arg(&mut args, "--verify-report")?)
                }
                other => {
                    return Err(format!(
                        "unknown operator SDK benchmark check argument: {other}"
                    ));
                }
            }
        }
        Ok(options)
    }
}

fn relative_arg(args: &mut impl Iterator<Item = OsString>, flag: &str) -> RunnerResult<String> {
    let value = args
        .next()
        .map(|value| value.to_string_lossy().to_string())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("{flag} requires a repository-relative path"))?;
    let path = PathBuf::from(&value);
    if path.is_absolute()
        || value.contains('\\')
        || path.components().any(|part| part.as_os_str() == "..")
    {
        return Err(format!(
            "{flag} must be a portable repository-relative path"
        ));
    }
    Ok(value)
}
