use super::report::LeasePhase;
use crate::qualification_support::{generated_at_unix_ms, read_json, repo_path};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

type RunnerResult<T> = Result<T, String>;

pub(crate) const CONTRACT_PATH: &str =
    "config/architecture/orchestra-installed-takeover-operational-qualification.json";
pub(crate) const CONTRACT_SCHEMA: &str =
    "kyuubiki.orchestra-installed-takeover-operational-qualification-contract/v1";
pub(crate) const REPORT_SCHEMA: &str =
    "kyuubiki.orchestra-installed-takeover-operational-qualification/v1";
pub(crate) const QUALIFICATION_ID: &str =
    "installer-managed-two-orchestra-postgresql-crash-takeover-operational";
pub(crate) const JOURNEY: &str =
    "installed-production-release-two-orchestra-postgresql-sigkill-takeover";
pub(crate) const DEFAULT_REPORT: &str = "releases/usability-evidence/2.15.0/orchestra-installed-takeover-operational-qualification.json";
pub(crate) const DEFAULT_CAPTURE: &str =
    "tmp/orchestra-installed-takeover-operational-qualification.json";

const REQUIRED_CHECKS: &[&str] = &[
    "sealed_runtime_payload",
    "installer_activation_observed",
    "production_otp_release",
    "source_tree_detached",
    "remote_postgresql_ready",
    "primary_owner_elected",
    "second_orchestra_standby",
    "owner_sigkill_injected",
    "owner_endpoint_closed",
    "standby_promoted",
    "fencing_token_incremented",
    "former_owner_identity_fenced",
    "immutable_payload_shared",
    "managed_state_isolated",
    "cleanup_complete",
    "retention_sanitized",
];

#[derive(Debug)]
pub(crate) struct InstallationEvidence {
    pub(crate) package_version: String,
    pub(crate) architecture: String,
    pub(crate) activation_generation: u64,
    pub(crate) payload_manifest_sha256: String,
    pub(crate) service_manifest_sha256: String,
    pub(crate) orchestra_executable_sha256: String,
    pub(crate) source_tree_detached: bool,
}

#[derive(Debug)]
pub(crate) struct JourneyEvidence {
    pub(crate) installation: InstallationEvidence,
    pub(crate) initial_owner: LeasePhase,
    pub(crate) initial_standby: LeasePhase,
    pub(crate) takeover: LeasePhase,
    pub(crate) former_owner_rejoin: LeasePhase,
    pub(crate) takeover_elapsed_ms: u128,
    pub(crate) primary_endpoint_closed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct CleanupEvidence {
    pub(crate) orchestra_processes_stopped: bool,
    pub(crate) orchestra_ports_closed: bool,
    pub(crate) remote_database_removed: bool,
    pub(crate) runtime_store_removed: bool,
    pub(crate) managed_run_root_removed: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct Report {
    schema_version: String,
    generated_at_unix_ms: u128,
    status: String,
    qualification_id: String,
    journey: String,
    installation: InstallationReport,
    topology: TopologyEvidence,
    lease_policy: LeasePolicyEvidence,
    phases: PhaseEvidence,
    cleanup: CleanupEvidence,
    checks: Vec<CheckEvidence>,
}

#[derive(Debug, Serialize, Deserialize)]
struct InstallationReport {
    package_version: String,
    platform: String,
    architecture: String,
    runtime_origin: String,
    activation_generation: u64,
    payload_manifest_sha256: String,
    service_manifest_sha256: String,
    orchestra_executable_sha256: String,
    source_tree_detached: bool,
    source_fallback: bool,
    immutable_payload_shared: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct TopologyEvidence {
    orchestra_host_role: String,
    database_host_role: String,
    orchestra_process_count: u64,
    database: String,
    database_ephemeral: bool,
    database_loopback_only: bool,
    transport: String,
    build_profile: String,
    managed_state_per_instance: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct LeasePolicyEvidence {
    lease_ttl_ms: u64,
    heartbeat_ms: u64,
    retry_ms: u64,
    failure_mode: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct PhaseEvidence {
    initial_owner: LeasePhase,
    initial_standby: LeasePhase,
    takeover: LeasePhase,
    former_owner_rejoin: LeasePhase,
    takeover_elapsed_ms: u128,
    primary_endpoint_closed: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct CheckEvidence {
    id: String,
    status: String,
}

#[derive(Debug, Deserialize)]
struct Contract {
    schema_version: String,
    qualification_id: String,
    target_coordinate: TargetCoordinate,
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
struct CaptureContract {
    package_version: String,
    orchestra_host_role: String,
    database_host_role: String,
    platform: String,
    architecture: String,
    database: String,
    transport: String,
    build_profile: String,
    failure_mode: String,
    orchestra_processes_minimum: u64,
    lease_ttl_ms_maximum: u64,
    fencing_increment_minimum: u64,
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

pub(crate) fn build_report(
    journey: JourneyEvidence,
    cleanup: CleanupEvidence,
) -> RunnerResult<Report> {
    Ok(Report {
        schema_version: REPORT_SCHEMA.to_string(),
        generated_at_unix_ms: generated_at_unix_ms()?,
        status: "pass".to_string(),
        qualification_id: QUALIFICATION_ID.to_string(),
        journey: JOURNEY.to_string(),
        installation: InstallationReport {
            package_version: journey.installation.package_version,
            platform: "linux".to_string(),
            architecture: journey.installation.architecture,
            runtime_origin: "installer-managed".to_string(),
            activation_generation: journey.installation.activation_generation,
            payload_manifest_sha256: journey.installation.payload_manifest_sha256,
            service_manifest_sha256: journey.installation.service_manifest_sha256,
            orchestra_executable_sha256: journey.installation.orchestra_executable_sha256,
            source_tree_detached: journey.installation.source_tree_detached,
            source_fallback: false,
            immutable_payload_shared: true,
        },
        topology: TopologyEvidence {
            orchestra_host_role: "remote-linux-qualification-host".to_string(),
            database_host_role: "remote-linux-qualification-host".to_string(),
            orchestra_process_count: 2,
            database: "postgresql".to_string(),
            database_ephemeral: true,
            database_loopback_only: true,
            transport: "local-loopback".to_string(),
            build_profile: "production-otp-release".to_string(),
            managed_state_per_instance: true,
        },
        lease_policy: LeasePolicyEvidence {
            lease_ttl_ms: super::runtime::LEASE_TTL_MS,
            heartbeat_ms: super::runtime::HEARTBEAT_MS,
            retry_ms: super::runtime::RETRY_MS,
            failure_mode: "sigkill".to_string(),
        },
        phases: PhaseEvidence {
            initial_owner: journey.initial_owner,
            initial_standby: journey.initial_standby,
            takeover: journey.takeover,
            former_owner_rejoin: journey.former_owner_rejoin,
            takeover_elapsed_ms: journey.takeover_elapsed_ms,
            primary_endpoint_closed: journey.primary_endpoint_closed,
        },
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

pub(crate) fn read_path(path: &Path) -> RunnerResult<Report> {
    let bytes =
        fs::read(path).map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    serde_json::from_slice(&bytes)
        .map_err(|error| format!("failed to parse {}: {error}", path.display()))
}

pub(crate) fn write_path(path: &Path, report: &Report) -> RunnerResult<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
    }
    let bytes = serde_json::to_vec_pretty(report).map_err(|error| error.to_string())?;
    fs::write(path, bytes).map_err(|error| format!("failed to write {}: {error}", path.display()))
}

pub(crate) fn validate_contract(root: &Path, require_report: bool) -> RunnerResult<()> {
    let contract: Contract = read_json(root, CONTRACT_PATH)?;
    if contract.schema_version != CONTRACT_SCHEMA
        || contract.qualification_id != QUALIFICATION_ID
        || contract.target_coordinate.module_id != "orchestra-control-plane"
        || contract.target_coordinate.paradigm != "runtime_api"
        || contract.target_coordinate.target_grade != "operational"
    {
        return Err("installed Orchestra takeover contract identity is invalid".to_string());
    }
    let capture = &contract.capture;
    if capture.package_version != "2.15.0"
        || capture.orchestra_host_role != "remote-linux-qualification-host"
        || capture.database_host_role != "remote-linux-qualification-host"
        || capture.platform != "linux"
        || capture.architecture != "x86_64"
        || capture.database != "postgresql"
        || capture.transport != "local-loopback"
        || capture.build_profile != "production-otp-release"
        || capture.failure_mode != "sigkill"
        || capture.orchestra_processes_minimum < 2
        || capture.lease_ttl_ms_maximum > 2_000
        || capture.fencing_increment_minimum < 1
        || !capture.cleanup_required
    {
        return Err("installed Orchestra takeover capture thresholds are invalid".to_string());
    }
    require_exact_set(
        contract.required_checks.iter().map(String::as_str),
        REQUIRED_CHECKS.iter().copied(),
        "contract required checks",
    )?;
    if contract.retention.report_schema != REPORT_SCHEMA
        || contract.retention.report_path != DEFAULT_REPORT
        || contract.retention.forbidden_content.len() < 6
    {
        return Err("installed Orchestra takeover retention contract is invalid".to_string());
    }
    validate_schema_const(root, &contract.retention.report_schema_path, REPORT_SCHEMA)?;
    validate_schema_const(
        root,
        "schemas/orchestra-installed-takeover-operational-qualification-contract.schema.json",
        CONTRACT_SCHEMA,
    )?;
    validate_source_guard(root, &contract.source_guard)?;
    if require_report && !root.join(DEFAULT_REPORT).is_file() {
        return Err(format!("missing retained report {DEFAULT_REPORT}"));
    }
    Ok(())
}

pub(crate) fn validate(root: &Path, report: &Report) -> RunnerResult<()> {
    let contract: Contract = read_json(root, CONTRACT_PATH)?;
    validate_semantics(report, &contract.capture)?;
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

pub(crate) fn validate_host_capture(report: &Report, package_version: &str) -> RunnerResult<()> {
    let capture = CaptureContract {
        package_version: package_version.to_string(),
        orchestra_host_role: "remote-linux-qualification-host".to_string(),
        database_host_role: "remote-linux-qualification-host".to_string(),
        platform: "linux".to_string(),
        architecture: std::env::consts::ARCH.to_string(),
        database: "postgresql".to_string(),
        transport: "local-loopback".to_string(),
        build_profile: "production-otp-release".to_string(),
        failure_mode: "sigkill".to_string(),
        orchestra_processes_minimum: 2,
        lease_ttl_ms_maximum: 2_000,
        fencing_increment_minimum: 1,
        cleanup_required: true,
    };
    validate_semantics(report, &capture)
}

fn validate_semantics(report: &Report, capture: &CaptureContract) -> RunnerResult<()> {
    if report.schema_version != REPORT_SCHEMA
        || report.status != "pass"
        || report.qualification_id != QUALIFICATION_ID
        || report.journey != JOURNEY
        || report.generated_at_unix_ms == 0
    {
        return Err("installed Orchestra takeover report identity is invalid".to_string());
    }
    let install = &report.installation;
    if install.package_version != capture.package_version
        || install.platform != capture.platform
        || install.architecture != capture.architecture
        || install.runtime_origin != "installer-managed"
        || install.activation_generation == 0
        || !valid_digest(&install.payload_manifest_sha256)
        || !valid_digest(&install.service_manifest_sha256)
        || !valid_digest(&install.orchestra_executable_sha256)
        || !install.source_tree_detached
        || install.source_fallback
        || !install.immutable_payload_shared
    {
        return Err("installed Orchestra takeover installation evidence is invalid".to_string());
    }
    let topology = &report.topology;
    if topology.orchestra_host_role != capture.orchestra_host_role
        || topology.database_host_role != capture.database_host_role
        || topology.orchestra_process_count < capture.orchestra_processes_minimum
        || topology.database != capture.database
        || !topology.database_ephemeral
        || !topology.database_loopback_only
        || topology.transport != capture.transport
        || topology.build_profile != capture.build_profile
        || !topology.managed_state_per_instance
    {
        return Err("installed Orchestra takeover topology is invalid".to_string());
    }
    if report.lease_policy.lease_ttl_ms > capture.lease_ttl_ms_maximum
        || report.lease_policy.heartbeat_ms >= report.lease_policy.lease_ttl_ms
        || report.lease_policy.retry_ms >= report.lease_policy.lease_ttl_ms
        || report.lease_policy.failure_mode != capture.failure_mode
    {
        return Err("installed Orchestra takeover lease policy is invalid".to_string());
    }
    validate_phases(&report.phases, capture.fencing_increment_minimum)?;
    if capture.cleanup_required && !cleanup_complete(&report.cleanup) {
        return Err("installed Orchestra takeover cleanup is incomplete".to_string());
    }
    require_exact_set(
        report.checks.iter().map(|check| check.id.as_str()),
        REQUIRED_CHECKS.iter().copied(),
        "report checks",
    )?;
    if report.checks.iter().any(|check| check.status != "pass") {
        return Err("installed Orchestra takeover report contains a failed check".to_string());
    }
    Ok(())
}

fn validate_phases(phases: &PhaseEvidence, fencing_increment: u64) -> RunnerResult<()> {
    validate_phase(&phases.initial_owner, "primary", "owner", "primary")?;
    validate_phase(&phases.initial_standby, "standby", "standby", "primary")?;
    validate_phase(&phases.takeover, "standby", "owner", "standby")?;
    validate_phase(
        &phases.former_owner_rejoin,
        "former-owner",
        "standby",
        "standby",
    )?;
    let initial = phases.initial_owner.fencing_token;
    let takeover = phases.takeover.fencing_token;
    if initial == 0
        || phases.initial_standby.fencing_token != initial
        || takeover < initial.saturating_add(fencing_increment)
        || phases.former_owner_rejoin.fencing_token != takeover
        || !phases.primary_endpoint_closed
        || phases.takeover_elapsed_ms == 0
        || phases.takeover_elapsed_ms > 30_000
    {
        return Err("installed Orchestra takeover fencing sequence is invalid".to_string());
    }
    Ok(())
}

fn validate_phase(
    phase: &LeasePhase,
    process_role: &str,
    lease_status: &str,
    owner_role: &str,
) -> RunnerResult<()> {
    if phase.process_role != process_role
        || phase.lease_status != lease_status
        || phase.observed_owner_role != owner_role
        || phase.fencing_token == 0
    {
        return Err(format!(
            "invalid installed Orchestra lease phase {process_role}"
        ));
    }
    Ok(())
}

fn cleanup_complete(cleanup: &CleanupEvidence) -> bool {
    cleanup.orchestra_processes_stopped
        && cleanup.orchestra_ports_closed
        && cleanup.remote_database_removed
        && cleanup.runtime_store_removed
        && cleanup.managed_run_root_removed
}

fn valid_digest(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn validate_schema_const(root: &Path, relative: &str, expected: &str) -> RunnerResult<()> {
    let schema: Value = read_json(root, relative)?;
    if schema
        .pointer("/properties/schema_version/const")
        .and_then(Value::as_str)
        != Some(expected)
    {
        return Err(format!("schema {relative} does not enforce {expected}"));
    }
    Ok(())
}

fn validate_source_guard(root: &Path, guard: &SourceGuard) -> RunnerResult<()> {
    let mut source = String::new();
    for relative in &guard.files {
        source.push_str(
            &fs::read_to_string(repo_path(root, relative)?)
                .map_err(|error| format!("failed to read source guard {relative}: {error}"))?,
        );
    }
    for required in &guard.required_text {
        if !source.contains(required) {
            return Err(format!(
                "installed Orchestra takeover source guard misses {required}"
            ));
        }
    }
    Ok(())
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
        Err(format!("{label} do not match the qualification contract"))
    }
}

pub(crate) fn validator_self_test(root: &Path) -> RunnerResult<()> {
    let install = InstallationEvidence {
        package_version: "2.15.0".to_string(),
        architecture: "x86_64".to_string(),
        activation_generation: 1,
        payload_manifest_sha256: "a".repeat(64),
        service_manifest_sha256: "b".repeat(64),
        orchestra_executable_sha256: "c".repeat(64),
        source_tree_detached: true,
    };
    let journey = JourneyEvidence {
        installation: install,
        initial_owner: phase("primary", "owner", "primary", 1),
        initial_standby: phase("standby", "standby", "primary", 1),
        takeover: phase("standby", "owner", "standby", 2),
        former_owner_rejoin: phase("former-owner", "standby", "standby", 2),
        takeover_elapsed_ms: 1_700,
        primary_endpoint_closed: true,
    };
    let cleanup = CleanupEvidence {
        orchestra_processes_stopped: true,
        orchestra_ports_closed: true,
        remote_database_removed: true,
        runtime_store_removed: true,
        managed_run_root_removed: true,
    };
    let mut report = build_report(journey, cleanup)?;
    validate(root, &report)?;
    report.phases.takeover.fencing_token = 1;
    if validate(root, &report).is_ok() {
        return Err("validator accepted a non-incrementing fencing token".to_string());
    }
    report.phases.takeover.fencing_token = 2;
    report.installation.source_tree_detached = false;
    if validate(root, &report).is_ok() {
        return Err("validator accepted an attached source tree".to_string());
    }
    report.installation.source_tree_detached = true;
    report.cleanup.runtime_store_removed = false;
    if validate(root, &report).is_ok() {
        return Err("validator accepted a retained Runtime store".to_string());
    }
    Ok(())
}

fn phase(process: &str, status: &str, owner: &str, fencing_token: u64) -> LeasePhase {
    LeasePhase {
        process_role: process.to_string(),
        lease_status: status.to_string(),
        observed_owner_role: owner.to_string(),
        fencing_token,
    }
}
