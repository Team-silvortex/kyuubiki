use super::report::{CleanupEvidence, LeasePhase};
use crate::qualification_support::{generated_at_unix_ms, read_json, write_json};
use serde::{Deserialize, Serialize};
use std::path::Path;

type RunnerResult<T> = Result<T, String>;

pub(crate) const CONTRACT_PATH: &str =
    "config/architecture/orchestra-network-partition-operational-qualification.json";
pub(crate) const CONTRACT_SCHEMA: &str =
    "kyuubiki.orchestra-network-partition-operational-qualification-contract/v1";
pub(crate) const REPORT_SCHEMA: &str =
    "kyuubiki.orchestra-network-partition-operational-qualification/v1";
pub(crate) const QUALIFICATION_ID: &str =
    "two-orchestra-postgresql-network-partition-fencing-operational";
pub(crate) const JOURNEY: &str = "primary-database-partition-standby-takeover-rejoin-fencing";
pub(crate) const DEFAULT_REPORT: &str =
    "releases/usability-evidence/2.18.3/orchestra-network-partition-operational-qualification.json";
pub(crate) const DEFAULT_CAPTURE: &str =
    "tmp/orchestra-network-partition-operational-qualification.json";

pub(super) const REQUIRED_CHECKS: &[&str] = &[
    "remote_postgresql_ready",
    "independent_database_tunnels_ready",
    "primary_owner_elected",
    "second_orchestra_standby",
    "primary_database_partition_injected",
    "primary_process_survived",
    "primary_failed_closed",
    "standby_database_path_remained_available",
    "standby_promoted",
    "fencing_token_incremented",
    "primary_network_restored",
    "former_owner_identity_fenced",
    "stale_owner_submission_rejected",
    "ephemeral_database_isolated",
    "cleanup_complete",
    "retention_sanitized",
];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct PartitionedOwnerPhase {
    pub(crate) process_role: String,
    pub(crate) lease_status: String,
    pub(crate) observed_owner_role: String,
    pub(crate) visible_fencing_token: Option<u64>,
    pub(crate) last_error: String,
}

#[derive(Debug)]
pub(crate) struct PartitionJourneyEvidence {
    pub(crate) database_architecture: String,
    pub(crate) orchestra_platform: String,
    pub(crate) initial_owner: LeasePhase,
    pub(crate) initial_standby: LeasePhase,
    pub(crate) partitioned_owner: PartitionedOwnerPhase,
    pub(crate) takeover: LeasePhase,
    pub(crate) former_owner_rejoin: LeasePhase,
    pub(crate) partition_to_fail_closed_elapsed_ms: u128,
    pub(crate) takeover_elapsed_ms: u128,
    pub(crate) primary_process_survived: bool,
    pub(crate) primary_endpoint_remained_open: bool,
    pub(crate) isolated_tunnel_closed: bool,
    pub(crate) standby_tunnel_remained_open: bool,
    pub(crate) stale_owner_submission_rejected: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct Report {
    pub(super) schema_version: String,
    pub(super) generated_at_unix_ms: u128,
    pub(super) status: String,
    pub(super) journey: String,
    pub(super) topology: TopologyEvidence,
    pub(super) lease_policy: LeasePolicyEvidence,
    pub(super) phases: PhaseEvidence,
    pub(super) cleanup: CleanupEvidence,
    pub(super) checks: Vec<CheckEvidence>,
}

#[derive(Debug, Serialize, Deserialize)]
pub(super) struct TopologyEvidence {
    pub(super) orchestra_host_role: String,
    pub(super) database_host_role: String,
    pub(super) orchestra_platform: String,
    pub(super) database_platform: String,
    pub(super) database_architecture: String,
    pub(super) orchestra_process_count: u64,
    pub(super) database: String,
    pub(super) database_ephemeral: bool,
    pub(super) database_loopback_only: bool,
    pub(super) transport: String,
    pub(super) independent_database_tunnel_count: u64,
    pub(super) build_profile: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub(super) struct LeasePolicyEvidence {
    pub(super) lease_ttl_ms: u64,
    pub(super) heartbeat_ms: u64,
    pub(super) retry_ms: u64,
    pub(super) failure_mode: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub(super) struct PhaseEvidence {
    pub(super) initial_owner: LeasePhase,
    pub(super) initial_standby: LeasePhase,
    pub(super) partitioned_owner: PartitionedOwnerPhase,
    pub(super) takeover: LeasePhase,
    pub(super) former_owner_rejoin: LeasePhase,
    pub(super) partition_to_fail_closed_elapsed_ms: u128,
    pub(super) takeover_elapsed_ms: u128,
    pub(super) primary_process_survived: bool,
    pub(super) primary_endpoint_remained_open: bool,
    pub(super) isolated_tunnel_closed: bool,
    pub(super) standby_tunnel_remained_open: bool,
    pub(super) stale_owner_submission_rejected: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub(super) struct CheckEvidence {
    pub(super) id: String,
    pub(super) status: String,
}

pub(crate) fn build_report(
    journey: PartitionJourneyEvidence,
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
            orchestra_platform: journey.orchestra_platform,
            database_platform: "linux".to_string(),
            database_architecture: journey.database_architecture,
            orchestra_process_count: 2,
            database: "postgresql".to_string(),
            database_ephemeral: true,
            database_loopback_only: true,
            transport: "independent-ssh-loopback-tunnels".to_string(),
            independent_database_tunnel_count: 2,
            build_profile: "development-no-compile".to_string(),
        },
        lease_policy: LeasePolicyEvidence {
            lease_ttl_ms: 1_500,
            heartbeat_ms: 400,
            retry_ms: 200,
            failure_mode: "primary-database-network-partition".to_string(),
        },
        phases: PhaseEvidence {
            initial_owner: journey.initial_owner,
            initial_standby: journey.initial_standby,
            partitioned_owner: journey.partitioned_owner,
            takeover: journey.takeover,
            former_owner_rejoin: journey.former_owner_rejoin,
            partition_to_fail_closed_elapsed_ms: journey.partition_to_fail_closed_elapsed_ms,
            takeover_elapsed_ms: journey.takeover_elapsed_ms,
            primary_process_survived: journey.primary_process_survived,
            primary_endpoint_remained_open: journey.primary_endpoint_remained_open,
            isolated_tunnel_closed: journey.isolated_tunnel_closed,
            standby_tunnel_remained_open: journey.standby_tunnel_remained_open,
            stale_owner_submission_rejected: journey.stale_owner_submission_rejected,
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

pub(crate) fn write(root: &Path, relative: &str, report: &Report) -> RunnerResult<()> {
    write_json(root, relative, report)
}

pub(crate) fn read(root: &Path, relative: &str) -> RunnerResult<Report> {
    read_json(root, relative)
}
