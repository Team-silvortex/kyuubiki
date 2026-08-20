use crate::qualification_support::{
    generated_at_unix_ms, parse_options, read_json, repo_path, write_json,
};
use serde::Deserialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::ffi::OsString;
use std::fs;
use std::path::Path;

type RunnerResult<T> = Result<T, String>;

const CONTRACT_PATH: &str = "config/architecture/orchestra-workflow-operational-qualification.json";
const CONTRACT_SCHEMA_PATH: &str =
    "schemas/orchestra-workflow-operational-qualification-contract.schema.json";
const CONTRACT_SCHEMA: &str = "kyuubiki.orchestra-workflow-operational-qualification-contract/v1";
const REPORT_SCHEMA: &str = "kyuubiki.orchestra-workflow-operational-qualification/v1";
const DEFAULT_OUT: &str = "tmp/orchestra-workflow-operational-qualification.json";
const JOURNEY: &str = "remote-distributed-workflow-restart-recovery";
const REQUIRED_CHECKS: &[&str] = &[
    "remote_linux_capture",
    "workflow_mesh_modes_covered",
    "secure_agent_registration",
    "async_workflow_completed",
    "numerical_result_verified",
    "orchestra_process_restart",
    "result_retained_after_restart",
    "unauthorized_submission_rejected",
    "malformed_submission_rejected",
    "rejected_submission_no_job",
    "agent_task_tamper_rejected",
    "agent_task_recovered",
    "process_loss_policy_verified",
    "cleanup_complete",
];

#[derive(Deserialize)]
struct Contract {
    schema_version: String,
    qualification_id: String,
    target_coordinate: TargetCoordinate,
    capture: CaptureContract,
    source_guard: SourceGuard,
    retention: Retention,
    required_checks: Vec<String>,
}

#[derive(Deserialize)]
struct TargetCoordinate {
    module_id: String,
    paradigm: String,
    target_grade: String,
}

#[derive(Deserialize)]
struct CaptureContract {
    workflow_mesh_summary: String,
    operational_probe: String,
    agent_task_ir_report: String,
    process_loss_report: String,
    execution_host_role: String,
    platform: String,
    minimum_test_count: u64,
    minimum_pass_count: u64,
    required_cases: Vec<RequiredCase>,
}

#[derive(Deserialize)]
struct RequiredCase {
    id: String,
    test_file: String,
    subtest: String,
}

#[derive(Deserialize)]
struct SourceGuard {
    files: Vec<String>,
    required_text: Vec<String>,
}

#[derive(Deserialize)]
struct Retention {
    report_schema: String,
    report_schema_path: String,
    report_path: String,
    forbidden_content: Vec<String>,
}

struct Captures {
    summary: Value,
    probe: Value,
    agent: Value,
    recovery: Value,
    summary_digest: String,
    probe_digest: String,
    agent_digest: String,
    recovery_digest: String,
}

pub(crate) fn run_check_orchestra_workflow_operational_qualification(
    root: &Path,
    args: Vec<OsString>,
) -> RunnerResult<u8> {
    let options = parse_options(args, "Orchestra workflow operational qualification")?;
    let contract: Contract = read_json(root, CONTRACT_PATH)?;
    validate_contract(root, &contract)?;

    if options.self_test {
        run_self_test(&contract)?;
        println!("Orchestra workflow operational qualification self-test passed");
        return Ok(0);
    }
    if let Some(path) = options.verify_report {
        let report: Value = read_json(root, &path)?;
        validate_report(&contract, &report)?;
        println!("Orchestra workflow operational qualification report passed: {path}");
        return Ok(0);
    }

    let captures = load_captures(root, &contract.capture)?;
    validate_captures(&contract, &captures)?;
    let report = build_report(&contract, &captures)?;
    validate_report(&contract, &report)?;
    let out = options.out.as_deref().unwrap_or(DEFAULT_OUT);
    write_json(root, out, &report)?;
    println!("Orchestra workflow operational qualification passed");
    println!("Operational qualification report written: {out}");
    Ok(0)
}

fn validate_contract(root: &Path, contract: &Contract) -> RunnerResult<()> {
    if contract.schema_version != CONTRACT_SCHEMA
        || contract.qualification_id != "remote-linux-orchestra-workflow-operational"
        || contract.target_coordinate.module_id != "orchestra-control-plane"
        || contract.target_coordinate.paradigm != "workflow_composition"
        || contract.target_coordinate.target_grade != "operational"
    {
        return Err("Orchestra workflow operational target contract is invalid".into());
    }
    if contract.capture.execution_host_role != "remote-linux-qualification-host"
        || contract.capture.platform != "linux"
        || contract.capture.minimum_test_count < 4
        || contract.capture.minimum_pass_count < 4
        || contract.capture.required_cases.len() != 4
    {
        return Err("Orchestra workflow operational capture thresholds are invalid".into());
    }
    require_exact_set(
        contract.required_checks.iter().map(String::as_str),
        REQUIRED_CHECKS.iter().copied(),
        "required checks",
    )?;
    require_unique(
        contract
            .capture
            .required_cases
            .iter()
            .map(|case| case.id.as_str()),
        "required case ids",
    )?;
    if contract.retention.report_schema != REPORT_SCHEMA
        || contract.retention.report_schema_path
            != "schemas/orchestra-workflow-operational-qualification-report.schema.json"
        || !contract
            .retention
            .report_path
            .starts_with("releases/usability-evidence/")
    {
        return Err("Orchestra workflow operational retention contract is invalid".into());
    }
    let forbidden = contract
        .retention
        .forbidden_content
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    for required in [
        "host_address",
        "hostname",
        "username",
        "credential",
        "absolute_host_path",
    ] {
        if !forbidden.contains(required) {
            return Err(format!(
                "retention contract misses forbidden content {required}"
            ));
        }
    }
    validate_schema_const(root, CONTRACT_SCHEMA_PATH, CONTRACT_SCHEMA)?;
    validate_schema_const(root, &contract.retention.report_schema_path, REPORT_SCHEMA)?;
    validate_source_guard(root, &contract.source_guard)
}

fn validate_schema_const(root: &Path, path: &str, expected: &str) -> RunnerResult<()> {
    let schema: Value = read_json(root, path)?;
    if schema
        .pointer("/properties/schema_version/const")
        .and_then(Value::as_str)
        != Some(expected)
    {
        return Err(format!("schema {path} does not enforce {expected}"));
    }
    Ok(())
}

fn validate_source_guard(root: &Path, guard: &SourceGuard) -> RunnerResult<()> {
    let mut source = String::new();
    for path in &guard.files {
        source.push_str(
            &fs::read_to_string(repo_path(root, path)?)
                .map_err(|error| format!("failed to read source guard {path}: {error}"))?,
        );
        source.push('\n');
    }
    for required in &guard.required_text {
        if !source.contains(required) {
            return Err(format!("Orchestra workflow source guard misses {required}"));
        }
    }
    Ok(())
}

fn load_captures(root: &Path, capture: &CaptureContract) -> RunnerResult<Captures> {
    let (summary, summary_digest) = read_capture(root, &capture.workflow_mesh_summary)?;
    let (probe, probe_digest) = read_capture(root, &capture.operational_probe)?;
    let (agent, agent_digest) = read_capture(root, &capture.agent_task_ir_report)?;
    let (recovery, recovery_digest) = read_capture(root, &capture.process_loss_report)?;
    Ok(Captures {
        summary,
        probe,
        agent,
        recovery,
        summary_digest,
        probe_digest,
        agent_digest,
        recovery_digest,
    })
}

fn read_capture(root: &Path, path: &str) -> RunnerResult<(Value, String)> {
    let bytes = fs::read(repo_path(root, path)?)
        .map_err(|error| format!("failed to read operational capture {path}: {error}"))?;
    let value = serde_json::from_slice(&bytes)
        .map_err(|error| format!("invalid operational capture {path}: {error}"))?;
    Ok((value, digest(&bytes)))
}

fn validate_captures(contract: &Contract, captures: &Captures) -> RunnerResult<()> {
    validate_mesh_summary(contract, &captures.summary)?;
    validate_operational_probe(contract, &captures.probe)?;
    validate_agent_report(&captures.agent)?;
    validate_recovery_report(&captures.recovery)
}

fn validate_mesh_summary(contract: &Contract, summary: &Value) -> RunnerResult<()> {
    require_str(
        summary,
        "/schema_version",
        "kyuubiki.workflow-mesh-regression-summary/v1",
    )?;
    require_str(summary, "/status", "passed")?;
    require_bool(summary, "/completed", true)?;
    require_str(
        summary,
        "/execution_host_role",
        &contract.capture.execution_host_role,
    )?;
    require_str(summary, "/platform/os", &contract.capture.platform)?;
    require_min_u64(summary, "/total_tests", contract.capture.minimum_test_count)?;
    require_min_u64(summary, "/total_pass", contract.capture.minimum_pass_count)?;
    require_u64(summary, "/total_fail", 0)?;
    let tests = summary
        .get("tests")
        .and_then(Value::as_array)
        .ok_or_else(|| "workflow mesh summary tests must be an array".to_string())?;
    for required in &contract.capture.required_cases {
        let case = tests
            .iter()
            .find(|case| case.get("test_file").and_then(Value::as_str) == Some(&required.test_file))
            .ok_or_else(|| format!("workflow mesh summary misses {}", required.id))?;
        require_str(case, "/subtest", &required.subtest)?;
        require_str(case, "/status", "passed")?;
        require_u64(case, "/pass", 1)?;
        require_u64(case, "/fail", 0)?;
    }
    require_suffix(
        summary,
        "/artifacts/orchestra_workflow_operational_probe",
        "orchestra-workflow-operational-probe.json",
    )?;
    require_suffix(
        summary,
        "/artifacts/agent_solver_qualification",
        "agent-task-ir-qualification.json",
    )
}

fn validate_operational_probe(contract: &Contract, probe: &Value) -> RunnerResult<()> {
    require_str(
        probe,
        "/schema_version",
        "kyuubiki.orchestra-workflow-operational-probe/v1",
    )?;
    require_str(probe, "/status", "pass")?;
    require_str(probe, "/journey", JOURNEY)?;
    require_str(
        probe,
        "/execution_host_role",
        &contract.capture.execution_host_role,
    )?;
    require_str(probe, "/platform/os", "linux")?;
    require_str(probe, "/orchestration/control_mode", "orch_managed")?;
    require_u64(probe, "/orchestration/registered_agent_count", 2)?;
    require_u64(probe, "/orchestration/orchestra_restart_count", 1)?;
    require_bool(probe, "/orchestration/result_retained_after_restart", true)?;
    require_str(
        probe,
        "/workflow/workflow_id",
        "workflow.distributed-heat-to-thermo-chain-16",
    )?;
    require_u64(probe, "/workflow/completed_node_count", 23)?;
    require_u64(probe, "/workflow/solve_node_count", 2)?;
    require_u64(probe, "/workflow/transform_node_count", 17)?;
    require_digest(probe, "/workflow/result_sha256")?;
    require_close(probe, "/workflow/max_temperature_delta", 100.0, 1.0e-12)?;
    require_close(
        probe,
        "/workflow/max_stress",
        57_462_686.567_164_175,
        1.0e-6,
    )?;
    require_close(probe, "/workflow/max_displacement", 0.0, 1.0e-12)?;
    for pointer in [
        "/boundaries/unauthorized_submission_rejected",
        "/boundaries/malformed_submission_rejected",
        "/cleanup/orchestrator_port_closed",
        "/cleanup/agent_ports_closed",
        "/cleanup/residue_free",
    ] {
        require_bool(probe, pointer, true)?;
    }
    require_bool(probe, "/boundaries/rejected_submission_created_job", false)
}

fn validate_agent_report(agent: &Value) -> RunnerResult<()> {
    require_str(
        agent,
        "/schema_version",
        "kyuubiki.agent-solver-qualification/v2",
    )?;
    require_str(agent, "/status", "passed")?;
    require_str(agent, "/transport", "tcp_framed_json")?;
    for stage in ["initial_execution", "recovery_execution"] {
        require_str(agent, &format!("/stages/{stage}/status"), "executed")?;
        require_bool(
            agent,
            &format!("/stages/{stage}/solver_execution_capability/accepted"),
            true,
        )?;
        require_bool(
            agent,
            &format!("/stages/{stage}/result_assertion/passed"),
            true,
        )?;
    }
    require_str(
        agent,
        "/stages/tamper_rejection/reason_code",
        "operator_task_digest_mismatch",
    )?;
    require_u64(agent, "/watchdog/active_execution_count", 0)
}

fn validate_recovery_report(recovery: &Value) -> RunnerResult<()> {
    require_str(
        recovery,
        "/schema_version",
        "kyuubiki.orchestra-process-loss-fault-injection/v1",
    )?;
    require_str(recovery, "/status", "pass")?;
    require_u64(recovery, "/scenario_count", 3)?;
    let scenarios = recovery
        .get("scenarios")
        .and_then(Value::as_array)
        .ok_or_else(|| "Orchestra process-loss scenarios must be an array".to_string())?;
    let idempotent = scenario(scenarios, "idempotent_task_process_loss_failover")?;
    require_bool(idempotent, "/observations/result_retained", true)?;
    require_str(
        idempotent,
        "/observations/recovery/retry_safety",
        "idempotent",
    )?;
    let blocked = scenario(scenarios, "side_effect_replay_blocked_without_checkpoint")?;
    require_bool(
        blocked,
        "/observations/duplicate_side_effect_prevented",
        true,
    )?;
    require_bool(
        blocked,
        "/observations/fallback_agent_received_request",
        false,
    )?;
    let checkpointed = scenario(scenarios, "checkpointed_side_effect_process_loss_failover")?;
    require_bool(
        checkpointed,
        "/observations/checkpointed_result_retained",
        true,
    )?;
    require_digest(checkpointed, "/observations/recovery/checkpoint_digest")
}

fn build_report(contract: &Contract, captures: &Captures) -> RunnerResult<Value> {
    let case_ids = contract
        .capture
        .required_cases
        .iter()
        .map(|case| case.id.clone())
        .collect::<Vec<_>>();
    Ok(json!({
        "schema_version": REPORT_SCHEMA,
        "generated_at_unix_ms": generated_at_unix_ms()?,
        "status": "pass",
        "qualification_id": contract.qualification_id,
        "journey": JOURNEY,
        "execution_host_role": contract.capture.execution_host_role,
        "platform": {
            "os": value_at(&captures.summary, "/platform/os")?,
            "architecture": value_at(&captures.summary, "/platform/architecture")?
        },
        "workflow_mesh": {
            "total_tests": value_at(&captures.summary, "/total_tests")?,
            "total_pass": value_at(&captures.summary, "/total_pass")?,
            "total_fail": value_at(&captures.summary, "/total_fail")?,
            "case_ids": case_ids,
            "orchestrated_mode": true,
            "offline_mesh_mode": true,
            "branch_diagnostics": true
        },
        "workflow": captures.probe["workflow"].clone(),
        "restart_recovery": captures.probe["orchestration"].clone(),
        "boundaries": captures.probe["boundaries"].clone(),
        "agent_task_ir": {
            "schema_version": captures.agent["schema_version"].clone(),
            "transport": captures.agent["transport"].clone(),
            "initial_execution": true,
            "tamper_rejected": true,
            "recovery_execution": true,
            "watchdog_quiescent": true
        },
        "process_loss_recovery": {
            "schema_version": captures.recovery["schema_version"].clone(),
            "scenario_count": captures.recovery["scenario_count"].clone(),
            "idempotent_retry": true,
            "unsafe_replay_blocked": true,
            "checkpointed_retry": true
        },
        "cleanup": captures.probe["cleanup"].clone(),
        "source_digests": {
            "workflow_mesh_summary_sha256": captures.summary_digest,
            "operational_probe_sha256": captures.probe_digest,
            "agent_task_ir_sha256": captures.agent_digest,
            "process_loss_sha256": captures.recovery_digest
        },
        "checks": contract.required_checks.iter().map(|id| json!({
            "id": id,
            "status": "pass"
        })).collect::<Vec<_>>()
    }))
}

fn validate_report(contract: &Contract, report: &Value) -> RunnerResult<()> {
    require_str(report, "/schema_version", REPORT_SCHEMA)?;
    require_str(report, "/status", "pass")?;
    require_str(report, "/qualification_id", &contract.qualification_id)?;
    require_str(report, "/journey", JOURNEY)?;
    require_str(
        report,
        "/execution_host_role",
        &contract.capture.execution_host_role,
    )?;
    require_str(report, "/platform/os", "linux")?;
    require_str(
        report,
        "/workflow/workflow_id",
        "workflow.distributed-heat-to-thermo-chain-16",
    )?;
    require_u64(report, "/workflow/completed_node_count", 23)?;
    require_u64(report, "/workflow/solve_node_count", 2)?;
    require_u64(report, "/workflow/transform_node_count", 17)?;
    require_close(report, "/workflow/max_temperature_delta", 100.0, 1.0e-12)?;
    require_close(
        report,
        "/workflow/max_stress",
        57_462_686.567_164_175,
        1.0e-6,
    )?;
    require_close(report, "/workflow/max_displacement", 0.0, 1.0e-12)?;
    require_str(report, "/restart_recovery/control_mode", "orch_managed")?;
    require_str(
        report,
        "/agent_task_ir/schema_version",
        "kyuubiki.agent-solver-qualification/v2",
    )?;
    require_str(report, "/agent_task_ir/transport", "tcp_framed_json")?;
    require_str(
        report,
        "/process_loss_recovery/schema_version",
        "kyuubiki.orchestra-process-loss-fault-injection/v1",
    )?;
    require_min_u64(
        report,
        "/workflow_mesh/total_tests",
        contract.capture.minimum_test_count,
    )?;
    require_min_u64(
        report,
        "/workflow_mesh/total_pass",
        contract.capture.minimum_pass_count,
    )?;
    require_u64(report, "/workflow_mesh/total_fail", 0)?;
    for pointer in [
        "/workflow_mesh/orchestrated_mode",
        "/workflow_mesh/offline_mesh_mode",
        "/workflow_mesh/branch_diagnostics",
        "/restart_recovery/result_retained_after_restart",
        "/boundaries/unauthorized_submission_rejected",
        "/boundaries/malformed_submission_rejected",
        "/agent_task_ir/initial_execution",
        "/agent_task_ir/tamper_rejected",
        "/agent_task_ir/recovery_execution",
        "/agent_task_ir/watchdog_quiescent",
        "/process_loss_recovery/idempotent_retry",
        "/process_loss_recovery/unsafe_replay_blocked",
        "/process_loss_recovery/checkpointed_retry",
        "/cleanup/orchestrator_port_closed",
        "/cleanup/agent_ports_closed",
        "/cleanup/residue_free",
    ] {
        require_bool(report, pointer, true)?;
    }
    require_bool(report, "/boundaries/rejected_submission_created_job", false)?;
    require_u64(report, "/restart_recovery/registered_agent_count", 2)?;
    require_u64(report, "/restart_recovery/orchestra_restart_count", 1)?;
    require_u64(report, "/process_loss_recovery/scenario_count", 3)?;
    require_digest(report, "/workflow/result_sha256")?;
    for pointer in [
        "/source_digests/workflow_mesh_summary_sha256",
        "/source_digests/operational_probe_sha256",
        "/source_digests/agent_task_ir_sha256",
        "/source_digests/process_loss_sha256",
    ] {
        require_digest(report, pointer)?;
    }
    let case_ids = report
        .pointer("/workflow_mesh/case_ids")
        .and_then(Value::as_array)
        .ok_or_else(|| "operational report case_ids must be an array".to_string())?;
    require_exact_set(
        case_ids.iter().filter_map(Value::as_str),
        contract
            .capture
            .required_cases
            .iter()
            .map(|case| case.id.as_str()),
        "operational report case ids",
    )?;
    let checks = report
        .get("checks")
        .and_then(Value::as_array)
        .ok_or_else(|| "operational report checks must be an array".to_string())?;
    if checks
        .iter()
        .any(|check| check.get("status").and_then(Value::as_str) != Some("pass"))
    {
        return Err("operational report contains a failed check".into());
    }
    require_exact_set(
        checks
            .iter()
            .filter_map(|check| check.get("id").and_then(Value::as_str)),
        contract.required_checks.iter().map(String::as_str),
        "operational report checks",
    )?;
    let rendered = serde_json::to_string(report)
        .map_err(|error| format!("failed to inspect operational report: {error}"))?;
    for forbidden in &contract.retention.forbidden_content {
        if rendered.contains(forbidden) {
            return Err(format!(
                "operational report retains forbidden content {forbidden}"
            ));
        }
    }
    Ok(())
}

fn run_self_test(contract: &Contract) -> RunnerResult<()> {
    let mut report = synthetic_report(contract);
    validate_report(contract, &report)?;
    report["workflow"]["max_stress"] = json!(1.0);
    if validate_report(contract, &report).is_ok() {
        return Err("self-test accepted tampered numerical evidence".into());
    }
    let mut report = synthetic_report(contract);
    report["cleanup"]["residue_free"] = Value::Bool(false);
    if validate_report(contract, &report).is_ok() {
        return Err("self-test accepted non-clean operational evidence".into());
    }
    Ok(())
}

fn synthetic_report(contract: &Contract) -> Value {
    let digest = "a".repeat(64);
    json!({
        "schema_version": REPORT_SCHEMA,
        "generated_at_unix_ms": 1,
        "status": "pass",
        "qualification_id": contract.qualification_id,
        "journey": JOURNEY,
        "execution_host_role": contract.capture.execution_host_role,
        "platform": {"os": "linux", "architecture": "x86_64"},
        "workflow_mesh": {
            "total_tests": 4, "total_pass": 4, "total_fail": 0,
            "case_ids": contract.capture.required_cases.iter().map(|case| &case.id).collect::<Vec<_>>(),
            "orchestrated_mode": true, "offline_mesh_mode": true, "branch_diagnostics": true
        },
        "workflow": {
            "workflow_id": "workflow.distributed-heat-to-thermo-chain-16",
            "completed_node_count": 23, "solve_node_count": 2, "transform_node_count": 17,
            "result_sha256": digest, "max_temperature_delta": 100.0,
            "max_stress": 57_462_686.567_164_175, "max_displacement": 0.0
        },
        "restart_recovery": {
            "control_mode": "orch_managed", "registered_agent_count": 2,
            "orchestra_restart_count": 1,
            "result_retained_after_restart": true
        },
        "boundaries": {
            "unauthorized_submission_rejected": true,
            "malformed_submission_rejected": true,
            "rejected_submission_created_job": false
        },
        "agent_task_ir": {
            "schema_version": "kyuubiki.agent-solver-qualification/v2",
            "transport": "tcp_framed_json",
            "initial_execution": true, "tamper_rejected": true,
            "recovery_execution": true, "watchdog_quiescent": true
        },
        "process_loss_recovery": {
            "schema_version": "kyuubiki.orchestra-process-loss-fault-injection/v1",
            "scenario_count": 3, "idempotent_retry": true,
            "unsafe_replay_blocked": true, "checkpointed_retry": true
        },
        "cleanup": {
            "orchestrator_port_closed": true, "agent_ports_closed": true, "residue_free": true
        },
        "source_digests": {
            "workflow_mesh_summary_sha256": "b".repeat(64),
            "operational_probe_sha256": "c".repeat(64),
            "agent_task_ir_sha256": "d".repeat(64),
            "process_loss_sha256": "e".repeat(64)
        },
        "checks": contract.required_checks.iter().map(|id| json!({"id": id, "status": "pass"})).collect::<Vec<_>>()
    })
}

fn scenario<'a>(scenarios: &'a [Value], id: &str) -> RunnerResult<&'a Value> {
    scenarios
        .iter()
        .find(|scenario| scenario.get("id").and_then(Value::as_str) == Some(id))
        .ok_or_else(|| format!("Orchestra process-loss report misses scenario {id}"))
}

fn value_at(value: &Value, pointer: &str) -> RunnerResult<Value> {
    value
        .pointer(pointer)
        .cloned()
        .ok_or_else(|| format!("capture misses {pointer}"))
}

fn require_str(value: &Value, pointer: &str, expected: &str) -> RunnerResult<()> {
    if value.pointer(pointer).and_then(Value::as_str) == Some(expected) {
        Ok(())
    } else {
        Err(format!("{pointer} must be {expected}"))
    }
}

fn require_bool(value: &Value, pointer: &str, expected: bool) -> RunnerResult<()> {
    if value.pointer(pointer).and_then(Value::as_bool) == Some(expected) {
        Ok(())
    } else {
        Err(format!("{pointer} must be {expected}"))
    }
}

fn require_u64(value: &Value, pointer: &str, expected: u64) -> RunnerResult<()> {
    if value.pointer(pointer).and_then(Value::as_u64) == Some(expected) {
        Ok(())
    } else {
        Err(format!("{pointer} must be {expected}"))
    }
}

fn require_min_u64(value: &Value, pointer: &str, minimum: u64) -> RunnerResult<()> {
    if value
        .pointer(pointer)
        .and_then(Value::as_u64)
        .is_some_and(|actual| actual >= minimum)
    {
        Ok(())
    } else {
        Err(format!("{pointer} must be at least {minimum}"))
    }
}

fn require_close(value: &Value, pointer: &str, expected: f64, tolerance: f64) -> RunnerResult<()> {
    if value
        .pointer(pointer)
        .and_then(Value::as_f64)
        .is_some_and(|actual| (actual - expected).abs() <= tolerance)
    {
        Ok(())
    } else {
        Err(format!("{pointer} is outside tolerance {tolerance}"))
    }
}

fn require_digest(value: &Value, pointer: &str) -> RunnerResult<()> {
    if value
        .pointer(pointer)
        .and_then(Value::as_str)
        .is_some_and(|text| {
            text.len() == 64
                && text
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        })
    {
        Ok(())
    } else {
        Err(format!("{pointer} must be a SHA-256 digest"))
    }
}

fn require_suffix(value: &Value, pointer: &str, suffix: &str) -> RunnerResult<()> {
    if value
        .pointer(pointer)
        .and_then(Value::as_str)
        .is_some_and(|text| text.ends_with(suffix))
    {
        Ok(())
    } else {
        Err(format!("{pointer} must end with {suffix}"))
    }
}

fn require_unique<'a>(values: impl Iterator<Item = &'a str>, label: &str) -> RunnerResult<()> {
    let values = values.collect::<Vec<_>>();
    if values.iter().all(|value| !value.is_empty())
        && values.iter().copied().collect::<BTreeSet<_>>().len() == values.len()
    {
        Ok(())
    } else {
        Err(format!("{label} must be unique and non-empty"))
    }
}

fn require_exact_set<'a, 'b>(
    actual: impl Iterator<Item = &'a str>,
    expected: impl Iterator<Item = &'b str>,
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

fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
