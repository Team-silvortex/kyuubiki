use super::long_workflow_runtime::{CleanupEvidence, JourneyEvidence};
use super::runtime::{HEARTBEAT_MS, LEASE_TTL_MS, RETRY_MS};
use crate::qualification_support::{generated_at_unix_ms, read_json, write_json};
use serde::{Deserialize, Serialize};
use std::path::Path;

type RunnerResult<T> = Result<T, String>;

pub(crate) const CONTRACT_PATH: &str =
    "config/architecture/orchestra-long-workflow-takeover-operational-qualification.json";
pub(crate) const CONTRACT_SCHEMA: &str =
    "kyuubiki.orchestra-long-workflow-takeover-operational-qualification-contract/v1";
pub(crate) const REPORT_SCHEMA: &str =
    "kyuubiki.orchestra-long-workflow-takeover-operational-qualification/v1";
pub(crate) const QUALIFICATION_ID: &str =
    "two-orchestra-postgresql-long-workflow-takeover-operational";
pub(crate) const JOURNEY: &str = "inflight-idempotent-resume-and-checkpoint-required-replay-block";
pub(crate) const DEFAULT_REPORT: &str = "releases/usability-evidence/2.18.3/orchestra-long-workflow-takeover-operational-qualification.json";
pub(crate) const DEFAULT_CAPTURE: &str =
    "tmp/orchestra-long-workflow-takeover-operational-qualification.json";

pub(super) const REQUIRED_CHECKS: &[&str] = &[
    "real_rust_agent",
    "real_remote_postgresql",
    "two_live_orchestra_instances",
    "exact_job_pause_barrier",
    "idempotent_inflight_observed",
    "first_takeover_fenced",
    "idempotent_replay_claimed",
    "single_terminal_commit",
    "solver_result_verified",
    "checkpoint_required_inflight_observed",
    "second_takeover_fenced",
    "unsafe_replay_blocked",
    "no_checkpoint_redispatch",
    "orphan_completion_did_not_mutate",
    "former_owners_rejoined",
    "followup_solver_verified",
    "cleanup_complete",
    "retention_sanitized",
];

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct Report {
    pub(super) schema_version: String,
    pub(super) generated_at_unix_ms: u128,
    pub(super) status: String,
    pub(super) journey: String,
    pub(super) topology: TopologyEvidence,
    pub(super) lease_policy: LeasePolicyEvidence,
    pub(super) evidence: JourneyEvidence,
    pub(super) cleanup: CleanupEvidence,
    pub(super) checks: Vec<CheckEvidence>,
}

#[derive(Debug, Serialize, Deserialize)]
pub(super) struct TopologyEvidence {
    pub(super) orchestra_host_role: String,
    pub(super) database_host_role: String,
    pub(super) agent_host_role: String,
    pub(super) orchestra_platform: String,
    pub(super) database_platform: String,
    pub(super) agent_platform: String,
    pub(super) orchestra_process_count: u64,
    pub(super) database: String,
    pub(super) database_ephemeral: bool,
    pub(super) database_loopback_only: bool,
    pub(super) agent_runtime: String,
    pub(super) workflow_operator: String,
    pub(super) transport: String,
    pub(super) build_profile: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub(super) struct LeasePolicyEvidence {
    pub(super) lease_ttl_ms: u64,
    pub(super) heartbeat_ms: u64,
    pub(super) retry_ms: u64,
    pub(super) max_workflow_attempts: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub(super) struct CheckEvidence {
    pub(super) id: String,
    pub(super) status: String,
}

pub(crate) fn build_report(
    evidence: JourneyEvidence,
    cleanup: CleanupEvidence,
) -> RunnerResult<Report> {
    Ok(Report {
        schema_version: REPORT_SCHEMA.to_string(),
        generated_at_unix_ms: generated_at_unix_ms()?,
        status: "pass".to_string(),
        journey: JOURNEY.to_string(),
        topology: TopologyEvidence {
            orchestra_host_role: "local-orchestra-qualification-host".to_string(),
            database_host_role: "remote-linux-qualification-host".to_string(),
            agent_host_role: "remote-linux-qualification-host".to_string(),
            orchestra_platform: evidence.orchestra_platform.clone(),
            database_platform: "linux".to_string(),
            agent_platform: "linux".to_string(),
            orchestra_process_count: 2,
            database: "postgresql".to_string(),
            database_ephemeral: true,
            database_loopback_only: true,
            agent_runtime: "kyuubiki-rust-agent".to_string(),
            workflow_operator: "solve.bar_1d".to_string(),
            transport: "independent-ssh-database-tunnels-plus-agent-tcp".to_string(),
            build_profile: "release-agent-development-orchestra".to_string(),
        },
        lease_policy: LeasePolicyEvidence {
            lease_ttl_ms: LEASE_TTL_MS,
            heartbeat_ms: HEARTBEAT_MS,
            retry_ms: RETRY_MS,
            max_workflow_attempts: 3,
        },
        evidence,
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
