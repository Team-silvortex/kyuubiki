use crate::Platform;
use crate::agent_update_payload::{
    AgentUpdateActivationRecord, active_agent_binary_in, agent_update_status_in,
    install_agent_update_package_into, prepare_agent_update_package, rollback_agent_update_in,
};
use serde::Serialize;
use std::fs;
use std::path::Path;
use std::process::Command;

pub const AGENT_UPDATE_QUALIFICATION_SCHEMA_VERSION: &str =
    "kyuubiki.agent-update-qualification/v1";

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct AgentUpdateQualificationReport {
    pub schema_version: String,
    pub status: String,
    pub journey: String,
    pub execution_host_role: String,
    pub platform: String,
    pub first_version: String,
    pub second_version: String,
    pub first_activation: AgentUpdateActivationRecord,
    pub second_activation: AgentUpdateActivationRecord,
    pub rollback_activation: AgentUpdateActivationRecord,
    pub active_after_upgrade: String,
    pub active_after_rollback: String,
    pub installed_versions: Vec<String>,
    pub probes: Vec<AgentUpdateExecutionProbe>,
    pub checks: Vec<AgentUpdateQualificationCheck>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct AgentUpdateExecutionProbe {
    pub phase: String,
    pub version: String,
    pub success: bool,
    pub job_id_observed: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct AgentUpdateQualificationCheck {
    pub id: String,
    pub ok: bool,
}

pub fn run_agent_update_qualification(
    first_binary: &Path,
    second_binary: &Path,
    work_root: &Path,
    first_version: &str,
    second_version: &str,
) -> Result<AgentUpdateQualificationReport, String> {
    if first_version == second_version {
        return Err("agent qualification versions must be distinct".to_string());
    }
    prepare_empty_root(work_root)?;
    let platform = Platform::current();
    let first_package = work_root.join("packages/first");
    let second_package = work_root.join("packages/second");
    let store = work_root.join("managed-store");
    prepare_agent_update_package(first_binary, &first_package, first_version, platform)?;
    prepare_agent_update_package(second_binary, &second_package, second_version, platform)?;

    let first_activation = install_agent_update_package_into(&first_package, &store, platform)?;
    let first_probe = run_agent_probe(
        &active_agent_binary_in(&store, platform)?,
        "initial-install",
        first_version,
    )?;
    let second_activation = install_agent_update_package_into(&second_package, &store, platform)?;
    let second_probe = run_agent_probe(
        &active_agent_binary_in(&store, platform)?,
        "upgraded-install",
        second_version,
    )?;
    let status_after_upgrade = agent_update_status_in(&store)?;
    let rollback_activation = rollback_agent_update_in(&store, platform)?;
    let rollback_probe = run_agent_probe(
        &active_agent_binary_in(&store, platform)?,
        "rollback",
        first_version,
    )?;
    let final_status = agent_update_status_in(&store)?;
    let checks = qualification_checks(
        &store,
        first_version,
        second_version,
        &first_activation,
        &second_activation,
        &rollback_activation,
        &status_after_upgrade.active_version,
        &final_status.active_version,
    );
    if checks.iter().any(|check| !check.ok) {
        return Err("agent update operational qualification checks failed".to_string());
    }

    Ok(AgentUpdateQualificationReport {
        schema_version: AGENT_UPDATE_QUALIFICATION_SCHEMA_VERSION.to_string(),
        status: "pass".to_string(),
        journey: "packaged-installed-agent-update-and-rollback".to_string(),
        execution_host_role: qualification_host_role(platform),
        platform: platform.as_str().to_string(),
        first_version: first_version.to_string(),
        second_version: second_version.to_string(),
        first_activation,
        second_activation,
        rollback_activation,
        active_after_upgrade: status_after_upgrade.active_version.unwrap_or_default(),
        active_after_rollback: final_status.active_version.unwrap_or_default(),
        installed_versions: final_status.installed_versions,
        probes: vec![first_probe, second_probe, rollback_probe],
        checks,
    })
}

pub fn write_agent_update_qualification_report(
    report: &AgentUpdateQualificationReport,
    path: &Path,
) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
    }
    let payload = serde_json::to_vec_pretty(report).map_err(|error| error.to_string())?;
    fs::write(path, payload).map_err(|error| format!("failed to write {}: {error}", path.display()))
}

fn prepare_empty_root(work_root: &Path) -> Result<(), String> {
    if work_root
        .symlink_metadata()
        .is_ok_and(|metadata| metadata.file_type().is_symlink())
    {
        return Err("agent qualification work root must not be a symlink".to_string());
    }
    if work_root.exists()
        && fs::read_dir(work_root)
            .map_err(|error| format!("failed to read {}: {error}", work_root.display()))?
            .next()
            .is_some()
    {
        return Err("agent qualification work root must be empty".to_string());
    }
    fs::create_dir_all(work_root)
        .map_err(|error| format!("failed to create {}: {error}", work_root.display()))
}

fn run_agent_probe(
    binary: &Path,
    phase: &str,
    version: &str,
) -> Result<AgentUpdateExecutionProbe, String> {
    let job_id = format!("agent-update-{phase}");
    let output = Command::new(binary)
        .args(["--steps", "1", "--job-id", &job_id])
        .output()
        .map_err(|error| format!("failed to execute installed agent: {error}"))?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let job_id_observed = stdout.contains(&job_id) || stderr.contains(&job_id);
    if !output.status.success() || !job_id_observed {
        return Err(format!(
            "installed agent probe failed for {phase}: status={} job_id_observed={job_id_observed}",
            output.status
        ));
    }
    Ok(AgentUpdateExecutionProbe {
        phase: phase.to_string(),
        version: version.to_string(),
        success: true,
        job_id_observed,
    })
}

fn qualification_host_role(platform: Platform) -> String {
    let transport = if std::env::var_os("SSH_CONNECTION").is_some() {
        "remote"
    } else {
        "local"
    };
    format!("{transport}-{}-qualification-host", platform.as_str())
}

#[allow(clippy::too_many_arguments)]
fn qualification_checks(
    store: &Path,
    first_version: &str,
    second_version: &str,
    first: &AgentUpdateActivationRecord,
    second: &AgentUpdateActivationRecord,
    rollback: &AgentUpdateActivationRecord,
    active_after_upgrade: &Option<String>,
    active_after_rollback: &Option<String>,
) -> Vec<AgentUpdateQualificationCheck> {
    vec![
        check("initial_activation", first.version == first_version),
        check(
            "upgrade_activation",
            second.version == second_version
                && second.previous_version.as_deref() == Some(first_version),
        ),
        check(
            "rollback_activation",
            rollback.version == first_version
                && rollback.previous_version.as_deref() == Some(second_version),
        ),
        check(
            "active_after_upgrade",
            active_after_upgrade.as_deref() == Some(second_version),
        ),
        check(
            "active_after_rollback",
            active_after_rollback.as_deref() == Some(first_version),
        ),
        check("atomic_generations", second.generation > first.generation),
        check(
            "rollback_generation",
            rollback.generation > second.generation,
        ),
        check(
            "payload_changed",
            first.entrypoint_sha256 != second.entrypoint_sha256,
        ),
        check("update_lock_clean", !store.join(".update.lock").exists()),
        check("staging_clean", directory_is_empty(&store.join("staging"))),
    ]
}

fn directory_is_empty(path: &Path) -> bool {
    fs::read_dir(path)
        .ok()
        .is_some_and(|mut entries| entries.next().is_none())
}

fn check(id: &str, ok: bool) -> AgentUpdateQualificationCheck {
    AgentUpdateQualificationCheck {
        id: id.to_string(),
        ok,
    }
}
