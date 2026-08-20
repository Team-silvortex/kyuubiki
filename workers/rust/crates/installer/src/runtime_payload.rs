use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};

use kyuubiki_platform::{Platform, desktop_preferences_dir};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const PAYLOAD_SCHEMA: &str = "kyuubiki.runtime-payload/v1";
pub const RUNTIME_ACTIVATION_SCHEMA_VERSION: &str = "kyuubiki.runtime-activation/v1";
const PAYLOAD_MANIFEST: &str = "manifests/runtime-payload.json";
const MUTABLE_ROOTS: &[&str] = &["data", "exports", "logs", "run"];

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
struct RuntimePayloadManifest {
    schema_version: String,
    version: String,
    platform: String,
    service_manifest: String,
    files: Vec<RuntimePayloadFile>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
struct RuntimePayloadFile {
    path: String,
    sha256: String,
    executable: bool,
}

#[derive(Clone, Debug, Deserialize)]
struct ServiceLaunchManifest {
    schema_version: String,
    services: Vec<ServiceLaunchEntry>,
}

#[derive(Clone, Debug, Deserialize)]
struct ServiceLaunchEntry {
    id: String,
    command: String,
    cwd: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RuntimeActivationRecord {
    pub schema_version: String,
    pub generation: u64,
    pub version: String,
    pub previous_version: Option<String>,
    pub relative_path: String,
    pub platform: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimePayloadStatus {
    pub store_root: String,
    pub active_version: Option<String>,
    pub previous_version: Option<String>,
    pub installed_versions: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RuntimeServiceLaunch {
    pub id: String,
    pub command: PathBuf,
    pub cwd: PathBuf,
}

impl RuntimeActivationRecord {
    pub fn render(&self) -> String {
        [
            "kyuubiki runtime activation".to_string(),
            format!("version: {}", self.version),
            format!(
                "previous_version: {}",
                self.previous_version.as_deref().unwrap_or("--")
            ),
            format!("generation: {}", self.generation),
            format!("relative_path: {}", self.relative_path),
            format!("platform: {}", self.platform),
        ]
        .join("\n")
    }
}

impl RuntimePayloadStatus {
    pub fn render(&self) -> String {
        [
            "kyuubiki runtime payload status".to_string(),
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

pub fn seal_runtime_payload(
    payload_root: &Path,
    version: &str,
    platform: Platform,
) -> Result<String, String> {
    validate_version(version)?;
    let service_manifest = payload_root.join("manifests/service-launch.json");
    validate_service_manifest(payload_root, &service_manifest)?;
    let mut paths = Vec::new();
    collect_files(payload_root, payload_root, &mut paths)?;
    paths.sort();
    let files = paths
        .into_iter()
        .filter(|path| path != Path::new(PAYLOAD_MANIFEST))
        .map(|relative| {
            let full = payload_root.join(&relative);
            Ok(RuntimePayloadFile {
                path: portable_path(&relative)?,
                sha256: sha256_file(&full)?,
                executable: is_executable(&full)?,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    let manifest = RuntimePayloadManifest {
        schema_version: PAYLOAD_SCHEMA.to_string(),
        version: version.to_string(),
        platform: platform.as_str().to_string(),
        service_manifest: "manifests/service-launch.json".to_string(),
        files,
    };
    let path = payload_root.join(PAYLOAD_MANIFEST);
    write_json(&path, &manifest)?;
    verify_payload(payload_root, Some(platform))?;
    Ok(path.display().to_string())
}

pub fn install_runtime_payload(source: &Path) -> Result<RuntimeActivationRecord, String> {
    let store = runtime_store_root()?;
    install_runtime_payload_into(source, &store, Platform::current())
}

pub fn rollback_runtime_payload() -> Result<RuntimeActivationRecord, String> {
    let store = runtime_store_root()?;
    rollback_runtime_payload_in(&store, Platform::current())
}

pub fn runtime_payload_status() -> Result<RuntimePayloadStatus, String> {
    runtime_payload_status_in(&runtime_store_root()?)
}

pub(crate) fn install_runtime_payload_into(
    source: &Path,
    store: &Path,
    platform: Platform,
) -> Result<RuntimeActivationRecord, String> {
    ensure_store(store)?;
    let _lock = RuntimeUpdateLock::acquire(store)?;
    reject_symlink(source, "runtime payload source")?;
    let manifest = verify_payload(source, Some(platform))?;
    let target = store.join("versions").join(&manifest.version);
    reject_symlink(&target, "runtime payload version target")?;
    if target.exists() {
        let installed = verify_installed_payload(&target, platform)?;
        if installed != manifest {
            return Err(format!(
                "runtime version {} already exists with different content; repair or remove it explicitly",
                manifest.version
            ));
        }
    } else {
        let staging =
            store
                .join("staging")
                .join(format!("{}-{}", manifest.version, next_generation(store)?));
        reject_symlink(&staging, "runtime payload staging target")?;
        if staging.exists() {
            fs::remove_dir_all(&staging)
                .map_err(|error| format!("failed to clear {}: {error}", staging.display()))?;
        }
        let staged = copy_manifest_files(source, &staging, &manifest)
            .and_then(|()| verify_installed_payload(&staging, platform).map(|_| ()));
        if let Err(error) = staged {
            let _ = fs::remove_dir_all(&staging);
            return Err(error);
        }
        fs::rename(&staging, &target).map_err(|error| {
            format!(
                "failed to promote runtime payload {} to {}: {error}",
                staging.display(),
                target.display()
            )
        })?;
    }
    activate_version(store, &manifest.version, platform)
}

pub(crate) fn rollback_runtime_payload_in(
    store: &Path,
    platform: Platform,
) -> Result<RuntimeActivationRecord, String> {
    ensure_store(store)?;
    let _lock = RuntimeUpdateLock::acquire(store)?;
    let active = latest_activation(store)?
        .ok_or_else(|| "no active installer-managed runtime is available".to_string())?;
    let previous = active
        .previous_version
        .clone()
        .or_else(|| {
            previous_distinct_version(store, &active.version)
                .ok()
                .flatten()
        })
        .ok_or_else(|| "no previous runtime version is available for rollback".to_string())?;
    let previous_root = store.join("versions").join(&previous);
    reject_symlink(&previous_root, "runtime payload rollback target")?;
    verify_installed_payload(&previous_root, platform)?;
    activate_version(store, &previous, platform)
}

pub(crate) fn runtime_payload_status_in(store: &Path) -> Result<RuntimePayloadStatus, String> {
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
    Ok(RuntimePayloadStatus {
        store_root: store.display().to_string(),
        active_version: active.as_ref().map(|record| record.version.clone()),
        previous_version: active.and_then(|record| record.previous_version),
        installed_versions,
    })
}

fn runtime_store_root() -> Result<PathBuf, String> {
    Ok(desktop_preferences_dir("kyuubiki")?.join("runtime"))
}

fn activate_version(
    store: &Path,
    version: &str,
    platform: Platform,
) -> Result<RuntimeActivationRecord, String> {
    validate_version(version)?;
    let version_root = store.join("versions").join(version);
    reject_symlink(&version_root, "runtime payload activation target")?;
    let manifest = verify_installed_payload(&version_root, platform)?;
    if manifest.version != version {
        return Err(format!(
            "runtime payload activation identity mismatch: expected {version}, found {}",
            manifest.version
        ));
    }
    let previous_version = latest_activation(store)?.and_then(|active| {
        if active.version == version {
            active.previous_version
        } else {
            Some(active.version)
        }
    });
    let generation = next_generation(store)?;
    let record = RuntimeActivationRecord {
        schema_version: RUNTIME_ACTIVATION_SCHEMA_VERSION.to_string(),
        generation,
        version: version.to_string(),
        previous_version,
        relative_path: format!("versions/{version}"),
        platform: platform.as_str().to_string(),
    };
    let activations = store.join("activations");
    fs::create_dir_all(&activations)
        .map_err(|error| format!("failed to create {}: {error}", activations.display()))?;
    let final_path = activations.join(format!("{generation:020}.json"));
    let temporary = activations.join(format!(".{generation:020}.tmp"));
    write_json(&temporary, &record)?;
    fs::rename(&temporary, &final_path).map_err(|error| {
        format!(
            "failed to atomically activate runtime {}: {error}",
            record.version
        )
    })?;
    Ok(record)
}

fn latest_activation(store: &Path) -> Result<Option<RuntimeActivationRecord>, String> {
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

fn activation_records(store: &Path) -> Result<Vec<RuntimeActivationRecord>, String> {
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
                Some(read_activation(&entry.path()))
            }
            Ok(_) => None,
            Err(error) => Some(Err(error.to_string())),
        })
        .collect()
}

fn read_activation(path: &Path) -> Result<RuntimeActivationRecord, String> {
    let record: RuntimeActivationRecord = read_json(path)?;
    if record.schema_version != RUNTIME_ACTIVATION_SCHEMA_VERSION {
        return Err(format!(
            "unsupported runtime activation schema in {}",
            path.display()
        ));
    }
    validate_version(&record.version)?;
    if record.relative_path != format!("versions/{}", record.version) {
        return Err(format!(
            "runtime activation path/version mismatch in {}",
            path.display()
        ));
    }
    checked_relative(&record.relative_path)?;
    Ok(record)
}

fn verify_payload(
    root: &Path,
    expected_platform: Option<Platform>,
) -> Result<RuntimePayloadManifest, String> {
    reject_symlink(root, "runtime payload root")?;
    let path = root.join(PAYLOAD_MANIFEST);
    let manifest: RuntimePayloadManifest = read_json(&path)?;
    if manifest.schema_version != PAYLOAD_SCHEMA {
        return Err(format!(
            "unsupported runtime payload schema in {}",
            path.display()
        ));
    }
    validate_version(&manifest.version)?;
    if let Some(platform) = expected_platform
        && manifest.platform != platform.as_str()
    {
        return Err(format!(
            "runtime payload targets {}, but this operation requires {}",
            manifest.platform,
            platform.as_str()
        ));
    }
    let mut seen = BTreeSet::new();
    for file in &manifest.files {
        checked_relative(&file.path)?;
        if !seen.insert(&file.path) {
            return Err(format!("duplicate runtime payload file: {}", file.path));
        }
        let full = root.join(&file.path);
        if full
            .symlink_metadata()
            .is_ok_and(|metadata| metadata.file_type().is_symlink())
        {
            return Err(format!(
                "runtime payload cannot contain symlinks: {}",
                file.path
            ));
        }
        if !full.is_file() {
            return Err(format!(
                "runtime payload file is missing: {}",
                full.display()
            ));
        }
        let actual = sha256_file(&full)?;
        if actual != file.sha256 {
            return Err(format!("runtime payload digest mismatch: {}", file.path));
        }
        if file.executable && !is_executable(&full)? {
            return Err(format!(
                "runtime payload executable bit is missing: {}",
                file.path
            ));
        }
    }
    let service_path = checked_relative(&manifest.service_manifest)?;
    validate_service_manifest(root, &root.join(service_path))?;
    Ok(manifest)
}

fn verify_installed_payload(
    root: &Path,
    platform: Platform,
) -> Result<RuntimePayloadManifest, String> {
    let manifest = verify_payload(root, Some(platform))?;
    for name in MUTABLE_ROOTS {
        if root.join(name).exists() {
            return Err(format!(
                "immutable runtime version contains mutable `{name}` state: {}",
                root.display()
            ));
        }
    }
    Ok(manifest)
}

pub(crate) fn runtime_payload_content_digest_in(
    root: &Path,
    platform: Platform,
) -> Result<String, String> {
    let manifest = verify_installed_payload(root, platform)?;
    let mut files = manifest.files;
    files.sort_by(|left, right| left.path.cmp(&right.path));
    let mut digest = Sha256::new();
    digest.update(PAYLOAD_SCHEMA.as_bytes());
    digest.update([0]);
    digest.update(platform.as_str().as_bytes());
    digest.update([0]);
    digest.update(manifest.service_manifest.as_bytes());
    digest.update([0]);
    for file in files {
        digest.update(file.path.as_bytes());
        digest.update([0]);
        digest.update(file.sha256.as_bytes());
        digest.update([u8::from(file.executable)]);
    }
    Ok(format!("{:x}", digest.finalize()))
}

pub(crate) fn verified_runtime_service_launches_in(
    root: &Path,
    platform: Platform,
) -> Result<Vec<RuntimeServiceLaunch>, String> {
    let manifest = verify_installed_payload(root, platform)?;
    let service_path = root.join(checked_relative(&manifest.service_manifest)?);
    let services: ServiceLaunchManifest = read_json(&service_path)?;
    services
        .services
        .into_iter()
        .map(|entry| {
            Ok(RuntimeServiceLaunch {
                id: entry.id,
                command: root.join(checked_relative(&entry.command.replace("{port}", "5001"))?),
                cwd: root.join(checked_relative(&entry.cwd.replace("{port}", "5001"))?),
            })
        })
        .collect()
}

fn validate_service_manifest(root: &Path, path: &Path) -> Result<(), String> {
    let manifest: ServiceLaunchManifest = read_json(path)?;
    if manifest.schema_version != "kyuubiki.service-launch/v1" {
        return Err(format!(
            "unsupported service launch schema in {}",
            path.display()
        ));
    }
    let mut services = BTreeMap::new();
    for entry in manifest.services {
        if entry.id.is_empty() || services.insert(entry.id.clone(), entry).is_some() {
            return Err(format!(
                "empty or duplicate service id in {}",
                path.display()
            ));
        }
    }
    for id in ["agent", "orchestrator", "frontend"] {
        let entry = services
            .get(id)
            .ok_or_else(|| format!("service launch manifest is missing `{id}`"))?;
        let command = root.join(checked_relative(&entry.command.replace("{port}", "5001"))?);
        let cwd = root.join(checked_relative(&entry.cwd.replace("{port}", "5001"))?);
        if !command.is_file() {
            return Err(format!(
                "service `{id}` command is missing: {}",
                command.display()
            ));
        }
        if !cwd.is_dir() {
            return Err(format!("service `{id}` cwd is missing: {}", cwd.display()));
        }
    }
    Ok(())
}

fn collect_files(root: &Path, current: &Path, output: &mut Vec<PathBuf>) -> Result<(), String> {
    for entry in fs::read_dir(current)
        .map_err(|error| format!("failed to read {}: {error}", current.display()))?
    {
        let entry = entry.map_err(|error| error.to_string())?;
        let metadata = entry
            .file_type()
            .map_err(|error| format!("failed to inspect {}: {error}", entry.path().display()))?;
        if metadata.is_symlink() {
            return Err(format!(
                "runtime payload cannot contain symlinks: {}",
                entry.path().display()
            ));
        }
        if metadata.is_dir() {
            if current == root
                && entry
                    .file_name()
                    .to_str()
                    .is_some_and(|name| MUTABLE_ROOTS.contains(&name))
            {
                continue;
            }
            collect_files(root, &entry.path(), output)?;
        } else if metadata.is_file() {
            output.push(
                entry
                    .path()
                    .strip_prefix(root)
                    .map_err(|error| error.to_string())?
                    .to_path_buf(),
            );
        }
    }
    Ok(())
}

fn copy_manifest_files(
    source: &Path,
    target: &Path,
    manifest: &RuntimePayloadManifest,
) -> Result<(), String> {
    fs::create_dir_all(target)
        .map_err(|error| format!("failed to create {}: {error}", target.display()))?;
    for file in &manifest.files {
        let relative = checked_relative(&file.path)?;
        copy_file(&source.join(&relative), &target.join(relative))?;
    }
    copy_file(
        &source.join(PAYLOAD_MANIFEST),
        &target.join(PAYLOAD_MANIFEST),
    )
}

fn copy_file(source: &Path, target: &Path) -> Result<(), String> {
    if source
        .symlink_metadata()
        .is_ok_and(|metadata| metadata.file_type().is_symlink())
    {
        return Err(format!(
            "runtime payload cannot contain symlinks: {}",
            source.display()
        ));
    }
    let parent = target.parent().ok_or_else(|| {
        format!(
            "runtime payload destination has no parent: {}",
            target.display()
        )
    })?;
    fs::create_dir_all(parent)
        .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
    fs::copy(source, target).map(|_| ()).map_err(|error| {
        format!(
            "failed to copy {} to {}: {error}",
            source.display(),
            target.display()
        )
    })
}

fn checked_relative(value: &str) -> Result<PathBuf, String> {
    let path = Path::new(value);
    if value.is_empty()
        || path.is_absolute()
        || path.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        return Err(format!(
            "runtime payload path must be relative and contained: {value}"
        ));
    }
    Ok(path.to_path_buf())
}

fn validate_version(version: &str) -> Result<(), String> {
    if version.is_empty()
        || !version
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'_'))
    {
        return Err(format!("unsafe runtime version token: {version}"));
    }
    Ok(())
}

fn portable_path(path: &Path) -> Result<String, String> {
    path.components()
        .map(|component| match component {
            Component::Normal(value) => value
                .to_str()
                .map(ToString::to_string)
                .ok_or_else(|| format!("runtime payload path is not UTF-8: {}", path.display())),
            _ => Err(format!(
                "runtime payload path is not relative: {}",
                path.display()
            )),
        })
        .collect::<Result<Vec<_>, _>>()
        .map(|parts| parts.join("/"))
}

fn sha256_file(path: &Path) -> Result<String, String> {
    let mut file =
        File::open(path).map_err(|error| format!("failed to open {}: {error}", path.display()))?;
    let mut digest = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let count = file
            .read(&mut buffer)
            .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
        if count == 0 {
            break;
        }
        digest.update(&buffer[..count]);
    }
    Ok(format!("{:x}", digest.finalize()))
}

#[cfg(unix)]
fn is_executable(path: &Path) -> Result<bool, String> {
    use std::os::unix::fs::PermissionsExt;
    path.metadata()
        .map(|metadata| metadata.permissions().mode() & 0o111 != 0)
        .map_err(|error| format!("failed to inspect {}: {error}", path.display()))
}

#[cfg(not(unix))]
fn is_executable(_path: &Path) -> Result<bool, String> {
    Ok(false)
}

fn read_json<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T, String> {
    let bytes =
        fs::read(path).map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    serde_json::from_slice(&bytes)
        .map_err(|error| format!("failed to parse {}: {error}", path.display()))
}

fn write_json(path: &Path, value: &impl Serialize) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
    }
    let bytes = serde_json::to_vec_pretty(value).map_err(|error| error.to_string())?;
    let mut file = File::create(path)
        .map_err(|error| format!("failed to create {}: {error}", path.display()))?;
    file.write_all(&bytes)
        .map_err(|error| format!("failed to write {}: {error}", path.display()))?;
    file.sync_all()
        .map_err(|error| format!("failed to sync {}: {error}", path.display()))
}

fn next_generation(store: &Path) -> Result<u64, String> {
    activation_records(store)?
        .iter()
        .map(|record| record.generation)
        .max()
        .unwrap_or(0)
        .checked_add(1)
        .ok_or_else(|| "runtime activation generation overflowed".to_string())
}

fn ensure_store(store: &Path) -> Result<(), String> {
    reject_symlink(store, "runtime payload store")?;
    fs::create_dir_all(store)
        .map_err(|error| format!("failed to create {}: {error}", store.display()))?;
    for child in ["versions", "staging", "activations"] {
        let path = store.join(child);
        reject_symlink(&path, "runtime payload managed directory")?;
        fs::create_dir_all(&path)
            .map_err(|error| format!("failed to create {}: {error}", path.display()))?;
    }
    Ok(())
}

fn reject_symlink(path: &Path, label: &str) -> Result<(), String> {
    if path
        .symlink_metadata()
        .is_ok_and(|metadata| metadata.file_type().is_symlink())
    {
        return Err(format!("{label} must not be a symlink: {}", path.display()));
    }
    Ok(())
}

struct RuntimeUpdateLock {
    path: PathBuf,
}

impl RuntimeUpdateLock {
    fn acquire(store: &Path) -> Result<Self, String> {
        let path = store.join(".update.lock");
        let mut file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&path)
            .map_err(|error| format!("runtime payload update lock is unavailable: {error}"))?;
        writeln!(file, "pid={}", std::process::id())
            .map_err(|error| format!("failed to write runtime payload update lock: {error}"))?;
        Ok(Self { path })
    }
}

impl Drop for RuntimeUpdateLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}
