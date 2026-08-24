use crate::operator_package_preflight;
use kyuubiki_operator_sdk::{
    DiscoveredOperatorPackage, OPERATOR_PACKAGE_MANIFEST_FILE, OperatorPackageLoadPlan,
    build_operator_package_load_plan, read_operator_package_manifest,
};
use kyuubiki_platform::desktop_preferences_dir;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub const MANAGED_OPERATOR_PACKAGE_RECEIPT_SCHEMA_VERSION: &str =
    "kyuubiki.managed-operator-package-receipt/v1";
pub const MANAGED_OPERATOR_PACKAGE_STATUS_SCHEMA_VERSION: &str =
    "kyuubiki.managed-operator-package-status/v1";
pub const MANAGED_OPERATOR_PACKAGE_REMOVAL_SCHEMA_VERSION: &str =
    "kyuubiki.managed-operator-package-removal/v2";

const RECEIPT_FILE: &str = "kyuubiki-managed-install.json";

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct ManagedOperatorPackageReceipt {
    pub schema_version: String,
    pub package_id: String,
    pub package_version: String,
    pub sdk_api_version: String,
    pub runtime: String,
    pub operator_ids: Vec<String>,
    pub relative_root: String,
    pub entrypoint_relative_path: String,
    pub entrypoint_sha256: String,
    pub entrypoint_size_bytes: u64,
    pub manifest_sha256: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ManagedOperatorPackageStatus {
    pub schema_version: String,
    pub store_root: String,
    pub packages_root: String,
    pub installed_package_count: usize,
    pub installed_packages: Vec<ManagedOperatorPackageReceipt>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ManagedOperatorPackageRemoval {
    pub schema_version: String,
    pub package_id: String,
    pub package_version: Option<String>,
    pub entrypoint_sha256: Option<String>,
    pub receipt_verified: Option<bool>,
    pub receipt_error: Option<String>,
    pub removed: bool,
    pub store_pruned: bool,
}

pub fn operator_package_store_root() -> Result<PathBuf, String> {
    Ok(desktop_preferences_dir("kyuubiki")?.join("operator-packages"))
}

pub fn install_operator_package(
    source_package_root: &Path,
) -> Result<ManagedOperatorPackageReceipt, String> {
    install_operator_package_into(source_package_root, &operator_package_store_root()?)
}

pub fn install_operator_package_into(
    source_package_root: &Path,
    store_root: &Path,
) -> Result<ManagedOperatorPackageReceipt, String> {
    let source_root = canonical_regular_directory(source_package_root, "operator package root")?;
    let source_plan = load_plan(&source_root)?;
    validate_component(&source_plan.manifest.package_id, "package_id")?;
    validate_component(&source_plan.manifest.package_version, "package_version")?;
    let entrypoint_relative = verified_entrypoint_relative(&source_root, &source_plan)?;
    ensure_store(store_root)?;
    let lock = OperatorPackageStoreLock::acquire(store_root)?;
    let target = store_root
        .join("packages")
        .join(&source_plan.manifest.package_id);
    reject_symlink(&target, "managed operator package target")?;

    let source_entrypoint_sha256 = sha256_file(&source_plan.entrypoint_path)?;
    let source_manifest_sha256 = sha256_file(&source_plan.manifest_path)?;
    if target.exists() {
        let installed = verify_managed_operator_package(&target)?;
        if installed.package_version == source_plan.manifest.package_version
            && installed.entrypoint_sha256 == source_entrypoint_sha256
            && installed.manifest_sha256 == source_manifest_sha256
        {
            drop(lock);
            return Ok(installed);
        }
        return Err(format!(
            "operator package {} is already installed with different content; uninstall it before replacement",
            source_plan.manifest.package_id
        ));
    }

    let staging_parent = match unique_staging_root(store_root, &source_plan.manifest.package_id) {
        Ok(path) => path,
        Err(error) => {
            drop(lock);
            let _ = prune_empty_store(store_root);
            return Err(error);
        }
    };
    let staging_package = staging_parent.join(&source_plan.manifest.package_id);
    let result = stage_and_promote(
        &source_plan,
        &entrypoint_relative,
        &staging_parent,
        &staging_package,
        &target,
        store_root,
    );
    let _ = fs::remove_dir_all(&staging_parent);
    drop(lock);
    if result.is_err() {
        let _ = prune_empty_store(store_root);
    }
    result
}

pub fn uninstall_operator_package(
    package_id: &str,
) -> Result<ManagedOperatorPackageRemoval, String> {
    uninstall_operator_package_from(&operator_package_store_root()?, package_id)
}

pub fn uninstall_operator_package_from(
    store_root: &Path,
    package_id: &str,
) -> Result<ManagedOperatorPackageRemoval, String> {
    validate_component(package_id, "package_id")?;
    ensure_store(store_root)?;
    let lock = OperatorPackageStoreLock::acquire(store_root)?;
    let target = store_root.join("packages").join(package_id);
    reject_symlink(&target, "managed operator package target")?;
    let removed = target.exists();
    let (receipt, receipt_verified, receipt_error) = if removed {
        match read_receipt(&target).and_then(|receipt| {
            validate_receipt(&receipt, &target)?;
            if receipt.package_id != package_id {
                return Err("managed operator package receipt identity mismatch".to_string());
            }
            Ok(receipt)
        }) {
            Ok(receipt) => (Some(receipt), Some(true), None),
            Err(error) => (None, Some(false), Some(error)),
        }
    } else {
        (None, None, None)
    };
    if removed {
        remove_package_target(&target)?;
    }
    drop(lock);
    let store_pruned = prune_empty_store(store_root)?;
    Ok(ManagedOperatorPackageRemoval {
        schema_version: MANAGED_OPERATOR_PACKAGE_REMOVAL_SCHEMA_VERSION.to_string(),
        package_id: package_id.to_string(),
        package_version: receipt.as_ref().map(|value| value.package_version.clone()),
        entrypoint_sha256: receipt.map(|value| value.entrypoint_sha256),
        receipt_verified,
        receipt_error,
        removed,
        store_pruned,
    })
}

pub fn managed_operator_package_status() -> Result<ManagedOperatorPackageStatus, String> {
    managed_operator_package_status_in(&operator_package_store_root()?)
}

pub fn managed_operator_package_status_in(
    store_root: &Path,
) -> Result<ManagedOperatorPackageStatus, String> {
    reject_symlink(store_root, "operator package store")?;
    let packages_root = store_root.join("packages");
    reject_symlink(&packages_root, "operator packages directory")?;
    let mut installed_packages = Vec::new();
    if packages_root.exists() {
        for entry in fs::read_dir(&packages_root)
            .map_err(|error| format!("failed to read {}: {error}", packages_root.display()))?
        {
            let entry = entry.map_err(|error| error.to_string())?;
            let file_type = entry.file_type().map_err(|error| {
                format!("failed to inspect {}: {error}", entry.path().display())
            })?;
            if file_type.is_symlink() || !file_type.is_dir() {
                return Err(format!(
                    "operator package store contains unsupported entry {}",
                    entry.path().display()
                ));
            }
            installed_packages.push(verify_managed_operator_package(&entry.path())?);
        }
    }
    installed_packages.sort_by(|left, right| left.package_id.cmp(&right.package_id));
    Ok(ManagedOperatorPackageStatus {
        schema_version: MANAGED_OPERATOR_PACKAGE_STATUS_SCHEMA_VERSION.to_string(),
        store_root: store_root.display().to_string(),
        packages_root: packages_root.display().to_string(),
        installed_package_count: installed_packages.len(),
        installed_packages,
    })
}

pub fn verify_managed_operator_package(
    package_root: &Path,
) -> Result<ManagedOperatorPackageReceipt, String> {
    let package_root = canonical_regular_directory(package_root, "managed operator package")?;
    let receipt = read_receipt(&package_root)?;
    validate_receipt(&receipt, &package_root)?;
    verify_managed_package_layout(&package_root, &receipt)?;
    let plan = load_plan(&package_root)?;
    let entrypoint_relative = verified_entrypoint_relative(&package_root, &plan)?;
    if receipt.package_id != plan.manifest.package_id
        || receipt.package_version != plan.manifest.package_version
        || receipt.sdk_api_version != plan.manifest.sdk_api_version
        || receipt.runtime != plan.manifest.runtime
    {
        return Err("managed operator package receipt does not match its manifest".to_string());
    }
    if receipt.entrypoint_relative_path != entrypoint_relative {
        return Err("managed operator package entrypoint path mismatch".to_string());
    }
    let metadata = plan
        .entrypoint_path
        .metadata()
        .map_err(|error| format!("failed to inspect managed entrypoint: {error}"))?;
    if metadata.len() != receipt.entrypoint_size_bytes
        || sha256_file(&plan.entrypoint_path)? != receipt.entrypoint_sha256
    {
        return Err("managed operator package entrypoint integrity mismatch".to_string());
    }
    if sha256_file(&plan.manifest_path)? != receipt.manifest_sha256 {
        return Err("managed operator package manifest integrity mismatch".to_string());
    }
    Ok(receipt)
}

fn stage_and_promote(
    source_plan: &OperatorPackageLoadPlan,
    entrypoint_relative: &str,
    staging_parent: &Path,
    staging_package: &Path,
    target: &Path,
    store_root: &Path,
) -> Result<ManagedOperatorPackageReceipt, String> {
    let staged_entrypoint = staging_package.join(entrypoint_relative);
    fs::create_dir_all(
        staged_entrypoint
            .parent()
            .ok_or_else(|| "operator entrypoint has no parent".to_string())?,
    )
    .map_err(|error| format!("failed to create operator package staging: {error}"))?;
    fs::copy(
        &source_plan.manifest_path,
        staging_package.join(OPERATOR_PACKAGE_MANIFEST_FILE),
    )
    .map_err(|error| format!("failed to stage operator package manifest: {error}"))?;
    fs::copy(&source_plan.entrypoint_path, &staged_entrypoint)
        .map_err(|error| format!("failed to stage operator package entrypoint: {error}"))?;

    let preflight = operator_package_preflight(staging_parent)?;
    preflight.ensure_no_rejections()?;
    preflight.ensure_no_readiness_warnings()?;
    if preflight.accepted_package_count != 1 {
        return Err("operator package staging must admit exactly one package".to_string());
    }
    let receipt = ManagedOperatorPackageReceipt {
        schema_version: MANAGED_OPERATOR_PACKAGE_RECEIPT_SCHEMA_VERSION.to_string(),
        package_id: source_plan.manifest.package_id.clone(),
        package_version: source_plan.manifest.package_version.clone(),
        sdk_api_version: source_plan.manifest.sdk_api_version.clone(),
        runtime: source_plan.manifest.runtime.clone(),
        operator_ids: source_plan
            .manifest
            .operators
            .iter()
            .map(|operator| operator.operator_id.clone())
            .collect(),
        relative_root: format!("packages/{}", source_plan.manifest.package_id),
        entrypoint_relative_path: entrypoint_relative.to_string(),
        entrypoint_sha256: sha256_file(&staged_entrypoint)?,
        entrypoint_size_bytes: staged_entrypoint
            .metadata()
            .map_err(|error| format!("failed to inspect staged entrypoint: {error}"))?
            .len(),
        manifest_sha256: sha256_file(&staging_package.join(OPERATOR_PACKAGE_MANIFEST_FILE))?,
    };
    write_json(&staging_package.join(RECEIPT_FILE), &receipt)?;
    verify_managed_operator_package(staging_package)?;
    fs::rename(staging_package, target).map_err(|error| {
        format!(
            "failed to atomically activate operator package {}: {error}",
            source_plan.manifest.package_id
        )
    })?;
    let activated = (|| {
        let installed = verify_managed_operator_package(target)?;
        let activated_relative = target
            .strip_prefix(store_root)
            .map_err(|_| "managed operator package activation escaped its store".to_string())?
            .to_string_lossy()
            .replace('\\', "/");
        if activated_relative != receipt.relative_root {
            return Err("managed operator package activation path mismatch".to_string());
        }
        Ok(installed)
    })();
    match activated {
        Ok(installed) => Ok(installed),
        Err(error) => match fs::remove_dir_all(target) {
            Ok(()) => Err(error),
            Err(cleanup_error) => Err(format!(
                "{error}; failed to roll back {}: {cleanup_error}",
                target.display()
            )),
        },
    }
}

fn remove_package_target(target: &Path) -> Result<(), String> {
    let metadata = target
        .symlink_metadata()
        .map_err(|error| format!("failed to inspect {}: {error}", target.display()))?;
    let result = if metadata.is_dir() {
        fs::remove_dir_all(target)
    } else if metadata.is_file() {
        fs::remove_file(target)
    } else {
        return Err(format!(
            "managed operator package target has unsupported type: {}",
            target.display()
        ));
    };
    result.map_err(|error| format!("failed to remove {}: {error}", target.display()))
}

fn load_plan(package_root: &Path) -> Result<OperatorPackageLoadPlan, String> {
    let manifest_path = package_root.join(OPERATOR_PACKAGE_MANIFEST_FILE);
    reject_symlink(&manifest_path, "operator package manifest")?;
    if !manifest_path.is_file() {
        return Err(format!(
            "operator package manifest is missing: {}",
            manifest_path.display()
        ));
    }
    let manifest =
        read_operator_package_manifest(&manifest_path).map_err(|error| error.to_string())?;
    Ok(build_operator_package_load_plan(
        DiscoveredOperatorPackage {
            package_root: package_root.to_path_buf(),
            manifest_path,
            manifest,
        },
    ))
}

fn verified_entrypoint_relative(
    package_root: &Path,
    plan: &OperatorPackageLoadPlan,
) -> Result<String, String> {
    reject_symlink(&plan.entrypoint_path, "operator package entrypoint")?;
    let metadata = plan.entrypoint_path.metadata().map_err(|error| {
        format!(
            "operator package entrypoint is missing {}: {error}",
            plan.entrypoint_path.display()
        )
    })?;
    if !metadata.is_file() || metadata.len() == 0 {
        return Err("operator package entrypoint must be a non-empty regular file".to_string());
    }
    let canonical_entrypoint = plan
        .entrypoint_path
        .canonicalize()
        .map_err(|error| format!("failed to resolve operator package entrypoint: {error}"))?;
    if !canonical_entrypoint.starts_with(package_root) {
        return Err("operator package entrypoint escapes its package root".to_string());
    }
    let relative = plan
        .entrypoint_path
        .strip_prefix(package_root)
        .map_err(|_| "operator package entrypoint is not package-relative".to_string())?;
    validate_relative_path(relative)?;
    Ok(relative.to_string_lossy().replace('\\', "/"))
}

fn validate_receipt(
    receipt: &ManagedOperatorPackageReceipt,
    package_root: &Path,
) -> Result<(), String> {
    if receipt.schema_version != MANAGED_OPERATOR_PACKAGE_RECEIPT_SCHEMA_VERSION {
        return Err("unsupported managed operator package receipt schema".to_string());
    }
    validate_component(&receipt.package_id, "package_id")?;
    validate_component(&receipt.package_version, "package_version")?;
    if receipt.relative_root != format!("packages/{}", receipt.package_id) {
        return Err("managed operator package receipt path is not canonical".to_string());
    }
    validate_digest(&receipt.entrypoint_sha256)?;
    validate_digest(&receipt.manifest_sha256)?;
    let directory_name = package_root
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| "managed operator package directory name is invalid".to_string())?;
    if directory_name != receipt.package_id {
        return Err("managed operator package directory identity mismatch".to_string());
    }
    Ok(())
}

fn verify_managed_package_layout(
    package_root: &Path,
    receipt: &ManagedOperatorPackageReceipt,
) -> Result<(), String> {
    let expected_files = BTreeSet::from([
        PathBuf::from(OPERATOR_PACKAGE_MANIFEST_FILE),
        PathBuf::from(RECEIPT_FILE),
        PathBuf::from(&receipt.entrypoint_relative_path),
    ]);
    if expected_files.len() != 3 {
        return Err("managed operator package paths collide".to_string());
    }
    let mut expected_directories = BTreeSet::new();
    let mut parent = Path::new(&receipt.entrypoint_relative_path).parent();
    while let Some(directory) = parent {
        if directory.as_os_str().is_empty() {
            break;
        }
        expected_directories.insert(directory.to_path_buf());
        parent = directory.parent();
    }

    let mut pending = vec![package_root.to_path_buf()];
    let mut seen_files = BTreeSet::new();
    let mut entry_count = 0_usize;
    while let Some(directory) = pending.pop() {
        for entry in fs::read_dir(&directory)
            .map_err(|error| format!("failed to read {}: {error}", directory.display()))?
        {
            let entry = entry.map_err(|error| error.to_string())?;
            entry_count += 1;
            if entry_count > 64 {
                return Err("managed operator package layout exceeds its entry limit".to_string());
            }
            let relative = entry
                .path()
                .strip_prefix(package_root)
                .map_err(|_| "managed operator package entry escaped its root".to_string())?
                .to_path_buf();
            let file_type = entry.file_type().map_err(|error| {
                format!("failed to inspect {}: {error}", entry.path().display())
            })?;
            if file_type.is_symlink() {
                return Err(format!(
                    "managed operator package contains a symlink: {}",
                    relative.display()
                ));
            }
            if file_type.is_dir() {
                if !expected_directories.contains(&relative) {
                    return Err(format!(
                        "managed operator package contains an unexpected directory: {}",
                        relative.display()
                    ));
                }
                pending.push(entry.path());
            } else if file_type.is_file() {
                if !expected_files.contains(&relative) {
                    return Err(format!(
                        "managed operator package contains an unexpected file: {}",
                        relative.display()
                    ));
                }
                seen_files.insert(relative);
            } else {
                return Err(format!(
                    "managed operator package contains an unsupported entry: {}",
                    relative.display()
                ));
            }
        }
    }
    if seen_files != expected_files {
        return Err("managed operator package layout is incomplete".to_string());
    }
    Ok(())
}

fn read_receipt(package_root: &Path) -> Result<ManagedOperatorPackageReceipt, String> {
    let path = package_root.join(RECEIPT_FILE);
    reject_symlink(&path, "managed operator package receipt")?;
    let payload =
        fs::read(&path).map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    serde_json::from_slice(&payload)
        .map_err(|error| format!("failed to parse {}: {error}", path.display()))
}

fn ensure_store(store_root: &Path) -> Result<(), String> {
    reject_symlink(store_root, "operator package store")?;
    fs::create_dir_all(store_root)
        .map_err(|error| format!("failed to create {}: {error}", store_root.display()))?;
    for child in ["packages", "staging"] {
        let path = store_root.join(child);
        reject_symlink(&path, "operator package managed directory")?;
        fs::create_dir_all(&path)
            .map_err(|error| format!("failed to create {}: {error}", path.display()))?;
    }
    Ok(())
}

fn canonical_regular_directory(path: &Path, label: &str) -> Result<PathBuf, String> {
    reject_symlink(path, label)?;
    let canonical = path
        .canonicalize()
        .map_err(|error| format!("failed to resolve {label} {}: {error}", path.display()))?;
    if !canonical.is_dir() {
        return Err(format!("{label} must be a directory"));
    }
    Ok(canonical)
}

fn unique_staging_root(store_root: &Path, package_id: &str) -> Result<PathBuf, String> {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| format!("system clock error: {error}"))?
        .as_nanos();
    let path =
        store_root
            .join("staging")
            .join(format!("{}-{}-{nonce}", package_id, std::process::id()));
    fs::create_dir(&path)
        .map_err(|error| format!("failed to create {}: {error}", path.display()))?;
    Ok(path)
}

fn validate_component(value: &str, label: &str) -> Result<(), String> {
    let bytes = value.as_bytes();
    if bytes.is_empty()
        || value.len() > 128
        || !bytes.first().is_some_and(u8::is_ascii_alphanumeric)
        || !bytes.last().is_some_and(u8::is_ascii_alphanumeric)
        || !bytes
            .iter()
            .copied()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'_'))
        || is_windows_reserved_component(value)
    {
        return Err(format!("{label} is not a safe portable path component"));
    }
    Ok(())
}

fn is_windows_reserved_component(value: &str) -> bool {
    let stem = value.split('.').next().unwrap_or_default();
    ["CON", "PRN", "AUX", "NUL"]
        .iter()
        .any(|reserved| stem.eq_ignore_ascii_case(reserved))
        || (stem.len() == 4
            && (stem[..3].eq_ignore_ascii_case("COM") || stem[..3].eq_ignore_ascii_case("LPT"))
            && matches!(stem.as_bytes()[3], b'1'..=b'9'))
}

fn validate_relative_path(path: &Path) -> Result<(), String> {
    if path.as_os_str().is_empty()
        || path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err("operator package entrypoint path must be strictly relative".to_string());
    }
    Ok(())
}

fn validate_digest(digest: &str) -> Result<(), String> {
    if digest.len() != 64
        || !digest
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    {
        return Err("managed operator package digest is malformed".to_string());
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
    let payload = serde_json::to_vec_pretty(value).map_err(|error| error.to_string())?;
    fs::write(path, payload).map_err(|error| format!("failed to write {}: {error}", path.display()))
}

fn prune_empty_store(store_root: &Path) -> Result<bool, String> {
    for child in ["staging", "packages"] {
        let path = store_root.join(child);
        if path.exists()
            && fs::read_dir(&path)
                .map_err(|error| format!("failed to read {}: {error}", path.display()))?
                .next()
                .is_none()
        {
            fs::remove_dir(&path)
                .map_err(|error| format!("failed to remove {}: {error}", path.display()))?;
        }
    }
    if store_root.exists()
        && fs::read_dir(store_root)
            .map_err(|error| format!("failed to read {}: {error}", store_root.display()))?
            .next()
            .is_none()
    {
        fs::remove_dir(store_root)
            .map_err(|error| format!("failed to remove {}: {error}", store_root.display()))?;
        return Ok(true);
    }
    Ok(false)
}

struct OperatorPackageStoreLock {
    path: PathBuf,
}

impl OperatorPackageStoreLock {
    fn acquire(store_root: &Path) -> Result<Self, String> {
        let path = store_root.join(".operator-package.lock");
        let mut file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&path)
            .map_err(|error| format!("operator package store lock is unavailable: {error}"))?;
        writeln!(file, "pid={}", std::process::id())
            .map_err(|error| format!("failed to write operator package store lock: {error}"))?;
        Ok(Self { path })
    }
}

impl Drop for OperatorPackageStoreLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}
