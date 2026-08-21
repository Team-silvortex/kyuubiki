use super::runtime::{Captured, CleanupCapture, ExecutionCounters, ScenarioCapture};
use crate::qualification_support::{generated_at_unix_ms, read_json, write_json};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

type RunnerResult<T> = Result<T, String>;

pub(crate) const CONTRACT_PATH: &str =
    "config/architecture/distributed-task-recovery-operational-qualification.json";
pub(crate) const CONTRACT_SCHEMA: &str =
    "kyuubiki.distributed-task-recovery-operational-qualification-contract/v1";
pub(crate) const REPORT_SCHEMA: &str =
    "kyuubiki.distributed-task-recovery-operational-qualification/v1";
pub(crate) const PROBE_SCHEMA: &str = "kyuubiki.distributed-task-recovery-operational-probe/v1";
pub(crate) const QUALIFICATION_ID: &str = "two-host-distributed-task-recovery-operational";
pub(crate) const JOURNEY: &str = "remote-agent-inflight-loss-policy-and-rejoin";
pub(crate) const DEFAULT_REPORT: &str =
    "releases/usability-evidence/2.14.8/distributed-task-recovery-operational-qualification.json";
pub(crate) const DEFAULT_CAPTURE: &str =
    "tmp/distributed-task-recovery-operational-qualification.json";

const REQUIRED_SCENARIOS: &[&str] = &["idempotent", "side_effect_blocked", "checkpointed"];
const REQUIRED_CHECKS: &[&str] = &[
    "real_rust_agents",
    "deterministic_inflight_fault_barrier",
    "primary_inflight_observed",
    "idempotent_fallback_completed",
    "side_effect_replay_blocked",
    "checkpoint_authorized_fallback",
    "fallback_counter_consistency",
    "remote_rejoined_after_each_fault",
    "followup_solver_verified",
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
    scenarios: Vec<ScenarioEvidence>,
    cleanup: CleanupEvidence,
    checks: Vec<CheckEvidence>,
}

#[derive(Debug, Serialize, Deserialize)]
struct TopologyEvidence {
    orchestration_host_role: String,
    primary_agent_host_role: String,
    fallback_agent_host_role: String,
    orchestration_platform: String,
    primary_agent_platform: String,
    fallback_agent_platform: String,
    primary_agent_architecture: String,
    transport: String,
    build_profile: String,
    fault_injection_barrier: String,
    scenario_count: u64,
    remote_process_restart_count: u64,
    same_remote_endpoint: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct ScenarioEvidence {
    id: String,
    job_id: String,
    method: String,
    injected_fault: String,
    outcome: String,
    retry_safety: String,
    primary_before: CounterEvidence,
    primary_inflight: CounterEvidence,
    fallback_before: CounterEvidence,
    fallback_after: CounterEvidence,
    fallback_delta: CounterDelta,
    recovery: RecoveryEvidence,
    checkpoint_digest: Option<String>,
    result_verified: bool,
    remote_rejoined: bool,
    followup_solver_verified: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
struct CounterEvidence {
    started: u64,
    completed: u64,
    failed: u64,
    active: u64,
}

#[derive(Debug, Serialize, Deserialize)]
struct CounterDelta {
    started: u64,
    completed: u64,
    failed: u64,
}

#[derive(Debug, Serialize, Deserialize)]
struct RecoveryEvidence {
    failure_stage: String,
    reason_code: String,
    process_loss: bool,
    retry_safety: String,
    checkpoint_digest: Option<String>,
    retryable: bool,
    remaining_agent_count: u64,
    safe_to_continue_other_tasks: bool,
    next_action: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct CleanupEvidence {
    local_agent_stopped: bool,
    remote_agent_stopped: bool,
    local_port_closed: bool,
    remote_port_closed: bool,
    managed_remote_root_removed: bool,
    local_work_root_removed: bool,
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
    required_scenarios: Vec<String>,
    orchestrator_platform: String,
    primary_agent_platform: String,
    fallback_agent_platform: String,
    build_profile: String,
    remote_restarts_minimum: u64,
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

pub(crate) fn build_report(captured: Captured) -> RunnerResult<Report> {
    let scenarios = captured
        .scenarios
        .iter()
        .map(build_scenario)
        .collect::<RunnerResult<Vec<_>>>()?;
    Ok(Report {
        schema_version: REPORT_SCHEMA.to_string(),
        generated_at_unix_ms: generated_at_unix_ms()?,
        status: "pass".to_string(),
        journey: JOURNEY.to_string(),
        topology: TopologyEvidence {
            orchestration_host_role: "local-macos-orchestra-runtime".to_string(),
            primary_agent_host_role: "remote-linux-primary-agent".to_string(),
            fallback_agent_host_role: "local-macos-fallback-agent".to_string(),
            orchestration_platform: "macos".to_string(),
            primary_agent_platform: "linux".to_string(),
            fallback_agent_platform: "macos".to_string(),
            primary_agent_architecture: captured.remote_architecture,
            transport: "lan-agent-rpc".to_string(),
            build_profile: "release".to_string(),
            fault_injection_barrier: "job-scoped-explicit-agent-hold".to_string(),
            scenario_count: scenarios.len() as u64,
            remote_process_restart_count: captured.remote_restart_count,
            same_remote_endpoint: true,
        },
        scenarios,
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

fn build_scenario(capture: &ScenarioCapture) -> RunnerResult<ScenarioEvidence> {
    require_probe_identity(capture)?;
    let observations = capture
        .probe
        .get("observations")
        .ok_or_else(|| format!("{} probe omitted observations", capture.id))?;
    let recovery = recovery_evidence(
        observations
            .get("recovery")
            .ok_or_else(|| format!("{} probe omitted recovery", capture.id))?,
    )?;
    let outcome = observations
        .get("outcome")
        .and_then(Value::as_str)
        .ok_or_else(|| format!("{} probe omitted outcome", capture.id))?
        .to_string();
    let checkpoint_digest = observations
        .get("checkpoint_digest")
        .and_then(Value::as_str)
        .map(str::to_string);
    let fallback_delta = CounterDelta {
        started: capture
            .fallback_after
            .started
            .saturating_sub(capture.fallback_before.started),
        completed: capture
            .fallback_after
            .completed
            .saturating_sub(capture.fallback_before.completed),
        failed: capture
            .fallback_after
            .failed
            .saturating_sub(capture.fallback_before.failed),
    };
    Ok(ScenarioEvidence {
        id: capture.id.clone(),
        job_id: capture.job_id.clone(),
        method: capture.method.clone(),
        injected_fault: "remote_agent_terminated_after_dispatch_before_result_commit".to_string(),
        outcome,
        retry_safety: recovery.retry_safety.clone(),
        primary_before: counter_evidence(capture.primary_before),
        primary_inflight: counter_evidence(capture.primary_inflight),
        fallback_before: counter_evidence(capture.fallback_before),
        fallback_after: counter_evidence(capture.fallback_after),
        fallback_delta,
        recovery,
        checkpoint_digest,
        result_verified: result_verified(capture, observations),
        remote_rejoined: capture.remote_rejoined,
        followup_solver_verified: capture.followup_solver_verified,
    })
}

fn require_probe_identity(capture: &ScenarioCapture) -> RunnerResult<()> {
    if capture.probe.get("schema_version").and_then(Value::as_str) != Some(PROBE_SCHEMA)
        || capture.probe.get("status").and_then(Value::as_str) != Some("pass")
        || capture.probe.get("scenario").and_then(Value::as_str) != Some(capture.id.as_str())
        || capture.probe.get("job_id").and_then(Value::as_str) != Some(capture.job_id.as_str())
        || capture
            .probe
            .get("progress_event_count")
            .and_then(Value::as_u64)
            .unwrap_or(0)
            == 0
    {
        return Err(format!("{} probe identity is invalid", capture.id));
    }
    Ok(())
}

fn recovery_evidence(value: &Value) -> RunnerResult<RecoveryEvidence> {
    Ok(RecoveryEvidence {
        failure_stage: required_string(value, "failure_stage")?,
        reason_code: required_string(value, "reason_code")?,
        process_loss: required_bool(value, "process_loss")?,
        retry_safety: required_string(value, "retry_safety")?,
        checkpoint_digest: value
            .get("checkpoint_digest")
            .and_then(Value::as_str)
            .map(str::to_string),
        retryable: required_bool(value, "retryable")?,
        remaining_agent_count: value
            .get("remaining_agent_count")
            .and_then(Value::as_u64)
            .ok_or("recovery omitted remaining_agent_count")?,
        safe_to_continue_other_tasks: required_bool(value, "safe_to_continue_other_tasks")?,
        next_action: required_string(value, "next_action")?,
    })
}

fn result_verified(capture: &ScenarioCapture, observations: &Value) -> bool {
    match capture.id.as_str() {
        "idempotent" => {
            observations
                .get("fallback_agent_id")
                .and_then(Value::as_str)
                == Some("local-fallback")
                && close_to(observations, "result_max_stress", 10.0, 1.0e-9)
                && close_to(observations, "result_tip_displacement", 0.01, 1.0e-12)
        }
        "side_effect_blocked" => {
            observations.get("outcome").and_then(Value::as_str) == Some("replay_blocked")
        }
        "checkpointed" => {
            observations
                .get("result_status")
                .and_then(Value::as_str)
                .is_some_and(|status| !status.is_empty())
                && observations
                    .get("checkpoint_digest")
                    .and_then(Value::as_str)
                    .is_some_and(is_digest)
        }
        _ => false,
    }
}

fn cleanup_evidence(capture: CleanupCapture) -> CleanupEvidence {
    CleanupEvidence {
        local_agent_stopped: capture.local_agent_stopped,
        remote_agent_stopped: capture.remote_agent_stopped,
        local_port_closed: capture.local_port_closed,
        remote_port_closed: capture.remote_port_closed,
        managed_remote_root_removed: capture.managed_remote_root_removed,
        local_work_root_removed: capture.local_work_root_removed,
    }
}

fn counter_evidence(counters: ExecutionCounters) -> CounterEvidence {
    CounterEvidence {
        started: counters.started,
        completed: counters.completed,
        failed: counters.failed,
        active: counters.active,
    }
}

pub(crate) fn write(root: &Path, relative: &str, report: &Report) -> RunnerResult<()> {
    write_json(root, relative, report)
}

pub(crate) fn read(root: &Path, relative: &str) -> RunnerResult<Report> {
    read_json(root, relative)
}

pub(crate) fn validate_contract(root: &Path) -> RunnerResult<()> {
    let contract: Contract = read_json(root, CONTRACT_PATH)?;
    if contract.schema_version != CONTRACT_SCHEMA
        || contract.qualification_id != QUALIFICATION_ID
        || contract.target_coordinate.module_id != "orchestra-control-plane"
        || contract.target_coordinate.paradigm != "fault_injection_and_recovery"
        || contract.target_coordinate.target_grade != "operational"
    {
        return Err("distributed recovery contract identity is invalid".to_string());
    }
    let capture = &contract.capture;
    require_exact_set(
        capture.required_scenarios.iter().map(String::as_str),
        REQUIRED_SCENARIOS.iter().copied(),
        "capture scenarios",
    )?;
    if capture.orchestrator_platform != "macos"
        || capture.primary_agent_platform != "linux"
        || capture.fallback_agent_platform != "macos"
        || capture.build_profile != "release"
        || capture.remote_restarts_minimum < 3
        || !capture.cleanup_required
    {
        return Err("distributed recovery capture policy is invalid".to_string());
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
        return Err("distributed recovery retention policy is invalid".to_string());
    }
    let schema: Value = read_json(root, &contract.retention.report_schema_path)?;
    if schema
        .pointer("/properties/schema_version/const")
        .and_then(Value::as_str)
        != Some(REPORT_SCHEMA)
    {
        return Err("distributed recovery report schema identity is invalid".to_string());
    }
    let mut source = String::new();
    for relative in &contract.source_guard.files {
        source.push_str(&fs::read_to_string(root.join(relative)).map_err(|error| {
            format!("failed to read distributed recovery source {relative}: {error}")
        })?);
    }
    for required in &contract.source_guard.required_text {
        if !source.contains(required) {
            return Err(format!(
                "distributed recovery source guard misses {required}"
            ));
        }
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
        return Err("distributed recovery report identity is invalid".to_string());
    }
    let topology = &report.topology;
    if topology.orchestration_platform != "macos"
        || topology.primary_agent_platform != "linux"
        || topology.fallback_agent_platform != "macos"
        || topology.primary_agent_architecture.is_empty()
        || topology.transport != "lan-agent-rpc"
        || topology.build_profile != "release"
        || topology.fault_injection_barrier != "job-scoped-explicit-agent-hold"
        || topology.scenario_count != 3
        || topology.remote_process_restart_count < 3
        || !topology.same_remote_endpoint
    {
        return Err("distributed recovery topology is invalid".to_string());
    }
    require_exact_set(
        report.scenarios.iter().map(|scenario| scenario.id.as_str()),
        REQUIRED_SCENARIOS.iter().copied(),
        "report scenarios",
    )?;
    for scenario in &report.scenarios {
        validate_scenario(scenario)?;
    }
    if !cleanup_complete(&report.cleanup) {
        return Err("distributed recovery cleanup is incomplete".to_string());
    }
    require_exact_set(
        report.checks.iter().map(|check| check.id.as_str()),
        REQUIRED_CHECKS.iter().copied(),
        "report checks",
    )?;
    if report.checks.iter().any(|check| check.status != "pass") {
        return Err("distributed recovery report contains failed checks".to_string());
    }
    let rendered = serde_json::to_string(report).map_err(|error| error.to_string())?;
    for forbidden in &contract.retention.forbidden_content {
        if rendered
            .to_ascii_lowercase()
            .contains(&forbidden.to_ascii_lowercase())
        {
            return Err(format!("distributed recovery report retains {forbidden}"));
        }
    }
    Ok(())
}

fn validate_scenario(scenario: &ScenarioEvidence) -> RunnerResult<()> {
    let primary_started = scenario
        .primary_inflight
        .started
        .saturating_sub(scenario.primary_before.started);
    if primary_started != 1
        || scenario.primary_inflight.active == 0
        || scenario.recovery.failure_stage != "receive"
        || scenario.recovery.reason_code != "agent_process_lost"
        || !scenario.recovery.process_loss
        || scenario.recovery.remaining_agent_count != 1
        || !scenario.recovery.safe_to_continue_other_tasks
        || !scenario.result_verified
        || !scenario.remote_rejoined
        || !scenario.followup_solver_verified
    {
        return Err(format!("{} recovery evidence is incomplete", scenario.id));
    }
    match scenario.id.as_str() {
        "idempotent" => require_policy(scenario, "idempotent", true, "retry_next_agent", 1, 1)?,
        "side_effect_blocked" => require_policy(
            scenario,
            "checkpoint_required",
            false,
            "checkpoint_before_retry",
            0,
            0,
        )?,
        "checkpointed" => {
            require_policy(scenario, "checkpointed", true, "retry_next_agent", 1, 1)?;
            let digest = scenario
                .checkpoint_digest
                .as_deref()
                .filter(|digest| is_digest(digest))
                .ok_or("checkpointed scenario omitted checkpoint digest")?;
            if scenario.recovery.checkpoint_digest.as_deref() != Some(digest) {
                return Err("checkpoint digest did not survive recovery receipt".to_string());
            }
        }
        _ => return Err("unknown distributed recovery scenario".to_string()),
    }
    if scenario.fallback_delta.failed != 0 {
        return Err(format!("{} fallback Agent recorded a failure", scenario.id));
    }
    Ok(())
}

fn require_policy(
    scenario: &ScenarioEvidence,
    safety: &str,
    retryable: bool,
    next_action: &str,
    fallback_started: u64,
    fallback_completed: u64,
) -> RunnerResult<()> {
    if scenario.retry_safety != safety
        || scenario.recovery.retry_safety != safety
        || scenario.recovery.retryable != retryable
        || scenario.recovery.next_action != next_action
        || scenario.fallback_delta.started != fallback_started
        || scenario.fallback_delta.completed != fallback_completed
    {
        return Err(format!("{} replay policy is invalid", scenario.id));
    }
    Ok(())
}

fn cleanup_complete(cleanup: &CleanupEvidence) -> bool {
    cleanup.local_agent_stopped
        && cleanup.remote_agent_stopped
        && cleanup.local_port_closed
        && cleanup.remote_port_closed
        && cleanup.managed_remote_root_removed
        && cleanup.local_work_root_removed
}

fn required_string(value: &Value, key: &str) -> RunnerResult<String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .ok_or_else(|| format!("recovery omitted {key}"))
}

fn required_bool(value: &Value, key: &str) -> RunnerResult<bool> {
    value
        .get(key)
        .and_then(Value::as_bool)
        .ok_or_else(|| format!("recovery omitted {key}"))
}

fn close_to(value: &Value, key: &str, expected: f64, tolerance: f64) -> bool {
    value
        .get(key)
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
    let actual = actual.collect::<BTreeSet<_>>();
    let expected = expected.collect::<BTreeSet<_>>();
    if actual == expected {
        Ok(())
    } else {
        Err(format!("{label} do not match the contract"))
    }
}

pub(crate) fn validator_self_test(root: &Path) -> RunnerResult<()> {
    let mut report = build_report(fixture_capture())?;
    validate(root, &report)?;
    report
        .scenarios
        .iter_mut()
        .find(|scenario| scenario.id == "side_effect_blocked")
        .ok_or("fixture misses side-effect scenario")?
        .fallback_delta
        .started = 1;
    if validate(root, &report).is_ok() {
        return Err("validator accepted unsafe side-effect replay".to_string());
    }
    Ok(())
}

fn fixture_capture() -> Captured {
    Captured {
        remote_architecture: "x86_64".to_string(),
        remote_restart_count: 3,
        scenarios: vec![
            fixture_scenario("idempotent", "solve_bar_1d", "idempotent", true, 1),
            fixture_scenario(
                "side_effect_blocked",
                "run_operator_task_ir",
                "checkpoint_required",
                false,
                0,
            ),
            fixture_scenario(
                "checkpointed",
                "run_operator_task_ir",
                "checkpointed",
                true,
                1,
            ),
        ],
        cleanup: CleanupCapture {
            local_agent_stopped: true,
            remote_agent_stopped: true,
            local_port_closed: true,
            remote_port_closed: true,
            managed_remote_root_removed: true,
            local_work_root_removed: true,
        },
    }
}

fn fixture_scenario(
    id: &str,
    method: &str,
    safety: &str,
    retryable: bool,
    fallback_delta: u64,
) -> ScenarioCapture {
    let checkpoint = (id == "checkpointed").then(|| "a".repeat(64));
    let outcome = match id {
        "idempotent" => "fallback_completed",
        "side_effect_blocked" => "replay_blocked",
        _ => "checkpoint_authorized_fallback",
    };
    let mut observations = json!({
        "outcome": outcome,
        "recovery": {
            "failure_stage": "receive",
            "reason_code": "agent_process_lost",
            "process_loss": true,
            "retry_safety": safety,
            "checkpoint_digest": checkpoint,
            "retryable": retryable,
            "remaining_agent_count": 1,
            "safe_to_continue_other_tasks": true,
            "next_action": if retryable { "retry_next_agent" } else { "checkpoint_before_retry" }
        }
    });
    if id == "idempotent" {
        observations["fallback_agent_id"] = json!("local-fallback");
        observations["result_max_stress"] = json!(10.0);
        observations["result_tip_displacement"] = json!(0.01);
    }
    if id == "checkpointed" {
        observations["checkpoint_digest"] = json!("a".repeat(64));
        observations["result_status"] = json!("verified_pending_engine_execution");
    }
    ScenarioCapture {
        id: id.to_string(),
        job_id: format!("distributed-recovery-{id}"),
        method: method.to_string(),
        primary_before: counters(2, 2, 0, 0),
        primary_inflight: counters(3, 2, 0, 1),
        fallback_before: counters(4, 4, 0, 0),
        fallback_after: counters(4 + fallback_delta, 4 + fallback_delta, 0, 0),
        probe: json!({
            "schema_version": PROBE_SCHEMA,
            "status": "pass",
            "scenario": id,
            "job_id": format!("distributed-recovery-{id}"),
            "progress_event_count": 3,
            "observations": observations
        }),
        remote_rejoined: true,
        followup_solver_verified: true,
    }
}

fn counters(started: u64, completed: u64, failed: u64, active: u64) -> ExecutionCounters {
    ExecutionCounters {
        started,
        completed,
        failed,
        active,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validator_rejects_unsafe_replay() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../..");
        validator_self_test(&root).expect("validator self-test");
    }
}
