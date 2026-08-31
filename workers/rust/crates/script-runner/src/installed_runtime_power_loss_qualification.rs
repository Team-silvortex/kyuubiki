use crate::qualification_support::{parse_options, read_json, repo_path};
use model::{QualificationReport, validate_report};
use serde::Deserialize;
use serde_json::Value;
use std::collections::BTreeSet;
use std::ffi::OsString;
use std::fs;
use std::path::Path;

type RunnerResult<T> = Result<T, String>;

mod host;
mod model;
mod remote;

pub(crate) const CONTRACT_PATH: &str =
    "config/architecture/installed-runtime-power-loss-qualification.json";
pub(crate) const CONTRACT_SCHEMA: &str =
    "kyuubiki.installed-runtime-power-loss-qualification-contract/v1";
pub(crate) const INTENT_SCHEMA: &str = "kyuubiki.installed-runtime-power-loss-intent/v1";
pub(crate) const CAPTURE_SCHEMA: &str = "kyuubiki.installed-runtime-power-loss-host-capture/v1";
pub(crate) const REPORT_SCHEMA: &str = "kyuubiki.installed-runtime-power-loss-qualification/v1";
pub(crate) const QUALIFICATION_ID: &str = "physical-linux-installed-runtime-power-loss-recovery";
pub(crate) const JOURNEY: &str = "installed-headless-orchestra-agent-across-host-reboot";
pub(crate) const REQUIRED_CHECKS: &[&str] = &[
    "remote_linux_host",
    "source_tree_detached",
    "installed_payload_verified",
    "intent_digest_verified",
    "pre_reboot_runtime_live",
    "pre_reboot_headless_solve_passed",
    "same_machine_after_reboot",
    "boot_identity_changed",
    "pre_reboot_processes_interrupted",
    "pre_reboot_ports_released",
    "installed_payload_persisted",
    "runtime_restarted_from_installation",
    "persisted_job_retrieved",
    "numerical_result_stable",
    "cleanup_complete",
    "retention_sanitized",
];

#[derive(Deserialize)]
pub(crate) struct Contract {
    schema_version: String,
    qualification_id: String,
    reboot_identity: RebootIdentity,
    persistent_state: PersistentState,
    pub(crate) execution: ExecutionContract,
    source_guard: SourceGuard,
    pub(crate) retention: Retention,
    required_checks: Vec<String>,
}

#[derive(Deserialize)]
struct RebootIdentity {
    boot_id_source: String,
    machine_id_source: String,
    require_changed_boot_id: bool,
    require_same_machine: bool,
}

#[derive(Deserialize)]
struct PersistentState {
    intent_schema: String,
    intent_schema_path: String,
    write_protocol: String,
    cleanup_policy: String,
    contains_absolute_paths: bool,
}

#[derive(Deserialize)]
pub(crate) struct ExecutionContract {
    pub(crate) execution_host_role: String,
    pub(crate) platform: String,
    pub(crate) architecture: String,
    pub(crate) package_version: String,
    pub(crate) workflow_id: String,
    pub(crate) minimum_agent_count: u64,
    pub(crate) frontend_loaded: bool,
    pub(crate) source_tree_detached: bool,
}

#[derive(Deserialize)]
struct SourceGuard {
    files: Vec<String>,
    required_text: Vec<String>,
}

#[derive(Deserialize)]
pub(crate) struct Retention {
    pub(crate) report_schema: String,
    pub(crate) report_schema_path: String,
    pub(crate) report_path: String,
    pub(crate) forbidden_content: Vec<String>,
}

pub(crate) fn run_remote(root: &Path, args: Vec<OsString>) -> RunnerResult<u8> {
    remote::run(root, args)
}

pub(crate) fn run_host(args: Vec<OsString>) -> RunnerResult<u8> {
    host::run(args)
}

pub(crate) fn run_check(root: &Path, args: Vec<OsString>) -> RunnerResult<u8> {
    let options = parse_options(args, "installed Runtime power-loss qualification")?;
    let contract = load_contract(root)?;
    validate_contract(root, &contract)?;
    if options.self_test {
        model::validator_self_test(&contract.retention.forbidden_content)?;
        println!("Installed Runtime power-loss qualification self-test passed");
        return Ok(0);
    }
    let path = options
        .verify_report
        .as_deref()
        .or(options.out.as_deref())
        .unwrap_or(&contract.retention.report_path);
    let report: QualificationReport = read_json(root, path)?;
    validate_report(&report, &contract.retention.forbidden_content)?;
    println!("Installed Runtime power-loss qualification report passed: {path}");
    Ok(0)
}

pub(crate) fn load_contract(root: &Path) -> RunnerResult<Contract> {
    read_json(root, CONTRACT_PATH)
}

pub(crate) fn validate_contract(root: &Path, contract: &Contract) -> RunnerResult<()> {
    let execution = &contract.execution;
    if contract.schema_version != CONTRACT_SCHEMA
        || contract.qualification_id != QUALIFICATION_ID
        || contract.reboot_identity.boot_id_source != "/proc/sys/kernel/random/boot_id"
        || contract.reboot_identity.machine_id_source != "/etc/machine-id"
        || !contract.reboot_identity.require_changed_boot_id
        || !contract.reboot_identity.require_same_machine
        || contract.persistent_state.intent_schema != INTENT_SCHEMA
        || contract.persistent_state.intent_schema_path
            != "schemas/installed-runtime-power-loss-intent.schema.json"
        || contract.persistent_state.write_protocol != "create-sync-rename-directory-sync"
        || contract.persistent_state.cleanup_policy != "validated-session-and-intent-only"
        || contract.persistent_state.contains_absolute_paths
        || execution.execution_host_role != "remote-linux-qualification-host"
        || execution.platform != "linux"
        || execution.architecture != "x86_64"
        || !valid_version(&execution.package_version)
        || execution.workflow_id
            != crate::installed_runtime_operational_qualification::support::WORKFLOW_ID
        || execution.minimum_agent_count < 2
        || execution.frontend_loaded
        || !execution.source_tree_detached
    {
        return Err("installed Runtime power-loss contract execution is invalid".into());
    }
    let checks = contract
        .required_checks
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    if checks != REQUIRED_CHECKS.iter().copied().collect() {
        return Err("installed Runtime power-loss required checks drifted".into());
    }
    if contract.retention.report_schema != REPORT_SCHEMA
        || contract.retention.report_schema_path
            != "schemas/installed-runtime-power-loss-qualification-report.schema.json"
        || !contract
            .retention
            .report_path
            .starts_with("releases/usability-evidence/")
    {
        return Err("installed Runtime power-loss retention contract is invalid".into());
    }
    for (path, schema) in [
        (
            "schemas/installed-runtime-power-loss-qualification-contract.schema.json",
            CONTRACT_SCHEMA,
        ),
        (
            "schemas/installed-runtime-power-loss-intent.schema.json",
            INTENT_SCHEMA,
        ),
        (&contract.retention.report_schema_path, REPORT_SCHEMA),
    ] {
        validate_schema_const(root, path, schema)?;
    }
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
    }
    for required in &guard.required_text {
        if !source.contains(required) {
            return Err(format!(
                "installed Runtime power-loss source guard misses {required}"
            ));
        }
    }
    Ok(())
}

pub(crate) fn valid_version(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'_'))
}
