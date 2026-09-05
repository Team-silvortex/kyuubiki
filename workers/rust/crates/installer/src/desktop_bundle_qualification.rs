use std::collections::BTreeMap;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::Platform;
use crate::desktop_bundle_package::{DesktopBundleSetManifest, prepare_desktop_bundle_set};
use crate::desktop_bundle_store::{
    DesktopBundleActivationRecord, active_desktop_bundle_entrypoints_in,
    active_desktop_bundle_manifest_in, desktop_bundle_set_status_in,
    install_desktop_bundle_set_into, rollback_desktop_bundle_set_in,
};

pub const DESKTOP_BUNDLE_QUALIFICATION_SCHEMA_VERSION: &str =
    "kyuubiki.desktop-bundle-update-qualification/v1";
const BOOT_RECEIPT_SCHEMA: &str = "kyuubiki.packaged-desktop-boot-receipt/v1";
const BOOT_RECEIPT_ENV: &str = "KYUUBIKI_PACKAGED_BOOT_RECEIPT";
const RUNTIME_VERSION: &str = env!("CARGO_PKG_VERSION");
const PROBE_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct DesktopBundleQualificationReport {
    pub schema_version: String,
    pub status: String,
    pub journey: String,
    pub execution_host_role: String,
    pub platform: String,
    pub runtime_version: String,
    pub first_version: String,
    pub second_version: String,
    pub first_payload: DesktopBundlePayloadObservation,
    pub second_payload: DesktopBundlePayloadObservation,
    pub rollback_payload: DesktopBundlePayloadObservation,
    pub first_activation: DesktopBundleActivationRecord,
    pub second_activation: DesktopBundleActivationRecord,
    pub rollback_activation: DesktopBundleActivationRecord,
    pub active_after_upgrade: String,
    pub active_after_rollback: String,
    pub installed_versions: Vec<String>,
    pub probes: Vec<DesktopBundleBootProbe>,
    pub checks: Vec<DesktopBundleQualificationCheck>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct DesktopBundlePayloadObservation {
    pub version: String,
    pub payload_sha256: String,
    pub components: Vec<DesktopBundleComponentObservation>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct DesktopBundleComponentObservation {
    pub component_id: String,
    pub content_sha256: String,
    pub entrypoint_sha256: String,
    pub file_count: usize,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct DesktopBundleBootProbe {
    pub phase: String,
    pub package_version: String,
    pub component_id: String,
    pub runtime_version: String,
    pub executable_sha256: String,
    pub pid: u32,
    pub success: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct DesktopBundleQualificationCheck {
    pub id: String,
    pub ok: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct BootReceipt {
    schema_version: String,
    surface: String,
    version: String,
    pid: u32,
}

struct ChildGuard(Option<Child>);

impl Drop for ChildGuard {
    fn drop(&mut self) {
        if let Some(mut child) = self.0.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

pub fn run_desktop_bundle_qualification(
    first_source: &Path,
    second_source: &Path,
    work_root: &Path,
    first_version: &str,
    second_version: &str,
) -> Result<DesktopBundleQualificationReport, String> {
    if first_version == second_version {
        return Err("desktop qualification versions must be distinct".to_string());
    }
    prepare_empty_root(work_root)?;
    let platform = Platform::current();
    let first_package = work_root.join("packages/first");
    let second_package = work_root.join("packages/second");
    let store = work_root.join("managed-store");
    let first_manifest =
        prepare_desktop_bundle_set(first_source, &first_package, first_version, platform)?;
    let second_manifest =
        prepare_desktop_bundle_set(second_source, &second_package, second_version, platform)?;
    ensure_changed_payloads(&first_manifest, &second_manifest)?;

    let first_activation = install_desktop_bundle_set_into(&first_package, &store, platform)?;
    let mut probes = run_active_probes(&store, platform, "initial-install", first_version)?;
    let second_activation = install_desktop_bundle_set_into(&second_package, &store, platform)?;
    probes.extend(run_active_probes(
        &store,
        platform,
        "upgraded-install",
        second_version,
    )?);
    let status_after_upgrade = desktop_bundle_set_status_in(&store)?;
    let rollback_activation = rollback_desktop_bundle_set_in(&store, platform)?;
    probes.extend(run_active_probes(
        &store,
        platform,
        "rollback",
        first_version,
    )?);
    let final_status = desktop_bundle_set_status_in(&store)?;
    let rollback_manifest = active_desktop_bundle_manifest_in(&store, platform)?;
    let first_payload = observe_payload(&first_package, &first_manifest)?;
    let second_payload = observe_payload(&second_package, &second_manifest)?;
    let rollback_root = store.join(&rollback_activation.relative_path);
    let rollback_payload = observe_payload(&rollback_root, &rollback_manifest)?;
    let checks = qualification_checks(
        &store,
        first_version,
        second_version,
        &first_activation,
        &second_activation,
        &rollback_activation,
        &first_payload,
        &second_payload,
        &rollback_payload,
        status_after_upgrade.active_version.as_deref(),
        final_status.active_version.as_deref(),
        &probes,
    );
    if checks.iter().any(|check| !check.ok) {
        let failed = checks
            .iter()
            .filter(|check| !check.ok)
            .map(|check| check.id.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        return Err(format!(
            "desktop bundle operational qualification checks failed: {failed}"
        ));
    }

    Ok(DesktopBundleQualificationReport {
        schema_version: DESKTOP_BUNDLE_QUALIFICATION_SCHEMA_VERSION.to_string(),
        status: "pass".to_string(),
        journey: "installer-managed-packaged-desktop-set-upgrade-and-rollback".to_string(),
        execution_host_role: qualification_host_role(platform),
        platform: platform.as_str().to_string(),
        runtime_version: RUNTIME_VERSION.to_string(),
        first_version: first_version.to_string(),
        second_version: second_version.to_string(),
        first_payload,
        second_payload,
        rollback_payload,
        first_activation,
        second_activation,
        rollback_activation,
        active_after_upgrade: status_after_upgrade.active_version.unwrap_or_default(),
        active_after_rollback: final_status.active_version.unwrap_or_default(),
        installed_versions: final_status.installed_versions,
        probes,
        checks,
    })
}

pub fn write_desktop_bundle_qualification_report(
    report: &DesktopBundleQualificationReport,
    path: &Path,
) -> Result<(), String> {
    crate::desktop_bundle_qualification_validation::validate_desktop_bundle_qualification_report(
        report,
    )
    .map_err(|errors| {
        format!(
            "desktop bundle qualification report is invalid: {}",
            errors.join("; ")
        )
    })?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
    }
    let bytes = serde_json::to_vec_pretty(report).map_err(|error| error.to_string())?;
    fs::write(path, bytes).map_err(|error| format!("failed to write {}: {error}", path.display()))
}

fn run_active_probes(
    store: &Path,
    platform: Platform,
    phase: &str,
    package_version: &str,
) -> Result<Vec<DesktopBundleBootProbe>, String> {
    active_desktop_bundle_entrypoints_in(store, platform)?
        .into_iter()
        .map(|entry| {
            run_boot_probe(
                &entry.executable_path,
                &entry.component_id,
                phase,
                package_version,
                platform,
            )
        })
        .collect()
}

fn run_boot_probe(
    executable: &Path,
    component_id: &str,
    phase: &str,
    package_version: &str,
    platform: Platform,
) -> Result<DesktopBundleBootProbe, String> {
    let receipt_root = receipt_root(component_id, phase)?;
    let receipt_path = receipt_root.join("boot.json");
    let mut command = launch_command(executable, platform);
    let child = command
        .env(BOOT_RECEIPT_ENV, &receipt_path)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| {
            format!("failed to launch desktop component {component_id} during {phase}: {error}")
        })?;
    let spawned_pid = child.id();
    let mut guard = ChildGuard(Some(child));
    let deadline = Instant::now() + PROBE_TIMEOUT;
    let receipt = loop {
        if receipt_path.is_file() {
            break read_boot_receipt(&receipt_path, component_id)?;
        }
        if let Some(status) = guard
            .0
            .as_mut()
            .expect("desktop probe child should be present")
            .try_wait()
            .map_err(|error| format!("failed to inspect desktop probe: {error}"))?
        {
            return Err(format!(
                "desktop component {component_id} exited before readiness during {phase}: {status}"
            ));
        }
        if Instant::now() >= deadline {
            return Err(format!(
                "desktop component {component_id} did not report readiness during {phase}"
            ));
        }
        thread::sleep(Duration::from_millis(100));
    };
    if platform != Platform::Linux && receipt.pid != spawned_pid {
        return Err(format!(
            "desktop component {component_id} receipt pid {} did not match child {spawned_pid}",
            receipt.pid
        ));
    }
    drop(guard);
    let _ = fs::remove_dir_all(&receipt_root);
    Ok(DesktopBundleBootProbe {
        phase: phase.to_string(),
        package_version: package_version.to_string(),
        component_id: component_id.to_string(),
        runtime_version: receipt.version,
        executable_sha256: sha256_file(executable)?,
        pid: receipt.pid,
        success: true,
    })
}

fn launch_command(executable: &Path, platform: Platform) -> Command {
    if platform == Platform::Linux {
        let mut command = Command::new("dbus-run-session");
        command
            .args(["--", "xvfb-run", "-a"])
            .arg(executable)
            .env("GDK_BACKEND", "x11")
            .env("WEBKIT_DISABLE_COMPOSITING_MODE", "1")
            .env("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
        command
    } else {
        Command::new(executable)
    }
}

fn read_boot_receipt(path: &Path, component_id: &str) -> Result<BootReceipt, String> {
    let bytes = fs::read(path)
        .map_err(|error| format!("failed to read boot receipt {}: {error}", path.display()))?;
    let receipt: BootReceipt = serde_json::from_slice(&bytes)
        .map_err(|error| format!("invalid desktop boot receipt: {error}"))?;
    if receipt.schema_version != BOOT_RECEIPT_SCHEMA
        || receipt.surface != component_id
        || receipt.version != RUNTIME_VERSION
        || receipt.pid == 0
    {
        return Err(format!(
            "desktop boot receipt is inconsistent for {component_id}"
        ));
    }
    Ok(receipt)
}

fn observe_payload(
    package_root: &Path,
    manifest: &DesktopBundleSetManifest,
) -> Result<DesktopBundlePayloadObservation, String> {
    let mut components = manifest
        .components
        .iter()
        .map(|component| {
            Ok(DesktopBundleComponentObservation {
                component_id: component.id.clone(),
                content_sha256: component.content_sha256.clone(),
                entrypoint_sha256: sha256_file(&package_root.join(&component.entrypoint))?,
                file_count: component.file_count,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    components.sort_by(|left, right| left.component_id.cmp(&right.component_id));
    Ok(DesktopBundlePayloadObservation {
        version: manifest.version.clone(),
        payload_sha256: manifest.payload_sha256.clone(),
        components,
    })
}

fn ensure_changed_payloads(
    first: &DesktopBundleSetManifest,
    second: &DesktopBundleSetManifest,
) -> Result<(), String> {
    if first.payload_sha256 == second.payload_sha256 {
        return Err("desktop qualification payloads must have different content".to_string());
    }
    let first_components = first
        .components
        .iter()
        .map(|component| (component.id.as_str(), component.content_sha256.as_str()))
        .collect::<BTreeMap<_, _>>();
    if second.components.iter().any(|component| {
        first_components.get(component.id.as_str()) == Some(&component.content_sha256.as_str())
    }) {
        return Err(
            "every desktop component must change between qualification payloads".to_string(),
        );
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn qualification_checks(
    store: &Path,
    first_version: &str,
    second_version: &str,
    first: &DesktopBundleActivationRecord,
    second: &DesktopBundleActivationRecord,
    rollback: &DesktopBundleActivationRecord,
    first_payload: &DesktopBundlePayloadObservation,
    second_payload: &DesktopBundlePayloadObservation,
    rollback_payload: &DesktopBundlePayloadObservation,
    active_after_upgrade: Option<&str>,
    active_after_rollback: Option<&str>,
    probes: &[DesktopBundleBootProbe],
) -> Vec<DesktopBundleQualificationCheck> {
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
            active_after_upgrade == Some(second_version),
        ),
        check(
            "active_after_rollback",
            active_after_rollback == Some(first_version),
        ),
        check(
            "monotonic_generations",
            first.generation < second.generation && second.generation < rollback.generation,
        ),
        check(
            "all_component_payloads_changed",
            components_all_changed(first_payload, second_payload),
        ),
        check(
            "rollback_payload_restored",
            first_payload == rollback_payload
                && first.payload_sha256 == rollback.payload_sha256
                && first.payload_sha256 != second.payload_sha256,
        ),
        check(
            "initial_three_shell_boot",
            phase_passed(probes, "initial-install", first_version),
        ),
        check(
            "upgraded_three_shell_boot",
            phase_passed(probes, "upgraded-install", second_version),
        ),
        check(
            "rollback_three_shell_boot",
            phase_passed(probes, "rollback", first_version),
        ),
        check(
            "runtime_version_aligned",
            probes
                .iter()
                .all(|probe| probe.runtime_version == RUNTIME_VERSION),
        ),
        check("update_lock_clean", !store.join("update.lock").exists()),
        check("staging_clean", directory_is_empty(&store.join("staging"))),
    ]
}

fn components_all_changed(
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
    second.components.len() == 3
        && second.components.iter().all(|component| {
            first.get(component.component_id.as_str()) != Some(&component.content_sha256.as_str())
        })
}

fn phase_passed(probes: &[DesktopBundleBootProbe], phase: &str, version: &str) -> bool {
    let phase_probes = probes
        .iter()
        .filter(|probe| probe.phase == phase)
        .collect::<Vec<_>>();
    phase_probes.len() == 3
        && phase_probes
            .iter()
            .all(|probe| probe.package_version == version && probe.success && probe.pid > 0)
}

fn prepare_empty_root(work_root: &Path) -> Result<(), String> {
    if work_root
        .symlink_metadata()
        .is_ok_and(|metadata| metadata.file_type().is_symlink())
    {
        return Err("desktop qualification work root must not be a symlink".to_string());
    }
    if work_root.exists()
        && fs::read_dir(work_root)
            .map_err(|error| format!("failed to read {}: {error}", work_root.display()))?
            .next()
            .is_some()
    {
        return Err("desktop qualification work root must be empty".to_string());
    }
    fs::create_dir_all(work_root)
        .map_err(|error| format!("failed to create {}: {error}", work_root.display()))
}

fn receipt_root(component_id: &str, phase: &str) -> Result<PathBuf, String> {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "kyuubiki-desktop-update-{}-{nonce}-{component_id}-{phase}",
        std::process::id()
    ));
    fs::create_dir(&root)
        .map_err(|error| format!("failed to create {}: {error}", root.display()))?;
    Ok(root)
}

fn qualification_host_role(platform: Platform) -> String {
    let transport = if std::env::var_os("SSH_CONNECTION").is_some() {
        "remote"
    } else {
        "local"
    };
    format!("{transport}-{}-qualification-host", platform.as_str())
}

fn directory_is_empty(path: &Path) -> bool {
    fs::read_dir(path)
        .ok()
        .is_some_and(|mut entries| entries.next().is_none())
}

fn check(id: &str, ok: bool) -> DesktopBundleQualificationCheck {
    DesktopBundleQualificationCheck {
        id: id.to_string(),
        ok,
    }
}

fn sha256_file(path: &Path) -> Result<String, String> {
    let mut file = fs::File::open(path)
        .map_err(|error| format!("failed to open {}: {error}", path.display()))?;
    let mut digest = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let count = file
            .read(&mut buffer)
            .map_err(|error| format!("failed to hash {}: {error}", path.display()))?;
        if count == 0 {
            break;
        }
        digest.update(&buffer[..count]);
    }
    Ok(format!("{:x}", digest.finalize()))
}

#[cfg(test)]
mod tests {
    use super::{DesktopBundleComponentObservation, DesktopBundlePayloadObservation};
    use super::{components_all_changed, phase_passed};

    #[test]
    fn component_change_requires_every_shell_to_change() {
        let first = payload("a", ["1", "2", "3"]);
        assert!(components_all_changed(
            &first,
            &payload("b", ["4", "5", "6"])
        ));
        assert!(!components_all_changed(
            &first,
            &payload("b", ["1", "5", "6"])
        ));
    }

    #[test]
    fn empty_probe_set_cannot_claim_a_phase() {
        assert!(!phase_passed(&[], "rollback", "2.16.9"));
    }

    fn payload(version: &str, digests: [&str; 3]) -> DesktopBundlePayloadObservation {
        DesktopBundlePayloadObservation {
            version: version.to_string(),
            payload_sha256: "0".repeat(64),
            components: ["hub", "installer", "workbench"]
                .into_iter()
                .zip(digests)
                .map(|(id, digest)| DesktopBundleComponentObservation {
                    component_id: id.to_string(),
                    content_sha256: digest.to_string(),
                    entrypoint_sha256: digest.to_string(),
                    file_count: 1,
                })
                .collect(),
        }
    }
}
