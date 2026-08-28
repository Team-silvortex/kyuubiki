use crate::fleet_update::{
    FLEET_UPDATE_TRANSACTION_SCHEMA_VERSION, FleetUpdateComponentState, FleetUpdateSnapshot,
    FleetUpdateTransactionReceipt,
};
use crate::fleet_update_qualification::{
    FLEET_UPDATE_QUALIFICATION_JOURNEY, FLEET_UPDATE_QUALIFICATION_SCHEMA_VERSION,
    FLEET_UPDATE_REQUIRED_CHECKS, FleetUpdateExecutionProbe, FleetUpdateQualificationReport,
};
use std::collections::{BTreeMap, BTreeSet};

const PHASES: &[(&str, VersionPhase)] = &[
    ("initial", VersionPhase::First),
    ("compensated", VersionPhase::First),
    ("upgraded", VersionPhase::Second),
    ("rolled-back", VersionPhase::First),
];
const RUNTIME_PROBES: &[&str] = &["runtime.agent", "runtime.orchestrator", "runtime.frontend"];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FleetUpdateQualificationSummary {
    pub execution_host_role: String,
    pub platform: String,
    pub first_version: String,
    pub second_version: String,
    pub agent_count: usize,
    pub probe_count: usize,
}

#[derive(Clone, Copy)]
enum VersionPhase {
    First,
    Second,
}

pub fn validate_fleet_update_qualification_report(
    report: &FleetUpdateQualificationReport,
) -> Result<FleetUpdateQualificationSummary, Vec<String>> {
    let mut errors = Vec::new();
    validate_header(report, &mut errors);
    let expected_ids = expected_component_ids(report.agent_count);
    validate_snapshot(
        &report.initial,
        &report.first_version,
        &expected_ids,
        "initial",
        &mut errors,
    );
    validate_snapshot(
        &report.compensated_after_failure,
        &report.first_version,
        &expected_ids,
        "compensated_after_failure",
        &mut errors,
    );
    validate_receipt(
        &report.upgrade_transaction,
        "upgrade",
        &report.first_version,
        &report.second_version,
        &expected_ids,
        &mut errors,
    );
    validate_receipt(
        &report.rollback_transaction,
        "rollback",
        &report.second_version,
        &report.first_version,
        &expected_ids,
        &mut errors,
    );
    validate_failure(report, &mut errors);
    validate_digest_chain(report, &expected_ids, &mut errors);
    validate_generations(report, &expected_ids, &mut errors);
    validate_probes(report, &mut errors);
    validate_checks(report, &mut errors);

    if errors.is_empty() {
        Ok(FleetUpdateQualificationSummary {
            execution_host_role: report.execution_host_role.clone(),
            platform: report.platform.clone(),
            first_version: report.first_version.clone(),
            second_version: report.second_version.clone(),
            agent_count: report.agent_count,
            probe_count: report.probes.len(),
        })
    } else {
        Err(errors)
    }
}

fn validate_header(report: &FleetUpdateQualificationReport, errors: &mut Vec<String>) {
    if report.schema_version != FLEET_UPDATE_QUALIFICATION_SCHEMA_VERSION {
        errors.push("schema_version is not supported".into());
    }
    if report.status != "pass" || report.journey != FLEET_UPDATE_QUALIFICATION_JOURNEY {
        errors.push("fleet qualification status or journey is invalid".into());
    }
    if !matches!(report.platform.as_str(), "macos" | "linux" | "windows") {
        errors.push("platform is invalid".into());
    }
    let local = format!("local-{}-qualification-host", report.platform);
    let remote = format!("remote-{}-qualification-host", report.platform);
    if report.execution_host_role != local && report.execution_host_role != remote {
        errors.push("execution_host_role does not match platform".into());
    }
    if report.agent_count < 2 || report.agent_count > 8 {
        errors.push("agent_count must be between 2 and 8".into());
    }
    if report.first_version == report.second_version
        || !valid_version(&report.first_version)
        || !valid_version(&report.second_version)
    {
        errors.push("qualification versions are invalid".into());
    }
}

fn validate_snapshot(
    snapshot: &FleetUpdateSnapshot,
    version: &str,
    expected_ids: &BTreeSet<String>,
    label: &str,
    errors: &mut Vec<String>,
) {
    if snapshot.active_version != version {
        errors.push(format!("{label}.active_version must be {version}"));
    }
    validate_components(&snapshot.components, version, expected_ids, label, errors);
}

fn validate_receipt(
    receipt: &FleetUpdateTransactionReceipt,
    operation: &str,
    before: &str,
    active: &str,
    expected_ids: &BTreeSet<String>,
    errors: &mut Vec<String>,
) {
    if receipt.schema_version != FLEET_UPDATE_TRANSACTION_SCHEMA_VERSION
        || receipt.operation != operation
        || receipt.before_version != before
        || receipt.active_version != active
    {
        errors.push(format!(
            "{operation}_transaction identity or version chain is invalid"
        ));
    }
    validate_components(
        &receipt.components,
        active,
        expected_ids,
        &format!("{operation}_transaction"),
        errors,
    );
    for component in &receipt.components {
        if component.previous_version.as_deref() != Some(before) {
            errors.push(format!(
                "{operation}_transaction.components/{} previous version must be {before}",
                component.component_id
            ));
        }
    }
}

fn validate_components(
    components: &[FleetUpdateComponentState],
    version: &str,
    expected_ids: &BTreeSet<String>,
    label: &str,
    errors: &mut Vec<String>,
) {
    let observed = components
        .iter()
        .map(|component| component.component_id.clone())
        .collect::<BTreeSet<_>>();
    if observed != *expected_ids || components.len() != expected_ids.len() {
        errors.push(format!(
            "{label}.components do not match the fleet topology"
        ));
    }
    for component in components {
        let expected_role = if component.component_id == "runtime" {
            "runtime"
        } else {
            "agent"
        };
        if component.role != expected_role
            || component.active_version != version
            || component.generation == 0
            || !valid_digest(&component.payload_sha256)
        {
            errors.push(format!(
                "{label}.components/{} is invalid",
                component.component_id
            ));
        }
    }
}

fn validate_failure(report: &FleetUpdateQualificationReport, errors: &mut Vec<String>) {
    let failure = &report.failure_injection;
    if failure.failed_component_id != "agent-02"
        || failure.failure_class != "injected-fault"
        || !failure.compensated
        || failure.compensation_error_count != 0
    {
        errors.push("failure injection did not prove clean mid-fleet compensation".into());
    }
}

fn validate_digest_chain(
    report: &FleetUpdateQualificationReport,
    ids: &BTreeSet<String>,
    errors: &mut Vec<String>,
) {
    let initial = component_map(&report.initial.components);
    let compensated = component_map(&report.compensated_after_failure.components);
    let upgraded = component_map(&report.upgrade_transaction.components);
    let rollback = component_map(&report.rollback_transaction.components);
    for id in ids {
        let Some(first) = initial.get(id) else {
            continue;
        };
        let Some(after_failure) = compensated.get(id) else {
            continue;
        };
        let Some(second) = upgraded.get(id) else {
            continue;
        };
        let Some(restored) = rollback.get(id) else {
            continue;
        };
        if first.payload_sha256 == second.payload_sha256
            || first.payload_sha256 != after_failure.payload_sha256
            || first.payload_sha256 != restored.payload_sha256
        {
            errors.push(format!("component {id} payload digest chain is invalid"));
        }
    }
}

fn validate_generations(
    report: &FleetUpdateQualificationReport,
    ids: &BTreeSet<String>,
    errors: &mut Vec<String>,
) {
    let states = [
        component_map(&report.initial.components),
        component_map(&report.compensated_after_failure.components),
        component_map(&report.upgrade_transaction.components),
        component_map(&report.rollback_transaction.components),
    ];
    for id in ids {
        let generations = states
            .iter()
            .filter_map(|state| state.get(id).map(|component| component.generation))
            .collect::<Vec<_>>();
        if generations.len() != states.len()
            || generations[1] < generations[0]
            || generations[2] <= generations[1]
            || generations[3] <= generations[2]
        {
            errors.push(format!("component {id} generation chain is invalid"));
        }
    }
}

fn validate_probes(report: &FleetUpdateQualificationReport, errors: &mut Vec<String>) {
    let mut expected = BTreeSet::new();
    for (phase, _) in PHASES {
        for runtime in RUNTIME_PROBES {
            expected.insert(((*phase).to_string(), (*runtime).to_string()));
        }
        for index in 0..report.agent_count {
            expected.insert(((*phase).to_string(), format!("agent-{:02}", index + 1)));
        }
    }
    let observed = report
        .probes
        .iter()
        .map(|probe| (probe.phase.clone(), probe.component_id.clone()))
        .collect::<BTreeSet<_>>();
    if observed != expected || report.probes.len() != expected.len() {
        errors.push("probes do not cover every fleet component and phase exactly once".into());
    }
    for probe in &report.probes {
        validate_probe(report, probe, errors);
    }
}

fn validate_probe(
    report: &FleetUpdateQualificationReport,
    probe: &FleetUpdateExecutionProbe,
    errors: &mut Vec<String>,
) {
    let version = PHASES
        .iter()
        .find(|(phase, _)| *phase == probe.phase)
        .map(|(_, version)| match version {
            VersionPhase::First => report.first_version.as_str(),
            VersionPhase::Second => report.second_version.as_str(),
        });
    let expected_role = if probe.component_id.starts_with("runtime.") {
        "runtime-service"
    } else {
        "agent"
    };
    if version != Some(probe.version.as_str())
        || probe.role != expected_role
        || !probe.success
        || !probe.marker_observed
    {
        errors.push(format!(
            "probe {}/{} is invalid",
            probe.phase, probe.component_id
        ));
    }
}

fn validate_checks(report: &FleetUpdateQualificationReport, errors: &mut Vec<String>) {
    let expected = FLEET_UPDATE_REQUIRED_CHECKS
        .iter()
        .map(|value| (*value).to_string())
        .collect::<BTreeSet<_>>();
    let observed = report
        .checks
        .iter()
        .map(|check| check.id.clone())
        .collect::<BTreeSet<_>>();
    if observed != expected || report.checks.len() != expected.len() {
        errors.push("checks do not match the required fleet qualification set".into());
    }
    for check in &report.checks {
        if !check.ok {
            errors.push(format!("check {} must pass", check.id));
        }
    }
}

fn expected_component_ids(agent_count: usize) -> BTreeSet<String> {
    let mut ids = BTreeSet::from(["runtime".to_string()]);
    for index in 0..agent_count {
        ids.insert(format!("agent-{:02}", index + 1));
    }
    ids
}

fn component_map(
    components: &[FleetUpdateComponentState],
) -> BTreeMap<String, &FleetUpdateComponentState> {
    components
        .iter()
        .map(|component| (component.component_id.clone(), component))
        .collect()
}

fn valid_version(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
}

fn valid_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

#[cfg(test)]
mod tests {
    use super::{
        expected_component_ids, valid_digest, valid_version,
        validate_fleet_update_qualification_report,
    };
    use crate::FleetUpdateQualificationReport;

    #[test]
    fn validates_portable_identifiers_and_topology() {
        assert!(valid_version("2.18.0"));
        assert!(!valid_version("../2.18.0"));
        assert!(valid_digest(&"a".repeat(64)));
        assert!(!valid_digest(&"A".repeat(64)));
        assert_eq!(expected_component_ids(2).len(), 3);
    }

    #[test]
    fn accepts_retained_remote_fleet_evidence() {
        validate_fleet_update_qualification_report(&retained_report()).unwrap();
    }

    #[test]
    fn rejects_a_broken_component_activation_chain() {
        let mut report = retained_report();
        report.upgrade_transaction.components[0].previous_version = None;
        let errors = validate_fleet_update_qualification_report(&report).unwrap_err();
        assert!(
            errors
                .iter()
                .any(|error| error.contains("previous version must be"))
        );
    }

    fn retained_report() -> FleetUpdateQualificationReport {
        serde_json::from_str(include_str!(
            "../../../../../releases/usability-evidence/2.17.0/fleet-update-operational-qualification.json"
        ))
        .unwrap()
    }
}
