use super::distribution::{DistributionEvidence, PACKAGE_ID, PACKAGE_VERSION, TARGET};
use super::installed::InstallationEvidence;
use crate::qualification_support::{generated_at_unix_ms, read_json, write_json};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

type RunnerResult<T> = Result<T, String>;

pub(crate) const CONTRACT_PATH: &str =
    "config/architecture/operator-package-acquisition-operational-qualification.json";
pub(crate) const CONTRACT_SCHEMA: &str =
    "kyuubiki.operator-package-acquisition-operational-qualification-contract/v1";
pub(crate) const REPORT_SCHEMA: &str =
    "kyuubiki.operator-package-acquisition-operational-qualification/v1";
pub(crate) const QUALIFICATION_ID: &str =
    "two-host-installer-managed-orchestra-package-acquisition";
pub(crate) const JOURNEY: &str = "orchestra-dispatch-fetch-execute-evict-refetch";
pub(crate) const DEFAULT_REPORT: &str = "releases/usability-evidence/2.19.0/operator-package-acquisition-operational-qualification.json";
pub(crate) const DEFAULT_CAPTURE: &str =
    "tmp/operator-package-acquisition-operational-qualification.json";

pub(crate) const REQUIRED_CHECKS: &[&str] = &[
    "two_physical_hosts",
    "real_elixir_orchestra",
    "installer_managed_rust_agent",
    "central_single_source_copy",
    "remote_build_artifact_removed",
    "empty_agent_cache_before_dispatch",
    "orchestra_http_task_dispatch",
    "authenticated_resolution_download",
    "manifest_and_entrypoint_integrity",
    "dynamic_operator_execution",
    "disposable_cache_eviction",
    "second_dispatch_refetched",
    "zero_active_packages_after_execution",
    "cleanup_complete",
    "retention_sanitized",
];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct AgentEvidence {
    pub(crate) program: String,
    pub(crate) package_runtime_ready: bool,
    pub(crate) activated_package_count: u64,
    pub(crate) control_link_state: String,
    pub(crate) successful_registration_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ExecutionEvidence {
    pub(crate) task_id: String,
    pub(crate) status: String,
    pub(crate) origin: String,
    pub(crate) cache_status: String,
    pub(crate) package_id: String,
    pub(crate) package_version: String,
    pub(crate) entrypoint_sha256: String,
    pub(crate) integrity_verified: bool,
    pub(crate) result_sum: f64,
    pub(crate) eviction_disposition: String,
    pub(crate) remaining_activated_package_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct CentralRequestEvidence {
    pub(crate) resolution_count: u64,
    pub(crate) manifest_count: u64,
    pub(crate) entrypoint_count: u64,
    pub(crate) successful_sequence_count: u64,
    pub(crate) protected_reads: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct CleanupEvidence {
    pub(crate) local_orchestra_stopped: bool,
    pub(crate) remote_agent_stopped: bool,
    pub(crate) local_port_closed: bool,
    pub(crate) remote_port_closed: bool,
    pub(crate) managed_remote_root_removed: bool,
    pub(crate) local_work_root_removed: bool,
    pub(crate) secret_files_removed: bool,
}

#[derive(Debug)]
pub(crate) struct JourneyEvidence {
    pub(crate) remote_architecture: String,
    pub(crate) installation: InstallationEvidence,
    pub(crate) distribution: DistributionEvidence,
    pub(crate) remote_operator_artifact_absent: bool,
    pub(crate) initial_agent: AgentEvidence,
    pub(crate) final_agent: AgentEvidence,
    pub(crate) executions: Vec<ExecutionEvidence>,
    pub(crate) central_requests: CentralRequestEvidence,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct Report {
    schema_version: String,
    generated_at_unix_ms: u128,
    status: String,
    qualification_id: String,
    journey: String,
    topology: TopologyEvidence,
    installation: InstallationEvidence,
    distribution: DistributionEvidence,
    remote_operator_artifact_absent: bool,
    initial_agent: AgentEvidence,
    final_agent: AgentEvidence,
    executions: Vec<ExecutionEvidence>,
    central_requests: CentralRequestEvidence,
    cleanup: CleanupEvidence,
    checks: Vec<CheckEvidence>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TopologyEvidence {
    orchestrator_host_role: String,
    agent_host_role: String,
    orchestrator_platform: String,
    agent_platform: String,
    agent_architecture: String,
    transport: String,
    control_plane_runtime: String,
    execution_runtime: String,
    deployment_owner: String,
    build_profile: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CheckEvidence {
    id: String,
    status: String,
}

#[derive(Debug, Deserialize)]
struct Contract {
    schema_version: String,
    qualification_id: String,
    target_coordinate: TargetCoordinate,
    ownership: Ownership,
    capture: CaptureContract,
    retention: RetentionContract,
    source_guard: SourceGuard,
    required_checks: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct TargetCoordinate {
    module_id: String,
    paradigm: String,
    target_grade: String,
}

#[derive(Debug, Deserialize)]
struct Ownership {
    control_plane_owner: String,
    execution_owner: String,
    deployment_owner: String,
    evidence_owner: String,
}

#[derive(Debug, Deserialize)]
struct CaptureContract {
    orchestrator_host_role: String,
    agent_host_role: String,
    orchestrator_platform: String,
    agent_platform: String,
    transport: String,
    build_profile: String,
    central_source_copy_count: u64,
    execution_count: u64,
    fetch_sequence_count: u64,
    cleanup_required: bool,
}

#[derive(Debug, Deserialize)]
struct RetentionContract {
    report_schema: String,
    report_schema_path: String,
    report_path: String,
    forbidden_content: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct SourceGuard {
    files: Vec<String>,
    required_text: Vec<String>,
}

pub(crate) fn build(journey: JourneyEvidence, cleanup: CleanupEvidence) -> RunnerResult<Report> {
    Ok(Report {
        schema_version: REPORT_SCHEMA.to_string(),
        generated_at_unix_ms: generated_at_unix_ms()?,
        status: "pass".to_string(),
        qualification_id: QUALIFICATION_ID.to_string(),
        journey: JOURNEY.to_string(),
        topology: TopologyEvidence {
            orchestrator_host_role: "local-macos-qualification-host".to_string(),
            agent_host_role: "remote-linux-qualification-host".to_string(),
            orchestrator_platform: "macos".to_string(),
            agent_platform: "linux".to_string(),
            agent_architecture: journey.remote_architecture,
            transport: "lan-http-plus-agent-rpc".to_string(),
            control_plane_runtime: "elixir-orchestra".to_string(),
            execution_runtime: "installer-managed-rust-agent".to_string(),
            deployment_owner: "kyuubiki-installer".to_string(),
            build_profile: "release".to_string(),
        },
        installation: journey.installation,
        distribution: journey.distribution,
        remote_operator_artifact_absent: journey.remote_operator_artifact_absent,
        initial_agent: journey.initial_agent,
        final_agent: journey.final_agent,
        executions: journey.executions,
        central_requests: journey.central_requests,
        cleanup,
        checks: REQUIRED_CHECKS
            .iter()
            .map(|id| CheckEvidence {
                id: (*id).to_string(),
                status: "pass".to_string(),
            })
            .collect(),
    })
}

pub(crate) fn write(root: &Path, relative: &str, report: &Report) -> RunnerResult<()> {
    write_json(root, relative, report)
}

pub(crate) fn read(root: &Path, relative: &str) -> RunnerResult<Report> {
    read_json(root, relative)
}

pub(crate) fn validate_contract(root: &Path, require_report: bool) -> RunnerResult<()> {
    let contract: Contract = read_json(root, CONTRACT_PATH)?;
    if contract.schema_version != CONTRACT_SCHEMA
        || contract.qualification_id != QUALIFICATION_ID
        || contract.target_coordinate.module_id != "runtime-agent-cli"
        || contract.target_coordinate.paradigm != "sdk_operator"
        || contract.target_coordinate.target_grade != "operational"
    {
        return Err("operator package acquisition contract identity is invalid".to_string());
    }
    if contract.ownership.control_plane_owner != "orchestra-control-plane"
        || contract.ownership.execution_owner != "runtime-agent-cli"
        || contract.ownership.deployment_owner != "runtime-installer"
        || contract.ownership.evidence_owner != "verification-evidence"
    {
        return Err("operator package acquisition ownership is invalid".to_string());
    }
    let capture = &contract.capture;
    if capture.orchestrator_host_role != "local-macos-qualification-host"
        || capture.agent_host_role != "remote-linux-qualification-host"
        || capture.orchestrator_platform != "macos"
        || capture.agent_platform != "linux"
        || capture.transport != "lan-http-plus-agent-rpc"
        || capture.build_profile != "release"
        || capture.central_source_copy_count != 1
        || capture.execution_count != 2
        || capture.fetch_sequence_count != 2
        || !capture.cleanup_required
    {
        return Err("operator package acquisition capture thresholds are invalid".to_string());
    }
    require_exact_set(
        contract.required_checks.iter().map(String::as_str),
        REQUIRED_CHECKS.iter().copied(),
        "contract checks",
    )?;
    if contract.retention.report_schema != REPORT_SCHEMA
        || contract.retention.report_path != DEFAULT_REPORT
        || contract.retention.forbidden_content.len() < 7
    {
        return Err("operator package acquisition retention identity is invalid".to_string());
    }
    validate_schema_const(root, &contract.retention.report_schema_path, REPORT_SCHEMA)?;
    validate_schema_const(
        root,
        "schemas/operator-package-acquisition-operational-qualification-contract.schema.json",
        CONTRACT_SCHEMA,
    )?;
    let mut guarded = String::new();
    for relative in &contract.source_guard.files {
        guarded.push_str(
            &fs::read_to_string(root.join(relative))
                .map_err(|error| format!("failed to read guarded source {relative}: {error}"))?,
        );
    }
    for required in &contract.source_guard.required_text {
        if !guarded.contains(required) {
            return Err(format!(
                "operator package acquisition source guard misses {required}"
            ));
        }
    }
    if require_report && !root.join(DEFAULT_REPORT).is_file() {
        return Err(format!("missing retained report {DEFAULT_REPORT}"));
    }
    Ok(())
}

pub(crate) fn validate(root: &Path, report: &Report) -> RunnerResult<()> {
    let contract: Contract = read_json(root, CONTRACT_PATH)?;
    if report.schema_version != REPORT_SCHEMA
        || report.status != "pass"
        || report.qualification_id != QUALIFICATION_ID
        || report.journey != JOURNEY
        || report.generated_at_unix_ms == 0
    {
        return Err("operator package acquisition report identity is invalid".to_string());
    }
    validate_topology(&report.topology)?;
    validate_installation(&report.installation)?;
    validate_distribution(&report.distribution)?;
    if !report.remote_operator_artifact_absent {
        return Err("remote host retained a preloaded operator artifact".to_string());
    }
    validate_agent(&report.initial_agent, 0, "initial Agent")?;
    validate_agent(&report.final_agent, 0, "final Agent")?;
    validate_executions(&report.executions, &report.distribution)?;
    validate_requests(&report.central_requests)?;
    if !cleanup_complete(&report.cleanup) {
        return Err("operator package acquisition cleanup is incomplete".to_string());
    }
    require_exact_set(
        report.checks.iter().map(|check| check.id.as_str()),
        REQUIRED_CHECKS.iter().copied(),
        "report checks",
    )?;
    if report.checks.iter().any(|check| check.status != "pass") {
        return Err("operator package acquisition report contains a failed check".to_string());
    }
    let rendered = serde_json::to_string(report).map_err(|error| error.to_string())?;
    for forbidden in &contract.retention.forbidden_content {
        if rendered
            .to_ascii_lowercase()
            .contains(&forbidden.to_ascii_lowercase())
        {
            return Err(format!("report retains forbidden content: {forbidden}"));
        }
    }
    Ok(())
}

fn validate_topology(topology: &TopologyEvidence) -> RunnerResult<()> {
    if topology.orchestrator_host_role != "local-macos-qualification-host"
        || topology.agent_host_role != "remote-linux-qualification-host"
        || topology.orchestrator_platform != "macos"
        || topology.agent_platform != "linux"
        || topology.agent_architecture.is_empty()
        || topology.transport != "lan-http-plus-agent-rpc"
        || topology.control_plane_runtime != "elixir-orchestra"
        || topology.execution_runtime != "installer-managed-rust-agent"
        || topology.deployment_owner != "kyuubiki-installer"
        || topology.build_profile != "release"
    {
        Err("operator package acquisition topology is invalid".to_string())
    } else {
        Ok(())
    }
}

fn validate_installation(value: &InstallationEvidence) -> RunnerResult<()> {
    if value.installer_owner != "kyuubiki-installer"
        || value.package_version.is_empty()
        || value.platform != "linux"
        || value.entrypoint_sha256.len() != 64
        || value.entrypoint_size_bytes == 0
        || value.activation_generation == 0
        || value.active_version != value.package_version
        || value.installed_version_count != 1
        || !value.operator_cache_initially_empty
    {
        Err("Installer-managed Agent evidence is invalid".to_string())
    } else {
        Ok(())
    }
}

fn validate_distribution(value: &DistributionEvidence) -> RunnerResult<()> {
    if value.package_id != PACKAGE_ID
        || value.package_version != PACKAGE_VERSION
        || value.target != TARGET
        || value.sdk_api_version != "kyuubiki.operator-sdk/v1"
        || value.execution_abi != "kyuubiki.operator-json-c/v1"
        || value.entrypoint_sha256.len() != 64
        || value.entrypoint_size_bytes == 0
        || value.manifest_sha256.len() != 64
        || value.distribution_sha256.len() != 64
        || value.authority_mode != "bound_orchestra"
        || value.source_copy_count != 1
    {
        Err("central operator distribution evidence is invalid".to_string())
    } else {
        Ok(())
    }
}

fn validate_agent(value: &AgentEvidence, expected_count: u64, label: &str) -> RunnerResult<()> {
    if value.program != "kyuubiki-rust-agent"
        || !value.package_runtime_ready
        || value.activated_package_count != expected_count
        || value.control_link_state != "registered"
        || value.successful_registration_count == 0
    {
        Err(format!("{label} evidence is invalid"))
    } else {
        Ok(())
    }
}

fn validate_executions(
    executions: &[ExecutionEvidence],
    distribution: &DistributionEvidence,
) -> RunnerResult<()> {
    if executions.len() != 2 {
        return Err("qualification must retain exactly two executions".to_string());
    }
    let mut task_ids = BTreeSet::new();
    for execution in executions {
        if execution.task_id.is_empty()
            || !task_ids.insert(execution.task_id.as_str())
            || execution.status != "executed"
            || execution.origin != "bound_orchestra_fetch"
            || execution.cache_status != "fetched_and_activated"
            || execution.package_id != PACKAGE_ID
            || execution.package_version != PACKAGE_VERSION
            || execution.entrypoint_sha256 != distribution.entrypoint_sha256
            || !execution.integrity_verified
            || (execution.result_sum - 14.0).abs() > 1.0e-12
            || execution.eviction_disposition != "evicted_after_execution"
            || execution.remaining_activated_package_count != 0
        {
            return Err("operator package execution evidence is invalid".to_string());
        }
    }
    Ok(())
}

fn validate_requests(value: &CentralRequestEvidence) -> RunnerResult<()> {
    if value.resolution_count != 2
        || value.manifest_count != 2
        || value.entrypoint_count != 2
        || value.successful_sequence_count != 2
        || !value.protected_reads
    {
        Err("central package request evidence is invalid".to_string())
    } else {
        Ok(())
    }
}

fn cleanup_complete(value: &CleanupEvidence) -> bool {
    value.local_orchestra_stopped
        && value.remote_agent_stopped
        && value.local_port_closed
        && value.remote_port_closed
        && value.managed_remote_root_removed
        && value.local_work_root_removed
        && value.secret_files_removed
}

fn validate_schema_const(root: &Path, relative: &str, expected: &str) -> RunnerResult<()> {
    let schema: Value = read_json(root, relative)?;
    if schema
        .pointer("/properties/schema_version/const")
        .and_then(Value::as_str)
        == Some(expected)
    {
        Ok(())
    } else {
        Err(format!("schema {relative} does not bind {expected}"))
    }
}

fn require_exact_set<'a>(
    actual: impl Iterator<Item = &'a str>,
    expected: impl Iterator<Item = &'a str>,
    label: &str,
) -> RunnerResult<()> {
    let actual = actual.collect::<BTreeSet<_>>();
    let expected = expected.collect::<BTreeSet<_>>();
    if actual == expected {
        Ok(())
    } else {
        Err(format!("{label} do not match the required set"))
    }
}

pub(crate) fn validator_self_test(root: &Path) -> RunnerResult<()> {
    let retained = root.join(DEFAULT_REPORT);
    if retained.is_file() {
        let report = read(root, DEFAULT_REPORT)?;
        validate(root, &report)?;
        let mut broken = report.clone();
        broken.central_requests.entrypoint_count = 1;
        if validate(root, &broken).is_ok() {
            return Err("validator accepted an incomplete fetch sequence".to_string());
        }
    }
    Ok(())
}
