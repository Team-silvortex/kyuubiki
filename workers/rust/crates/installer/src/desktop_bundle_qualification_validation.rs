use std::collections::{BTreeMap, BTreeSet};

use crate::desktop_bundle_qualification::{
    DESKTOP_BUNDLE_QUALIFICATION_SCHEMA_VERSION, DesktopBundlePayloadObservation,
    DesktopBundleQualificationReport,
};
use crate::desktop_bundle_store::DESKTOP_BUNDLE_ACTIVATION_SCHEMA_VERSION;

const JOURNEY: &str = "installer-managed-packaged-desktop-set-upgrade-and-rollback";
const COMPONENTS: [&str; 3] = ["hub", "installer", "workbench"];
const PHASES: [(&str, bool); 3] = [
    ("initial-install", false),
    ("upgraded-install", true),
    ("rollback", false),
];
const REQUIRED_CHECKS: [&str; 14] = [
    "initial_activation",
    "upgrade_activation",
    "rollback_activation",
    "active_after_upgrade",
    "active_after_rollback",
    "monotonic_generations",
    "all_component_payloads_changed",
    "rollback_payload_restored",
    "initial_three_shell_boot",
    "upgraded_three_shell_boot",
    "rollback_three_shell_boot",
    "runtime_version_aligned",
    "update_lock_clean",
    "staging_clean",
];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DesktopBundleQualificationSummary {
    pub platform: String,
    pub first_version: String,
    pub second_version: String,
    pub probe_count: usize,
    pub check_count: usize,
}

pub fn validate_desktop_bundle_qualification_report(
    report: &DesktopBundleQualificationReport,
) -> Result<DesktopBundleQualificationSummary, Vec<String>> {
    let mut errors = Vec::new();
    if report.schema_version != DESKTOP_BUNDLE_QUALIFICATION_SCHEMA_VERSION
        || report.status != "pass"
        || report.journey != JOURNEY
    {
        errors.push("desktop qualification report header is invalid".to_string());
    }
    if !matches!(report.platform.as_str(), "macos" | "linux" | "windows") {
        errors.push("desktop qualification platform is unsupported".to_string());
    }
    let expected_host_roles = [
        format!("local-{}-qualification-host", report.platform),
        format!("remote-{}-qualification-host", report.platform),
    ];
    if !expected_host_roles.contains(&report.execution_host_role) {
        errors.push("desktop qualification host role is inconsistent".to_string());
    }
    if report.runtime_version.is_empty()
        || report.first_version == report.second_version
        || !semver(&report.first_version)
        || !semver(&report.second_version)
    {
        errors.push("desktop qualification versions are invalid".to_string());
    }
    validate_payload(
        &report.first_payload,
        &report.first_version,
        "first payload",
        &mut errors,
    );
    validate_payload(
        &report.second_payload,
        &report.second_version,
        "second payload",
        &mut errors,
    );
    validate_payload(
        &report.rollback_payload,
        &report.first_version,
        "rollback payload",
        &mut errors,
    );
    validate_activations(report, &mut errors);
    validate_probes(report, &mut errors);
    validate_checks(report, &mut errors);
    if report.first_payload != report.rollback_payload
        || report.first_payload.payload_sha256 == report.second_payload.payload_sha256
        || !components_changed(&report.first_payload, &report.second_payload)
    {
        errors.push("desktop rollback did not restore a distinctly changed payload".to_string());
    }
    let installed = report
        .installed_versions
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    if installed
        != BTreeSet::from([
            report.first_version.as_str(),
            report.second_version.as_str(),
        ])
        || report.active_after_upgrade != report.second_version
        || report.active_after_rollback != report.first_version
    {
        errors.push("desktop installed or active version summary is inconsistent".to_string());
    }
    if errors.is_empty() {
        Ok(DesktopBundleQualificationSummary {
            platform: report.platform.clone(),
            first_version: report.first_version.clone(),
            second_version: report.second_version.clone(),
            probe_count: report.probes.len(),
            check_count: report.checks.len(),
        })
    } else {
        Err(errors)
    }
}

fn validate_payload(
    payload: &DesktopBundlePayloadObservation,
    expected_version: &str,
    label: &str,
    errors: &mut Vec<String>,
) {
    let ids = payload
        .components
        .iter()
        .map(|component| component.component_id.as_str())
        .collect::<BTreeSet<_>>();
    if payload.version != expected_version
        || !sha256(&payload.payload_sha256)
        || ids != BTreeSet::from(COMPONENTS)
        || payload.components.len() != COMPONENTS.len()
        || payload.components.iter().any(|component| {
            !sha256(&component.content_sha256)
                || !sha256(&component.entrypoint_sha256)
                || component.file_count == 0
        })
    {
        errors.push(format!("{label} is invalid"));
    }
}

fn validate_activations(report: &DesktopBundleQualificationReport, errors: &mut Vec<String>) {
    for (label, activation, version, previous, generation, digest) in [
        (
            "first",
            &report.first_activation,
            report.first_version.as_str(),
            None,
            1,
            report.first_payload.payload_sha256.as_str(),
        ),
        (
            "second",
            &report.second_activation,
            report.second_version.as_str(),
            Some(report.first_version.as_str()),
            2,
            report.second_payload.payload_sha256.as_str(),
        ),
        (
            "rollback",
            &report.rollback_activation,
            report.first_version.as_str(),
            Some(report.second_version.as_str()),
            3,
            report.rollback_payload.payload_sha256.as_str(),
        ),
    ] {
        if activation.schema_version != DESKTOP_BUNDLE_ACTIVATION_SCHEMA_VERSION
            || activation.version != version
            || activation.previous_version.as_deref() != previous
            || activation.generation != generation
            || activation.platform != report.platform
            || activation.relative_path != format!("versions/{version}")
            || activation.payload_sha256 != digest
        {
            errors.push(format!("{label} desktop activation is inconsistent"));
        }
    }
}

fn validate_probes(report: &DesktopBundleQualificationReport, errors: &mut Vec<String>) {
    let mut seen = BTreeSet::new();
    let payloads = BTreeMap::from([
        ("initial-install", &report.first_payload),
        ("upgraded-install", &report.second_payload),
        ("rollback", &report.rollback_payload),
    ]);
    for probe in &report.probes {
        let key = (probe.phase.as_str(), probe.component_id.as_str());
        let Some(payload) = payloads.get(probe.phase.as_str()) else {
            errors.push(format!("unexpected desktop probe phase: {}", probe.phase));
            continue;
        };
        let component = payload
            .components
            .iter()
            .find(|component| component.component_id == probe.component_id);
        let expected_version = if probe.phase == "upgraded-install" {
            &report.second_version
        } else {
            &report.first_version
        };
        if !seen.insert(key)
            || !COMPONENTS.contains(&probe.component_id.as_str())
            || probe.package_version != *expected_version
            || probe.runtime_version != report.runtime_version
            || !probe.success
            || probe.pid == 0
            || component
                .is_none_or(|component| component.entrypoint_sha256 != probe.executable_sha256)
        {
            errors.push(format!(
                "desktop boot probe is inconsistent: {}/{}",
                probe.phase, probe.component_id
            ));
        }
    }
    let expected = PHASES
        .into_iter()
        .flat_map(|(phase, _)| COMPONENTS.map(|component| (phase, component)))
        .collect::<BTreeSet<_>>();
    if seen != expected || report.probes.len() != expected.len() {
        errors.push("desktop qualification must contain nine unique boot probes".to_string());
    }
}

fn validate_checks(report: &DesktopBundleQualificationReport, errors: &mut Vec<String>) {
    let ids = report
        .checks
        .iter()
        .map(|check| check.id.as_str())
        .collect::<BTreeSet<_>>();
    if ids != BTreeSet::from(REQUIRED_CHECKS)
        || report.checks.len() != REQUIRED_CHECKS.len()
        || report.checks.iter().any(|check| !check.ok)
    {
        errors.push("desktop qualification check set is incomplete".to_string());
    }
}

fn components_changed(
    first: &DesktopBundlePayloadObservation,
    second: &DesktopBundlePayloadObservation,
) -> bool {
    let first = first
        .components
        .iter()
        .map(|component| {
            (
                component.component_id.as_str(),
                component.content_sha256.as_str(),
            )
        })
        .collect::<BTreeMap<_, _>>();
    second.components.iter().all(|component| {
        first.get(component.component_id.as_str()) != Some(&component.content_sha256.as_str())
    })
}

fn semver(value: &str) -> bool {
    let parts = value.split('.').collect::<Vec<_>>();
    parts.len() == 3
        && parts
            .iter()
            .all(|part| !part.is_empty() && part.parse::<u64>().is_ok())
}

fn sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

#[cfg(test)]
mod tests {
    use super::REQUIRED_CHECKS;

    #[test]
    fn required_check_ids_are_unique() {
        let unique = REQUIRED_CHECKS
            .into_iter()
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(unique.len(), REQUIRED_CHECKS.len());
    }
}
