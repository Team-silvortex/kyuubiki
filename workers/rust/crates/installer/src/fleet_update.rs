use crate::Platform;
use crate::agent_update_payload::{
    activate_agent_version_in, active_agent_activation_in, agent_update_status_in,
    install_agent_update_package_into, rollback_agent_update_in, verify_agent_update_package,
};
use crate::runtime_payload::{
    activate_runtime_version_in, active_runtime_activation_in, install_runtime_payload_into,
    rollback_runtime_payload_in, runtime_payload_content_digest_in, runtime_payload_status_in,
    verified_runtime_payload_version_in,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fmt;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

pub const FLEET_UPDATE_TRANSACTION_SCHEMA_VERSION: &str = "kyuubiki.fleet-update-transaction/v1";

#[derive(Clone, Debug)]
pub struct FleetAgentUpdateTarget {
    pub node_id: String,
    pub package_root: PathBuf,
    pub store_root: PathBuf,
}

#[derive(Clone, Debug)]
pub struct FleetUpdatePlan {
    pub runtime_package_root: PathBuf,
    pub runtime_store_root: PathBuf,
    pub agents: Vec<FleetAgentUpdateTarget>,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FleetUpdateComponentState {
    pub component_id: String,
    pub role: String,
    pub generation: u64,
    pub active_version: String,
    pub previous_version: Option<String>,
    pub payload_sha256: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FleetUpdateSnapshot {
    pub active_version: String,
    pub components: Vec<FleetUpdateComponentState>,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FleetUpdateTransactionReceipt {
    pub schema_version: String,
    pub operation: String,
    pub before_version: String,
    pub active_version: String,
    pub components: Vec<FleetUpdateComponentState>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FleetUpdateTransactionFailure {
    pub failed_component_id: String,
    pub failure_class: String,
    pub compensated: bool,
    pub compensation_errors: Vec<String>,
    pub cause: String,
}

impl fmt::Display for FleetUpdateTransactionFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "fleet update failed at {} ({}): {}; compensated={}",
            self.failed_component_id, self.failure_class, self.cause, self.compensated
        )?;
        if !self.compensation_errors.is_empty() {
            write!(
                formatter,
                "; compensation errors: {}",
                self.compensation_errors.join("; ")
            )?;
        }
        Ok(())
    }
}

impl std::error::Error for FleetUpdateTransactionFailure {}

pub fn apply_fleet_update_transaction(
    plan: &FleetUpdatePlan,
    platform: Platform,
) -> Result<FleetUpdateTransactionReceipt, FleetUpdateTransactionFailure> {
    apply_fleet_update_transaction_with_hook(plan, platform, |_| Ok(()))
}

pub(crate) fn apply_fleet_update_transaction_with_hook(
    plan: &FleetUpdatePlan,
    platform: Platform,
    mut checkpoint: impl FnMut(&str) -> Result<(), String>,
) -> Result<FleetUpdateTransactionReceipt, FleetUpdateTransactionFailure> {
    validate_plan_shape(plan).map_err(|cause| preflight_failure("fleet", cause))?;
    let _fleet_lock = FleetTransactionLock::acquire(&plan.runtime_store_root)
        .map_err(|cause| preflight_failure("fleet", cause))?;
    let (before, target_version) =
        preflight_upgrade(plan, platform).map_err(|cause| preflight_failure("fleet", cause))?;
    let mut runtime_applied = false;
    let mut applied_agents = Vec::new();

    let result = (|| -> Result<(), (String, String, String)> {
        install_runtime_payload_into(
            &plan.runtime_package_root,
            &plan.runtime_store_root,
            platform,
        )
        .map_err(|cause| ("runtime".into(), "runtime-activation".into(), cause))?;
        runtime_applied = true;
        checkpoint("runtime")
            .map_err(|cause| ("runtime".into(), "injected-fault".into(), cause))?;

        for (index, agent) in plan.agents.iter().enumerate() {
            checkpoint(&agent.node_id)
                .map_err(|cause| (agent.node_id.clone(), "injected-fault".into(), cause))?;
            install_agent_update_package_into(&agent.package_root, &agent.store_root, platform)
                .map_err(|cause| (agent.node_id.clone(), "agent-activation".into(), cause))?;
            applied_agents.push(index);
        }
        checkpoint("fleet:verify")
            .map_err(|cause| ("fleet".into(), "injected-fault".into(), cause))?;
        Ok(())
    })();

    if let Err((component, class, cause)) = result {
        let compensation_errors = compensate_upgrade(
            plan,
            platform,
            runtime_applied,
            &applied_agents,
            &before.active_version,
        );
        return Err(FleetUpdateTransactionFailure {
            failed_component_id: component,
            failure_class: class,
            compensated: compensation_errors.is_empty(),
            compensation_errors,
            cause,
        });
    }

    let all_agents = (0..plan.agents.len()).collect::<Vec<_>>();
    let after = match inspect_fleet_update_state(plan, platform) {
        Ok(snapshot) => snapshot,
        Err(cause) => {
            let compensation_errors =
                compensate_upgrade(plan, platform, true, &all_agents, &before.active_version);
            return Err(FleetUpdateTransactionFailure {
                failed_component_id: "fleet".into(),
                failure_class: "post-upgrade-verification".into(),
                compensated: compensation_errors.is_empty(),
                compensation_errors,
                cause,
            });
        }
    };
    if after.active_version != target_version {
        let compensation_errors =
            compensate_upgrade(plan, platform, true, &all_agents, &before.active_version);
        return Err(FleetUpdateTransactionFailure {
            failed_component_id: "fleet".into(),
            failure_class: "post-upgrade-version-drift".into(),
            compensated: compensation_errors.is_empty(),
            compensation_errors,
            cause: format!(
                "fleet activated {}, expected {target_version}",
                after.active_version
            ),
        });
    }
    Ok(receipt("upgrade", &before.active_version, after))
}

pub fn rollback_fleet_update_transaction(
    plan: &FleetUpdatePlan,
    platform: Platform,
) -> Result<FleetUpdateTransactionReceipt, FleetUpdateTransactionFailure> {
    validate_plan_shape(plan).map_err(|cause| preflight_failure("fleet", cause))?;
    let _fleet_lock = FleetTransactionLock::acquire(&plan.runtime_store_root)
        .map_err(|cause| preflight_failure("fleet", cause))?;
    let before = inspect_fleet_update_state(plan, platform)
        .map_err(|cause| preflight_failure("fleet", cause))?;
    let previous =
        common_previous_version(&before).map_err(|cause| preflight_failure("fleet", cause))?;
    let target =
        target_version(plan, platform).map_err(|cause| preflight_failure("fleet", cause))?;
    if target != before.active_version {
        return Err(preflight_failure(
            "fleet",
            format!(
                "rollback packages target {target}, but the fleet runs {}",
                before.active_version
            ),
        ));
    }

    let mut rolled_agents = Vec::new();
    for index in (0..plan.agents.len()).rev() {
        let agent = &plan.agents[index];
        if let Err(cause) = rollback_agent_update_in(&agent.store_root, platform) {
            return Err(compensate_rollback_failure(
                plan,
                platform,
                &before.active_version,
                &rolled_agents,
                false,
                &agent.node_id,
                "agent-rollback",
                cause,
            ));
        }
        rolled_agents.push(index);
    }
    if let Err(cause) = rollback_runtime_payload_in(&plan.runtime_store_root, platform) {
        return Err(compensate_rollback_failure(
            plan,
            platform,
            &before.active_version,
            &rolled_agents,
            false,
            "runtime",
            "runtime-rollback",
            cause,
        ));
    }

    let after = inspect_fleet_update_state(plan, platform).map_err(|cause| {
        compensate_rollback_failure(
            plan,
            platform,
            &before.active_version,
            &rolled_agents,
            true,
            "fleet",
            "post-rollback-verification",
            cause,
        )
    })?;
    if after.active_version != previous {
        return Err(compensate_rollback_failure(
            plan,
            platform,
            &before.active_version,
            &rolled_agents,
            true,
            "fleet",
            "post-rollback-version-drift",
            format!(
                "fleet activated {}, expected {previous}",
                after.active_version
            ),
        ));
    }
    Ok(receipt("rollback", &before.active_version, after))
}

pub fn inspect_fleet_update_state(
    plan: &FleetUpdatePlan,
    platform: Platform,
) -> Result<FleetUpdateSnapshot, String> {
    validate_plan_shape(plan)?;
    let runtime = active_runtime_activation_in(&plan.runtime_store_root, platform)?;
    let runtime_digest = runtime_payload_content_digest_in(
        &plan.runtime_store_root.join(&runtime.relative_path),
        platform,
    )?;
    let mut components = vec![FleetUpdateComponentState {
        component_id: "runtime".into(),
        role: "runtime".into(),
        generation: runtime.generation,
        active_version: runtime.version.clone(),
        previous_version: runtime.previous_version,
        payload_sha256: runtime_digest,
    }];
    for agent in &plan.agents {
        let active = active_agent_activation_in(&agent.store_root, platform)?;
        components.push(FleetUpdateComponentState {
            component_id: agent.node_id.clone(),
            role: "agent".into(),
            generation: active.generation,
            active_version: active.version,
            previous_version: active.previous_version,
            payload_sha256: active.entrypoint_sha256,
        });
    }
    let active_version = common_active_version(&components)?;
    Ok(FleetUpdateSnapshot {
        active_version,
        components,
    })
}

fn preflight_upgrade(
    plan: &FleetUpdatePlan,
    platform: Platform,
) -> Result<(FleetUpdateSnapshot, String), String> {
    let before = inspect_fleet_update_state(plan, platform)?;
    let target = target_version(plan, platform)?;
    if target == before.active_version {
        return Err("fleet update target must differ from the active version".into());
    }
    Ok((before, target))
}

fn target_version(plan: &FleetUpdatePlan, platform: Platform) -> Result<String, String> {
    validate_plan_shape(plan)?;
    let runtime = verified_runtime_payload_version_in(&plan.runtime_package_root, platform)?;
    for agent in &plan.agents {
        let manifest = verify_agent_update_package(&agent.package_root, platform)?;
        if manifest.version != runtime {
            return Err(format!(
                "agent {} targets {}, but runtime targets {runtime}",
                agent.node_id, manifest.version
            ));
        }
    }
    Ok(runtime)
}

fn validate_plan_shape(plan: &FleetUpdatePlan) -> Result<(), String> {
    if plan.agents.is_empty() {
        return Err("fleet update requires at least one Agent target".into());
    }
    let mut ids = BTreeSet::from(["runtime".to_string()]);
    let mut stores = vec![(
        "runtime".to_string(),
        canonical_store(&plan.runtime_store_root, "runtime")?,
    )];
    for agent in &plan.agents {
        if !valid_node_id(&agent.node_id) || !ids.insert(agent.node_id.clone()) {
            return Err(format!(
                "invalid or duplicate fleet node id {}",
                agent.node_id
            ));
        }
        let store = canonical_store(&agent.store_root, &agent.node_id)?;
        for (other_id, other_store) in &stores {
            if store.starts_with(other_store) || other_store.starts_with(&store) {
                return Err(format!(
                    "fleet node {} store overlaps component {other_id}",
                    agent.node_id
                ));
            }
        }
        stores.push((agent.node_id.clone(), store));
    }
    Ok(())
}

fn canonical_store(path: &Path, component_id: &str) -> Result<PathBuf, String> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|error| format!("fleet component {component_id} store is unavailable: {error}"))?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(format!(
            "fleet component {component_id} store must be a real directory"
        ));
    }
    fs::canonicalize(path)
        .map_err(|error| format!("failed to resolve fleet component {component_id} store: {error}"))
}

fn valid_node_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
}

fn common_active_version(components: &[FleetUpdateComponentState]) -> Result<String, String> {
    let versions = components
        .iter()
        .map(|component| component.active_version.as_str())
        .collect::<BTreeSet<_>>();
    if versions.len() != 1 {
        return Err("fleet components do not share one active version".into());
    }
    Ok(versions.into_iter().next().unwrap_or_default().to_string())
}

fn common_previous_version(snapshot: &FleetUpdateSnapshot) -> Result<String, String> {
    let versions = snapshot
        .components
        .iter()
        .map(|component| component.previous_version.as_deref())
        .collect::<BTreeSet<_>>();
    if versions.len() != 1 || versions.contains(&None) {
        return Err("fleet components do not share one rollback version".into());
    }
    Ok(versions
        .into_iter()
        .next()
        .flatten()
        .unwrap_or_default()
        .to_string())
}

fn compensate_upgrade(
    plan: &FleetUpdatePlan,
    platform: Platform,
    runtime_applied: bool,
    applied_agents: &[usize],
    expected_version: &str,
) -> Vec<String> {
    let mut errors = Vec::new();
    for index in applied_agents.iter().rev() {
        let agent = &plan.agents[*index];
        if let Err(error) = restore_agent_version(agent, expected_version, platform) {
            errors.push(format!(
                "agent {} compensation failed: {error}",
                agent.node_id
            ));
        }
    }
    if runtime_applied && let Err(error) = restore_runtime_version(plan, expected_version, platform)
    {
        errors.push(format!("runtime compensation failed: {error}"));
    }
    match inspect_fleet_update_state(plan, platform) {
        Ok(snapshot) if snapshot.active_version == expected_version => {}
        Ok(snapshot) => errors.push(format!(
            "compensated fleet runs {}, expected {expected_version}",
            snapshot.active_version
        )),
        Err(error) => errors.push(format!("compensated fleet verification failed: {error}")),
    }
    errors
}

#[allow(clippy::too_many_arguments)]
fn compensate_rollback_failure(
    plan: &FleetUpdatePlan,
    platform: Platform,
    expected_version: &str,
    rolled_agents: &[usize],
    runtime_rolled: bool,
    component: &str,
    class: &str,
    cause: String,
) -> FleetUpdateTransactionFailure {
    let mut compensation_errors = Vec::new();
    if runtime_rolled && let Err(error) = restore_runtime_version(plan, expected_version, platform)
    {
        compensation_errors.push(format!("runtime roll-forward failed: {error}"));
    }
    for index in rolled_agents.iter().rev() {
        let agent = &plan.agents[*index];
        if let Err(error) = restore_agent_version(agent, expected_version, platform) {
            compensation_errors.push(format!(
                "agent {} roll-forward failed: {error}",
                agent.node_id
            ));
        }
    }
    match inspect_fleet_update_state(plan, platform) {
        Ok(snapshot) if snapshot.active_version == expected_version => {}
        Ok(snapshot) => compensation_errors.push(format!(
            "roll-forward fleet runs {}, expected {expected_version}",
            snapshot.active_version
        )),
        Err(error) => {
            compensation_errors.push(format!("roll-forward verification failed: {error}"))
        }
    }
    FleetUpdateTransactionFailure {
        failed_component_id: component.to_string(),
        failure_class: class.to_string(),
        compensated: compensation_errors.is_empty(),
        compensation_errors,
        cause,
    }
}

fn restore_agent_version(
    agent: &FleetAgentUpdateTarget,
    expected_version: &str,
    platform: Platform,
) -> Result<(), String> {
    if active_agent_activation_in(&agent.store_root, platform)
        .is_ok_and(|active| active.version == expected_version)
    {
        return Ok(());
    }
    activate_agent_version_in(&agent.store_root, expected_version, platform).map(|_| ())
}

fn restore_runtime_version(
    plan: &FleetUpdatePlan,
    expected_version: &str,
    platform: Platform,
) -> Result<(), String> {
    if active_runtime_activation_in(&plan.runtime_store_root, platform)
        .is_ok_and(|active| active.version == expected_version)
    {
        return Ok(());
    }
    activate_runtime_version_in(&plan.runtime_store_root, expected_version, platform).map(|_| ())
}

fn preflight_failure(component: &str, cause: String) -> FleetUpdateTransactionFailure {
    FleetUpdateTransactionFailure {
        failed_component_id: component.to_string(),
        failure_class: "preflight".into(),
        compensated: true,
        compensation_errors: Vec::new(),
        cause,
    }
}

fn receipt(
    operation: &str,
    before_version: &str,
    snapshot: FleetUpdateSnapshot,
) -> FleetUpdateTransactionReceipt {
    FleetUpdateTransactionReceipt {
        schema_version: FLEET_UPDATE_TRANSACTION_SCHEMA_VERSION.into(),
        operation: operation.to_string(),
        before_version: before_version.to_string(),
        active_version: snapshot.active_version,
        components: snapshot.components,
    }
}

pub(crate) fn fleet_store_is_clean(store: &Path) -> bool {
    !store.join(".update.lock").exists()
        && !store.join(".fleet-transaction.lock").exists()
        && std::fs::read_dir(store.join("staging"))
            .ok()
            .is_some_and(|mut entries| entries.next().is_none())
}

pub(crate) fn fleet_status_versions(plan: &FleetUpdatePlan) -> Result<Vec<String>, String> {
    let mut versions = Vec::new();
    versions.extend(runtime_payload_status_in(&plan.runtime_store_root)?.installed_versions);
    for agent in &plan.agents {
        versions.extend(agent_update_status_in(&agent.store_root)?.installed_versions);
    }
    Ok(versions)
}

struct FleetTransactionLock {
    path: PathBuf,
}

impl FleetTransactionLock {
    fn acquire(runtime_store: &Path) -> Result<Self, String> {
        let path = runtime_store.join(".fleet-transaction.lock");
        let mut file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&path)
            .map_err(|error| format!("fleet transaction lock is unavailable: {error}"))?;
        if let Err(error) = writeln!(file, "pid={}", std::process::id()) {
            drop(file);
            let _ = fs::remove_file(&path);
            return Err(format!("failed to write fleet transaction lock: {error}"));
        }
        Ok(Self { path })
    }
}

impl Drop for FleetTransactionLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}
