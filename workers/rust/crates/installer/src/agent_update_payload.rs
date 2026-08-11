use kyuubiki_platform::{Platform, desktop_preferences_dir};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

pub const AGENT_UPDATE_PACKAGE_SCHEMA_VERSION: &str = "kyuubiki.agent-update-package/v1";
pub const AGENT_UPDATE_ACTIVATION_SCHEMA_VERSION: &str = "kyuubiki.agent-update-activation/v1";
const PACKAGE_MANIFEST: &str = "manifests/agent-update.json";

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct AgentUpdatePackageManifest {
    pub schema_version: String,
    pub version: String,
    pub platform: String,
    pub entrypoint: String,
    pub entrypoint_sha256: String,
    pub entrypoint_size_bytes: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct AgentUpdateActivationRecord {
    pub schema_version: String,
    pub generation: u64,
    pub version: String,
    pub previous_version: Option<String>,
    pub relative_path: String,
    pub platform: String,
    pub entrypoint_sha256: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct AgentUpdateStatus {
    pub store_root: String,
    pub active_version: Option<String>,
    pub previous_version: Option<String>,
    pub installed_versions: Vec<String>,
}

impl AgentUpdateActivationRecord {
    pub fn render(&self) -> String {
        [
            "kyuubiki agent update activation".to_string(),
            format!("version: {}", self.version),
            format!(
                "previous_version: {}",
                self.previous_version.as_deref().unwrap_or("--")
            ),
            format!("generation: {}", self.generation),
            format!("relative_path: {}", self.relative_path),
            format!("platform: {}", self.platform),
            format!("entrypoint_sha256: {}", self.entrypoint_sha256),
        ]
        .join("\n")
    }
}

impl AgentUpdateStatus {
    pub fn render(&self) -> String {
        [
            "kyuubiki agent update status".to_string(),
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

pub fn seal_agent_update_package(
    package_root: &Path,
    version: &str,
    platform: Platform,
) -> Result<AgentUpdatePackageManifest, String> {
    validate_version(version)?;
    let entrypoint = agent_entrypoint(platform);
    let binary = package_root.join(entrypoint);
    verify_regular_executable(&binary, platform)?;
    ensure_package_shape(package_root, entrypoint, false)?;
    let manifest = AgentUpdatePackageManifest {
        schema_version: AGENT_UPDATE_PACKAGE_SCHEMA_VERSION.to_string(),
        version: version.to_string(),
        platform: platform.as_str().to_string(),
        entrypoint: entrypoint.to_string(),
        entrypoint_sha256: sha256_file(&binary)?,
        entrypoint_size_bytes: binary
            .metadata()
            .map_err(|error| format!("failed to inspect {}: {error}", binary.display()))?
            .len(),
    };
    write_json(&package_root.join(PACKAGE_MANIFEST), &manifest)?;
    verify_agent_update_package(package_root, platform)
}

pub fn prepare_agent_update_package(
    binary: &Path,
    package_root: &Path,
    version: &str,
    platform: Platform,
) -> Result<AgentUpdatePackageManifest, String> {
    validate_version(version)?;
    verify_regular_executable(binary, platform)?;
    reject_symlink(package_root, "agent update package root")?;
    if package_root.exists()
        && fs::read_dir(package_root)
            .map_err(|error| format!("failed to read {}: {error}", package_root.display()))?
            .next()
            .is_some()
    {
        return Err("agent update package root must be empty".to_string());
    }
    let target = package_root.join(agent_entrypoint(platform));
    fs::create_dir_all(
        target
            .parent()
            .ok_or_else(|| "agent update entrypoint has no parent".to_string())?,
    )
    .map_err(|error| format!("failed to create agent update package: {error}"))?;
    fs::copy(binary, &target)
        .map_err(|error| format!("failed to copy agent update entrypoint: {error}"))?;
    seal_agent_update_package(package_root, version, platform)
}

pub fn verify_agent_update_package(
    package_root: &Path,
    platform: Platform,
) -> Result<AgentUpdatePackageManifest, String> {
    let manifest: AgentUpdatePackageManifest = read_json(&package_root.join(PACKAGE_MANIFEST))?;
    if manifest.schema_version != AGENT_UPDATE_PACKAGE_SCHEMA_VERSION {
        return Err("unsupported agent update package schema".to_string());
    }
    validate_version(&manifest.version)?;
    if manifest.platform != platform.as_str() {
        return Err(format!(
            "agent update package targets {}, current platform is {}",
            manifest.platform,
            platform.as_str()
        ));
    }
    let expected_entrypoint = agent_entrypoint(platform);
    if manifest.entrypoint != expected_entrypoint {
        return Err(format!(
            "agent update entrypoint must be {expected_entrypoint}"
        ));
    }
    ensure_package_shape(package_root, expected_entrypoint, true)?;
    let binary = package_root.join(expected_entrypoint);
    verify_regular_executable(&binary, platform)?;
    let metadata = binary
        .metadata()
        .map_err(|error| format!("failed to inspect {}: {error}", binary.display()))?;
    if metadata.len() != manifest.entrypoint_size_bytes {
        return Err("agent update entrypoint size mismatch".to_string());
    }
    if sha256_file(&binary)? != manifest.entrypoint_sha256 {
        return Err("agent update entrypoint digest mismatch".to_string());
    }
    Ok(manifest)
}

pub fn install_agent_update_package(
    package_root: &Path,
) -> Result<AgentUpdateActivationRecord, String> {
    install_agent_update_package_into(
        package_root,
        &agent_update_store_root()?,
        Platform::current(),
    )
}

pub fn rollback_agent_update() -> Result<AgentUpdateActivationRecord, String> {
    rollback_agent_update_in(&agent_update_store_root()?, Platform::current())
}

pub fn agent_update_status() -> Result<AgentUpdateStatus, String> {
    agent_update_status_in(&agent_update_store_root()?)
}

pub fn active_agent_binary() -> Result<PathBuf, String> {
    active_agent_binary_in(&agent_update_store_root()?, Platform::current())
}

pub fn launch_managed_agent(args: &[String]) -> Result<i32, String> {
    let binary = active_agent_binary()?;
    let status = Command::new(&binary)
        .args(args)
        .status()
        .map_err(|error| format!("failed to launch {}: {error}", binary.display()))?;
    Ok(status.code().unwrap_or(1))
}

pub(crate) fn install_agent_update_package_into(
    package_root: &Path,
    store: &Path,
    platform: Platform,
) -> Result<AgentUpdateActivationRecord, String> {
    ensure_store(store)?;
    let _lock = AgentUpdateLock::acquire(store)?;
    let manifest = verify_agent_update_package(package_root, platform)?;
    let target = store.join("versions").join(&manifest.version);
    reject_symlink(&target, "agent version target")?;
    if target.exists() {
        let installed = verify_agent_update_package(&target, platform)?;
        if installed != manifest {
            return Err(format!(
                "agent version {} already exists with different content",
                manifest.version
            ));
        }
    } else {
        install_new_version(package_root, store, &manifest, platform)?;
    }
    activate_version(store, &manifest, platform)
}

pub(crate) fn rollback_agent_update_in(
    store: &Path,
    platform: Platform,
) -> Result<AgentUpdateActivationRecord, String> {
    ensure_store(store)?;
    let _lock = AgentUpdateLock::acquire(store)?;
    let active = latest_activation(store)?
        .ok_or_else(|| "no active installer-managed agent is available".to_string())?;
    let previous = active
        .previous_version
        .ok_or_else(|| "no previous agent version is available for rollback".to_string())?;
    let manifest = verify_agent_update_package(&store.join("versions").join(previous), platform)?;
    activate_version(store, &manifest, platform)
}

pub(crate) fn agent_update_status_in(store: &Path) -> Result<AgentUpdateStatus, String> {
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
    Ok(AgentUpdateStatus {
        store_root: store.display().to_string(),
        active_version: active.as_ref().map(|record| record.version.clone()),
        previous_version: active.and_then(|record| record.previous_version),
        installed_versions,
    })
}

pub(crate) fn active_agent_binary_in(store: &Path, platform: Platform) -> Result<PathBuf, String> {
    let active = latest_activation(store)?
        .ok_or_else(|| "no active installer-managed agent is available".to_string())?;
    validate_activation_record(&active, platform)?;
    let package = store.join(&active.relative_path);
    let manifest = verify_agent_update_package(&package, platform)?;
    if manifest.version != active.version || manifest.entrypoint_sha256 != active.entrypoint_sha256
    {
        return Err("active agent activation does not match its verified package".to_string());
    }
    Ok(package.join(manifest.entrypoint))
}

fn install_new_version(
    package_root: &Path,
    store: &Path,
    manifest: &AgentUpdatePackageManifest,
    platform: Platform,
) -> Result<(), String> {
    let staging = store.join("staging").join(&manifest.version);
    reject_symlink(&staging, "agent staging target")?;
    if staging.exists() {
        fs::remove_dir_all(&staging)
            .map_err(|error| format!("failed to reset {}: {error}", staging.display()))?;
    }
    fs::create_dir_all(staging.join("bin"))
        .map_err(|error| format!("failed to create agent staging bin: {error}"))?;
    fs::create_dir_all(staging.join("manifests"))
        .map_err(|error| format!("failed to create agent staging manifests: {error}"))?;
    for relative in [&manifest.entrypoint, PACKAGE_MANIFEST] {
        fs::copy(package_root.join(relative), staging.join(relative))
            .map_err(|error| format!("failed to stage agent update {relative}: {error}"))?;
    }
    if let Err(error) = verify_agent_update_package(&staging, platform) {
        let _ = fs::remove_dir_all(&staging);
        return Err(error);
    }
    fs::rename(&staging, store.join("versions").join(&manifest.version)).map_err(|error| {
        format!(
            "failed to promote agent version {}: {error}",
            manifest.version
        )
    })
}

fn activate_version(
    store: &Path,
    manifest: &AgentUpdatePackageManifest,
    platform: Platform,
) -> Result<AgentUpdateActivationRecord, String> {
    verify_agent_update_package(&store.join("versions").join(&manifest.version), platform)?;
    let previous_version = latest_activation(store)?.and_then(|record| {
        if record.version == manifest.version {
            record.previous_version
        } else {
            Some(record.version)
        }
    });
    let generation = next_generation(store)?;
    let record = AgentUpdateActivationRecord {
        schema_version: AGENT_UPDATE_ACTIVATION_SCHEMA_VERSION.to_string(),
        generation,
        version: manifest.version.clone(),
        previous_version,
        relative_path: format!("versions/{}", manifest.version),
        platform: platform.as_str().to_string(),
        entrypoint_sha256: manifest.entrypoint_sha256.clone(),
    };
    let final_path = store
        .join("activations")
        .join(format!("{generation:020}.json"));
    let temporary = store
        .join("activations")
        .join(format!(".{generation:020}.tmp"));
    write_json(&temporary, &record)?;
    fs::rename(&temporary, &final_path)
        .map_err(|error| format!("failed to atomically activate agent version: {error}"))?;
    Ok(record)
}

fn ensure_store(store: &Path) -> Result<(), String> {
    reject_symlink(store, "agent update store")?;
    fs::create_dir_all(store)
        .map_err(|error| format!("failed to create {}: {error}", store.display()))?;
    for child in ["versions", "staging", "activations"] {
        let path = store.join(child);
        reject_symlink(&path, "agent update managed directory")?;
        fs::create_dir_all(&path)
            .map_err(|error| format!("failed to create {}: {error}", path.display()))?;
    }
    Ok(())
}

fn validate_activation_record(
    record: &AgentUpdateActivationRecord,
    platform: Platform,
) -> Result<(), String> {
    if record.schema_version != AGENT_UPDATE_ACTIVATION_SCHEMA_VERSION {
        return Err("unsupported agent update activation schema".to_string());
    }
    validate_version(&record.version)?;
    if record.platform != platform.as_str() {
        return Err("active agent activation targets another platform".to_string());
    }
    if record.relative_path != format!("versions/{}", record.version) {
        return Err("active agent activation path is not canonical".to_string());
    }
    if record.entrypoint_sha256.len() != 64
        || !record
            .entrypoint_sha256
            .bytes()
            .all(|value| value.is_ascii_hexdigit() && !value.is_ascii_uppercase())
    {
        return Err("active agent activation digest is malformed".to_string());
    }
    Ok(())
}

fn latest_activation(store: &Path) -> Result<Option<AgentUpdateActivationRecord>, String> {
    let mut records = activation_records(store)?;
    records.sort_by_key(|record| record.generation);
    Ok(records.pop())
}

fn activation_records(store: &Path) -> Result<Vec<AgentUpdateActivationRecord>, String> {
    let path = store.join("activations");
    if !path.exists() {
        return Ok(Vec::new());
    }
    fs::read_dir(&path)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().extension().and_then(|value| value.to_str()) == Some("json"))
        .map(|entry| read_json(&entry.path()))
        .collect()
}

fn next_generation(store: &Path) -> Result<u64, String> {
    Ok(activation_records(store)?
        .iter()
        .map(|record| record.generation)
        .max()
        .unwrap_or(0)
        + 1)
}

fn ensure_package_shape(
    root: &Path,
    entrypoint: &str,
    manifest_required: bool,
) -> Result<(), String> {
    let mut observed = Vec::new();
    collect_package_files(root, root, &mut observed)?;
    observed.sort();
    let mut expected = vec![entrypoint.to_string()];
    if manifest_required {
        expected.push(PACKAGE_MANIFEST.to_string());
    }
    expected.sort();
    if observed != expected {
        return Err(format!(
            "agent update package contains undeclared files: expected {expected:?}, observed {observed:?}"
        ));
    }
    Ok(())
}

fn collect_package_files(
    root: &Path,
    current: &Path,
    output: &mut Vec<String>,
) -> Result<(), String> {
    for entry in fs::read_dir(current)
        .map_err(|error| format!("failed to read {}: {error}", current.display()))?
    {
        let entry = entry.map_err(|error| error.to_string())?;
        let file_type = entry
            .file_type()
            .map_err(|error| format!("failed to inspect {}: {error}", entry.path().display()))?;
        if file_type.is_symlink() {
            return Err(format!(
                "agent update package cannot contain symlinks: {}",
                entry.path().display()
            ));
        }
        if file_type.is_dir() {
            collect_package_files(root, &entry.path(), output)?;
        } else if file_type.is_file() {
            output.push(portable_relative(root, &entry.path())?);
        } else {
            return Err(format!(
                "agent update package contains unsupported entry: {}",
                entry.path().display()
            ));
        }
    }
    Ok(())
}

fn verify_regular_executable(path: &Path, platform: Platform) -> Result<(), String> {
    let metadata = fs::symlink_metadata(path).map_err(|error| {
        format!(
            "agent update entrypoint is missing {}: {error}",
            path.display()
        )
    })?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err("agent update entrypoint must be a regular file".to_string());
    }
    if metadata.len() == 0 {
        return Err("agent update entrypoint must not be empty".to_string());
    }
    #[cfg(unix)]
    if platform != Platform::Windows {
        use std::os::unix::fs::PermissionsExt;
        if metadata.permissions().mode() & 0o111 == 0 {
            return Err("agent update entrypoint is not executable".to_string());
        }
    }
    Ok(())
}

fn validate_version(version: &str) -> Result<(), String> {
    if version.is_empty()
        || version.len() > 64
        || !version
            .bytes()
            .all(|value| value.is_ascii_alphanumeric() || matches!(value, b'.' | b'-' | b'_'))
    {
        return Err("agent update version contains unsupported characters".to_string());
    }
    Ok(())
}

fn agent_entrypoint(platform: Platform) -> &'static str {
    if platform == Platform::Windows {
        "bin/kyuubiki-agent.exe"
    } else {
        "bin/kyuubiki-agent"
    }
}

fn agent_update_store_root() -> Result<PathBuf, String> {
    Ok(desktop_preferences_dir("kyuubiki")?.join("agent"))
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

fn portable_relative(root: &Path, path: &Path) -> Result<String, String> {
    path.strip_prefix(root)
        .map_err(|_| format!("path escaped agent package: {}", path.display()))
        .map(|relative| relative.to_string_lossy().replace('\\', "/"))
}

fn sha256_file(path: &Path) -> Result<String, String> {
    let mut file =
        File::open(path).map_err(|error| format!("failed to open {}: {error}", path.display()))?;
    let mut digest = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
        if read == 0 {
            break;
        }
        digest.update(&buffer[..read]);
    }
    Ok(format!("{:x}", digest.finalize()))
}

fn write_json<T: Serialize>(path: &Path, value: &T) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
    }
    let payload = serde_json::to_vec_pretty(value).map_err(|error| error.to_string())?;
    fs::write(path, payload).map_err(|error| format!("failed to write {}: {error}", path.display()))
}

fn read_json<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T, String> {
    let payload =
        fs::read(path).map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    serde_json::from_slice(&payload)
        .map_err(|error| format!("failed to parse {}: {error}", path.display()))
}

struct AgentUpdateLock {
    path: PathBuf,
}

impl AgentUpdateLock {
    fn acquire(store: &Path) -> Result<Self, String> {
        let path = store.join(".update.lock");
        let mut file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&path)
            .map_err(|error| format!("agent update lock is unavailable: {error}"))?;
        writeln!(file, "pid={}", std::process::id())
            .map_err(|error| format!("failed to write agent update lock: {error}"))?;
        Ok(Self { path })
    }
}

impl Drop for AgentUpdateLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}
