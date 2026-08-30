use super::partition_report::{
    CONTRACT_PATH, CONTRACT_SCHEMA, DEFAULT_REPORT, JOURNEY, PartitionJourneyEvidence,
    PartitionedOwnerPhase, QUALIFICATION_ID, REPORT_SCHEMA, REQUIRED_CHECKS, Report,
    TopologyEvidence, build_report, read,
};
use super::report::{
    CleanupEvidence, LeasePhase, cleanup_complete, require_exact_set, validate_phase,
    validate_schema_const,
};
use crate::qualification_support::{read_json, repo_path};
use serde::Deserialize;
use std::fs;
use std::path::Path;

type RunnerResult<T> = Result<T, String>;

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
    orchestra_host_role: String,
    database_host_role: String,
    database_platform: String,
    database: String,
    transport: String,
    failure_mode: String,
    orchestra_processes_minimum: u64,
    independent_database_tunnels_minimum: u64,
    lease_ttl_ms_maximum: u64,
    fail_closed_ms_maximum: u64,
    takeover_ms_maximum: u64,
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

pub(crate) fn validate_contract(root: &Path, require_report: bool) -> RunnerResult<()> {
    let contract: Contract = read_json(root, CONTRACT_PATH)?;
    if contract.schema_version != CONTRACT_SCHEMA
        || contract.qualification_id != QUALIFICATION_ID
        || contract.target_coordinate.module_id != "orchestra-control-plane"
        || contract.target_coordinate.paradigm != "runtime_api"
        || contract.target_coordinate.target_grade != "operational"
    {
        return Err("Orchestra network-partition contract identity is invalid".to_string());
    }
    validate_capture_contract(&contract.capture)?;
    require_exact_set(
        contract.required_checks.iter().map(String::as_str),
        REQUIRED_CHECKS.iter().copied(),
        "partition contract required checks",
    )?;
    if contract.retention.report_schema != REPORT_SCHEMA
        || contract.retention.report_path != DEFAULT_REPORT
        || contract.retention.forbidden_content.len() < 6
    {
        return Err("Orchestra network-partition retention contract is invalid".to_string());
    }
    validate_schema_const(root, &contract.retention.report_schema_path, REPORT_SCHEMA)?;
    validate_schema_const(
        root,
        "schemas/orchestra-network-partition-operational-qualification-contract.schema.json",
        CONTRACT_SCHEMA,
    )?;
    validate_source_guard(root, &contract.source_guard)?;
    if require_report && !root.join(DEFAULT_REPORT).is_file() {
        return Err(format!("missing retained report {DEFAULT_REPORT}"));
    }
    Ok(())
}

fn validate_capture_contract(capture: &CaptureContract) -> RunnerResult<()> {
    if capture.orchestra_host_role != "local-orchestra-qualification-host"
        || capture.database_host_role != "remote-linux-qualification-host"
        || capture.database_platform != "linux"
        || capture.database != "postgresql"
        || capture.transport != "independent-ssh-loopback-tunnels"
        || capture.failure_mode != "primary-database-network-partition"
        || capture.orchestra_processes_minimum < 2
        || capture.independent_database_tunnels_minimum < 2
        || capture.lease_ttl_ms_maximum > 2_000
        || capture.fail_closed_ms_maximum > 10_000
        || capture.takeover_ms_maximum > 30_000
        || capture.fencing_increment_minimum < 1
        || !capture.cleanup_required
    {
        return Err("Orchestra network-partition capture thresholds are invalid".to_string());
    }
    Ok(())
}

pub(crate) fn validate(root: &Path, report: &Report) -> RunnerResult<()> {
    let contract: Contract = read_json(root, CONTRACT_PATH)?;
    if report.schema_version != REPORT_SCHEMA
        || report.status != "pass"
        || report.journey != JOURNEY
        || report.generated_at_unix_ms == 0
    {
        return Err("Orchestra network-partition report identity is invalid".to_string());
    }
    validate_topology(&report.topology)?;
    if report.lease_policy.lease_ttl_ms > contract.capture.lease_ttl_ms_maximum
        || report.lease_policy.heartbeat_ms >= report.lease_policy.lease_ttl_ms
        || report.lease_policy.retry_ms >= report.lease_policy.lease_ttl_ms
        || report.lease_policy.failure_mode != "primary-database-network-partition"
    {
        return Err("Orchestra network-partition lease policy is invalid".to_string());
    }
    validate_phases(&contract.capture, &report.phases)?;
    if !cleanup_complete(&report.cleanup) {
        return Err("Orchestra network-partition cleanup is incomplete".to_string());
    }
    require_exact_set(
        report.checks.iter().map(|check| check.id.as_str()),
        REQUIRED_CHECKS.iter().copied(),
        "partition report checks",
    )?;
    if report.checks.iter().any(|check| check.status != "pass") {
        return Err("Orchestra network-partition report contains a failed check".to_string());
    }
    validate_retention(&contract, report)
}

fn validate_topology(topology: &TopologyEvidence) -> RunnerResult<()> {
    if topology.orchestra_host_role != "local-orchestra-qualification-host"
        || topology.database_host_role != "remote-linux-qualification-host"
        || !matches!(topology.orchestra_platform.as_str(), "macos" | "linux")
        || topology.database_platform != "linux"
        || topology.database_architecture.is_empty()
        || topology.orchestra_process_count < 2
        || topology.database != "postgresql"
        || !topology.database_ephemeral
        || !topology.database_loopback_only
        || topology.transport != "independent-ssh-loopback-tunnels"
        || topology.independent_database_tunnel_count < 2
        || topology.build_profile != "development-no-compile"
    {
        return Err("Orchestra network-partition topology is invalid".to_string());
    }
    Ok(())
}

fn validate_phases(
    contract: &CaptureContract,
    phases: &super::partition_report::PhaseEvidence,
) -> RunnerResult<()> {
    validate_phase(&phases.initial_owner, "primary", "owner", "primary")?;
    validate_phase(&phases.initial_standby, "standby", "standby", "primary")?;
    validate_phase(&phases.takeover, "standby", "owner", "standby")?;
    validate_phase(
        &phases.former_owner_rejoin,
        "former-owner",
        "standby",
        "standby",
    )?;
    let isolated = &phases.partitioned_owner;
    if isolated.process_role != "partitioned-owner"
        || isolated.lease_status != "standby"
        || isolated.observed_owner_role != "none"
        || isolated.visible_fencing_token.is_some()
        || isolated.last_error != "orchestra_lease_store_unavailable"
    {
        return Err("partitioned owner did not fail closed".to_string());
    }
    let initial = phases.initial_owner.fencing_token;
    let takeover = phases.takeover.fencing_token;
    if initial == 0
        || phases.initial_standby.fencing_token != initial
        || takeover < initial.saturating_add(contract.fencing_increment_minimum)
        || phases.former_owner_rejoin.fencing_token != takeover
        || phases.partition_to_fail_closed_elapsed_ms == 0
        || phases.partition_to_fail_closed_elapsed_ms > contract.fail_closed_ms_maximum as u128
        || phases.takeover_elapsed_ms == 0
        || phases.takeover_elapsed_ms > contract.takeover_ms_maximum as u128
        || !phases.primary_process_survived
        || !phases.primary_endpoint_remained_open
        || !phases.isolated_tunnel_closed
        || !phases.standby_tunnel_remained_open
        || !phases.stale_owner_submission_rejected
    {
        return Err("Orchestra network-partition fencing sequence is invalid".to_string());
    }
    Ok(())
}

fn validate_retention(contract: &Contract, report: &Report) -> RunnerResult<()> {
    let rendered = serde_json::to_string(report).map_err(|error| error.to_string())?;
    for forbidden in &contract.retention.forbidden_content {
        if rendered
            .to_ascii_lowercase()
            .contains(&forbidden.to_ascii_lowercase())
        {
            return Err(format!(
                "partition report retains forbidden content: {forbidden}"
            ));
        }
    }
    Ok(())
}

fn validate_source_guard(root: &Path, guard: &SourceGuard) -> RunnerResult<()> {
    let mut source = String::new();
    for relative in &guard.files {
        source.push_str(
            &fs::read_to_string(repo_path(root, relative)?).map_err(|error| {
                format!("failed to read partition source guard {relative}: {error}")
            })?,
        );
    }
    for required in &guard.required_text {
        if !source.contains(required) {
            return Err(format!(
                "Orchestra partition source guard misses {required}"
            ));
        }
    }
    Ok(())
}

pub(crate) fn validator_self_test(root: &Path) -> RunnerResult<()> {
    let mut report = build_report(fixture_journey(), fixture_cleanup())?;
    validate(root, &report)?;
    report.phases.partitioned_owner.visible_fencing_token = Some(1);
    if validate(root, &report).is_ok() {
        return Err("validator accepted cached fencing during partition".to_string());
    }
    report.phases.partitioned_owner.visible_fencing_token = None;
    report.phases.stale_owner_submission_rejected = false;
    if validate(root, &report).is_ok() {
        return Err("validator accepted a stale-owner write".to_string());
    }
    report.phases.stale_owner_submission_rejected = true;
    report.cleanup.remote_database_removed = false;
    if validate(root, &report).is_ok() {
        return Err("validator accepted incomplete partition cleanup".to_string());
    }
    report.cleanup.remote_database_removed = true;
    report.topology.database_architecture = "192.168.1.12".to_string();
    if validate(root, &report).is_ok() {
        return Err("validator accepted retained partition host identity".to_string());
    }
    Ok(())
}

pub(crate) fn read_and_validate(root: &Path, relative: &str) -> RunnerResult<()> {
    let report = read(root, relative)?;
    validate(root, &report)
}

fn fixture_journey() -> PartitionJourneyEvidence {
    PartitionJourneyEvidence {
        database_architecture: "x86_64".to_string(),
        orchestra_platform: "macos".to_string(),
        initial_owner: phase("primary", "owner", "primary", 1),
        initial_standby: phase("standby", "standby", "primary", 1),
        partitioned_owner: PartitionedOwnerPhase {
            process_role: "partitioned-owner".to_string(),
            lease_status: "standby".to_string(),
            observed_owner_role: "none".to_string(),
            visible_fencing_token: None,
            last_error: "orchestra_lease_store_unavailable".to_string(),
        },
        takeover: phase("standby", "owner", "standby", 2),
        former_owner_rejoin: phase("former-owner", "standby", "standby", 2),
        partition_to_fail_closed_elapsed_ms: 600,
        takeover_elapsed_ms: 1_800,
        primary_process_survived: true,
        primary_endpoint_remained_open: true,
        isolated_tunnel_closed: true,
        standby_tunnel_remained_open: true,
        stale_owner_submission_rejected: true,
    }
}

fn fixture_cleanup() -> CleanupEvidence {
    CleanupEvidence {
        orchestra_processes_stopped: true,
        orchestra_ports_closed: true,
        ssh_tunnel_stopped: true,
        tunnel_port_closed: true,
        remote_database_removed: true,
        local_work_root_removed: true,
    }
}

fn phase(process: &str, status: &str, owner: &str, fencing_token: u64) -> LeasePhase {
    LeasePhase {
        process_role: process.to_string(),
        lease_status: status.to_string(),
        observed_owner_role: owner.to_string(),
        fencing_token,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn report_identity_is_distinct_from_sigkill() {
        assert!(REPORT_SCHEMA.contains("network-partition"));
        assert!(JOURNEY.contains("partition"));
        assert_ne!(REPORT_SCHEMA, super::super::report::REPORT_SCHEMA);
    }
}
