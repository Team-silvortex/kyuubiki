use crate::agent_replacement::AGENT_REPLACEMENT_RECEIPT_SCHEMA_VERSION;
use crate::agent_rolling_qualification::{
    AGENT_ROLLING_QUALIFICATION_JOURNEY, AGENT_ROLLING_QUALIFICATION_SCHEMA_VERSION,
    AGENT_ROLLING_REQUIRED_CHECKS, AgentRollingExecutionProbe, AgentRollingInstanceObservation,
    AgentRollingQualificationReport,
};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AgentRollingQualificationSummary {
    pub execution_host_role: String,
    pub first_version: String,
    pub second_version: String,
    pub agent_count: usize,
    pub replacement_count: usize,
    pub probe_count: usize,
}

pub fn validate_agent_rolling_qualification_report(
    report: &AgentRollingQualificationReport,
) -> Result<AgentRollingQualificationSummary, Vec<String>> {
    let mut errors = Vec::new();
    if report.schema_version != AGENT_ROLLING_QUALIFICATION_SCHEMA_VERSION {
        errors.push("unsupported rolling qualification schema".to_string());
    }
    if report.status != "pass" || report.journey != AGENT_ROLLING_QUALIFICATION_JOURNEY {
        errors.push("rolling qualification status or journey is invalid".to_string());
    }
    if !valid_host_role(&report.execution_host_role, &report.platform) {
        errors.push("rolling qualification execution host role is invalid".to_string());
    }
    if report.agent_count != 2
        || report.first_version == report.second_version
        || !valid_version(&report.first_version)
        || !valid_version(&report.second_version)
    {
        errors.push("rolling qualification version or Agent topology is invalid".to_string());
    }
    if !valid_digest(&report.first_binary_sha256)
        || !valid_digest(&report.second_binary_sha256)
        || report.first_binary_sha256 == report.second_binary_sha256
    {
        errors.push("rolling qualification binary digests do not prove a changed payload".into());
    }

    let initial = validate_instances(
        &report.initial_instances,
        &report.first_binary_sha256,
        "initial",
        &mut errors,
    );
    let final_instances = validate_instances(
        &report.final_instances,
        &report.second_binary_sha256,
        "final",
        &mut errors,
    );
    validate_replacements(report, &initial, &final_instances, &mut errors);
    validate_probes(&report.execution_probes, &mut errors);
    validate_checks(report, &mut errors);

    if errors.is_empty() {
        Ok(AgentRollingQualificationSummary {
            execution_host_role: report.execution_host_role.clone(),
            first_version: report.first_version.clone(),
            second_version: report.second_version.clone(),
            agent_count: report.agent_count,
            replacement_count: report.replacements.len(),
            probe_count: report.execution_probes.len(),
        })
    } else {
        Err(errors)
    }
}

fn validate_instances<'a>(
    instances: &'a [AgentRollingInstanceObservation],
    expected_digest: &str,
    phase: &str,
    errors: &mut Vec<String>,
) -> BTreeMap<&'a str, &'a AgentRollingInstanceObservation> {
    let map = instances
        .iter()
        .map(|instance| (instance.node_id.as_str(), instance))
        .collect::<BTreeMap<_, _>>();
    let expected = BTreeSet::from(["agent-01", "agent-02"]);
    if map.keys().copied().collect::<BTreeSet<_>>() != expected || map.len() != instances.len() {
        errors.push(format!(
            "{phase} rolling instances do not cover exactly two Agents"
        ));
    }
    for instance in instances {
        if instance.binary_sha256 != expected_digest
            || !instance.accepting_new_work
            || !valid_instance_id(&instance.process_instance_id)
        {
            errors.push(format!(
                "{phase} rolling instance {} is inconsistent",
                instance.node_id
            ));
        }
    }
    map
}

fn validate_replacements(
    report: &AgentRollingQualificationReport,
    initial: &BTreeMap<&str, &AgentRollingInstanceObservation>,
    final_instances: &BTreeMap<&str, &AgentRollingInstanceObservation>,
    errors: &mut Vec<String>,
) {
    let replacements = report
        .replacements
        .iter()
        .map(|receipt| (receipt.node_id.as_str(), receipt))
        .collect::<BTreeMap<_, _>>();
    if replacements.len() != 2 || replacements.len() != report.replacements.len() {
        errors.push("rolling replacement receipts must cover exactly two Agents".into());
    }
    for node_id in ["agent-01", "agent-02"] {
        let Some(receipt) = replacements.get(node_id) else {
            errors.push(format!("rolling replacement receipt is missing {node_id}"));
            continue;
        };
        let identity_matches = initial.get(node_id).is_some_and(|instance| {
            instance.process_instance_id == receipt.previous_process_instance_id
        }) && final_instances.get(node_id).is_some_and(|instance| {
            instance.process_instance_id == receipt.active_process_instance_id
        });
        if receipt.schema_version != AGENT_REPLACEMENT_RECEIPT_SCHEMA_VERSION
            || receipt.controller_id != "installer-rolling-qualification"
            || receipt.drain_generation == 0
            || !receipt.quiescent_observed
            || !receipt.replacement_verified
            || receipt.previous_process_instance_id == receipt.active_process_instance_id
            || !identity_matches
        {
            errors.push(format!(
                "rolling replacement receipt for {node_id} is invalid"
            ));
        }
    }
}

fn validate_probes(probes: &[AgentRollingExecutionProbe], errors: &mut Vec<String>) {
    let expected = BTreeSet::from([
        ("initial", "agent-01"),
        ("initial", "agent-02"),
        ("during-agent-01-replacement", "agent-02"),
        ("during-agent-02-replacement", "agent-01"),
        ("final", "agent-01"),
        ("final", "agent-02"),
    ]);
    let actual = probes
        .iter()
        .map(|probe| (probe.phase.as_str(), probe.node_id.as_str()))
        .collect::<BTreeSet<_>>();
    if probes.len() != expected.len() || actual != expected {
        errors.push("rolling execution probes do not cover the required continuity phases".into());
    }
    for probe in probes {
        if !probe.success
            || !probe.max_stress.is_finite()
            || !probe.tip_displacement.is_finite()
            || (probe.max_stress - 10.0).abs() > 1.0e-9
            || (probe.tip_displacement - 0.01).abs() > 1.0e-12
        {
            errors.push(format!(
                "rolling execution probe {}/{} is invalid",
                probe.phase, probe.node_id
            ));
        }
    }
}

fn validate_checks(report: &AgentRollingQualificationReport, errors: &mut Vec<String>) {
    let ids = report
        .checks
        .iter()
        .map(|check| check.id.as_str())
        .collect::<BTreeSet<_>>();
    let required = AGENT_ROLLING_REQUIRED_CHECKS
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    if ids != required
        || report.checks.len() != required.len()
        || report.checks.iter().any(|check| !check.ok)
    {
        errors.push("rolling qualification checks are incomplete or failed".to_string());
    }
}

fn valid_host_role(role: &str, platform: &str) -> bool {
    matches!(platform, "macos" | "linux" | "windows")
        && matches!(
            role.strip_suffix(&format!("-{platform}-qualification-host")),
            Some("local" | "remote")
        )
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

fn valid_instance_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 256
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b':' | b'/' | b'-')
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent_replacement::AgentReplacementReceipt;
    use crate::agent_rolling_qualification::{
        AgentRollingQualificationCheck, AgentRollingQualificationReport,
    };

    #[test]
    fn accepts_a_complete_two_agent_replacement_chain() {
        validate_agent_rolling_qualification_report(&fixture()).unwrap();
    }

    #[test]
    fn qualification_schema_matches_the_runtime_contract() {
        let schema: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../../schemas/agent-rolling-replacement-qualification-report.schema.json"
        ))
        .unwrap();
        assert_eq!(
            schema["properties"]["schema_version"]["const"],
            AGENT_ROLLING_QUALIFICATION_SCHEMA_VERSION
        );
        assert_eq!(
            schema["$defs"]["replacement"]["properties"]["schema_version"]["const"],
            AGENT_REPLACEMENT_RECEIPT_SCHEMA_VERSION
        );
    }

    #[test]
    fn rejects_reused_process_identity() {
        let mut report = fixture();
        report.replacements[0].active_process_instance_id = "agent-01-old".to_string();
        assert!(
            validate_agent_rolling_qualification_report(&report)
                .unwrap_err()
                .iter()
                .any(|error| error.contains("agent-01"))
        );
    }

    fn fixture() -> AgentRollingQualificationReport {
        let first_digest = "1".repeat(64);
        let second_digest = "2".repeat(64);
        let initial_instances = instances("old", &first_digest);
        let final_instances = instances("new", &second_digest);
        let replacements = ["agent-01", "agent-02"]
            .into_iter()
            .map(|node_id| AgentReplacementReceipt {
                schema_version: AGENT_REPLACEMENT_RECEIPT_SCHEMA_VERSION.to_string(),
                node_id: node_id.to_string(),
                controller_id: "installer-rolling-qualification".to_string(),
                drain_generation: 1,
                previous_process_instance_id: format!("{node_id}-old"),
                active_process_instance_id: format!("{node_id}-new"),
                quiescent_observed: true,
                replacement_verified: true,
            })
            .collect();
        let execution_probes = [
            ("initial", "agent-01"),
            ("initial", "agent-02"),
            ("during-agent-01-replacement", "agent-02"),
            ("during-agent-02-replacement", "agent-01"),
            ("final", "agent-01"),
            ("final", "agent-02"),
        ]
        .into_iter()
        .map(|(phase, node_id)| AgentRollingExecutionProbe {
            phase: phase.to_string(),
            node_id: node_id.to_string(),
            success: true,
            max_stress: 10.0,
            tip_displacement: 0.01,
        })
        .collect();
        AgentRollingQualificationReport {
            schema_version: AGENT_ROLLING_QUALIFICATION_SCHEMA_VERSION.to_string(),
            status: "pass".to_string(),
            journey: AGENT_ROLLING_QUALIFICATION_JOURNEY.to_string(),
            execution_host_role: "remote-linux-qualification-host".to_string(),
            platform: "linux".to_string(),
            first_version: "2.16.9".to_string(),
            second_version: "2.17.0".to_string(),
            first_binary_sha256: first_digest,
            second_binary_sha256: second_digest,
            agent_count: 2,
            initial_instances,
            replacements,
            final_instances,
            execution_probes,
            checks: AGENT_ROLLING_REQUIRED_CHECKS
                .iter()
                .map(|id| AgentRollingQualificationCheck {
                    id: (*id).to_string(),
                    ok: true,
                })
                .collect(),
        }
    }

    fn instances(suffix: &str, digest: &str) -> Vec<AgentRollingInstanceObservation> {
        ["agent-01", "agent-02"]
            .into_iter()
            .map(|node_id| AgentRollingInstanceObservation {
                node_id: node_id.to_string(),
                process_instance_id: format!("{node_id}-{suffix}"),
                binary_sha256: digest.to_string(),
                accepting_new_work: true,
            })
            .collect()
    }
}
