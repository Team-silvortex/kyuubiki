use super::long_workflow_report::{
    CONTRACT_PATH, CONTRACT_SCHEMA, DEFAULT_REPORT, JOURNEY, QUALIFICATION_ID, REPORT_SCHEMA,
    REQUIRED_CHECKS, Report, TopologyEvidence, build_report, read,
};
use super::long_workflow_runtime::{
    AgentCounters, CheckpointBlockedEvidence, CleanupEvidence, IdempotentTakeoverEvidence,
    JourneyEvidence, WorkflowObservation,
};
use super::report::{
    CleanupEvidence as OrchestraCleanupEvidence, cleanup_complete, require_exact_set,
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
    required_scenarios: Vec<String>,
    orchestra_processes_minimum: u64,
    lease_ttl_ms_maximum: u64,
    takeover_ms_maximum: u64,
    idempotent_agent_dispatches: u64,
    checkpoint_required_agent_dispatches: u64,
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
        || contract.target_coordinate.paradigm != "fault_injection_and_recovery"
        || contract.target_coordinate.target_grade != "operational"
    {
        return Err("long workflow takeover contract identity is invalid".to_string());
    }
    validate_capture_contract(&contract.capture)?;
    require_exact_set(
        contract.required_checks.iter().map(String::as_str),
        REQUIRED_CHECKS.iter().copied(),
        "long workflow contract checks",
    )?;
    if contract.retention.report_schema != REPORT_SCHEMA
        || contract.retention.report_path != DEFAULT_REPORT
        || contract.retention.forbidden_content.len() < 8
    {
        return Err("long workflow retention contract is invalid".to_string());
    }
    validate_schema_const(root, &contract.retention.report_schema_path, REPORT_SCHEMA)?;
    validate_schema_const(
        root,
        "schemas/orchestra-long-workflow-takeover-operational-qualification-contract.schema.json",
        CONTRACT_SCHEMA,
    )?;
    validate_source_guard(root, &contract.source_guard)?;
    if require_report && !root.join(DEFAULT_REPORT).is_file() {
        return Err(format!("missing retained report {DEFAULT_REPORT}"));
    }
    Ok(())
}

fn validate_capture_contract(capture: &CaptureContract) -> RunnerResult<()> {
    require_exact_set(
        capture.required_scenarios.iter().map(String::as_str),
        ["idempotent_resume", "checkpoint_required_block"].into_iter(),
        "long workflow scenarios",
    )?;
    if capture.orchestra_processes_minimum < 2
        || capture.lease_ttl_ms_maximum > 2_000
        || capture.takeover_ms_maximum > 30_000
        || capture.idempotent_agent_dispatches != 2
        || capture.checkpoint_required_agent_dispatches != 1
        || !capture.cleanup_required
    {
        return Err("long workflow capture thresholds are invalid".to_string());
    }
    Ok(())
}

pub(crate) fn validate(root: &Path, report: &Report) -> RunnerResult<()> {
    let contract: Contract = read_json(root, CONTRACT_PATH)?;
    if report.schema_version != REPORT_SCHEMA
        || report.generated_at_unix_ms == 0
        || report.status != "pass"
        || report.journey != JOURNEY
    {
        return Err("long workflow report identity is invalid".to_string());
    }
    validate_topology(&report.topology)?;
    if report.lease_policy.lease_ttl_ms > contract.capture.lease_ttl_ms_maximum
        || report.lease_policy.heartbeat_ms >= report.lease_policy.lease_ttl_ms
        || report.lease_policy.retry_ms >= report.lease_policy.lease_ttl_ms
        || report.lease_policy.max_workflow_attempts < 2
    {
        return Err("long workflow lease policy is invalid".to_string());
    }
    validate_journey(&contract.capture, &report.evidence)?;
    validate_cleanup(&report.cleanup)?;
    require_exact_set(
        report.checks.iter().map(|check| check.id.as_str()),
        REQUIRED_CHECKS.iter().copied(),
        "long workflow report checks",
    )?;
    if report.checks.iter().any(|check| check.status != "pass") {
        return Err("long workflow report contains a failed check".to_string());
    }
    validate_retention(&contract, report)
}

fn validate_topology(topology: &TopologyEvidence) -> RunnerResult<()> {
    if topology.orchestra_host_role != "local-orchestra-qualification-host"
        || topology.database_host_role != "remote-linux-qualification-host"
        || topology.agent_host_role != "remote-linux-qualification-host"
        || !matches!(topology.orchestra_platform.as_str(), "macos" | "linux")
        || topology.database_platform != "linux"
        || topology.agent_platform != "linux"
        || topology.orchestra_process_count < 2
        || topology.database != "postgresql"
        || !topology.database_ephemeral
        || !topology.database_loopback_only
        || topology.agent_runtime != "kyuubiki-rust-agent"
        || topology.workflow_operator != "solve.bar_1d"
        || topology.transport != "independent-ssh-database-tunnels-plus-agent-tcp"
        || topology.build_profile != "release-agent-development-orchestra"
    {
        return Err("long workflow topology is invalid".to_string());
    }
    Ok(())
}

fn validate_journey(contract: &CaptureContract, evidence: &JourneyEvidence) -> RunnerResult<()> {
    if evidence.database_architecture.is_empty()
        || evidence.agent_architecture.is_empty()
        || evidence.initial_fencing_token == 0
        || evidence.first_takeover_fencing_token <= evidence.initial_fencing_token
        || evidence.second_takeover_fencing_token <= evidence.first_takeover_fencing_token
        || evidence.idempotent.takeover_fencing_token != evidence.first_takeover_fencing_token
        || evidence.checkpoint_required.takeover_fencing_token
            != evidence.second_takeover_fencing_token
        || !evidence.former_primary_rejoined_standby
        || !evidence.former_standby_rejoined_standby
        || !evidence.followup_solver_verified
    {
        return Err("long workflow takeover sequence is invalid".to_string());
    }
    validate_idempotent(contract, &evidence.idempotent)?;
    validate_checkpoint_required(contract, &evidence.checkpoint_required)
}

fn validate_idempotent(
    contract: &CaptureContract,
    evidence: &IdempotentTakeoverEvidence,
) -> RunnerResult<()> {
    validate_observation(
        &evidence.before_loss,
        "solving",
        "running",
        "idempotent",
        1,
        1,
    )?;
    validate_observation(
        &evidence.after_takeover,
        "solving",
        "running",
        "idempotent",
        2,
        2,
    )?;
    validate_observation(
        &evidence.terminal,
        "completed",
        "completed",
        "idempotent",
        2,
        2,
    )?;
    let started_delta = evidence
        .agent_counters_after
        .started
        .saturating_sub(evidence.agent_counters_before.started);
    let finished_delta = evidence
        .agent_counters_after
        .completed
        .saturating_sub(evidence.agent_counters_before.completed)
        + evidence
            .agent_counters_after
            .failed
            .saturating_sub(evidence.agent_counters_before.failed);
    if evidence.takeover_elapsed_ms == 0
        || evidence.takeover_elapsed_ms > contract.takeover_ms_maximum as u128
        || evidence.active_execution_count_before_loss != 1
        || evidence.active_execution_count_after_takeover != 2
        || evidence.agent_started_delta != contract.idempotent_agent_dispatches
        || started_delta != evidence.agent_started_delta
        || finished_delta < contract.idempotent_agent_dispatches
        || evidence.agent_counters_after.active != 0
        || !evidence.initial_claim_seen
        || !evidence.restart_claim_seen
        || evidence.completed_history_count != 1
        || !evidence.result_verified
        || !evidence.terminal_state_stable
    {
        return Err("idempotent long workflow takeover evidence is invalid".to_string());
    }
    Ok(())
}

fn validate_checkpoint_required(
    contract: &CaptureContract,
    evidence: &CheckpointBlockedEvidence,
) -> RunnerResult<()> {
    validate_observation(
        &evidence.before_loss,
        "solving",
        "running",
        "checkpoint_required",
        1,
        1,
    )?;
    validate_observation(
        &evidence.after_takeover,
        "failed",
        "recovery_blocked",
        "checkpoint_required",
        1,
        1,
    )?;
    let started_at_block = evidence
        .agent_counters_at_block
        .started
        .saturating_sub(evidence.agent_counters_before.started);
    if evidence.takeover_elapsed_ms == 0
        || evidence.takeover_elapsed_ms > contract.takeover_ms_maximum as u128
        || evidence.active_execution_count_before_loss != 1
        || evidence.active_execution_count_at_block != 1
        || evidence.agent_started_delta_at_block != contract.checkpoint_required_agent_dispatches
        || started_at_block != evidence.agent_started_delta_at_block
        || evidence.agent_counters_after_release.started != evidence.agent_counters_at_block.started
        || evidence.agent_counters_after_release.active != 0
        || !evidence.recovery_block_reason_retained
        || !evidence.no_recovery_redispatch
        || !evidence.remained_blocked_after_orphan_completion
    {
        return Err("checkpoint-required long workflow evidence is invalid".to_string());
    }
    Ok(())
}

fn validate_observation(
    observation: &WorkflowObservation,
    job_status: &str,
    recovery_state: &str,
    retry_safety: &str,
    generation: u64,
    attempt: u64,
) -> RunnerResult<()> {
    if observation.job_status != job_status
        || observation.recovery_state != recovery_state
        || observation.retry_safety != retry_safety
        || observation.generation != generation
        || observation.attempt != attempt
    {
        return Err(format!(
            "long workflow observation is invalid: {:?}",
            observation
        ));
    }
    Ok(())
}

fn validate_cleanup(cleanup: &CleanupEvidence) -> RunnerResult<()> {
    if !cleanup_complete(&cleanup.orchestra)
        || !cleanup.remote_agent_stopped
        || !cleanup.remote_agent_port_closed
        || !cleanup.managed_remote_agent_root_removed
        || !cleanup.local_agent_work_root_removed
    {
        return Err("long workflow cleanup is incomplete".to_string());
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
                "long workflow report retains forbidden content: {forbidden}"
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
                format!("failed to read long workflow source guard {relative}: {error}")
            })?,
        );
    }
    for required in &guard.required_text {
        if !source.contains(required) {
            return Err(format!("long workflow source guard misses {required}"));
        }
    }
    Ok(())
}

pub(crate) fn read_and_validate(root: &Path, relative: &str) -> RunnerResult<()> {
    let report = read(root, relative)?;
    validate(root, &report)
}

pub(crate) fn validator_self_test(root: &Path) -> RunnerResult<()> {
    let mut report = build_report(fixture_journey(), fixture_cleanup())?;
    validate(root, &report)?;
    report.evidence.idempotent.completed_history_count = 2;
    if validate(root, &report).is_ok() {
        return Err("validator accepted duplicate terminal commits".to_string());
    }
    report.evidence.idempotent.completed_history_count = 1;
    report.evidence.checkpoint_required.no_recovery_redispatch = false;
    if validate(root, &report).is_ok() {
        return Err("validator accepted unsafe checkpoint replay".to_string());
    }
    report.evidence.checkpoint_required.no_recovery_redispatch = true;
    report.cleanup.remote_agent_stopped = false;
    if validate(root, &report).is_ok() {
        return Err("validator accepted incomplete Agent cleanup".to_string());
    }
    report.cleanup.remote_agent_stopped = true;
    report.evidence.agent_architecture = "192.168.1.12".to_string();
    if validate(root, &report).is_ok() {
        return Err("validator accepted retained host identity".to_string());
    }
    Ok(())
}

fn fixture_journey() -> JourneyEvidence {
    JourneyEvidence {
        orchestra_platform: "macos".to_string(),
        database_architecture: "x86_64".to_string(),
        agent_architecture: "x86_64".to_string(),
        initial_fencing_token: 1,
        first_takeover_fencing_token: 2,
        second_takeover_fencing_token: 3,
        former_primary_rejoined_standby: true,
        former_standby_rejoined_standby: true,
        idempotent: IdempotentTakeoverEvidence {
            before_loss: observation("solving", "running", "idempotent", 1, 1),
            after_takeover: observation("solving", "running", "idempotent", 2, 2),
            terminal: observation("completed", "completed", "idempotent", 2, 2),
            takeover_elapsed_ms: 1_800,
            takeover_fencing_token: 2,
            agent_counters_before: counters(0, 0, 0, 0),
            agent_counters_after: counters(2, 2, 0, 0),
            active_execution_count_before_loss: 1,
            active_execution_count_after_takeover: 2,
            agent_started_delta: 2,
            initial_claim_seen: true,
            restart_claim_seen: true,
            completed_history_count: 1,
            result_verified: true,
            terminal_state_stable: true,
        },
        checkpoint_required: CheckpointBlockedEvidence {
            before_loss: observation("solving", "running", "checkpoint_required", 1, 1),
            after_takeover: observation("failed", "recovery_blocked", "checkpoint_required", 1, 1),
            takeover_elapsed_ms: 1_800,
            takeover_fencing_token: 3,
            agent_counters_before: counters(2, 2, 0, 0),
            agent_counters_at_block: counters(3, 2, 0, 1),
            agent_counters_after_release: counters(3, 3, 0, 0),
            active_execution_count_before_loss: 1,
            active_execution_count_at_block: 1,
            agent_started_delta_at_block: 1,
            recovery_block_reason_retained: true,
            no_recovery_redispatch: true,
            remained_blocked_after_orphan_completion: true,
        },
        followup_solver_verified: true,
    }
}

fn fixture_cleanup() -> CleanupEvidence {
    CleanupEvidence {
        orchestra: OrchestraCleanupEvidence {
            orchestra_processes_stopped: true,
            orchestra_ports_closed: true,
            ssh_tunnel_stopped: true,
            tunnel_port_closed: true,
            remote_database_removed: true,
            local_work_root_removed: true,
        },
        remote_agent_stopped: true,
        remote_agent_port_closed: true,
        managed_remote_agent_root_removed: true,
        local_agent_work_root_removed: true,
    }
}

fn observation(
    job: &str,
    state: &str,
    safety: &str,
    generation: u64,
    attempt: u64,
) -> WorkflowObservation {
    WorkflowObservation {
        job_status: job.to_string(),
        recovery_state: state.to_string(),
        retry_safety: safety.to_string(),
        generation,
        attempt,
    }
}

fn counters(started: u64, completed: u64, failed: u64, active: u64) -> AgentCounters {
    AgentCounters {
        started,
        completed,
        failed,
        active,
    }
}
