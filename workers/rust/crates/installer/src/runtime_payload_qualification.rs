use crate::Platform;
use crate::runtime_payload::{
    RuntimeActivationRecord, install_runtime_payload_into, rollback_runtime_payload_in,
    runtime_payload_content_digest_in, runtime_payload_status_in, seal_runtime_payload,
    verified_runtime_service_launches_in,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;
use std::process::Command;

pub const RUNTIME_PAYLOAD_QUALIFICATION_SCHEMA_VERSION: &str =
    "kyuubiki.runtime-payload-qualification/v1";

const SERVICE_IDS: &[&str] = &["agent", "orchestrator", "frontend"];

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimePayloadQualificationReport {
    pub schema_version: String,
    pub status: String,
    pub journey: String,
    pub execution_host_role: String,
    pub platform: String,
    pub first_version: String,
    pub second_version: String,
    pub first_activation: RuntimeActivationRecord,
    pub second_activation: RuntimeActivationRecord,
    pub rollback_activation: RuntimeActivationRecord,
    pub active_after_upgrade: String,
    pub active_after_rollback: String,
    pub installed_versions: Vec<String>,
    pub first_payload_sha256: String,
    pub second_payload_sha256: String,
    pub rollback_payload_sha256: String,
    pub probes: Vec<RuntimePayloadExecutionProbe>,
    pub checks: Vec<RuntimePayloadQualificationCheck>,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimePayloadExecutionProbe {
    pub phase: String,
    pub version: String,
    pub service_id: String,
    pub success: bool,
    pub job_id_observed: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimePayloadQualificationCheck {
    pub id: String,
    pub ok: bool,
}

pub fn run_runtime_payload_qualification(
    first_binary: &Path,
    second_binary: &Path,
    work_root: &Path,
    first_version: &str,
    second_version: &str,
) -> Result<RuntimePayloadQualificationReport, String> {
    if first_version == second_version {
        return Err("runtime qualification versions must be distinct".to_string());
    }
    prepare_empty_root(work_root)?;
    let work_root = fs::canonicalize(work_root).map_err(|error| {
        format!(
            "failed to resolve runtime qualification root {}: {error}",
            work_root.display()
        )
    })?;
    let platform = Platform::current();
    let first_payload = work_root.join("payloads/first");
    let second_payload = work_root.join("payloads/second");
    let store = work_root.join("managed-store");
    prepare_runtime_payload(first_binary, &first_payload, first_version, platform)?;
    prepare_runtime_payload(second_binary, &second_payload, second_version, platform)?;

    let first_activation = install_runtime_payload_into(&first_payload, &store, platform)?;
    let first_payload_sha256 = active_payload_digest(&store, &first_activation, platform)?;
    let mut probes = run_service_probes(&store, &first_activation, platform, "initial-install")?;

    let second_activation = install_runtime_payload_into(&second_payload, &store, platform)?;
    let second_payload_sha256 = active_payload_digest(&store, &second_activation, platform)?;
    probes.extend(run_service_probes(
        &store,
        &second_activation,
        platform,
        "upgraded-install",
    )?);
    let status_after_upgrade = runtime_payload_status_in(&store)?;

    let rollback_activation = rollback_runtime_payload_in(&store, platform)?;
    let rollback_payload_sha256 = active_payload_digest(&store, &rollback_activation, platform)?;
    probes.extend(run_service_probes(
        &store,
        &rollback_activation,
        platform,
        "rollback",
    )?);
    let final_status = runtime_payload_status_in(&store)?;

    let checks = qualification_checks(
        &store,
        first_version,
        second_version,
        &first_activation,
        &second_activation,
        &rollback_activation,
        &status_after_upgrade.active_version,
        &final_status.active_version,
        &first_payload_sha256,
        &second_payload_sha256,
        &rollback_payload_sha256,
        &probes,
        platform,
    );
    if checks.iter().any(|check| !check.ok) {
        return Err("runtime payload operational qualification checks failed".to_string());
    }

    let report = RuntimePayloadQualificationReport {
        schema_version: RUNTIME_PAYLOAD_QUALIFICATION_SCHEMA_VERSION.to_string(),
        status: "pass".to_string(),
        journey: "sealed-installed-runtime-payload-upgrade-and-rollback".to_string(),
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
        first_payload_sha256,
        second_payload_sha256,
        rollback_payload_sha256,
        probes,
        checks,
    };
    crate::runtime_payload_qualification_validation::validate_runtime_payload_qualification_report(
        &report,
    )
    .map_err(|errors| {
        format!(
            "runtime payload qualification is invalid: {}",
            errors.join("; ")
        )
    })?;
    Ok(report)
}

pub fn write_runtime_payload_qualification_report(
    report: &RuntimePayloadQualificationReport,
    path: &Path,
) -> Result<(), String> {
    crate::runtime_payload_qualification_validation::validate_runtime_payload_qualification_report(
        report,
    )
    .map_err(|errors| {
        format!(
            "runtime payload qualification report is invalid: {}",
            errors.join("; ")
        )
    })?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
    }
    let payload = serde_json::to_vec_pretty(report).map_err(|error| error.to_string())?;
    fs::write(path, payload).map_err(|error| format!("failed to write {}: {error}", path.display()))
}

fn prepare_runtime_payload(
    binary: &Path,
    payload: &Path,
    version: &str,
    platform: Platform,
) -> Result<(), String> {
    verify_probe_binary(binary, platform)?;
    let entries = service_entries(platform);
    for (_, command, _) in &entries {
        let target = payload.join(command);
        fs::create_dir_all(
            target
                .parent()
                .ok_or_else(|| "runtime probe target has no parent".to_string())?,
        )
        .map_err(|error| format!("failed to create runtime probe directory: {error}"))?;
        fs::copy(binary, &target).map_err(|error| {
            format!(
                "failed to copy {} to {}: {error}",
                binary.display(),
                target.display()
            )
        })?;
    }
    let services = entries
        .iter()
        .map(|(id, command, cwd)| serde_json::json!({"id": id, "command": command, "cwd": cwd}))
        .collect::<Vec<_>>();
    let manifest_path = payload.join("manifests/service-launch.json");
    fs::create_dir_all(manifest_path.parent().unwrap())
        .map_err(|error| format!("failed to create service manifest directory: {error}"))?;
    let manifest = serde_json::json!({
        "schema_version": "kyuubiki.service-launch/v1",
        "services": services
    });
    fs::write(
        &manifest_path,
        serde_json::to_vec_pretty(&manifest).map_err(|error| error.to_string())?,
    )
    .map_err(|error| format!("failed to write {}: {error}", manifest_path.display()))?;
    seal_runtime_payload(payload, version, platform).map(|_| ())
}

fn verify_probe_binary(binary: &Path, platform: Platform) -> Result<(), String> {
    let metadata = binary
        .symlink_metadata()
        .map_err(|error| format!("failed to inspect {}: {error}", binary.display()))?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err("runtime qualification probe must be a regular file".to_string());
    }
    #[cfg(unix)]
    if platform != Platform::Windows {
        use std::os::unix::fs::PermissionsExt;
        if metadata.permissions().mode() & 0o111 == 0 {
            return Err("runtime qualification probe must be executable".to_string());
        }
    }
    Ok(())
}

fn service_entries(platform: Platform) -> Vec<(&'static str, String, &'static str)> {
    let suffix = if platform == Platform::Windows {
        ".exe"
    } else {
        ""
    };
    vec![
        ("agent", format!("bin/kyuubiki-cli{suffix}"), "."),
        (
            "orchestrator",
            format!("services/orchestrator/bin/kyuubiki_web{suffix}"),
            "services/orchestrator",
        ),
        (
            "frontend",
            format!("services/frontend/bin/kyuubiki_frontend_probe{suffix}"),
            "services/frontend",
        ),
    ]
}

fn run_service_probes(
    store: &Path,
    activation: &RuntimeActivationRecord,
    platform: Platform,
    phase: &str,
) -> Result<Vec<RuntimePayloadExecutionProbe>, String> {
    let root = store.join(&activation.relative_path);
    let launches = verified_runtime_service_launches_in(&root, platform)?;
    let observed = launches
        .iter()
        .map(|launch| launch.id.as_str())
        .collect::<BTreeSet<_>>();
    let expected = SERVICE_IDS.iter().copied().collect::<BTreeSet<_>>();
    if observed != expected || launches.len() != SERVICE_IDS.len() {
        return Err("runtime qualification service set drifted".to_string());
    }
    let mut probes = Vec::new();
    for service_id in SERVICE_IDS {
        let launch = launches
            .iter()
            .find(|launch| launch.id == *service_id)
            .ok_or_else(|| format!("missing runtime service {service_id}"))?;
        let job_id = format!("runtime-payload-{phase}-{service_id}");
        let output = Command::new(&launch.command)
            .current_dir(&launch.cwd)
            .args(["--steps", "1", "--job-id", &job_id])
            .output()
            .map_err(|error| format!("failed to execute runtime service {service_id}: {error}"))?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        let job_id_observed = stdout.contains(&job_id) || stderr.contains(&job_id);
        if !output.status.success() || !job_id_observed {
            return Err(format!(
                "runtime service probe failed for {phase}/{service_id}: status={} job_id_observed={job_id_observed}",
                output.status
            ));
        }
        probes.push(RuntimePayloadExecutionProbe {
            phase: phase.to_string(),
            version: activation.version.clone(),
            service_id: service_id.to_string(),
            success: true,
            job_id_observed,
        });
    }
    Ok(probes)
}

fn active_payload_digest(
    store: &Path,
    activation: &RuntimeActivationRecord,
    platform: Platform,
) -> Result<String, String> {
    runtime_payload_content_digest_in(&store.join(&activation.relative_path), platform)
}

#[allow(clippy::too_many_arguments)]
fn qualification_checks(
    store: &Path,
    first_version: &str,
    second_version: &str,
    first: &RuntimeActivationRecord,
    second: &RuntimeActivationRecord,
    rollback: &RuntimeActivationRecord,
    active_after_upgrade: &Option<String>,
    active_after_rollback: &Option<String>,
    first_digest: &str,
    second_digest: &str,
    rollback_digest: &str,
    probes: &[RuntimePayloadExecutionProbe],
    platform: Platform,
) -> Vec<RuntimePayloadQualificationCheck> {
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
        check(
            "monotonic_generations",
            first.generation == 1 && second.generation == 2 && rollback.generation == 3,
        ),
        check("payload_changed", first_digest != second_digest),
        check(
            "rollback_payload_restored",
            first_digest != second_digest && rollback_digest == first_digest,
        ),
        check(
            "all_service_entries_executed",
            probes.len() == SERVICE_IDS.len() * 3
                && probes
                    .iter()
                    .all(|probe| probe.success && probe.job_id_observed),
        ),
        check(
            "immutable_versions_verified",
            runtime_payload_content_digest_in(
                &store.join("versions").join(first_version),
                platform,
            )
            .is_ok()
                && runtime_payload_content_digest_in(
                    &store.join("versions").join(second_version),
                    platform,
                )
                .is_ok(),
        ),
        check("update_lock_clean", !store.join(".update.lock").exists()),
        check("staging_clean", directory_is_empty(&store.join("staging"))),
    ]
}

fn qualification_host_role(platform: Platform) -> String {
    let transport = if std::env::var_os("SSH_CONNECTION").is_some() {
        "remote"
    } else {
        "local"
    };
    format!("{transport}-{}-qualification-host", platform.as_str())
}

fn prepare_empty_root(work_root: &Path) -> Result<(), String> {
    if work_root
        .symlink_metadata()
        .is_ok_and(|metadata| metadata.file_type().is_symlink())
    {
        return Err("runtime qualification work root must not be a symlink".to_string());
    }
    if work_root.exists()
        && fs::read_dir(work_root)
            .map_err(|error| format!("failed to read {}: {error}", work_root.display()))?
            .next()
            .is_some()
    {
        return Err("runtime qualification work root must be empty".to_string());
    }
    fs::create_dir_all(work_root)
        .map_err(|error| format!("failed to create {}: {error}", work_root.display()))
}

fn directory_is_empty(path: &Path) -> bool {
    fs::read_dir(path)
        .ok()
        .is_some_and(|mut entries| entries.next().is_none())
}

fn check(id: &str, ok: bool) -> RuntimePayloadQualificationCheck {
    RuntimePayloadQualificationCheck {
        id: id.to_string(),
        ok,
    }
}
