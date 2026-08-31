use super::agent_process::{CleanupCapture, HIGH_AGENT_ID, LOW_AGENT_ID};
use super::runtime::Captured;
use crate::qualification_support::{generated_at_unix_ms, read_json};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

type RunnerResult<T> = Result<T, String>;

pub(crate) const CONTRACT_PATH: &str =
    "config/architecture/fleet-scheduling-operational-qualification.json";
pub(crate) const CONTRACT_SCHEMA: &str =
    "kyuubiki.fleet-scheduling-operational-qualification-contract/v1";
pub(crate) const REPORT_SCHEMA: &str = "kyuubiki.fleet-scheduling-operational-qualification/v1";
const PROBE_SCHEMA: &str = "kyuubiki.fleet-scheduling-operational-probe/v1";
const QUALIFICATION_ID: &str = "remote-linux-installer-managed-fleet-scheduling-operational";
const JOURNEY: &str = "installer-managed-agent-fleet-capacity-failover-and-rejoin";
pub(crate) const DEFAULT_REPORT: &str =
    "releases/usability-evidence/2.19.0/fleet-scheduling-operational-qualification.json";
pub(crate) const DEFAULT_CAPTURE: &str = "tmp/fleet-scheduling-operational-qualification.json";

const REQUIRED_CHECKS: &[&str] = &[
    "installer_packaged_agents",
    "real_rust_agents",
    "declared_capacity_visible",
    "normalized_capacity_distribution",
    "scheduling_metadata_emitted",
    "solver_execution_both_agents",
    "numerical_consistency",
    "unavailable_agent_fallback",
    "cooldown_deprioritization",
    "restarted_agent_rejoined",
    "scheduling_resumed",
    "cleanup_complete",
    "retention_sanitized",
];

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct Report {
    schema_version: String,
    generated_at_unix_ms: u128,
    status: String,
    journey: String,
    topology: TopologyEvidence,
    installation: Value,
    agent_descriptors: Value,
    capacity_distribution: Value,
    solver_runs: Value,
    failure_recovery: Value,
    cleanup: CleanupEvidence,
    checks: Vec<CheckEvidence>,
}

#[derive(Debug, Serialize, Deserialize)]
struct TopologyEvidence {
    execution_host_role: String,
    platform: String,
    architecture: String,
    orchestration_runtime: String,
    execution_runtime: String,
    deployment_owner: String,
    transport: String,
    build_profile: String,
    agent_count: u64,
    declared_capacity_slots: u64,
}

#[derive(Debug, Serialize, Deserialize)]
struct CleanupEvidence {
    high_agent_stopped: bool,
    low_agent_stopped: bool,
    high_port_closed: bool,
    low_port_closed: bool,
    managed_install_root_removed: bool,
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
    execution_host_role: String,
    platform: String,
    build_profile: String,
    agent_count: u64,
    high_capacity: u64,
    low_capacity: u64,
    installer_managed: bool,
    fault_injection: String,
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

pub(crate) fn build(captured: Captured) -> RunnerResult<Report> {
    validate_probe(&captured.baseline_probe, "baseline")?;
    validate_probe(&captured.recovery_probe, "failover_recovery")?;
    let installation = &captured.installation;
    let version = installation.package.version.clone();
    let digest = installation.package.entrypoint_sha256.clone();

    Ok(Report {
        schema_version: REPORT_SCHEMA.to_string(),
        generated_at_unix_ms: generated_at_unix_ms()?,
        status: "pass".to_string(),
        journey: JOURNEY.to_string(),
        topology: TopologyEvidence {
            execution_host_role: "remote-linux-qualification-host".to_string(),
            platform: "linux".to_string(),
            architecture: captured.architecture,
            orchestration_runtime: "elixir-orchestra-runtime".to_string(),
            execution_runtime: "installed-rust-agent".to_string(),
            deployment_owner: "kyuubiki-installer".to_string(),
            transport: "loopback-agent-rpc".to_string(),
            build_profile: "release".to_string(),
            agent_count: 2,
            declared_capacity_slots: 4,
        },
        installation: json!({
            "package_schema": installation.package.schema_version,
            "package_version": version,
            "package_platform": installation.package.platform,
            "entrypoint_sha256": digest,
            "entrypoint_size_bytes": installation.package.entrypoint_size_bytes,
            "activations": [
                {
                    "agent_id": HIGH_AGENT_ID,
                    "schema_version": installation.high_activation.schema_version,
                    "generation": installation.high_activation.generation,
                    "version": installation.high_activation.version,
                    "entrypoint_sha256": installation.high_activation.entrypoint_sha256,
                    "active_version": installation.high_active_version
                },
                {
                    "agent_id": LOW_AGENT_ID,
                    "schema_version": installation.low_activation.schema_version,
                    "generation": installation.low_activation.generation,
                    "version": installation.low_activation.version,
                    "entrypoint_sha256": installation.low_activation.entrypoint_sha256,
                    "active_version": installation.low_active_version
                }
            ]
        }),
        agent_descriptors: captured.baseline_probe["agent_descriptors"].clone(),
        capacity_distribution: captured.baseline_probe["capacity_distribution"].clone(),
        solver_runs: captured.baseline_probe["solver_runs"].clone(),
        failure_recovery: json!({
            "fault": "high_capacity_agent_unavailable_before_dispatch",
            "failover": captured.recovery_probe["failover"].clone(),
            "cooldown": captured.recovery_probe["cooldown"].clone(),
            "recovery": captured.recovery_probe["recovery"].clone(),
            "high_agent_process_changed": captured.high_process_changed
        }),
        cleanup: cleanup_evidence(captured.cleanup),
        checks: REQUIRED_CHECKS
            .iter()
            .map(|id| CheckEvidence {
                id: (*id).to_string(),
                status: "pass".to_string(),
            })
            .collect(),
    })
}

pub(crate) fn validate_contract(root: &Path) -> RunnerResult<()> {
    let contract: Contract = read_json(root, CONTRACT_PATH)?;
    if contract.schema_version != CONTRACT_SCHEMA
        || contract.qualification_id != QUALIFICATION_ID
        || contract.target_coordinate.module_id != "orchestra-control-plane"
        || contract.target_coordinate.paradigm != "workflow_composition"
        || contract.target_coordinate.target_grade != "operational"
    {
        return Err("fleet scheduling contract identity is invalid".to_string());
    }
    if contract.ownership.control_plane_owner != "orchestra-control-plane"
        || contract.ownership.execution_owner != "runtime-agent-cli"
        || contract.ownership.deployment_owner != "runtime-installer"
        || contract.ownership.evidence_owner != "verification-evidence"
    {
        return Err("fleet scheduling ownership contract is invalid".to_string());
    }
    let capture = &contract.capture;
    if capture.execution_host_role != "remote-linux-qualification-host"
        || capture.platform != "linux"
        || capture.build_profile != "release"
        || capture.agent_count != 2
        || capture.high_capacity != 3
        || capture.low_capacity != 1
        || !capture.installer_managed
        || capture.fault_injection != "high_capacity_agent_unavailable_before_dispatch"
        || !capture.cleanup_required
    {
        return Err("fleet scheduling capture policy is invalid".to_string());
    }
    require_exact_set(
        contract.required_checks.iter().map(String::as_str),
        REQUIRED_CHECKS.iter().copied(),
        "contract checks",
    )?;
    if contract.retention.report_schema != REPORT_SCHEMA
        || contract.retention.report_path != DEFAULT_REPORT
        || contract.retention.forbidden_content.len() < 6
    {
        return Err("fleet scheduling retention policy is invalid".to_string());
    }
    let schema: Value = read_json(root, &contract.retention.report_schema_path)?;
    if schema
        .pointer("/properties/schema_version/const")
        .and_then(Value::as_str)
        != Some(REPORT_SCHEMA)
    {
        return Err("fleet scheduling report schema identity is invalid".to_string());
    }
    let contract_schema: Value = read_json(
        root,
        "schemas/fleet-scheduling-operational-qualification-contract.schema.json",
    )?;
    if contract_schema
        .pointer("/properties/schema_version/const")
        .and_then(Value::as_str)
        != Some(CONTRACT_SCHEMA)
    {
        return Err("fleet scheduling contract schema identity is invalid".to_string());
    }
    validate_source_guard(root, &contract.source_guard)
}

pub(crate) fn validate(root: &Path, report: &Report) -> RunnerResult<()> {
    let contract: Contract = read_json(root, CONTRACT_PATH)?;
    if report.schema_version != REPORT_SCHEMA
        || report.generated_at_unix_ms == 0
        || report.status != "pass"
        || report.journey != JOURNEY
    {
        return Err("fleet scheduling report identity is invalid".to_string());
    }
    validate_topology(&report.topology)?;
    validate_installation(&report.installation)?;
    validate_descriptors(&report.agent_descriptors)?;
    validate_capacity_distribution(&report.capacity_distribution)?;
    validate_solver_runs(&report.solver_runs)?;
    validate_failure_recovery(&report.failure_recovery)?;
    if !cleanup_complete(&report.cleanup) {
        return Err("fleet scheduling cleanup is incomplete".to_string());
    }
    require_exact_set(
        report.checks.iter().map(|check| check.id.as_str()),
        REQUIRED_CHECKS.iter().copied(),
        "report checks",
    )?;
    if report.checks.iter().any(|check| check.status != "pass") {
        return Err("fleet scheduling report contains failed checks".to_string());
    }
    let rendered = serde_json::to_string(report).map_err(|error| error.to_string())?;
    for forbidden in &contract.retention.forbidden_content {
        if rendered
            .to_ascii_lowercase()
            .contains(&forbidden.to_ascii_lowercase())
        {
            return Err(format!("fleet scheduling report retains {forbidden}"));
        }
    }
    Ok(())
}

fn validate_topology(topology: &TopologyEvidence) -> RunnerResult<()> {
    if topology.execution_host_role != "remote-linux-qualification-host"
        || topology.platform != "linux"
        || topology.architecture.is_empty()
        || !topology
            .architecture
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
        || topology.orchestration_runtime != "elixir-orchestra-runtime"
        || topology.execution_runtime != "installed-rust-agent"
        || topology.deployment_owner != "kyuubiki-installer"
        || topology.transport != "loopback-agent-rpc"
        || topology.build_profile != "release"
        || topology.agent_count != 2
        || topology.declared_capacity_slots != 4
    {
        return Err("fleet scheduling topology is invalid".to_string());
    }
    Ok(())
}

fn validate_installation(installation: &Value) -> RunnerResult<()> {
    let version = required_string(installation, "/package_version")?;
    let digest = required_string(installation, "/entrypoint_sha256")?;
    let activations = installation
        .pointer("/activations")
        .and_then(Value::as_array)
        .ok_or("fleet installation omits activations")?;
    if installation
        .pointer("/package_schema")
        .and_then(Value::as_str)
        != Some("kyuubiki.agent-update-package/v1")
        || installation
            .pointer("/package_platform")
            .and_then(Value::as_str)
            != Some("linux")
        || !is_digest(&digest)
        || installation
            .pointer("/entrypoint_size_bytes")
            .and_then(Value::as_u64)
            .unwrap_or(0)
            == 0
        || activations.len() != 2
    {
        return Err("fleet Installer package evidence is invalid".to_string());
    }
    let ids = activations
        .iter()
        .filter_map(|activation| activation.get("agent_id").and_then(Value::as_str))
        .collect::<BTreeSet<_>>();
    if ids != BTreeSet::from([HIGH_AGENT_ID, LOW_AGENT_ID]) {
        return Err("fleet Installer activation identities are invalid".to_string());
    }
    for activation in activations {
        if activation.get("schema_version").and_then(Value::as_str)
            != Some("kyuubiki.agent-update-activation/v1")
            || activation.get("generation").and_then(Value::as_u64) != Some(1)
            || activation.get("version").and_then(Value::as_str) != Some(version.as_str())
            || activation.get("active_version").and_then(Value::as_str) != Some(version.as_str())
            || activation.get("entrypoint_sha256").and_then(Value::as_str) != Some(digest.as_str())
        {
            return Err("fleet Installer activation evidence is invalid".to_string());
        }
    }
    Ok(())
}

fn validate_descriptors(descriptors: &Value) -> RunnerResult<()> {
    let descriptors = descriptors
        .as_array()
        .filter(|entries| entries.len() == 2)
        .ok_or("fleet Agent descriptors must contain two entries")?;
    let ids = descriptors
        .iter()
        .filter_map(|entry| entry.get("agent_id").and_then(Value::as_str))
        .collect::<BTreeSet<_>>();
    if ids != BTreeSet::from([HIGH_AGENT_ID, LOW_AGENT_ID])
        || descriptors.iter().any(|entry| {
            entry.get("program").and_then(Value::as_str) != Some("kyuubiki-rust-agent")
                || entry.get("role").and_then(Value::as_str) != Some("solver_agent")
                || entry.get("rpc_version").and_then(Value::as_u64) != Some(1)
        })
    {
        return Err("fleet endpoints are not two real Rust solver Agents".to_string());
    }
    Ok(())
}

fn validate_capacity_distribution(capacity: &Value) -> RunnerResult<()> {
    let sequence = string_array(capacity, "/lease_sequence")?;
    let expected = vec![HIGH_AGENT_ID, LOW_AGENT_ID, HIGH_AGENT_ID, HIGH_AGENT_ID];
    if capacity.pointer("/policy").and_then(Value::as_str) != Some("least_utilized_capacity_v1")
        || required_u64(capacity, "/declared_capacity/fleet-high-capacity")? != 3
        || required_u64(capacity, "/declared_capacity/fleet-low-capacity")? != 1
        || sequence != expected
        || required_u64(capacity, "/selected_counts/fleet-high-capacity")? != 3
        || required_u64(capacity, "/selected_counts/fleet-low-capacity")? != 1
        || required_u64(capacity, "/snapshot/active_lease_count")? != 4
        || required_u64(capacity, "/snapshot/capacity_slots")? != 4
    {
        return Err("capacity-normalized fleet distribution is invalid".to_string());
    }
    let decisions = capacity
        .pointer("/decisions")
        .and_then(Value::as_array)
        .ok_or("fleet capacity evidence omits decisions")?;
    if decisions.len() != 4
        || decisions.iter().any(|decision| {
            decision.get("selection_policy").and_then(Value::as_str)
                != Some("least_utilized_capacity_v1")
        })
    {
        return Err("fleet scheduling decision metadata is incomplete".to_string());
    }
    Ok(())
}

fn validate_solver_runs(runs: &Value) -> RunnerResult<()> {
    let runs = runs
        .as_array()
        .filter(|entries| entries.len() == 2)
        .ok_or("fleet solver evidence must contain two runs")?;
    let ids = runs
        .iter()
        .filter_map(|run| run.get("selected_agent_id").and_then(Value::as_str))
        .collect::<BTreeSet<_>>();
    if ids != BTreeSet::from([HIGH_AGENT_ID, LOW_AGENT_ID])
        || runs.iter().any(|run| !solver_run_valid(run))
    {
        return Err("fleet solver execution evidence is invalid".to_string());
    }
    Ok(())
}

fn validate_failure_recovery(recovery: &Value) -> RunnerResult<()> {
    let failover = &recovery["failover"];
    let cooldown = &recovery["cooldown"];
    let resumed = &recovery["recovery"]["resumed_run"];
    let failover_agents = scheduler_agent_ids(failover)?;
    if recovery.get("fault").and_then(Value::as_str)
        != Some("high_capacity_agent_unavailable_before_dispatch")
        || recovery
            .get("high_agent_process_changed")
            .and_then(Value::as_bool)
            != Some(true)
        || failover.get("selected_agent_id").and_then(Value::as_str) != Some(LOW_AGENT_ID)
        || failover_agents != vec![HIGH_AGENT_ID, LOW_AGENT_ID]
        || failover
            .pointer("/recovery/reason_code")
            .and_then(Value::as_str)
            .is_none_or(str::is_empty)
        || !solver_run_valid(failover)
        || cooldown.get("selected_agent_id").and_then(Value::as_str) != Some(LOW_AGENT_ID)
        || cooldown
            .pointer("/health/cooling_down_count")
            .and_then(Value::as_u64)
            != Some(1)
        || cooldown
            .pointer("/health/failed_agent_cooling_down")
            .and_then(Value::as_bool)
            != Some(true)
        || cooldown
            .pointer("/health/failed_agent_failure_count")
            .and_then(Value::as_u64)
            != Some(1)
        || !solver_run_valid(cooldown)
        || recovery
            .pointer("/recovery/ping_verified")
            .and_then(Value::as_bool)
            != Some(true)
        || resumed.get("selected_agent_id").and_then(Value::as_str) != Some(HIGH_AGENT_ID)
        || !solver_run_valid(resumed)
        || recovery
            .pointer("/recovery/health/cooling_down_count")
            .and_then(Value::as_u64)
            != Some(0)
    {
        return Err("fleet failover, cooldown, or recovery evidence is invalid".to_string());
    }
    Ok(())
}

fn scheduler_agent_ids(run: &Value) -> RunnerResult<Vec<&str>> {
    run.get("scheduler_events")
        .and_then(Value::as_array)
        .ok_or_else(|| "solver run omits scheduler events".to_string())?
        .iter()
        .map(|event| {
            event
                .get("agent_id")
                .and_then(Value::as_str)
                .ok_or_else(|| "scheduler event omits agent_id".to_string())
        })
        .collect()
}

fn solver_run_valid(run: &Value) -> bool {
    close_to(run, "/max_stress", 10.0, 1.0e-9)
        && close_to(run, "/tip_displacement", 0.01, 1.0e-12)
        && run
            .get("scheduler_events")
            .and_then(Value::as_array)
            .is_some_and(|events| {
                !events.is_empty()
                    && events.iter().all(|event| {
                        event.get("policy").and_then(Value::as_str)
                            == Some("least_utilized_capacity_v1")
                    })
            })
}

fn validate_probe(probe: &Value, phase: &str) -> RunnerResult<()> {
    if probe.get("schema_version").and_then(Value::as_str) != Some(PROBE_SCHEMA)
        || probe.get("status").and_then(Value::as_str) != Some("pass")
        || probe.get("phase").and_then(Value::as_str) != Some(phase)
    {
        return Err(format!("fleet {phase} probe identity is invalid"));
    }
    Ok(())
}

fn cleanup_evidence(cleanup: CleanupCapture) -> CleanupEvidence {
    CleanupEvidence {
        high_agent_stopped: cleanup.high_agent_stopped,
        low_agent_stopped: cleanup.low_agent_stopped,
        high_port_closed: cleanup.high_port_closed,
        low_port_closed: cleanup.low_port_closed,
        managed_install_root_removed: cleanup.managed_install_root_removed,
    }
}

fn cleanup_complete(cleanup: &CleanupEvidence) -> bool {
    cleanup.high_agent_stopped
        && cleanup.low_agent_stopped
        && cleanup.high_port_closed
        && cleanup.low_port_closed
        && cleanup.managed_install_root_removed
}

fn validate_source_guard(root: &Path, guard: &SourceGuard) -> RunnerResult<()> {
    let mut source = String::new();
    for relative in &guard.files {
        source.push_str(&fs::read_to_string(root.join(relative)).map_err(|error| {
            format!("failed to read fleet scheduling source {relative}: {error}")
        })?);
    }
    for required in &guard.required_text {
        if !source.contains(required) {
            return Err(format!("fleet scheduling source guard misses {required}"));
        }
    }
    Ok(())
}

fn required_string(value: &Value, pointer: &str) -> RunnerResult<String> {
    value
        .pointer(pointer)
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .ok_or_else(|| format!("fleet evidence omits {pointer}"))
}

fn required_u64(value: &Value, pointer: &str) -> RunnerResult<u64> {
    value
        .pointer(pointer)
        .and_then(Value::as_u64)
        .ok_or_else(|| format!("fleet evidence omits {pointer}"))
}

fn string_array<'a>(value: &'a Value, pointer: &str) -> RunnerResult<Vec<&'a str>> {
    value
        .pointer(pointer)
        .and_then(Value::as_array)
        .ok_or_else(|| format!("fleet evidence omits {pointer}"))?
        .iter()
        .map(|entry| {
            entry
                .as_str()
                .ok_or_else(|| format!("fleet evidence {pointer} is not a string array"))
        })
        .collect()
}

fn close_to(value: &Value, pointer: &str, expected: f64, tolerance: f64) -> bool {
    value
        .pointer(pointer)
        .and_then(Value::as_f64)
        .is_some_and(|actual| (actual - expected).abs() <= tolerance)
}

fn is_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

fn require_exact_set<'a>(
    actual: impl Iterator<Item = &'a str>,
    expected: impl Iterator<Item = &'a str>,
    label: &str,
) -> RunnerResult<()> {
    if actual.collect::<BTreeSet<_>>() == expected.collect::<BTreeSet<_>>() {
        Ok(())
    } else {
        Err(format!("{label} do not match the contract"))
    }
}

pub(crate) fn write_path(path: &Path, report: &Report) -> RunnerResult<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
    }
    let rendered = serde_json::to_string_pretty(report).map_err(|error| error.to_string())?;
    fs::write(path, format!("{rendered}\n"))
        .map_err(|error| format!("failed to write {}: {error}", path.display()))
}

pub(crate) fn read(root: &Path, relative: &str) -> RunnerResult<Report> {
    read_json(root, relative)
}

pub(crate) fn read_path(path: &Path) -> RunnerResult<Report> {
    let text = fs::read_to_string(path)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    serde_json::from_str(&text).map_err(|error| format!("invalid {}: {error}", path.display()))
}

pub(crate) fn validator_self_test(root: &Path) -> RunnerResult<()> {
    let mut report = fixture_report();
    validate(root, &report)?;
    report.capacity_distribution["lease_sequence"] =
        json!([HIGH_AGENT_ID, HIGH_AGENT_ID, HIGH_AGENT_ID, LOW_AGENT_ID]);
    if validate(root, &report).is_ok() {
        return Err("fleet scheduling validator accepted a biased capacity sequence".to_string());
    }
    Ok(())
}

fn fixture_report() -> Report {
    let scheduler = |agent_id: &str, capacity: u64| {
        json!({
            "policy": "least_utilized_capacity_v1",
            "agent_id": agent_id,
            "active_slots_before": 0,
            "active_slots_after": 1,
            "capacity_slots": capacity,
            "utilization_before": 0.0,
            "utilization_after": 1.0 / capacity as f64
        })
    };
    let solver = |agent_id: &str| {
        json!({
            "job_id": format!("fixture-{agent_id}"),
            "selected_agent_id": agent_id,
            "max_stress": 10.0,
            "tip_displacement": 0.01,
            "scheduler_events": [scheduler(agent_id, if agent_id == HIGH_AGENT_ID { 3 } else { 1 })],
            "recovery": null
        })
    };
    let digest = "a".repeat(64);
    Report {
        schema_version: REPORT_SCHEMA.to_string(),
        generated_at_unix_ms: 1,
        status: "pass".to_string(),
        journey: JOURNEY.to_string(),
        topology: TopologyEvidence {
            execution_host_role: "remote-linux-qualification-host".to_string(),
            platform: "linux".to_string(),
            architecture: "x86_64".to_string(),
            orchestration_runtime: "elixir-orchestra-runtime".to_string(),
            execution_runtime: "installed-rust-agent".to_string(),
            deployment_owner: "kyuubiki-installer".to_string(),
            transport: "loopback-agent-rpc".to_string(),
            build_profile: "release".to_string(),
            agent_count: 2,
            declared_capacity_slots: 4,
        },
        installation: json!({
            "package_schema": "kyuubiki.agent-update-package/v1",
            "package_version": "2.19.0",
            "package_platform": "linux",
            "entrypoint_sha256": digest,
            "entrypoint_size_bytes": 1024,
            "activations": [
                {"agent_id": HIGH_AGENT_ID, "schema_version": "kyuubiki.agent-update-activation/v1", "generation": 1, "version": "2.19.0", "entrypoint_sha256": digest, "active_version": "2.19.0"},
                {"agent_id": LOW_AGENT_ID, "schema_version": "kyuubiki.agent-update-activation/v1", "generation": 1, "version": "2.19.0", "entrypoint_sha256": digest, "active_version": "2.19.0"}
            ]
        }),
        agent_descriptors: json!([
            {"agent_id": HIGH_AGENT_ID, "program": "kyuubiki-rust-agent", "role": "solver_agent", "rpc_version": 1},
            {"agent_id": LOW_AGENT_ID, "program": "kyuubiki-rust-agent", "role": "solver_agent", "rpc_version": 1}
        ]),
        capacity_distribution: json!({
            "policy": "least_utilized_capacity_v1",
            "declared_capacity": {HIGH_AGENT_ID: 3, LOW_AGENT_ID: 1},
            "lease_sequence": [HIGH_AGENT_ID, LOW_AGENT_ID, HIGH_AGENT_ID, HIGH_AGENT_ID],
            "selected_counts": {HIGH_AGENT_ID: 3, LOW_AGENT_ID: 1},
            "decisions": [
                {"selection_policy": "least_utilized_capacity_v1"},
                {"selection_policy": "least_utilized_capacity_v1"},
                {"selection_policy": "least_utilized_capacity_v1"},
                {"selection_policy": "least_utilized_capacity_v1"}
            ],
            "snapshot": {"active_lease_count": 4, "capacity_slots": 4}
        }),
        solver_runs: json!([solver(HIGH_AGENT_ID), solver(LOW_AGENT_ID)]),
        failure_recovery: json!({
            "fault": "high_capacity_agent_unavailable_before_dispatch",
            "high_agent_process_changed": true,
            "failover": {
                "selected_agent_id": LOW_AGENT_ID, "max_stress": 10.0, "tip_displacement": 0.01,
                "scheduler_events": [scheduler(HIGH_AGENT_ID, 3), scheduler(LOW_AGENT_ID, 1)],
                "recovery": {"reason_code": "endpoint_unreachable"}
            },
            "cooldown": {
                "selected_agent_id": LOW_AGENT_ID, "max_stress": 10.0, "tip_displacement": 0.01,
                "scheduler_events": [scheduler(LOW_AGENT_ID, 1)],
                "health": {"cooling_down_count": 1, "failed_agent_cooling_down": true, "failed_agent_failure_count": 1}
            },
            "recovery": {
                "ping_verified": true,
                "resumed_run": solver(HIGH_AGENT_ID),
                "health": {"cooling_down_count": 0}
            }
        }),
        cleanup: CleanupEvidence {
            high_agent_stopped: true,
            low_agent_stopped: true,
            high_port_closed: true,
            low_port_closed: true,
            managed_install_root_removed: true,
        },
        checks: REQUIRED_CHECKS
            .iter()
            .map(|id| CheckEvidence {
                id: (*id).to_string(),
                status: "pass".to_string(),
            })
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validator_rejects_biased_capacity_distribution() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../..");
        validator_self_test(&root).expect("fleet validator self-test");
    }
}
