use crate::qualification_support::{generated_at_unix_ms, read_json, write_json};
use kyuubiki_protocol::AgentControlLinkDescriptor;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

type RunnerResult<T> = Result<T, String>;

pub(crate) const CONTRACT_PATH: &str =
    "config/architecture/agent-control-link-operational-qualification.json";
pub(crate) const CONTRACT_SCHEMA: &str =
    "kyuubiki.agent-control-link-operational-qualification-contract/v1";
pub(crate) const REPORT_SCHEMA: &str = "kyuubiki.agent-control-link-operational-qualification/v1";
pub(crate) const QUALIFICATION_ID: &str = "two-host-agent-control-link-rejoin-operational";
pub(crate) const JOURNEY: &str = "remote-agent-survives-orchestra-process-loss";
pub(crate) const DEFAULT_REPORT: &str =
    "releases/usability-evidence/2.14.7/agent-control-link-operational-qualification.json";
pub(crate) const DEFAULT_CAPTURE: &str = "tmp/agent-control-link-operational-qualification.json";
pub(crate) const REQUIRED_CHECKS: &[&str] = &[
    "initial_registration",
    "initial_heartbeat",
    "orchestra_process_loss",
    "agent_process_survived",
    "degraded_state_observed",
    "orchestra_restart",
    "full_reregistration",
    "heartbeat_recovered",
    "registry_rehydrated",
    "cleanup_complete",
    "retention_sanitized",
];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct PhaseEvidence {
    pub(crate) orchestrator_available: bool,
    pub(crate) registry_visible: bool,
    pub(crate) agent_process_alive: bool,
    pub(crate) control_link: AgentControlLinkDescriptor,
}

#[derive(Debug)]
pub(crate) struct JourneyPhases {
    pub(crate) remote_architecture: String,
    pub(crate) initial: PhaseEvidence,
    pub(crate) outage: PhaseEvidence,
    pub(crate) recovered: PhaseEvidence,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct CleanupEvidence {
    pub(crate) local_orchestra_stopped: bool,
    pub(crate) remote_agent_stopped: bool,
    pub(crate) local_port_closed: bool,
    pub(crate) remote_port_closed: bool,
    pub(crate) managed_remote_root_removed: bool,
    pub(crate) secret_files_removed: bool,
    pub(crate) local_work_root_removed: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct Report {
    schema_version: String,
    generated_at_unix_ms: u128,
    status: String,
    journey: String,
    topology: TopologyEvidence,
    phases: PhasesEvidence,
    cleanup: CleanupEvidence,
    checks: Vec<CheckEvidence>,
}

#[derive(Debug, Serialize, Deserialize)]
struct TopologyEvidence {
    orchestrator_host_role: String,
    agent_host_role: String,
    orchestrator_platform: String,
    agent_platform: String,
    agent_architecture: String,
    transport: String,
    orchestra_restart_count: u64,
    same_agent_process: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct PhasesEvidence {
    initial: PhaseEvidence,
    outage: PhaseEvidence,
    recovered: PhaseEvidence,
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
    orchestrator_host_role: String,
    agent_host_role: String,
    orchestrator_platform: String,
    agent_platform: String,
    build_profile: String,
    transport: String,
    orchestra_restarts_minimum: u64,
    successful_registrations_minimum: u64,
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
    phases: JourneyPhases,
    cleanup: CleanupEvidence,
) -> RunnerResult<Report> {
    Ok(Report {
        schema_version: REPORT_SCHEMA.to_string(),
        generated_at_unix_ms: generated_at_unix_ms()?,
        status: "pass".to_string(),
        journey: JOURNEY.to_string(),
        topology: TopologyEvidence {
            orchestrator_host_role: "local-macos-qualification-host".to_string(),
            agent_host_role: "remote-linux-qualification-host".to_string(),
            orchestrator_platform: "macos".to_string(),
            agent_platform: "linux".to_string(),
            agent_architecture: phases.remote_architecture,
            transport: "lan-http-plus-agent-rpc".to_string(),
            orchestra_restart_count: 1,
            same_agent_process: true,
        },
        phases: PhasesEvidence {
            initial: phases.initial,
            outage: phases.outage,
            recovered: phases.recovered,
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

pub(crate) fn validate_contract(root: &Path, require_report: bool) -> RunnerResult<()> {
    let contract: Contract = read_json(root, CONTRACT_PATH)?;
    if contract.schema_version != CONTRACT_SCHEMA
        || contract.qualification_id != QUALIFICATION_ID
        || contract.target_coordinate.module_id != "runtime-agent-cli"
        || contract.target_coordinate.paradigm != "runtime_api"
        || contract.target_coordinate.target_grade != "operational"
    {
        return Err("Agent control-link operational contract identity is invalid".to_string());
    }
    let capture = &contract.capture;
    if capture.orchestrator_host_role != "local-macos-qualification-host"
        || capture.agent_host_role != "remote-linux-qualification-host"
        || capture.orchestrator_platform != "macos"
        || capture.agent_platform != "linux"
        || capture.build_profile != "release"
        || capture.transport != "lan-http-plus-agent-rpc"
        || capture.orchestra_restarts_minimum < 1
        || capture.successful_registrations_minimum < 2
        || !capture.cleanup_required
    {
        return Err("Agent control-link capture thresholds are invalid".to_string());
    }
    require_exact_set(
        contract.required_checks.iter().map(String::as_str),
        REQUIRED_CHECKS.iter().copied(),
        "contract required checks",
    )?;
    if contract.retention.report_schema != REPORT_SCHEMA
        || contract.retention.report_path != DEFAULT_REPORT
    {
        return Err("Agent control-link retention identity is invalid".to_string());
    }
    let schema_path = root.join(&contract.retention.report_schema_path);
    let schema: Value = serde_json::from_slice(
        &fs::read(&schema_path)
            .map_err(|error| format!("failed to read {}: {error}", schema_path.display()))?,
    )
    .map_err(|error| format!("invalid JSON {}: {error}", schema_path.display()))?;
    if schema
        .pointer("/properties/schema_version/const")
        .and_then(Value::as_str)
        != Some(REPORT_SCHEMA)
    {
        return Err("Agent control-link report schema identity is invalid".to_string());
    }
    let mut guarded_source = String::new();
    for relative in &contract.source_guard.files {
        let path = root.join(relative);
        guarded_source.push_str(&fs::read_to_string(&path).map_err(|error| {
            format!("failed to read guarded source {}: {error}", path.display())
        })?);
    }
    for required in &contract.source_guard.required_text {
        if !guarded_source.contains(required) {
            return Err(format!("Agent control-link source guard misses {required}"));
        }
    }
    if contract.retention.forbidden_content.len() < 6 {
        return Err("Agent control-link forbidden-content policy is too weak".to_string());
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
        || report.journey != JOURNEY
        || report.generated_at_unix_ms == 0
    {
        return Err("Agent control-link report identity is invalid".to_string());
    }
    let topology = &report.topology;
    if topology.orchestrator_host_role != "local-macos-qualification-host"
        || topology.agent_host_role != "remote-linux-qualification-host"
        || topology.orchestrator_platform != "macos"
        || topology.agent_platform != "linux"
        || topology.agent_architecture.is_empty()
        || topology.transport != "lan-http-plus-agent-rpc"
        || topology.orchestra_restart_count < 1
        || !topology.same_agent_process
    {
        return Err("Agent control-link report topology is invalid".to_string());
    }
    validate_initial(&report.phases.initial)?;
    validate_outage(&report.phases.outage)?;
    validate_recovered(&report.phases.initial, &report.phases.recovered)?;
    if !all_cleanup_checks_pass(&report.cleanup) {
        return Err("Agent control-link report cleanup is incomplete".to_string());
    }
    require_exact_set(
        report.checks.iter().map(|check| check.id.as_str()),
        REQUIRED_CHECKS.iter().copied(),
        "report checks",
    )?;
    if report.checks.iter().any(|check| check.status != "pass") {
        return Err("Agent control-link report contains a failed check".to_string());
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

fn validate_initial(phase: &PhaseEvidence) -> RunnerResult<()> {
    let link = &phase.control_link;
    if !phase.orchestrator_available
        || !phase.registry_visible
        || !phase.agent_process_alive
        || link.state != "registered"
        || link.consecutive_failure_count != 0
        || link.successful_registration_count < 1
        || link.successful_heartbeat_count < 1
    {
        return Err("initial control-link phase is invalid".to_string());
    }
    Ok(())
}

fn validate_outage(phase: &PhaseEvidence) -> RunnerResult<()> {
    let link = &phase.control_link;
    let classified_failure = matches!(
        (
            link.last_failure_code.as_deref(),
            link.last_failure_message.as_deref()
        ),
        (
            Some("endpoint_unreachable"),
            Some("orchestrator endpoint is unreachable")
        ) | (
            Some("transport_failed"),
            Some("orchestrator control-plane transport failed")
        )
    );
    if phase.orchestrator_available
        || phase.registry_visible
        || !phase.agent_process_alive
        || link.state != "degraded"
        || link.consecutive_failure_count < 1
        || !classified_failure
    {
        return Err("outage control-link phase is invalid".to_string());
    }
    Ok(())
}

fn validate_recovered(initial: &PhaseEvidence, recovered: &PhaseEvidence) -> RunnerResult<()> {
    let before = &initial.control_link;
    let after = &recovered.control_link;
    if !recovered.orchestrator_available
        || !recovered.registry_visible
        || !recovered.agent_process_alive
        || after.state != "registered"
        || after.consecutive_failure_count != 0
        || after.last_failure_code.is_some()
        || after.last_failure_message.is_some()
        || after.successful_registration_count <= before.successful_registration_count
        || after.successful_heartbeat_count <= before.successful_heartbeat_count
        || after.attempt_count <= before.attempt_count
    {
        return Err("recovered control-link phase is invalid".to_string());
    }
    Ok(())
}

fn all_cleanup_checks_pass(cleanup: &CleanupEvidence) -> bool {
    cleanup.local_orchestra_stopped
        && cleanup.remote_agent_stopped
        && cleanup.local_port_closed
        && cleanup.remote_port_closed
        && cleanup.managed_remote_root_removed
        && cleanup.secret_files_removed
        && cleanup.local_work_root_removed
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
        Err(format!("{label} do not match the contract"))
    }
}

pub(crate) fn validator_self_test(root: &Path) -> RunnerResult<()> {
    let initial = fixture_phase(true, true, "registered", 1, 2, 0);
    let outage = fixture_phase(false, false, "degraded", 1, 2, 1);
    let recovered = fixture_phase(true, true, "registered", 2, 3, 0);
    let phases = JourneyPhases {
        remote_architecture: "x86_64".to_string(),
        initial,
        outage,
        recovered,
    };
    let cleanup = CleanupEvidence {
        local_orchestra_stopped: true,
        remote_agent_stopped: true,
        local_port_closed: true,
        remote_port_closed: true,
        managed_remote_root_removed: true,
        secret_files_removed: true,
        local_work_root_removed: true,
    };
    let mut report = build_report(phases, cleanup)?;
    validate(root, &report)?;
    report
        .phases
        .recovered
        .control_link
        .successful_registration_count = 1;
    if validate(root, &report).is_ok() {
        return Err("validator accepted recovery without re-registration".to_string());
    }
    Ok(())
}

fn fixture_phase(
    available: bool,
    visible: bool,
    state: &str,
    registrations: u64,
    heartbeats: u64,
    failures: u32,
) -> PhaseEvidence {
    let degraded = state == "degraded";
    PhaseEvidence {
        orchestrator_available: available,
        registry_visible: visible,
        agent_process_alive: true,
        control_link: AgentControlLinkDescriptor {
            state: state.to_string(),
            operation: if degraded { "register" } else { "heartbeat" }.to_string(),
            orchestrator_bound: true,
            attempt_count: registrations + heartbeats + u64::from(failures),
            consecutive_failure_count: failures,
            successful_registration_count: registrations,
            successful_heartbeat_count: heartbeats,
            last_success_unix_ms: Some(1),
            last_failure_unix_ms: degraded.then_some(2),
            last_failure_code: degraded.then(|| "endpoint_unreachable".to_string()),
            last_failure_message: degraded
                .then(|| "orchestrator endpoint is unreachable".to_string()),
            next_retry_delay_ms: 250,
            ..AgentControlLinkDescriptor::default()
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retained_report_validator_rejects_missing_reregistration() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../..");
        validator_self_test(&root).expect("validator self-test");
    }
}
