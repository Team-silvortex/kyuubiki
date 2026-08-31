use kyuubiki_installer::{
    AgentSolverOperationalQualificationReport, AgentUpdateActivationRecord,
    AgentUpdatePackageManifest, validate_agent_solver_operational_qualification_report,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

pub(crate) const CONTRACT_PATH: &str =
    "config/architecture/linux-host-power-loss-qualification.json";
pub(crate) const CONTRACT_SCHEMA: &str = "kyuubiki.linux-host-power-loss-qualification-contract/v1";
pub(crate) const INTENT_SCHEMA: &str = "kyuubiki.linux-host-power-loss-intent/v1";
pub(crate) const REPORT_SCHEMA: &str = "kyuubiki.linux-host-power-loss-qualification/v1";
pub(crate) const QUALIFICATION_ID: &str = "physical-linux-host-power-loss-recovery";
pub(crate) const JOURNEY: &str = "installer-managed-agent-engine-across-host-reboot";
pub(crate) const REQUIRED_CHECKS: &[&str] = &[
    "remote_linux_host",
    "intent_digest_verified",
    "same_machine_after_reboot",
    "boot_identity_changed",
    "prepared_agent_interrupted",
    "managed_package_persisted",
    "pre_reboot_solver_passed",
    "post_reboot_solver_passed",
    "solver_result_stable",
    "tamper_recovery_stable",
    "watchdog_quiescent",
    "state_cleanup_complete",
    "retention_sanitized",
];

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct HostPowerLossIntent {
    pub schema_version: String,
    pub payload: HostPowerLossPreparation,
    pub intent_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct HostPowerLossPreparation {
    pub qualification_id: String,
    pub execution_host_role: String,
    pub platform: String,
    pub architecture: String,
    pub runtime_version: String,
    pub prepared_at_unix_ms: u128,
    pub machine_id_sha256: String,
    pub pre_boot_id_sha256: String,
    pub pre_uptime_seconds: u64,
    pub package: AgentUpdatePackageManifest,
    pub activation: AgentUpdateActivationRecord,
    pub active_entrypoint_sha256: String,
    pub sentinel: SentinelObservation,
    pub preflight: AgentSolverOperationalQualificationReport,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SentinelObservation {
    pub process_id: u32,
    pub port: u16,
    pub executable_sha256: String,
    pub ready_before_reboot: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct HostPowerLossQualificationReport {
    pub schema_version: String,
    pub qualification_id: String,
    pub status: String,
    pub journey: String,
    pub execution_host_role: String,
    pub platform: String,
    pub architecture: String,
    pub runtime_version: String,
    pub generated_at_unix_ms: u128,
    pub preparation: HostPowerLossPreparation,
    pub intent_sha256: String,
    pub recovery: RebootRecoveryObservation,
    pub postflight: AgentSolverOperationalQualificationReport,
    pub cleanup: HostPowerLossCleanup,
    pub checks: Vec<QualificationCheck>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RebootRecoveryObservation {
    pub post_boot_id_sha256: String,
    pub post_machine_id_sha256: String,
    pub post_uptime_seconds: u64,
    pub boot_identity_changed: bool,
    pub same_machine: bool,
    pub sentinel_port_free_before_resume: bool,
    pub active_entrypoint_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct HostPowerLossCleanup {
    pub scope: String,
    pub state_root_removed: bool,
    pub sentinel_port_released: bool,
    pub residue_count: u64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct QualificationCheck {
    pub id: String,
    pub ok: bool,
}

pub(crate) struct QualificationSummary {
    pub runtime_version: String,
    pub architecture: String,
    pub check_count: usize,
}

pub(crate) fn validate_contract(root: &Path) -> Result<(), String> {
    let contract = read_value(&root.join(CONTRACT_PATH))?;
    require_str(&contract, "/schema_version", CONTRACT_SCHEMA)?;
    require_str(&contract, "/qualification_id", QUALIFICATION_ID)?;
    require_str(&contract, "/platform", "linux")?;
    require_str(
        &contract,
        "/reboot_identity/boot_id_source",
        "/proc/sys/kernel/random/boot_id",
    )?;
    require_str(
        &contract,
        "/reboot_identity/machine_id_source",
        "/etc/machine-id",
    )?;
    require_str(&contract, "/retention/report_schema", REPORT_SCHEMA)?;
    let configured = contract
        .pointer("/required_checks")
        .and_then(Value::as_array)
        .ok_or("host power-loss contract misses required_checks")?
        .iter()
        .filter_map(Value::as_str)
        .collect::<BTreeSet<_>>();
    if configured != REQUIRED_CHECKS.iter().copied().collect() {
        return Err("host power-loss contract required checks drifted".into());
    }
    validate_schema_const(
        root,
        "schemas/linux-host-power-loss-qualification-contract.schema.json",
        CONTRACT_SCHEMA,
    )?;
    validate_schema_const(
        root,
        "schemas/linux-host-power-loss-intent.schema.json",
        INTENT_SCHEMA,
    )?;
    validate_schema_const(
        root,
        "schemas/linux-host-power-loss-qualification-report.schema.json",
        REPORT_SCHEMA,
    )
}

pub(crate) fn validate_intent(intent: &HostPowerLossIntent) -> Result<(), String> {
    if intent.schema_version != INTENT_SCHEMA {
        return Err("host power-loss intent schema is invalid".into());
    }
    if intent.intent_sha256 != digest_serializable(&intent.payload)? {
        return Err("host power-loss intent digest mismatch".into());
    }
    let payload = &intent.payload;
    if payload.qualification_id != QUALIFICATION_ID
        || payload.execution_host_role != "remote-linux-qualification-host"
        || payload.platform != "linux"
        || payload.architecture.is_empty()
        || payload.runtime_version != payload.package.version
        || payload.activation.version != payload.package.version
        || payload.activation.platform != "linux"
        || payload.package.platform != "linux"
        || payload.active_entrypoint_sha256 != payload.package.entrypoint_sha256
        || payload.sentinel.executable_sha256 != payload.package.entrypoint_sha256
        || !payload.sentinel.ready_before_reboot
        || payload.sentinel.process_id == 0
        || payload.sentinel.port == 0
        || !valid_digest(&payload.machine_id_sha256)
        || !valid_digest(&payload.pre_boot_id_sha256)
    {
        return Err("host power-loss intent semantics are invalid".into());
    }
    validate_solver_report(&payload.preflight)?;
    if payload.preflight.execution_host_role != payload.execution_host_role
        || payload.preflight.platform != payload.platform
        || payload.preflight.architecture != payload.architecture
        || payload.preflight.package.version != payload.runtime_version
        || payload.preflight.package.entrypoint_sha256 != payload.active_entrypoint_sha256
    {
        return Err("pre-reboot Agent qualification does not match the sealed intent".into());
    }
    Ok(())
}

pub(crate) fn qualification_checks(
    report: &HostPowerLossQualificationReport,
) -> Vec<QualificationCheck> {
    let pre_valid = validate_solver_report(&report.preparation.preflight).is_ok();
    let post_valid = validate_solver_report(&report.postflight).is_ok();
    let stable_result = solver_values(&report.preparation.preflight, RESULT_POINTER)
        == solver_values(&report.postflight, RESULT_POINTER)
        && solver_values(&report.preparation.preflight, RECOVERY_POINTER)
            == solver_values(&report.postflight, RECOVERY_POINTER)
        && solver_values(&report.preparation.preflight, TASK_DIGEST_POINTER)
            == solver_values(&report.postflight, TASK_DIGEST_POINTER);
    let stable_recovery = solver_values(&report.preparation.preflight, TAMPER_POINTER)
        == solver_values(&report.postflight, TAMPER_POINTER)
        && solver_values(&report.preparation.preflight, UNSUPPORTED_POINTER)
            == solver_values(&report.postflight, UNSUPPORTED_POINTER);
    vec![
        check(
            "remote_linux_host",
            report.execution_host_role == "remote-linux-qualification-host"
                && report.platform == "linux",
        ),
        check(
            "intent_digest_verified",
            report.intent_sha256 == digest_serializable(&report.preparation).unwrap_or_default(),
        ),
        check(
            "same_machine_after_reboot",
            report.recovery.same_machine
                && report.recovery.post_machine_id_sha256 == report.preparation.machine_id_sha256,
        ),
        check(
            "boot_identity_changed",
            report.recovery.boot_identity_changed
                && report.recovery.post_boot_id_sha256 != report.preparation.pre_boot_id_sha256,
        ),
        check(
            "prepared_agent_interrupted",
            report.recovery.sentinel_port_free_before_resume,
        ),
        check(
            "managed_package_persisted",
            report.recovery.active_entrypoint_sha256 == report.preparation.active_entrypoint_sha256
                && report.postflight.package.entrypoint_sha256
                    == report.preparation.active_entrypoint_sha256,
        ),
        check("pre_reboot_solver_passed", pre_valid),
        check("post_reboot_solver_passed", post_valid),
        check("solver_result_stable", stable_result),
        check("tamper_recovery_stable", stable_recovery),
        check(
            "watchdog_quiescent",
            watchdog_quiescent(&report.preparation.preflight)
                && watchdog_quiescent(&report.postflight),
        ),
        check(
            "state_cleanup_complete",
            report.cleanup.scope == "qualification-state-root"
                && report.cleanup.state_root_removed
                && report.cleanup.sentinel_port_released
                && report.cleanup.residue_count == 0,
        ),
        check("retention_sanitized", portable_report(report)),
    ]
}

pub(crate) fn validate_report(
    report: &HostPowerLossQualificationReport,
) -> Result<QualificationSummary, String> {
    if report.schema_version != REPORT_SCHEMA
        || report.qualification_id != QUALIFICATION_ID
        || report.status != "pass"
        || report.journey != JOURNEY
        || report.execution_host_role != report.preparation.execution_host_role
        || report.platform != report.preparation.platform
        || report.architecture != report.preparation.architecture
        || report.runtime_version != report.preparation.runtime_version
        || !valid_digest(&report.recovery.post_boot_id_sha256)
        || !valid_digest(&report.recovery.post_machine_id_sha256)
        || report.intent_sha256 != digest_serializable(&report.preparation)?
    {
        return Err("host power-loss qualification report identity is invalid".into());
    }
    validate_intent(&HostPowerLossIntent {
        schema_version: INTENT_SCHEMA.to_string(),
        payload: report.preparation.clone(),
        intent_sha256: report.intent_sha256.clone(),
    })?;
    validate_solver_report(&report.postflight)?;
    if report.postflight.execution_host_role != report.execution_host_role
        || report.postflight.platform != report.platform
        || report.postflight.architecture != report.architecture
        || report.postflight.package.version != report.runtime_version
        || report.postflight.package.entrypoint_sha256
            != report.preparation.active_entrypoint_sha256
    {
        return Err("post-reboot Agent qualification does not match the reboot intent".into());
    }
    let expected = qualification_checks(report);
    if report.checks.len() != REQUIRED_CHECKS.len()
        || report
            .checks
            .iter()
            .map(|entry| entry.id.as_str())
            .collect::<BTreeSet<_>>()
            != REQUIRED_CHECKS.iter().copied().collect()
        || report.checks.iter().any(|entry| !entry.ok)
        || report.checks.iter().any(|entry| {
            expected
                .iter()
                .find(|item| item.id == entry.id)
                .is_none_or(|item| !item.ok)
        })
    {
        return Err("host power-loss qualification checks failed or drifted".into());
    }
    Ok(QualificationSummary {
        runtime_version: report.runtime_version.clone(),
        architecture: report.architecture.clone(),
        check_count: report.checks.len(),
    })
}

pub(crate) fn digest_serializable(value: &impl Serialize) -> Result<String, String> {
    let bytes = serde_json::to_vec(value).map_err(|error| error.to_string())?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

fn validate_solver_report(
    report: &AgentSolverOperationalQualificationReport,
) -> Result<(), String> {
    let value = serde_json::to_value(report).map_err(|error| error.to_string())?;
    validate_agent_solver_operational_qualification_report(&value)
        .map(|_| ())
        .map_err(|errors| errors.join("; "))
}

const RESULT_POINTER: &str = "/stages/initial_execution/result_assertion/actual";
const RECOVERY_POINTER: &str = "/stages/recovery_execution/result_assertion/actual";
const TASK_DIGEST_POINTER: &str = "/task_digest";
const TAMPER_POINTER: &str = "/stages/tamper_rejection/reason_code";
const UNSUPPORTED_POINTER: &str = "/stages/unsupported_solver_rejection/reason_code";

fn solver_values<'a>(
    report: &'a AgentSolverOperationalQualificationReport,
    pointer: &str,
) -> Option<Vec<&'a Value>> {
    report
        .solver_runs
        .iter()
        .map(|run| run.qualification.pointer(pointer))
        .collect()
}

fn watchdog_quiescent(report: &AgentSolverOperationalQualificationReport) -> bool {
    report.solver_runs.iter().all(|run| {
        run.qualification
            .pointer("/watchdog/active_execution_count")
            .and_then(Value::as_u64)
            == Some(0)
    })
}

fn portable_report(report: &HostPowerLossQualificationReport) -> bool {
    let Ok(text) = serde_json::to_string(report) else {
        return false;
    };
    let text = text.to_ascii_lowercase();
    [
        "/home/",
        "/users/",
        "192.168.",
        "kyuubiki-lab",
        "username",
        "credential",
    ]
    .iter()
    .all(|forbidden| !text.contains(forbidden))
}

fn check(id: &str, ok: bool) -> QualificationCheck {
    QualificationCheck {
        id: id.to_string(),
        ok,
    }
}

fn valid_digest(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn validate_schema_const(root: &Path, relative: &str, expected: &str) -> Result<(), String> {
    let schema = read_value(&root.join(relative))?;
    require_str(&schema, "/properties/schema_version/const", expected)
}

fn read_value(path: &Path) -> Result<Value, String> {
    let bytes =
        fs::read(path).map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    serde_json::from_slice(&bytes)
        .map_err(|error| format!("invalid JSON {}: {error}", path.display()))
}

fn require_str(value: &Value, pointer: &str, expected: &str) -> Result<(), String> {
    if value.pointer(pointer).and_then(Value::as_str) == Some(expected) {
        Ok(())
    } else {
        Err(format!("expected {pointer} to equal {expected}"))
    }
}
