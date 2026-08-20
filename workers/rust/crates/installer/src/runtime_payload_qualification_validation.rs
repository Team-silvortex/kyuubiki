use crate::RuntimeActivationRecord;
use crate::runtime_payload::RUNTIME_ACTIVATION_SCHEMA_VERSION;
use crate::runtime_payload_qualification::{
    RUNTIME_PAYLOAD_QUALIFICATION_SCHEMA_VERSION, RuntimePayloadQualificationReport,
};
use std::collections::BTreeSet;

const JOURNEY: &str = "sealed-installed-runtime-payload-upgrade-and-rollback";
const SERVICE_IDS: &[&str] = &["agent", "orchestrator", "frontend"];
const REQUIRED_CHECKS: &[&str] = &[
    "initial_activation",
    "upgrade_activation",
    "rollback_activation",
    "active_after_upgrade",
    "active_after_rollback",
    "monotonic_generations",
    "payload_changed",
    "rollback_payload_restored",
    "all_service_entries_executed",
    "immutable_versions_verified",
    "update_lock_clean",
    "staging_clean",
];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimePayloadQualificationSummary {
    pub execution_host_role: String,
    pub platform: String,
    pub first_version: String,
    pub second_version: String,
    pub restored_sha256: String,
    pub executed_probe_count: usize,
}

pub fn validate_runtime_payload_qualification_report(
    report: &RuntimePayloadQualificationReport,
) -> Result<RuntimePayloadQualificationSummary, Vec<String>> {
    let mut errors = Vec::new();
    validate_header(report, &mut errors);
    validate_activation(
        &report.first_activation,
        &report.platform,
        &report.first_version,
        None,
        1,
        "first_activation",
        &mut errors,
    );
    validate_activation(
        &report.second_activation,
        &report.platform,
        &report.second_version,
        Some(&report.first_version),
        2,
        "second_activation",
        &mut errors,
    );
    validate_activation(
        &report.rollback_activation,
        &report.platform,
        &report.first_version,
        Some(&report.second_version),
        3,
        "rollback_activation",
        &mut errors,
    );
    validate_payload_chain(report, &mut errors);
    validate_installed_versions(report, &mut errors);
    validate_probes(report, &mut errors);
    validate_checks(report, &mut errors);

    if errors.is_empty() {
        Ok(RuntimePayloadQualificationSummary {
            execution_host_role: report.execution_host_role.clone(),
            platform: report.platform.clone(),
            first_version: report.first_version.clone(),
            second_version: report.second_version.clone(),
            restored_sha256: report.rollback_payload_sha256.clone(),
            executed_probe_count: report.probes.len(),
        })
    } else {
        Err(errors)
    }
}

fn validate_header(report: &RuntimePayloadQualificationReport, errors: &mut Vec<String>) {
    if report.schema_version != RUNTIME_PAYLOAD_QUALIFICATION_SCHEMA_VERSION {
        errors.push("schema_version is not supported".to_string());
    }
    if report.status != "pass" {
        errors.push("status must be pass".to_string());
    }
    if report.journey != JOURNEY {
        errors.push(format!("journey must be {JOURNEY}"));
    }
    if !valid_platform(&report.platform) {
        errors.push("platform must be macos, linux, or windows".to_string());
    }
    let expected_local = format!("local-{}-qualification-host", report.platform);
    let expected_remote = format!("remote-{}-qualification-host", report.platform);
    if report.execution_host_role != expected_local && report.execution_host_role != expected_remote
    {
        errors.push("execution_host_role does not match platform".to_string());
    }
    if !valid_version(&report.first_version) || !valid_version(&report.second_version) {
        errors.push("qualification versions are invalid".to_string());
    }
    if report.first_version == report.second_version {
        errors.push("qualification versions must be distinct".to_string());
    }
}

#[allow(clippy::too_many_arguments)]
fn validate_activation(
    activation: &RuntimeActivationRecord,
    platform: &str,
    version: &str,
    previous_version: Option<&String>,
    generation: u64,
    label: &str,
    errors: &mut Vec<String>,
) {
    if activation.schema_version != RUNTIME_ACTIVATION_SCHEMA_VERSION {
        errors.push(format!("{label}.schema_version is not supported"));
    }
    if activation.generation != generation {
        errors.push(format!("{label}.generation must be {generation}"));
    }
    if activation.version != version {
        errors.push(format!("{label}.version does not match the version chain"));
    }
    if activation.previous_version.as_ref() != previous_version {
        errors.push(format!(
            "{label}.previous_version does not match the version chain"
        ));
    }
    if activation.relative_path != format!("versions/{version}") {
        errors.push(format!("{label}.relative_path is not canonical"));
    }
    if activation.platform != platform {
        errors.push(format!("{label}.platform does not match the report"));
    }
}

fn validate_payload_chain(report: &RuntimePayloadQualificationReport, errors: &mut Vec<String>) {
    for (label, digest) in [
        ("first_payload_sha256", &report.first_payload_sha256),
        ("second_payload_sha256", &report.second_payload_sha256),
        ("rollback_payload_sha256", &report.rollback_payload_sha256),
    ] {
        if !lower_hex_digest(digest) {
            errors.push(format!("{label} is malformed"));
        }
    }
    if report.first_payload_sha256 == report.second_payload_sha256 {
        errors.push("upgrade must change immutable runtime content".to_string());
    }
    if report.rollback_payload_sha256 != report.first_payload_sha256 {
        errors.push("rollback payload digest must equal the initial payload digest".to_string());
    }
    if report.active_after_upgrade != report.second_version {
        errors.push("active_after_upgrade must equal second_version".to_string());
    }
    if report.active_after_rollback != report.first_version {
        errors.push("active_after_rollback must equal first_version".to_string());
    }
}

fn validate_installed_versions(
    report: &RuntimePayloadQualificationReport,
    errors: &mut Vec<String>,
) {
    let observed = report
        .installed_versions
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    let expected = [
        report.first_version.as_str(),
        report.second_version.as_str(),
    ]
    .into_iter()
    .collect::<BTreeSet<_>>();
    if observed != expected || report.installed_versions.len() != 2 {
        errors.push("installed_versions must contain exactly both qualified versions".to_string());
    }
}

fn validate_probes(report: &RuntimePayloadQualificationReport, errors: &mut Vec<String>) {
    let expected = [
        ("initial-install", report.first_version.as_str()),
        ("upgraded-install", report.second_version.as_str()),
        ("rollback", report.first_version.as_str()),
    ];
    if report.probes.len() != expected.len() * SERVICE_IDS.len() {
        errors.push("probes must cover all three services in every lifecycle phase".to_string());
        return;
    }
    for (phase_index, (phase, version)) in expected.iter().enumerate() {
        for (service_index, service_id) in SERVICE_IDS.iter().enumerate() {
            let index = phase_index * SERVICE_IDS.len() + service_index;
            let probe = &report.probes[index];
            if probe.phase != *phase || probe.version != *version || probe.service_id != *service_id
            {
                errors.push(format!("probes/{index} does not match the lifecycle order"));
            }
            if !probe.success || !probe.job_id_observed {
                errors.push(format!("probes/{index} did not prove executable success"));
            }
        }
    }
}

fn validate_checks(report: &RuntimePayloadQualificationReport, errors: &mut Vec<String>) {
    let observed = report
        .checks
        .iter()
        .map(|check| check.id.as_str())
        .collect::<BTreeSet<_>>();
    let expected = REQUIRED_CHECKS.iter().copied().collect::<BTreeSet<_>>();
    if observed != expected || report.checks.len() != REQUIRED_CHECKS.len() {
        errors.push("checks must contain the exact qualification check set".to_string());
    }
    for check in &report.checks {
        if !check.ok {
            errors.push(format!("check {} must pass", check.id));
        }
    }
}

fn valid_platform(value: &str) -> bool {
    matches!(value, "macos" | "linux" | "windows")
}

fn valid_version(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
}

fn lower_hex_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

#[cfg(test)]
mod tests {
    use super::validate_runtime_payload_qualification_report;
    use crate::runtime_payload::RUNTIME_ACTIVATION_SCHEMA_VERSION;
    use crate::{
        RUNTIME_PAYLOAD_QUALIFICATION_SCHEMA_VERSION, RuntimeActivationRecord,
        RuntimePayloadExecutionProbe, RuntimePayloadQualificationCheck,
        RuntimePayloadQualificationReport,
    };

    #[test]
    fn accepts_a_complete_runtime_update_chain() {
        validate_runtime_payload_qualification_report(&fixture()).unwrap();
    }

    #[test]
    fn rejects_metadata_only_runtime_rollback() {
        let mut report = fixture();
        report.rollback_payload_sha256 = report.second_payload_sha256.clone();
        let errors = validate_runtime_payload_qualification_report(&report).unwrap_err();
        assert!(
            errors
                .iter()
                .any(|error| error.contains("rollback payload digest"))
        );
    }

    fn fixture() -> RuntimePayloadQualificationReport {
        let first = "1".repeat(64);
        let second = "2".repeat(64);
        RuntimePayloadQualificationReport {
            schema_version: RUNTIME_PAYLOAD_QUALIFICATION_SCHEMA_VERSION.to_string(),
            status: "pass".to_string(),
            journey: "sealed-installed-runtime-payload-upgrade-and-rollback".to_string(),
            execution_host_role: "remote-linux-qualification-host".to_string(),
            platform: "linux".to_string(),
            first_version: "2.14.2".to_string(),
            second_version: "2.14.3".to_string(),
            first_activation: activation(1, "2.14.2", None),
            second_activation: activation(2, "2.14.3", Some("2.14.2")),
            rollback_activation: activation(3, "2.14.2", Some("2.14.3")),
            active_after_upgrade: "2.14.3".to_string(),
            active_after_rollback: "2.14.2".to_string(),
            installed_versions: vec!["2.14.2".to_string(), "2.14.3".to_string()],
            first_payload_sha256: first.clone(),
            second_payload_sha256: second,
            rollback_payload_sha256: first,
            probes: [
                ("initial-install", "2.14.2"),
                ("upgraded-install", "2.14.3"),
                ("rollback", "2.14.2"),
            ]
            .into_iter()
            .flat_map(|(phase, version)| {
                ["agent", "orchestrator", "frontend"].map(move |service_id| {
                    RuntimePayloadExecutionProbe {
                        phase: phase.to_string(),
                        version: version.to_string(),
                        service_id: service_id.to_string(),
                        success: true,
                        job_id_observed: true,
                    }
                })
            })
            .collect(),
            checks: [
                "initial_activation",
                "upgrade_activation",
                "rollback_activation",
                "active_after_upgrade",
                "active_after_rollback",
                "monotonic_generations",
                "payload_changed",
                "rollback_payload_restored",
                "all_service_entries_executed",
                "immutable_versions_verified",
                "update_lock_clean",
                "staging_clean",
            ]
            .into_iter()
            .map(|id| RuntimePayloadQualificationCheck {
                id: id.to_string(),
                ok: true,
            })
            .collect(),
        }
    }

    fn activation(
        generation: u64,
        version: &str,
        previous_version: Option<&str>,
    ) -> RuntimeActivationRecord {
        RuntimeActivationRecord {
            schema_version: RUNTIME_ACTIVATION_SCHEMA_VERSION.to_string(),
            generation,
            version: version.to_string(),
            previous_version: previous_version.map(str::to_string),
            relative_path: format!("versions/{version}"),
            platform: "linux".to_string(),
        }
    }
}
