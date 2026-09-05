use super::{
    ActivationCase, CONTRACT_PATH, CONTRACT_SCHEMA, ConcurrentCase, LIMITATIONS, LatencyCase,
    Measurement, OptimizationContract, PACKAGE_ID, PackageIdentity, Platform, QUALIFICATION_ID,
    QualificationContract, QualificationReport, REPORT_SCHEMA, Thresholds,
};
use crate::RunnerResult;
use crate::qualification_support::{read_json, repo_path};
use kyuubiki_operator_sdk::OPERATOR_JSON_ABI_SCHEMA_VERSION;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::Path;

const CONTRACT_SCHEMA_PATH: &str =
    "schemas/operator-sdk-performance-qualification-contract.schema.json";
const REPORT_SCHEMA_PATH: &str =
    "schemas/operator-sdk-performance-qualification-report.schema.json";
const REQUIRED_SOURCES: &[&str] = &[
    "config/architecture/operator-sdk-performance-qualification.json",
    "schemas/operator-sdk-performance-qualification-contract.schema.json",
    "schemas/operator-sdk-performance-qualification-report.schema.json",
    "workers/rust/Cargo.lock",
    "workers/rust/Cargo.toml",
    "workers/rust/crates/engine/Cargo.toml",
    "workers/rust/crates/engine/src/operator_sdk_dynamic_abi.rs",
    "workers/rust/crates/engine/src/operator_sdk_host.rs",
    "workers/rust/crates/engine/src/operator_sdk_runtime.rs",
    "workers/rust/crates/operator-sdk/Cargo.toml",
    "workers/rust/crates/operator-sdk/src/json_abi.rs",
    "workers/rust/crates/protocol/Cargo.toml",
    "workers/rust/crates/script-runner/Cargo.toml",
    "workers/rust/crates/script-runner/src/operator_sdk_performance_qualification/mod.rs",
    "workers/rust/crates/script-runner/src/operator_sdk_performance_qualification/validation.rs",
    "workers/rust/templates/operator-crate-template/Cargo.lock",
    "workers/rust/templates/operator-crate-template/Cargo.toml",
    "workers/rust/templates/operator-crate-template/kyuubiki-operator.json",
    "workers/rust/templates/operator-crate-template/src/lib.rs",
];

pub(super) fn load_and_validate_contract(root: &Path) -> RunnerResult<QualificationContract> {
    let contract: QualificationContract = read_json(root, CONTRACT_PATH)?;
    if contract.schema_version != CONTRACT_SCHEMA
        || contract.qualification_id != QUALIFICATION_ID
        || contract.execution_abi != OPERATOR_JSON_ABI_SCHEMA_VERSION
    {
        return Err("operator SDK performance qualification contract identity drifted".into());
    }
    if contract.source_files != strings(REQUIRED_SOURCES) {
        return Err("operator SDK performance qualification source_files drifted".into());
    }
    if contract.report.schema_version != REPORT_SCHEMA
        || contract.report.schema_path != REPORT_SCHEMA_PATH
        || contract.report.default_output != "tmp/operator-sdk-performance-qualification.json"
        || contract.report.retained_path
            != "releases/usability-evidence/2.19.0/operator-sdk-performance-qualification.json"
    {
        return Err("operator SDK performance report policy drifted".into());
    }
    validate_measurement(&contract.measurement)?;
    validate_thresholds(&contract.thresholds)?;
    for path in contract
        .source_files
        .iter()
        .chain([&contract.report.schema_path])
    {
        if !repo_path(root, path)?.is_file() {
            return Err(format!(
                "operator SDK performance source is missing: {path}"
            ));
        }
    }
    validate_schema_const(root, CONTRACT_SCHEMA_PATH, CONTRACT_SCHEMA)?;
    validate_schema_const(root, REPORT_SCHEMA_PATH, REPORT_SCHEMA)?;
    Ok(contract)
}

pub(super) fn validate_report(
    root: &Path,
    contract: &QualificationContract,
    report: &QualificationReport,
) -> RunnerResult<()> {
    if report.schema_version != REPORT_SCHEMA
        || report.qualification_id != QUALIFICATION_ID
        || report.generated_at_unix_ms == 0
        || report.status != "pass"
    {
        return Err("operator SDK performance report identity or status is invalid".into());
    }
    if report.source_tree_sha256 != source_tree_digest(root, &contract.source_files)? {
        return Err("operator SDK performance source tree digest mismatch".into());
    }
    if report.platform.os.is_empty()
        || report.platform.arch.is_empty()
        || report.platform.build_profile != "release"
    {
        return Err("operator SDK performance platform is invalid".into());
    }
    validate_package(&report.package)?;
    if report.measurement != contract.measurement || report.thresholds != contract.thresholds {
        return Err("operator SDK performance measurement contract drifted".into());
    }
    validate_latency(
        &report.metrics.compact,
        report.measurement.compact_iterations,
        report.thresholds.compact_p95_max_ns,
        "compact",
    )?;
    validate_latency(
        &report.metrics.medium,
        report.measurement.medium_iterations,
        report.thresholds.medium_p95_max_ns,
        "medium",
    )?;
    if report.metrics.medium.request_bytes <= report.metrics.compact.request_bytes {
        return Err("operator SDK medium request is not larger than compact request".into());
    }
    validate_activation(
        &report.metrics.activation,
        &report.measurement,
        &report.thresholds,
    )?;
    if report.metrics.first_dispatch_ns > report.thresholds.first_dispatch_max_ns {
        return Err("operator SDK first dispatch exceeded its threshold".into());
    }
    validate_concurrent(
        &report.metrics.concurrent,
        &report.measurement,
        &report.thresholds,
    )?;
    validate_optimization(&report.optimization_contract)?;
    if report.limitations != strings(LIMITATIONS) {
        return Err("operator SDK performance limitations drifted".into());
    }
    let encoded = serde_json::to_string(report)
        .map_err(|error| format!("failed to inspect operator SDK performance report: {error}"))?;
    if encoded.contains("/Users/") || encoded.contains(":\\") {
        return Err("operator SDK performance report leaks an absolute host path".into());
    }
    Ok(())
}

fn validate_package(package: &PackageIdentity) -> RunnerResult<()> {
    if package.package_id != PACKAGE_ID
        || package.operator_id != super::OPERATOR_ID
        || package.execution_abi != OPERATOR_JSON_ABI_SCHEMA_VERSION
    {
        return Err("operator SDK performance package identity drifted".into());
    }
    Ok(())
}

fn validate_latency(
    case: &LatencyCase,
    expected_iterations: usize,
    p95_max_ns: u64,
    label: &str,
) -> RunnerResult<()> {
    if case.iterations != expected_iterations
        || case.request_bytes == 0
        || case.errors != 0
        || case.p50_ns == 0
        || case.p50_ns > case.p95_ns
        || case.p95_ns > case.max_ns
        || case.p95_ns > p95_max_ns
        || !case.mean_ns.is_finite()
        || case.mean_ns <= 0.0
    {
        return Err(format!("operator SDK {label} latency evidence is invalid"));
    }
    Ok(())
}

fn validate_concurrent(
    case: &ConcurrentCase,
    measurement: &Measurement,
    thresholds: &Thresholds,
) -> RunnerResult<()> {
    let expected_total =
        measurement.concurrent_workers * measurement.concurrent_iterations_per_worker;
    if case.workers != measurement.concurrent_workers
        || case.total_dispatches != expected_total
        || case.elapsed_ns == 0
        || case.errors != 0
        || !case.dispatches_per_second.is_finite()
        || case.dispatches_per_second < thresholds.concurrent_min_dispatches_per_second
    {
        return Err("operator SDK concurrent dispatch evidence is invalid".into());
    }
    Ok(())
}

fn validate_activation(
    case: &ActivationCase,
    measurement: &Measurement,
    thresholds: &Thresholds,
) -> RunnerResult<()> {
    if case.cold_ns == 0
        || case.cold_ns > thresholds.cold_activation_max_ns
        || case.resident_iterations != measurement.resident_activation_iterations
        || case.resident_p50_ns == 0
        || case.resident_p50_ns > case.resident_p95_ns
        || case.resident_p95_ns > case.resident_max_ns
        || case.resident_p95_ns > thresholds.resident_activation_p95_max_ns
        || !case.resident_mean_ns.is_finite()
        || case.resident_mean_ns <= 0.0
        || case.errors != 0
    {
        return Err("operator SDK activation evidence is invalid".into());
    }
    Ok(())
}

fn validate_optimization(optimization: &OptimizationContract) -> RunnerResult<()> {
    if optimization.host_response_copy_bytes_per_dispatch != 0
        || optimization.registry_initializations_per_library != 1
        || optimization.response_buffer_release != "same-library-raii"
    {
        return Err("operator SDK optimization contract drifted".into());
    }
    Ok(())
}

fn validate_measurement(measurement: &Measurement) -> RunnerResult<()> {
    if measurement.resident_activation_iterations < 3
        || measurement.warmup_iterations < 10
        || measurement.compact_iterations < 100
        || measurement.medium_iterations < 50
        || measurement.medium_value_count < 1024
        || measurement.concurrent_workers < 2
        || measurement.concurrent_iterations_per_worker < 50
    {
        return Err("operator SDK performance measurement is too weak".into());
    }
    Ok(())
}

fn validate_thresholds(thresholds: &Thresholds) -> RunnerResult<()> {
    if thresholds.cold_activation_max_ns == 0
        || thresholds.resident_activation_p95_max_ns == 0
        || thresholds.first_dispatch_max_ns == 0
        || thresholds.compact_p95_max_ns == 0
        || thresholds.medium_p95_max_ns == 0
        || !thresholds.concurrent_min_dispatches_per_second.is_finite()
        || thresholds.concurrent_min_dispatches_per_second <= 0.0
    {
        return Err("operator SDK performance thresholds are invalid".into());
    }
    Ok(())
}

fn validate_schema_const(root: &Path, path: &str, expected: &str) -> RunnerResult<()> {
    let schema: Value = read_json(root, path)?;
    if schema
        .pointer("/properties/schema_version/const")
        .and_then(Value::as_str)
        != Some(expected)
    {
        return Err(format!("{path} schema_version const mismatch"));
    }
    Ok(())
}

pub(super) fn source_tree_digest(root: &Path, paths: &[String]) -> RunnerResult<String> {
    let mut ordered = paths.to_vec();
    ordered.sort();
    let mut hasher = Sha256::new();
    for path in ordered {
        let bytes = fs::read(repo_path(root, &path)?)
            .map_err(|error| format!("failed to read benchmark source {path}: {error}"))?;
        hasher.update(path.len().to_le_bytes());
        hasher.update(path.as_bytes());
        hasher.update(bytes.len().to_le_bytes());
        hasher.update(bytes);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

pub(super) fn validator_self_test(
    root: &Path,
    contract: &QualificationContract,
) -> RunnerResult<()> {
    let report = QualificationReport {
        schema_version: REPORT_SCHEMA.to_string(),
        qualification_id: QUALIFICATION_ID.to_string(),
        generated_at_unix_ms: 1,
        status: "pass".to_string(),
        source_tree_sha256: source_tree_digest(root, &contract.source_files)?,
        platform: Platform {
            os: "fixture-os".to_string(),
            arch: "fixture-arch".to_string(),
            build_profile: "release".to_string(),
        },
        package: PackageIdentity {
            package_id: PACKAGE_ID.to_string(),
            operator_id: super::OPERATOR_ID.to_string(),
            execution_abi: OPERATOR_JSON_ABI_SCHEMA_VERSION.to_string(),
        },
        measurement: contract.measurement.clone(),
        thresholds: contract.thresholds.clone(),
        metrics: super::Metrics {
            activation: ActivationCase {
                cold_ns: contract.thresholds.cold_activation_max_ns / 2,
                resident_iterations: contract.measurement.resident_activation_iterations,
                resident_p50_ns: 1_000,
                resident_p95_ns: 2_000,
                resident_max_ns: 3_000,
                resident_mean_ns: 1_500.0,
                errors: 0,
            },
            first_dispatch_ns: contract.thresholds.first_dispatch_max_ns / 2,
            compact: latency(contract.measurement.compact_iterations, 512, 1_000),
            medium: latency(contract.measurement.medium_iterations, 32_768, 2_000),
            concurrent: ConcurrentCase {
                workers: contract.measurement.concurrent_workers,
                total_dispatches: contract.measurement.concurrent_workers
                    * contract.measurement.concurrent_iterations_per_worker,
                elapsed_ns: 1_000_000,
                dispatches_per_second: 10_000.0,
                errors: 0,
            },
        },
        optimization_contract: OptimizationContract {
            host_response_copy_bytes_per_dispatch: 0,
            registry_initializations_per_library: 1,
            response_buffer_release: "same-library-raii".to_string(),
        },
        limitations: strings(LIMITATIONS),
    };
    validate_report(root, contract, &report)?;

    let mut copied = report.clone();
    copied
        .optimization_contract
        .host_response_copy_bytes_per_dispatch = 1;
    if validate_report(root, contract, &copied).is_ok() {
        return Err("performance validator accepted a response copy regression".into());
    }
    let mut slow = report.clone();
    slow.metrics.compact.p95_ns = contract.thresholds.compact_p95_max_ns + 1;
    slow.metrics.compact.max_ns = slow.metrics.compact.p95_ns;
    if validate_report(root, contract, &slow).is_ok() {
        return Err("performance validator accepted a latency regression".into());
    }
    let mut slow_activation = report.clone();
    slow_activation.metrics.activation.resident_p95_ns =
        contract.thresholds.resident_activation_p95_max_ns + 1;
    slow_activation.metrics.activation.resident_max_ns =
        slow_activation.metrics.activation.resident_p95_ns;
    if validate_report(root, contract, &slow_activation).is_ok() {
        return Err("performance validator accepted an activation regression".into());
    }
    let mut failing = report;
    failing.metrics.concurrent.errors = 1;
    if validate_report(root, contract, &failing).is_ok() {
        return Err("performance validator accepted a dispatch error".into());
    }
    Ok(())
}

fn latency(iterations: usize, request_bytes: usize, value: u64) -> LatencyCase {
    LatencyCase {
        iterations,
        request_bytes,
        p50_ns: value,
        p95_ns: value,
        max_ns: value,
        mean_ns: value as f64,
        errors: 0,
    }
}

fn strings(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_string()).collect()
}
