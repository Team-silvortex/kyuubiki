use super::{
    CAPTURE_SCHEMA, INTENT_SCHEMA, JOURNEY, QUALIFICATION_ID, REPORT_SCHEMA, REQUIRED_CHECKS,
};
use crate::installed_runtime_operational_qualification::support::Ports;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

type RunnerResult<T> = Result<T, String>;

const DIGEST_KEYS: &[&str] = &[
    "kyuubiki-cli",
    "kyuubiki-headless",
    "kyuubiki-runtime",
    "runtime-payload.json",
    "service-launch.json",
];
const PROCESS_ROLES: &[&str] = &["agent_one", "agent_two", "orchestrator"];

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PowerLossIntent {
    pub schema_version: String,
    pub payload: Preparation,
    pub intent_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Preparation {
    pub qualification_id: String,
    pub execution_host_role: String,
    pub platform: String,
    pub architecture: String,
    pub runtime_version: String,
    pub prepared_at_unix_ms: u128,
    pub machine_id_sha256: String,
    pub pre_boot_id_sha256: String,
    pub pre_uptime_seconds: u64,
    pub source_tree_detached: bool,
    pub payload_digests: BTreeMap<String, String>,
    pub ports: Ports,
    pub process_identities: Vec<ProcessIdentity>,
    pub workflow_id: String,
    pub job_id: String,
    pub numerical_result: NumericalResult,
    pub runtime_status_verified: bool,
    pub agent_count: u64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ProcessIdentity {
    pub role: String,
    pub process_id: u32,
    pub executable_sha256: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct NumericalResult {
    pub tip_displacement: f64,
    pub max_stress: f64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct HostCapture {
    pub schema_version: String,
    pub preparation: Preparation,
    pub intent_sha256: String,
    pub recovery: Recovery,
    pub runtime_cleanup: RuntimeCleanup,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Recovery {
    pub post_boot_id_sha256: String,
    pub post_machine_id_sha256: String,
    pub post_uptime_seconds: u64,
    pub boot_identity_changed: bool,
    pub same_machine: bool,
    pub interrupted_process_count: u64,
    pub pre_reboot_ports_released: bool,
    pub payload_digests: BTreeMap<String, String>,
    pub source_tree_detached: bool,
    pub runtime_policy: String,
    pub agent_count: u64,
    pub job_id: String,
    pub job_status: String,
    pub numerical_result: NumericalResult,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RuntimeCleanup {
    pub runtime_stopped: bool,
    pub ports_closed: bool,
    pub pid_files_removed: bool,
    pub residue_count: u64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct QualificationReport {
    pub schema_version: String,
    pub qualification_id: String,
    pub status: String,
    pub journey: String,
    pub execution_host_role: String,
    pub platform: String,
    pub architecture: String,
    pub runtime_version: String,
    pub generated_at_unix_ms: u128,
    pub preparation: Preparation,
    pub intent_sha256: String,
    pub recovery: Recovery,
    pub cleanup: Cleanup,
    pub checks: Vec<QualificationCheck>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Cleanup {
    pub runtime_stopped: bool,
    pub ports_closed: bool,
    pub pid_files_removed: bool,
    pub managed_remote_root_removed: bool,
    pub residue_count: u64,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct QualificationCheck {
    pub id: String,
    pub status: String,
}

pub(crate) fn seal_intent(payload: Preparation) -> RunnerResult<PowerLossIntent> {
    let intent = PowerLossIntent {
        schema_version: INTENT_SCHEMA.to_string(),
        intent_sha256: digest_serializable(&payload)?,
        payload,
    };
    validate_intent(&intent)?;
    Ok(intent)
}

pub(crate) fn validate_intent(intent: &PowerLossIntent) -> RunnerResult<()> {
    if intent.schema_version != INTENT_SCHEMA
        || intent.intent_sha256 != digest_serializable(&intent.payload)?
    {
        return Err("installed Runtime power-loss intent identity or digest is invalid".into());
    }
    validate_preparation(&intent.payload)
}

pub(crate) fn validate_capture(capture: &HostCapture) -> RunnerResult<()> {
    if capture.schema_version != CAPTURE_SCHEMA
        || capture.intent_sha256 != digest_serializable(&capture.preparation)?
    {
        return Err("installed Runtime power-loss host capture identity is invalid".into());
    }
    validate_preparation(&capture.preparation)?;
    validate_recovery(&capture.preparation, &capture.recovery)?;
    if !capture.runtime_cleanup.runtime_stopped
        || !capture.runtime_cleanup.ports_closed
        || !capture.runtime_cleanup.pid_files_removed
        || capture.runtime_cleanup.residue_count != 0
    {
        return Err("installed Runtime power-loss host cleanup is incomplete".into());
    }
    Ok(())
}

pub(crate) fn build_report(
    capture: HostCapture,
    generated_at_unix_ms: u128,
    forbidden_content: &[String],
) -> RunnerResult<QualificationReport> {
    validate_capture(&capture)?;
    let preparation = capture.preparation;
    let mut report = QualificationReport {
        schema_version: REPORT_SCHEMA.to_string(),
        qualification_id: QUALIFICATION_ID.to_string(),
        status: "pass".to_string(),
        journey: JOURNEY.to_string(),
        execution_host_role: preparation.execution_host_role.clone(),
        platform: preparation.platform.clone(),
        architecture: preparation.architecture.clone(),
        runtime_version: preparation.runtime_version.clone(),
        generated_at_unix_ms,
        preparation,
        intent_sha256: capture.intent_sha256,
        recovery: capture.recovery,
        cleanup: Cleanup {
            runtime_stopped: capture.runtime_cleanup.runtime_stopped,
            ports_closed: capture.runtime_cleanup.ports_closed,
            pid_files_removed: capture.runtime_cleanup.pid_files_removed,
            managed_remote_root_removed: true,
            residue_count: capture.runtime_cleanup.residue_count,
        },
        checks: Vec::new(),
    };
    report.checks = qualification_checks(&report, true);
    validate_report(&report, forbidden_content)?;
    Ok(report)
}

pub(crate) fn validate_report(
    report: &QualificationReport,
    forbidden_content: &[String],
) -> RunnerResult<()> {
    if report.schema_version != REPORT_SCHEMA
        || report.qualification_id != QUALIFICATION_ID
        || report.status != "pass"
        || report.journey != JOURNEY
        || report.execution_host_role != "remote-linux-qualification-host"
        || report.platform != "linux"
        || report.architecture.is_empty()
        || report.runtime_version != report.preparation.runtime_version
        || report.intent_sha256 != digest_serializable(&report.preparation)?
    {
        return Err("installed Runtime power-loss report identity is invalid".into());
    }
    validate_preparation(&report.preparation)?;
    validate_recovery(&report.preparation, &report.recovery)?;
    let serialized = serde_json::to_string(report).map_err(|error| error.to_string())?;
    let sanitized = forbidden_content
        .iter()
        .filter(|value| !value.is_empty())
        .all(|value| !serialized.contains(value));
    if report.checks != qualification_checks(report, sanitized)
        || report.checks.len() != REQUIRED_CHECKS.len()
        || report.checks.iter().any(|check| check.status != "pass")
    {
        return Err("installed Runtime power-loss checks failed or drifted".into());
    }
    Ok(())
}

fn validate_preparation(preparation: &Preparation) -> RunnerResult<()> {
    if preparation.qualification_id != QUALIFICATION_ID
        || preparation.execution_host_role != "remote-linux-qualification-host"
        || preparation.platform != "linux"
        || preparation.architecture.is_empty()
        || !super::valid_version(&preparation.runtime_version)
        || preparation.prepared_at_unix_ms == 0
        || !valid_digest(&preparation.machine_id_sha256)
        || !valid_digest(&preparation.pre_boot_id_sha256)
        || !preparation.source_tree_detached
        || preparation.workflow_id
            != crate::installed_runtime_operational_qualification::support::WORKFLOW_ID
        || preparation.job_id.is_empty()
        || !valid_result(preparation.numerical_result)
        || !preparation.runtime_status_verified
        || preparation.agent_count < 2
        || preparation.ports.all().into_iter().any(|port| port == 0)
    {
        return Err("installed Runtime power-loss preparation is invalid".into());
    }
    validate_digest_map(&preparation.payload_digests)?;
    validate_processes(&preparation.process_identities)
}

fn validate_recovery(preparation: &Preparation, recovery: &Recovery) -> RunnerResult<()> {
    if !valid_digest(&recovery.post_boot_id_sha256)
        || !valid_digest(&recovery.post_machine_id_sha256)
        || !recovery.boot_identity_changed
        || recovery.post_boot_id_sha256 == preparation.pre_boot_id_sha256
        || !recovery.same_machine
        || recovery.post_machine_id_sha256 != preparation.machine_id_sha256
        || recovery.interrupted_process_count != preparation.process_identities.len() as u64
        || !recovery.pre_reboot_ports_released
        || recovery.payload_digests != preparation.payload_digests
        || !recovery.source_tree_detached
        || recovery.runtime_policy != "installer-managed"
        || recovery.agent_count < 2
        || recovery.job_id != preparation.job_id
        || recovery.job_status != "completed"
        || !same_result(recovery.numerical_result, preparation.numerical_result)
    {
        return Err("installed Runtime power-loss recovery is invalid".into());
    }
    validate_digest_map(&recovery.payload_digests)
}

fn qualification_checks(
    report: &QualificationReport,
    retention_sanitized: bool,
) -> Vec<QualificationCheck> {
    let preparation = &report.preparation;
    let recovery = &report.recovery;
    let cleanup = &report.cleanup;
    let values = [
        ("remote_linux_host", report.platform == "linux"),
        (
            "source_tree_detached",
            preparation.source_tree_detached && recovery.source_tree_detached,
        ),
        (
            "installed_payload_verified",
            validate_digest_map(&preparation.payload_digests).is_ok(),
        ),
        (
            "intent_digest_verified",
            report.intent_sha256 == digest_serializable(preparation).unwrap_or_default(),
        ),
        (
            "pre_reboot_runtime_live",
            preparation.runtime_status_verified && preparation.process_identities.len() == 3,
        ),
        (
            "pre_reboot_headless_solve_passed",
            !preparation.job_id.is_empty() && valid_result(preparation.numerical_result),
        ),
        ("same_machine_after_reboot", recovery.same_machine),
        ("boot_identity_changed", recovery.boot_identity_changed),
        (
            "pre_reboot_processes_interrupted",
            recovery.interrupted_process_count == preparation.process_identities.len() as u64,
        ),
        (
            "pre_reboot_ports_released",
            recovery.pre_reboot_ports_released,
        ),
        (
            "installed_payload_persisted",
            recovery.payload_digests == preparation.payload_digests,
        ),
        (
            "runtime_restarted_from_installation",
            recovery.runtime_policy == "installer-managed" && recovery.agent_count >= 2,
        ),
        (
            "persisted_job_retrieved",
            recovery.job_id == preparation.job_id && recovery.job_status == "completed",
        ),
        (
            "numerical_result_stable",
            same_result(recovery.numerical_result, preparation.numerical_result),
        ),
        (
            "cleanup_complete",
            cleanup.runtime_stopped
                && cleanup.ports_closed
                && cleanup.pid_files_removed
                && cleanup.managed_remote_root_removed
                && cleanup.residue_count == 0,
        ),
        ("retention_sanitized", retention_sanitized),
    ];
    values
        .into_iter()
        .map(|(id, ok)| QualificationCheck {
            id: id.to_string(),
            status: if ok { "pass" } else { "fail" }.to_string(),
        })
        .collect()
}

fn validate_digest_map(values: &BTreeMap<String, String>) -> RunnerResult<()> {
    let keys = values.keys().map(String::as_str).collect::<BTreeSet<_>>();
    if keys != DIGEST_KEYS.iter().copied().collect()
        || values.values().any(|value| !valid_digest(value))
    {
        return Err("installed Runtime payload digest set is invalid".into());
    }
    Ok(())
}

fn validate_processes(values: &[ProcessIdentity]) -> RunnerResult<()> {
    let roles = values
        .iter()
        .map(|value| value.role.as_str())
        .collect::<BTreeSet<_>>();
    if values.len() != PROCESS_ROLES.len()
        || roles != PROCESS_ROLES.iter().copied().collect()
        || values
            .iter()
            .any(|value| value.process_id == 0 || !valid_digest(&value.executable_sha256))
    {
        return Err("installed Runtime process identity set is invalid".into());
    }
    Ok(())
}

fn valid_result(value: NumericalResult) -> bool {
    value.tip_displacement.is_finite()
        && value.tip_displacement > 0.0
        && value.max_stress.is_finite()
        && value.max_stress > 0.0
}

fn same_result(left: NumericalResult, right: NumericalResult) -> bool {
    left.tip_displacement == right.tip_displacement && left.max_stress == right.max_stress
}

fn valid_digest(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

pub(crate) fn digest_serializable(value: &impl Serialize) -> RunnerResult<String> {
    let bytes = serde_json::to_vec(value).map_err(|error| error.to_string())?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

pub(crate) fn validator_self_test(forbidden_content: &[String]) -> RunnerResult<()> {
    let preparation = fixture_preparation();
    let intent = seal_intent(preparation.clone())?;
    let capture = HostCapture {
        schema_version: CAPTURE_SCHEMA.to_string(),
        preparation,
        intent_sha256: intent.intent_sha256,
        recovery: Recovery {
            post_boot_id_sha256: "e".repeat(64),
            post_machine_id_sha256: "a".repeat(64),
            post_uptime_seconds: 2,
            boot_identity_changed: true,
            same_machine: true,
            interrupted_process_count: 3,
            pre_reboot_ports_released: true,
            payload_digests: intent.payload.payload_digests.clone(),
            source_tree_detached: true,
            runtime_policy: "installer-managed".into(),
            agent_count: 2,
            job_id: intent.payload.job_id.clone(),
            job_status: "completed".into(),
            numerical_result: intent.payload.numerical_result,
        },
        runtime_cleanup: RuntimeCleanup {
            runtime_stopped: true,
            ports_closed: true,
            pid_files_removed: true,
            residue_count: 0,
        },
    };
    let report = build_report(capture, 3, forbidden_content)?;
    validate_report(&report, forbidden_content)?;
    let mut forged = report;
    forged.recovery.numerical_result.max_stress += 1.0;
    if validate_report(&forged, forbidden_content).is_ok() {
        return Err("validator accepted a changed post-reboot result".into());
    }
    Ok(())
}

fn fixture_preparation() -> Preparation {
    Preparation {
        qualification_id: QUALIFICATION_ID.into(),
        execution_host_role: "remote-linux-qualification-host".into(),
        platform: "linux".into(),
        architecture: "x86_64".into(),
        runtime_version: "2.19.0".into(),
        prepared_at_unix_ms: 1,
        machine_id_sha256: "a".repeat(64),
        pre_boot_id_sha256: "b".repeat(64),
        pre_uptime_seconds: 1,
        source_tree_detached: true,
        payload_digests: DIGEST_KEYS
            .iter()
            .map(|key| ((*key).to_string(), "c".repeat(64)))
            .collect(),
        ports: Ports {
            orchestrator: 4100,
            agent_one: 5101,
            agent_two: 5102,
        },
        process_identities: PROCESS_ROLES
            .iter()
            .enumerate()
            .map(|(index, role)| ProcessIdentity {
                role: (*role).to_string(),
                process_id: index as u32 + 10,
                executable_sha256: "d".repeat(64),
            })
            .collect(),
        workflow_id: crate::installed_runtime_operational_qualification::support::WORKFLOW_ID
            .into(),
        job_id: "job-1".into(),
        numerical_result: NumericalResult {
            tip_displacement: 1.0,
            max_stress: 2.0,
        },
        runtime_status_verified: true,
        agent_count: 2,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn intent_digest_rejects_mutation() {
        let mut intent = seal_intent(fixture_preparation()).expect("intent");
        intent.payload.job_id.push_str("-mutated");
        assert!(validate_intent(&intent).is_err());
    }
}
