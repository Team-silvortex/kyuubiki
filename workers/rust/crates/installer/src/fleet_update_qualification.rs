use crate::Platform;
use crate::agent_update_payload::{
    active_agent_binary_in, install_agent_update_package_into, prepare_agent_update_package,
};
use crate::fleet_update::{
    FleetAgentUpdateTarget, FleetUpdatePlan, FleetUpdateSnapshot, FleetUpdateTransactionReceipt,
    apply_fleet_update_transaction, apply_fleet_update_transaction_with_hook,
    fleet_status_versions, fleet_store_is_clean, inspect_fleet_update_state,
    rollback_fleet_update_transaction,
};
use crate::runtime_payload::{install_runtime_payload_into, verified_runtime_service_launches_in};
use crate::runtime_payload_qualification::prepare_runtime_payload;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;
use std::process::Command;

pub const FLEET_UPDATE_QUALIFICATION_SCHEMA_VERSION: &str =
    "kyuubiki.fleet-update-qualification/v1";
pub(crate) const FLEET_UPDATE_QUALIFICATION_JOURNEY: &str =
    "installer-managed-runtime-agent-fleet-upgrade-compensation-and-rollback";
pub(crate) const FLEET_UPDATE_REQUIRED_CHECKS: &[&str] = &[
    "initial_fleet_aligned",
    "mid_fleet_failure_injected",
    "failed_upgrade_compensated",
    "compensated_fleet_executable",
    "successful_upgrade_aligned",
    "upgraded_fleet_executable",
    "explicit_rollback_aligned",
    "rolled_back_fleet_executable",
    "payloads_changed",
    "payloads_restored",
    "generation_progression_valid",
    "stores_clean",
];
const SERVICE_IDS: &[&str] = &["agent", "orchestrator", "frontend"];

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FleetUpdateQualificationReport {
    pub schema_version: String,
    pub status: String,
    pub journey: String,
    pub execution_host_role: String,
    pub platform: String,
    pub first_version: String,
    pub second_version: String,
    pub agent_count: usize,
    pub initial: FleetUpdateSnapshot,
    pub failure_injection: FleetUpdateFailureObservation,
    pub compensated_after_failure: FleetUpdateSnapshot,
    pub upgrade_transaction: FleetUpdateTransactionReceipt,
    pub rollback_transaction: FleetUpdateTransactionReceipt,
    pub probes: Vec<FleetUpdateExecutionProbe>,
    pub checks: Vec<FleetUpdateQualificationCheck>,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FleetUpdateFailureObservation {
    pub failed_component_id: String,
    pub failure_class: String,
    pub compensated: bool,
    pub compensation_error_count: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FleetUpdateExecutionProbe {
    pub phase: String,
    pub component_id: String,
    pub role: String,
    pub version: String,
    pub success: bool,
    pub marker_observed: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FleetUpdateQualificationCheck {
    pub id: String,
    pub ok: bool,
}

pub fn run_fleet_update_qualification(
    first_binary: &Path,
    second_binary: &Path,
    work_root: &Path,
    first_version: &str,
    second_version: &str,
    agent_count: usize,
) -> Result<FleetUpdateQualificationReport, String> {
    if first_version == second_version {
        return Err("fleet qualification versions must be distinct".into());
    }
    if !(2..=8).contains(&agent_count) {
        return Err("fleet qualification requires between 2 and 8 Agents".into());
    }
    prepare_empty_root(work_root)?;
    let platform = Platform::current();
    let first_runtime = work_root.join("packages/runtime-first");
    let second_runtime = work_root.join("packages/runtime-second");
    let first_agent = work_root.join("packages/agent-first");
    let second_agent = work_root.join("packages/agent-second");
    prepare_runtime_payload(first_binary, &first_runtime, first_version, platform)?;
    prepare_runtime_payload(second_binary, &second_runtime, second_version, platform)?;
    prepare_agent_update_package(first_binary, &first_agent, first_version, platform)?;
    prepare_agent_update_package(second_binary, &second_agent, second_version, platform)?;

    let runtime_store = work_root.join("stores/runtime");
    let agents = (0..agent_count)
        .map(|index| FleetAgentUpdateTarget {
            node_id: format!("agent-{:02}", index + 1),
            package_root: second_agent.clone(),
            store_root: work_root.join(format!("stores/agent-{:02}", index + 1)),
        })
        .collect::<Vec<_>>();
    let plan = FleetUpdatePlan {
        runtime_package_root: second_runtime,
        runtime_store_root: runtime_store,
        agents,
    };

    install_runtime_payload_into(&first_runtime, &plan.runtime_store_root, platform)?;
    for agent in &plan.agents {
        install_agent_update_package_into(&first_agent, &agent.store_root, platform)?;
    }
    let initial = inspect_fleet_update_state(&plan, platform)?;
    let mut probes = run_fleet_probes(&plan, platform, "initial", first_version)?;

    let injected_node = plan.agents[1].node_id.clone();
    let failure = match apply_fleet_update_transaction_with_hook(&plan, platform, |component| {
        if component == injected_node {
            Err("qualification fault injected before second Agent activation".into())
        } else {
            Ok(())
        }
    }) {
        Ok(_) => {
            return Err("fleet qualification did not reach its injected failure boundary".into());
        }
        Err(failure) => failure,
    };
    let failure_injection = FleetUpdateFailureObservation {
        failed_component_id: failure.failed_component_id,
        failure_class: failure.failure_class,
        compensated: failure.compensated,
        compensation_error_count: failure.compensation_errors.len(),
    };
    let compensated_after_failure = inspect_fleet_update_state(&plan, platform)?;
    probes.extend(run_fleet_probes(
        &plan,
        platform,
        "compensated",
        first_version,
    )?);

    let upgrade_transaction = apply_fleet_update_transaction(&plan, platform)
        .map_err(|error| format!("fleet upgrade transaction failed: {error}"))?;
    probes.extend(run_fleet_probes(
        &plan,
        platform,
        "upgraded",
        second_version,
    )?);
    let rollback_transaction = rollback_fleet_update_transaction(&plan, platform)
        .map_err(|error| format!("fleet rollback transaction failed: {error}"))?;
    probes.extend(run_fleet_probes(
        &plan,
        platform,
        "rolled-back",
        first_version,
    )?);

    let checks = qualification_checks(
        &plan,
        first_version,
        second_version,
        &initial,
        &failure_injection,
        &compensated_after_failure,
        &upgrade_transaction,
        &rollback_transaction,
        &probes,
    );
    if checks.iter().any(|check| !check.ok) {
        return Err("fleet update operational qualification checks failed".into());
    }
    let report = FleetUpdateQualificationReport {
        schema_version: FLEET_UPDATE_QUALIFICATION_SCHEMA_VERSION.into(),
        status: "pass".into(),
        journey: FLEET_UPDATE_QUALIFICATION_JOURNEY.into(),
        execution_host_role: qualification_host_role(platform),
        platform: platform.as_str().to_string(),
        first_version: first_version.to_string(),
        second_version: second_version.to_string(),
        agent_count,
        initial,
        failure_injection,
        compensated_after_failure,
        upgrade_transaction,
        rollback_transaction,
        probes,
        checks,
    };
    crate::fleet_update_qualification_validation::validate_fleet_update_qualification_report(
        &report,
    )
    .map_err(|errors| format!("fleet qualification is invalid: {}", errors.join("; ")))?;
    fs::remove_dir_all(work_root).map_err(|error| {
        format!(
            "fleet qualification passed but failed to remove {}: {error}",
            work_root.display()
        )
    })?;
    Ok(report)
}

pub fn write_fleet_update_qualification_report(
    report: &FleetUpdateQualificationReport,
    path: &Path,
) -> Result<(), String> {
    crate::fleet_update_qualification_validation::validate_fleet_update_qualification_report(
        report,
    )
    .map_err(|errors| {
        format!(
            "fleet qualification report is invalid: {}",
            errors.join("; ")
        )
    })?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
    }
    fs::write(
        path,
        serde_json::to_vec_pretty(report).map_err(|error| error.to_string())?,
    )
    .map_err(|error| format!("failed to write {}: {error}", path.display()))
}

fn run_fleet_probes(
    plan: &FleetUpdatePlan,
    platform: Platform,
    phase: &str,
    version: &str,
) -> Result<Vec<FleetUpdateExecutionProbe>, String> {
    let runtime_root = plan.runtime_store_root.join("versions").join(version);
    let launches = verified_runtime_service_launches_in(&runtime_root, platform)?;
    let observed = launches
        .iter()
        .map(|launch| launch.id.as_str())
        .collect::<BTreeSet<_>>();
    if observed != SERVICE_IDS.iter().copied().collect() {
        return Err("fleet Runtime service set drifted".into());
    }
    let mut probes = Vec::new();
    for service_id in SERVICE_IDS {
        let launch = launches
            .iter()
            .find(|launch| launch.id == *service_id)
            .ok_or_else(|| format!("missing Runtime service {service_id}"))?;
        probes.push(run_probe(
            &launch.command,
            Some(&launch.cwd),
            phase,
            &format!("runtime.{service_id}"),
            "runtime-service",
            version,
        )?);
    }
    for agent in &plan.agents {
        let binary = active_agent_binary_in(&agent.store_root, platform)?;
        probes.push(run_probe(
            &binary,
            None,
            phase,
            &agent.node_id,
            "agent",
            version,
        )?);
    }
    Ok(probes)
}

fn run_probe(
    binary: &Path,
    cwd: Option<&Path>,
    phase: &str,
    component_id: &str,
    role: &str,
    version: &str,
) -> Result<FleetUpdateExecutionProbe, String> {
    let marker = format!("fleet-{phase}-{}", component_id.replace('.', "-"));
    let mut command = Command::new(binary);
    if let Some(cwd) = cwd {
        command.current_dir(cwd);
    }
    let output = command
        .args(["--steps", "1", "--job-id", &marker])
        .output()
        .map_err(|error| format!("failed to execute {component_id}: {error}"))?;
    let marker_observed = String::from_utf8_lossy(&output.stdout).contains(&marker)
        || String::from_utf8_lossy(&output.stderr).contains(&marker);
    if !output.status.success() || !marker_observed {
        return Err(format!(
            "fleet probe failed for {phase}/{component_id}: status={} marker_observed={marker_observed}",
            output.status
        ));
    }
    Ok(FleetUpdateExecutionProbe {
        phase: phase.to_string(),
        component_id: component_id.to_string(),
        role: role.to_string(),
        version: version.to_string(),
        success: true,
        marker_observed,
    })
}

#[allow(clippy::too_many_arguments)]
fn qualification_checks(
    plan: &FleetUpdatePlan,
    first_version: &str,
    second_version: &str,
    initial: &FleetUpdateSnapshot,
    failure: &FleetUpdateFailureObservation,
    compensated: &FleetUpdateSnapshot,
    upgrade: &FleetUpdateTransactionReceipt,
    rollback: &FleetUpdateTransactionReceipt,
    probes: &[FleetUpdateExecutionProbe],
) -> Vec<FleetUpdateQualificationCheck> {
    let expected_probes = (SERVICE_IDS.len() + plan.agents.len()) * 4;
    let clean = fleet_store_is_clean(&plan.runtime_store_root)
        && plan
            .agents
            .iter()
            .all(|agent| fleet_store_is_clean(&agent.store_root));
    let installed = fleet_status_versions(plan).unwrap_or_default();
    vec![
        check(
            "initial_fleet_aligned",
            initial.active_version == first_version,
        ),
        check(
            "mid_fleet_failure_injected",
            failure.failed_component_id == plan.agents[1].node_id
                && failure.failure_class == "injected-fault",
        ),
        check(
            "failed_upgrade_compensated",
            failure.compensated
                && failure.compensation_error_count == 0
                && compensated.active_version == first_version,
        ),
        check(
            "compensated_fleet_executable",
            phase_probes_pass(probes, "compensated", first_version),
        ),
        check(
            "successful_upgrade_aligned",
            upgrade.active_version == second_version,
        ),
        check(
            "upgraded_fleet_executable",
            phase_probes_pass(probes, "upgraded", second_version),
        ),
        check(
            "explicit_rollback_aligned",
            rollback.active_version == first_version,
        ),
        check(
            "rolled_back_fleet_executable",
            phase_probes_pass(probes, "rolled-back", first_version),
        ),
        check(
            "payloads_changed",
            component_digests_differ(initial, &upgrade.components),
        ),
        check(
            "payloads_restored",
            component_digests_match(initial, &rollback.components)
                && component_digests_match(initial, &compensated.components),
        ),
        check(
            "generation_progression_valid",
            generations_progress(initial, compensated, upgrade, rollback),
        ),
        check(
            "stores_clean",
            clean
                && probes.len() == expected_probes
                && installed
                    .iter()
                    .all(|version| version == first_version || version == second_version),
        ),
    ]
}

fn phase_probes_pass(probes: &[FleetUpdateExecutionProbe], phase: &str, version: &str) -> bool {
    probes
        .iter()
        .filter(|probe| probe.phase == phase)
        .all(|probe| probe.version == version && probe.success && probe.marker_observed)
}

fn component_digests_differ(
    initial: &FleetUpdateSnapshot,
    other: &[crate::FleetUpdateComponentState],
) -> bool {
    initial.components.iter().all(|component| {
        other
            .iter()
            .find(|candidate| candidate.component_id == component.component_id)
            .is_some_and(|candidate| candidate.payload_sha256 != component.payload_sha256)
    })
}

fn component_digests_match(
    initial: &FleetUpdateSnapshot,
    other: &[crate::FleetUpdateComponentState],
) -> bool {
    initial.components.iter().all(|component| {
        other
            .iter()
            .find(|candidate| candidate.component_id == component.component_id)
            .is_some_and(|candidate| candidate.payload_sha256 == component.payload_sha256)
    })
}

fn generations_progress(
    initial: &FleetUpdateSnapshot,
    compensated: &FleetUpdateSnapshot,
    upgrade: &FleetUpdateTransactionReceipt,
    rollback: &FleetUpdateTransactionReceipt,
) -> bool {
    initial.components.iter().all(|component| {
        let values = [
            state_generation(&compensated.components, &component.component_id),
            state_generation(&upgrade.components, &component.component_id),
            state_generation(&rollback.components, &component.component_id),
        ];
        values[0].is_some_and(|value| value >= component.generation)
            && values[1].is_some_and(|value| value > values[0].unwrap_or_default())
            && values[2].is_some_and(|value| value > values[1].unwrap_or_default())
    })
}

fn state_generation(states: &[crate::FleetUpdateComponentState], id: &str) -> Option<u64> {
    states
        .iter()
        .find(|state| state.component_id == id)
        .map(|state| state.generation)
}

fn check(id: &str, ok: bool) -> FleetUpdateQualificationCheck {
    FleetUpdateQualificationCheck {
        id: id.to_string(),
        ok,
    }
}

fn prepare_empty_root(work_root: &Path) -> Result<(), String> {
    if work_root
        .symlink_metadata()
        .is_ok_and(|metadata| metadata.file_type().is_symlink())
    {
        return Err("fleet qualification work root must not be a symlink".into());
    }
    if work_root.exists()
        && fs::read_dir(work_root)
            .map_err(|error| format!("failed to read {}: {error}", work_root.display()))?
            .next()
            .is_some()
    {
        return Err("fleet qualification work root must be empty".into());
    }
    fs::create_dir_all(work_root)
        .map_err(|error| format!("failed to create {}: {error}", work_root.display()))
}

fn qualification_host_role(platform: Platform) -> String {
    let transport = if std::env::var_os("SSH_CONNECTION").is_some() {
        "remote"
    } else {
        "local"
    };
    format!("{transport}-{}-qualification-host", platform.as_str())
}
