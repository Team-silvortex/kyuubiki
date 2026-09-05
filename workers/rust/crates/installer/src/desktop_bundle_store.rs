use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use kyuubiki_platform::{Platform, desktop_preferences_dir};
use serde::{Deserialize, Serialize};

use crate::desktop_bundle_package::{
    DesktopBundleSetManifest, copy_verified_desktop_bundle_set, manifest_by_component,
    verify_desktop_bundle_set,
};

pub const DESKTOP_BUNDLE_ACTIVATION_SCHEMA_VERSION: &str = "kyuubiki.desktop-bundle-activation/v1";

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct DesktopBundleActivationRecord {
    pub schema_version: String,
    pub generation: u64,
    pub version: String,
    pub previous_version: Option<String>,
    pub relative_path: String,
    pub platform: String,
    pub payload_sha256: String,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct DesktopBundleSetStatus {
    pub store_root: String,
    pub active_version: Option<String>,
    pub previous_version: Option<String>,
    pub active_payload_sha256: Option<String>,
    pub installed_versions: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DesktopBundleEntrypoint {
    pub component_id: String,
    pub bundle_path: PathBuf,
    pub executable_path: PathBuf,
}

impl DesktopBundleActivationRecord {
    pub fn render(&self) -> String {
        [
            "kyuubiki desktop bundle activation".to_string(),
            format!("version: {}", self.version),
            format!(
                "previous_version: {}",
                self.previous_version.as_deref().unwrap_or("--")
            ),
            format!("generation: {}", self.generation),
            format!("relative_path: {}", self.relative_path),
            format!("platform: {}", self.platform),
            format!("payload_sha256: {}", self.payload_sha256),
        ]
        .join("\n")
    }
}

impl DesktopBundleSetStatus {
    pub fn render(&self) -> String {
        [
            "kyuubiki desktop bundle status".to_string(),
            format!("store_root: {}", self.store_root),
            format!(
                "active_version: {}",
                self.active_version.as_deref().unwrap_or("--")
            ),
            format!(
                "previous_version: {}",
                self.previous_version.as_deref().unwrap_or("--")
            ),
            format!(
                "active_payload_sha256: {}",
                self.active_payload_sha256.as_deref().unwrap_or("--")
            ),
            format!(
                "installed_versions: {}",
                if self.installed_versions.is_empty() {
                    "--".to_string()
                } else {
                    self.installed_versions.join(", ")
                }
            ),
        ]
        .join("\n")
    }
}

pub fn install_desktop_bundle_set(
    package_root: &Path,
) -> Result<DesktopBundleActivationRecord, String> {
    install_desktop_bundle_set_into(
        package_root,
        &desktop_bundle_store_root()?,
        Platform::current(),
    )
}

pub fn rollback_desktop_bundle_set() -> Result<DesktopBundleActivationRecord, String> {
    rollback_desktop_bundle_set_in(&desktop_bundle_store_root()?, Platform::current())
}

pub fn desktop_bundle_set_status() -> Result<DesktopBundleSetStatus, String> {
    desktop_bundle_set_status_in(&desktop_bundle_store_root()?)
}

pub fn active_desktop_bundle_root() -> Result<PathBuf, String> {
    active_desktop_bundle_root_in(&desktop_bundle_store_root()?, Platform::current())
}

pub fn active_desktop_bundle_entrypoints() -> Result<Vec<DesktopBundleEntrypoint>, String> {
    active_desktop_bundle_entrypoints_in(&desktop_bundle_store_root()?, Platform::current())
}

pub(crate) fn install_desktop_bundle_set_into(
    package_root: &Path,
    store: &Path,
    platform: Platform,
) -> Result<DesktopBundleActivationRecord, String> {
    ensure_store(store)?;
    let _lock = DesktopBundleUpdateLock::acquire(store)?;
    clean_staging(store)?;
    let manifest = verify_desktop_bundle_set(package_root, platform)?;
    let target = version_root(store, &manifest.version);
    reject_symlink(&target, "desktop bundle version target")?;
    if target.exists() {
        let installed = verify_desktop_bundle_set(&target, platform)?;
        if installed != manifest {
            return Err(format!(
                "desktop bundle version {} already exists with different content",
                manifest.version
            ));
        }
    } else {
        let staging = store.join("staging").join(format!(
            "{}-{:020}",
            manifest.version,
            next_generation(store)?
        ));
        copy_verified_desktop_bundle_set(package_root, &staging, platform).inspect_err(|_| {
            let _ = remove_path(&staging);
        })?;
        fs::rename(&staging, &target).map_err(|error| {
            format!(
                "failed to promote desktop bundle {}: {error}",
                manifest.version
            )
        })?;
    }
    activate_version(store, &manifest, platform)
}

pub(crate) fn rollback_desktop_bundle_set_in(
    store: &Path,
    platform: Platform,
) -> Result<DesktopBundleActivationRecord, String> {
    ensure_store(store)?;
    let _lock = DesktopBundleUpdateLock::acquire(store)?;
    let active = latest_activation(store)?
        .ok_or_else(|| "no active installer-managed desktop bundle is available".to_string())?;
    validate_activation(&active, platform)?;
    let previous = active
        .previous_version
        .clone()
        .or_else(|| {
            previous_distinct_version(store, &active.version)
                .ok()
                .flatten()
        })
        .ok_or_else(|| {
            "no previous desktop bundle version is available for rollback".to_string()
        })?;
    let manifest = verify_desktop_bundle_set(&version_root(store, &previous), platform)?;
    activate_version(store, &manifest, platform)
}

pub(crate) fn desktop_bundle_set_status_in(store: &Path) -> Result<DesktopBundleSetStatus, String> {
    let active = latest_activation(store)?;
    let mut installed_versions = fs::read_dir(store.join("versions"))
        .ok()
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .filter(|entry| entry.path().is_dir())
        .filter_map(|entry| entry.file_name().into_string().ok())
        .collect::<Vec<_>>();
    installed_versions.sort();
    Ok(DesktopBundleSetStatus {
        store_root: store.display().to_string(),
        active_version: active.as_ref().map(|record| record.version.clone()),
        previous_version: active
            .as_ref()
            .and_then(|record| record.previous_version.clone()),
        active_payload_sha256: active.map(|record| record.payload_sha256),
        installed_versions,
    })
}

pub(crate) fn active_desktop_bundle_root_in(
    store: &Path,
    platform: Platform,
) -> Result<PathBuf, String> {
    let active = active_activation_in(store, platform)?;
    Ok(store.join(active.relative_path))
}

pub(crate) fn active_desktop_bundle_entrypoints_in(
    store: &Path,
    platform: Platform,
) -> Result<Vec<DesktopBundleEntrypoint>, String> {
    let root = active_desktop_bundle_root_in(store, platform)?;
    let manifest = verify_desktop_bundle_set(&root, platform)?;
    let components = manifest_by_component(&manifest);
    ["hub", "installer", "workbench"]
        .into_iter()
        .map(|id| {
            let component = components
                .get(id)
                .ok_or_else(|| format!("active desktop bundle is missing `{id}`"))?;
            Ok(DesktopBundleEntrypoint {
                component_id: id.to_string(),
                bundle_path: root.join(&component.bundle_path),
                executable_path: root.join(&component.entrypoint),
            })
        })
        .collect()
}

pub(crate) fn active_desktop_bundle_manifest_in(
    store: &Path,
    platform: Platform,
) -> Result<DesktopBundleSetManifest, String> {
    let root = active_desktop_bundle_root_in(store, platform)?;
    verify_desktop_bundle_set(&root, platform)
}

fn active_activation_in(
    store: &Path,
    platform: Platform,
) -> Result<DesktopBundleActivationRecord, String> {
    let active = latest_activation(store)?
        .ok_or_else(|| "no active installer-managed desktop bundle is available".to_string())?;
    validate_activation(&active, platform)?;
    let root = store.join(&active.relative_path);
    let manifest = verify_desktop_bundle_set(&root, platform)?;
    if manifest.version != active.version || manifest.payload_sha256 != active.payload_sha256 {
        return Err("active desktop activation does not match its verified bundle set".to_string());
    }
    Ok(active)
}

fn activate_version(
    store: &Path,
    manifest: &DesktopBundleSetManifest,
    platform: Platform,
) -> Result<DesktopBundleActivationRecord, String> {
    let installed = verify_desktop_bundle_set(&version_root(store, &manifest.version), platform)?;
    if installed != *manifest {
        return Err("desktop activation target changed after verification".to_string());
    }
    let previous_version = latest_activation(store)?.and_then(|record| {
        if record.version == manifest.version {
            record.previous_version
        } else {
            Some(record.version)
        }
    });
    let generation = next_generation(store)?;
    let record = DesktopBundleActivationRecord {
        schema_version: DESKTOP_BUNDLE_ACTIVATION_SCHEMA_VERSION.to_string(),
        generation,
        version: manifest.version.clone(),
        previous_version,
        relative_path: format!("versions/{}", manifest.version),
        platform: platform.as_str().to_string(),
        payload_sha256: manifest.payload_sha256.clone(),
    };
    let temporary = store
        .join("activations")
        .join(format!(".{generation:020}.tmp"));
    let final_path = store
        .join("activations")
        .join(format!("{generation:020}.json"));
    if temporary.exists() {
        fs::remove_file(&temporary)
            .map_err(|error| format!("failed to clear {}: {error}", temporary.display()))?;
    }
    write_json(&temporary, &record)?;
    fs::rename(&temporary, &final_path).map_err(|error| {
        format!(
            "failed to atomically activate desktop bundle {}: {error}",
            record.version
        )
    })?;
    Ok(record)
}

fn validate_activation(
    record: &DesktopBundleActivationRecord,
    platform: Platform,
) -> Result<(), String> {
    if record.schema_version != DESKTOP_BUNDLE_ACTIVATION_SCHEMA_VERSION {
        return Err("unsupported desktop bundle activation schema".to_string());
    }
    if record.generation == 0
        || record.platform != platform.as_str()
        || record.relative_path != format!("versions/{}", record.version)
        || !valid_sha256(&record.payload_sha256)
    {
        return Err("desktop bundle activation record is inconsistent".to_string());
    }
    Ok(())
}

fn latest_activation(store: &Path) -> Result<Option<DesktopBundleActivationRecord>, String> {
    let mut records = activation_records(store)?;
    records.sort_by_key(|record| record.generation);
    Ok(records.pop())
}

fn previous_distinct_version(store: &Path, active: &str) -> Result<Option<String>, String> {
    let mut records = activation_records(store)?;
    records.sort_by_key(|record| record.generation);
    Ok(records
        .into_iter()
        .rev()
        .find(|record| record.version != active)
        .map(|record| record.version))
}

fn activation_records(store: &Path) -> Result<Vec<DesktopBundleActivationRecord>, String> {
    let directory = store.join("activations");
    if !directory.exists() {
        return Ok(Vec::new());
    }
    fs::read_dir(&directory)
        .map_err(|error| format!("failed to read {}: {error}", directory.display()))?
        .filter_map(|entry| match entry {
            Ok(entry)
                if entry.path().extension().and_then(|value| value.to_str()) == Some("json") =>
            {
                Some(read_json::<DesktopBundleActivationRecord>(&entry.path()))
            }
            Ok(_) => None,
            Err(error) => Some(Err(error.to_string())),
        })
        .collect()
}

fn next_generation(store: &Path) -> Result<u64, String> {
    activation_records(store)?
        .into_iter()
        .map(|record| record.generation)
        .max()
        .unwrap_or(0)
        .checked_add(1)
        .ok_or_else(|| "desktop bundle activation generation overflowed".to_string())
}

fn ensure_store(store: &Path) -> Result<(), String> {
    reject_symlink(store, "desktop bundle store")?;
    for directory in ["versions", "staging", "activations"] {
        let path = store.join(directory);
        reject_symlink(&path, "desktop bundle store directory")?;
        fs::create_dir_all(&path)
            .map_err(|error| format!("failed to create {}: {error}", path.display()))?;
    }
    Ok(())
}

fn clean_staging(store: &Path) -> Result<(), String> {
    let staging = store.join("staging");
    for entry in fs::read_dir(&staging)
        .map_err(|error| format!("failed to read {}: {error}", staging.display()))?
    {
        let path = entry
            .map_err(|error| format!("failed to read {}: {error}", staging.display()))?
            .path();
        remove_path(&path)?;
    }
    Ok(())
}

fn remove_path(path: &Path) -> Result<(), String> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|error| format!("failed to inspect {}: {error}", path.display()))?;
    if metadata.is_dir() && !metadata.file_type().is_symlink() {
        fs::remove_dir_all(path)
            .map_err(|error| format!("failed to remove {}: {error}", path.display()))
    } else {
        fs::remove_file(path)
            .map_err(|error| format!("failed to remove {}: {error}", path.display()))
    }
}

fn version_root(store: &Path, version: &str) -> PathBuf {
    store.join("versions").join(version)
}

fn desktop_bundle_store_root() -> Result<PathBuf, String> {
    Ok(desktop_preferences_dir("kyuubiki")?.join("desktop-bundles"))
}

fn reject_symlink(path: &Path, label: &str) -> Result<(), String> {
    if let Ok(metadata) = fs::symlink_metadata(path)
        && metadata.file_type().is_symlink()
    {
        return Err(format!("{label} cannot be a symlink: {}", path.display()));
    }
    Ok(())
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

fn write_json<T: Serialize>(path: &Path, value: &T) -> Result<(), String> {
    let bytes = serde_json::to_vec_pretty(value)
        .map_err(|error| format!("failed to serialize {}: {error}", path.display()))?;
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(path)
        .map_err(|error| format!("failed to create {}: {error}", path.display()))?;
    file.write_all(&bytes)
        .map_err(|error| format!("failed to write {}: {error}", path.display()))?;
    file.sync_all()
        .map_err(|error| format!("failed to sync {}: {error}", path.display()))
}

fn read_json<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T, String> {
    let bytes =
        fs::read(path).map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    serde_json::from_slice(&bytes)
        .map_err(|error| format!("failed to parse {}: {error}", path.display()))
}

struct DesktopBundleUpdateLock {
    path: PathBuf,
    _file: File,
}

impl DesktopBundleUpdateLock {
    fn acquire(store: &Path) -> Result<Self, String> {
        let path = store.join("update.lock");
        reject_symlink(&path, "desktop bundle update lock")?;
        let mut file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&path)
            .map_err(|error| format!("desktop bundle update is already locked: {error}"))?;
        writeln!(file, "{}", std::process::id())
            .map_err(|error| format!("failed to write desktop bundle update lock: {error}"))?;
        file.sync_all()
            .map_err(|error| format!("failed to sync desktop bundle update lock: {error}"))?;
        Ok(Self { path, _file: file })
    }
}

impl Drop for DesktopBundleUpdateLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}
