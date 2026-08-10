use std::collections::BTreeSet;
use std::fs;
use std::io::Read;
use std::path::{Component, Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use crate::{Platform, UpdateArtifactRef, unified_update_plan, workspace_root};

const UPDATE_SOURCE_SCHEMA_VERSION: &str = "kyuubiki.update-source/v1";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UpdateSourceConfig {
    pub schema_version: String,
    pub catalog_path: String,
    pub artifact_root: String,
    pub download_dir: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DownloadedUpdateRecord {
    pub channel: String,
    pub target_version: String,
    pub download_dir: String,
    pub manifest_path: String,
    pub downloaded_paths: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AppliedUpdateRecord {
    pub channel: String,
    pub target_version: String,
    pub apply_dir: String,
    pub manifest_path: String,
    pub source_download_manifest_path: String,
}

impl UpdateSourceConfig {
    pub fn render(&self) -> String {
        [
            "kyuubiki update source".to_string(),
            format!("catalog_path: {}", self.catalog_path),
            format!("artifact_root: {}", self.artifact_root),
            format!("download_dir: {}", self.download_dir),
        ]
        .join("\n")
    }
}

impl DownloadedUpdateRecord {
    pub fn render(&self) -> String {
        let mut lines = vec![
            "kyuubiki downloaded update".to_string(),
            format!("channel: {}", self.channel),
            format!("target_version: {}", self.target_version),
            format!("download_dir: {}", self.download_dir),
            format!("manifest_path: {}", self.manifest_path),
        ];
        for path in &self.downloaded_paths {
            lines.push(format!("[downloaded] {path}"));
        }
        lines.join("\n")
    }
}

impl AppliedUpdateRecord {
    pub fn render(&self) -> String {
        [
            "kyuubiki applied update".to_string(),
            format!("channel: {}", self.channel),
            format!("target_version: {}", self.target_version),
            format!("apply_dir: {}", self.apply_dir),
            format!("manifest_path: {}", self.manifest_path),
            format!(
                "source_download_manifest_path: {}",
                self.source_download_manifest_path
            ),
        ]
        .join("\n")
    }
}

pub(crate) fn current_update_catalog_path(root: &Path) -> Result<PathBuf, String> {
    let config = read_update_source_config()?;
    managed_path(root, &config.catalog_path, "catalog_path")
}

pub fn read_update_source_config() -> Result<UpdateSourceConfig, String> {
    let path = update_source_config_path();
    if !path.exists() {
        return Ok(default_update_source_config());
    }
    let contents = fs::read_to_string(&path)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    let value: Value = serde_json::from_str(&contents)
        .map_err(|error| format!("failed to parse {}: {error}", path.display()))?;
    let config = UpdateSourceConfig {
        schema_version: value_string(value.get("schema_version"), UPDATE_SOURCE_SCHEMA_VERSION),
        catalog_path: value_string(value.get("catalog_path"), "releases/update-catalog.json"),
        artifact_root: value_string(value.get("artifact_root"), "."),
        download_dir: value_string(value.get("download_dir"), "dist/downloads"),
    };
    validate_update_source_config(&config)?;
    Ok(config)
}

pub fn write_update_source_config(
    catalog_path: String,
    artifact_root: String,
    download_dir: String,
) -> Result<String, String> {
    let config = UpdateSourceConfig {
        schema_version: UPDATE_SOURCE_SCHEMA_VERSION.to_string(),
        catalog_path: nonempty_or_default(catalog_path, "releases/update-catalog.json"),
        artifact_root: nonempty_or_default(artifact_root, "."),
        download_dir: nonempty_or_default(download_dir, "dist/downloads"),
    };
    validate_update_source_config(&config)?;
    let path = update_source_config_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
    }
    fs::write(
        &path,
        serde_json::to_string_pretty(&json!({
            "schema_version": config.schema_version,
            "catalog_path": config.catalog_path,
            "artifact_root": config.artifact_root,
            "download_dir": config.download_dir,
        }))
        .map_err(|error| error.to_string())?,
    )
    .map_err(|error| format!("failed to write {}: {error}", path.display()))?;
    Ok(config.render())
}

pub fn download_update(
    channel: Option<String>,
    platform: Platform,
) -> Result<DownloadedUpdateRecord, String> {
    let root = workspace_root();
    let config = read_update_source_config()?;
    let plan = unified_update_plan(channel)?;
    download_update_at(&root, &config, &plan, platform)
}

pub(crate) fn download_update_at(
    root: &Path,
    config: &UpdateSourceConfig,
    plan: &crate::UnifiedUpdatePlan,
    platform: Platform,
) -> Result<DownloadedUpdateRecord, String> {
    validate_update_source_config(config)?;
    let download_root = managed_path(root, &config.download_dir, "download_dir")?;
    let latest_path = download_root.join("latest-downloaded-update.json");
    let source_root = source_root_path(root, &config.artifact_root)?;
    let channel = safe_component(&plan.target_channel, "target channel")?;
    let version = safe_component(&plan.target_version, "target version")?;
    let target_root = download_root.join(format!("{channel}-{version}"));
    ensure_managed_target(root, &target_root)?;
    let platform_key = platform.as_str();
    let artifacts: Vec<&UpdateArtifactRef> = plan
        .artifacts
        .iter()
        .filter(|artifact| artifact.platform == platform_key)
        .collect();
    if artifacts.is_empty() {
        return Err(format!(
            "no desktop artifacts declared for platform {} on channel {}",
            platform_key, plan.target_channel
        ));
    }

    let mut prepared = Vec::new();
    for artifact in artifacts {
        let source = source_artifact_path(&source_root, &artifact.path)?;
        let product = safe_component(&artifact.product, "artifact product")?;
        let kind = safe_component(&artifact.kind, "artifact kind")?;
        let file_name = source
            .file_name()
            .and_then(|value| value.to_str())
            .ok_or_else(|| format!("invalid artifact path {}", source.display()))?;
        let target = target_root.join(product).join(kind).join(file_name);
        ensure_managed_target(root, &target)?;
        let source_sha256 = digest_path(&source)?;
        prepared.push((artifact, source, target, source_sha256));
    }

    if target_root.exists() {
        fs::remove_dir_all(&target_root)
            .map_err(|error| format!("failed to reset {}: {error}", target_root.display()))?;
    }
    fs::create_dir_all(&target_root)
        .map_err(|error| format!("failed to create {}: {error}", target_root.display()))?;
    let manifests_dir = target_root.join("manifests");
    fs::create_dir_all(&manifests_dir)
        .map_err(|error| format!("failed to create {}: {error}", manifests_dir.display()))?;

    let mut downloaded_paths = Vec::new();
    let mut downloaded_artifacts = Vec::new();
    for (artifact, source, target, source_sha256) in prepared {
        copy_path(&source, &target)?;
        let downloaded_sha256 = digest_path(&target)?;
        if source_sha256 != downloaded_sha256 {
            return Err(format!(
                "downloaded artifact digest mismatch for {}",
                artifact.path
            ));
        }
        let downloaded_path = portable_path(root, &target)?;
        downloaded_paths.push(downloaded_path.clone());
        downloaded_artifacts.push(json!({
            "product": artifact.product,
            "kind": artifact.kind,
            "source_path": artifact.path,
            "downloaded_path": downloaded_path,
            "sha256": downloaded_sha256,
        }));
    }

    let manifest_path = manifests_dir.join("downloaded-update.json");
    let target_root_relative = portable_path(root, &target_root)?;
    let manifest_path_relative = portable_path(root, &manifest_path)?;
    fs::write(
        &manifest_path,
        serde_json::to_string_pretty(&json!({
            "schema_version": "kyuubiki.downloaded-update/v1",
            "generated_at": unix_timestamp(),
            "channel": plan.target_channel,
            "target_version": plan.target_version,
            "platform": platform_key,
            "download_dir": target_root_relative,
            "source": {
                "catalog_path": config.catalog_path,
                "artifact_root": config.artifact_root,
            },
            "downloaded_paths": downloaded_paths,
            "artifacts": downloaded_artifacts,
        }))
        .map_err(|error| error.to_string())?,
    )
    .map_err(|error| format!("failed to write {}: {error}", manifest_path.display()))?;
    fs::write(
        &latest_path,
        serde_json::to_string_pretty(&json!({
            "schema_version": "kyuubiki.downloaded-update-pointer/v1",
            "channel": plan.target_channel,
            "target_version": plan.target_version,
            "download_dir": target_root_relative,
            "manifest_path": manifest_path_relative,
            "downloaded_paths": downloaded_paths,
        }))
        .map_err(|error| error.to_string())?,
    )
    .map_err(|error| format!("failed to write {}: {error}", latest_path.display()))?;

    Ok(DownloadedUpdateRecord {
        channel: plan.target_channel.clone(),
        target_version: plan.target_version.clone(),
        download_dir: target_root_relative,
        manifest_path: manifest_path_relative,
        downloaded_paths,
    })
}

pub fn latest_downloaded_update_record() -> Result<Option<DownloadedUpdateRecord>, String> {
    let root = workspace_root();
    let config = read_update_source_config()?;
    let path = managed_path(&root, &config.download_dir, "download_dir")?
        .join("latest-downloaded-update.json");
    if !path.exists() {
        return Ok(None);
    }
    let contents = fs::read_to_string(&path)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    let value: Value = serde_json::from_str(&contents)
        .map_err(|error| format!("failed to parse {}: {error}", path.display()))?;
    Ok(Some(DownloadedUpdateRecord {
        channel: value_string(value.get("channel"), "unknown"),
        target_version: value_string(value.get("target_version"), "unknown"),
        download_dir: value_string(value.get("download_dir"), ""),
        manifest_path: value_string(value.get("manifest_path"), ""),
        downloaded_paths: value
            .get("downloaded_paths")
            .and_then(Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .filter_map(Value::as_str)
                    .map(ToString::to_string)
                    .collect()
            })
            .unwrap_or_default(),
    }))
}

pub fn apply_downloaded_update() -> Result<AppliedUpdateRecord, String> {
    let downloaded = latest_downloaded_update_record()?
        .ok_or_else(|| "no downloaded update record is available".to_string())?;
    let config = read_update_source_config()?;
    let root = workspace_root();
    apply_downloaded_update_at(&root, &config, &downloaded)
}

pub(crate) fn apply_downloaded_update_at(
    root: &Path,
    config: &UpdateSourceConfig,
    downloaded: &DownloadedUpdateRecord,
) -> Result<AppliedUpdateRecord, String> {
    validate_update_source_config(config)?;
    let download_root = managed_path(root, &config.download_dir, "download_dir")?;
    let verified = verify_downloaded_artifacts(root, &download_root, downloaded)?;
    let channel = safe_component(&downloaded.channel, "downloaded channel")?;
    let version = safe_component(&downloaded.target_version, "downloaded target version")?;
    let apply_root = download_root
        .join("applied")
        .join(format!("{channel}-{version}"));
    ensure_managed_target(root, &apply_root)?;
    if apply_root.exists() {
        fs::remove_dir_all(&apply_root)
            .map_err(|error| format!("failed to reset {}: {error}", apply_root.display()))?;
    }
    let manifests_dir = apply_root.join("manifests");
    fs::create_dir_all(&manifests_dir)
        .map_err(|error| format!("failed to create {}: {error}", manifests_dir.display()))?;
    let mut applied_artifacts = Vec::new();
    for artifact in verified {
        let target = apply_root.join(&artifact.relative_path);
        ensure_managed_target(root, &target)?;
        copy_path(&artifact.source_path, &target)?;
        let applied_sha256 = digest_path(&target)?;
        if applied_sha256 != artifact.sha256 {
            return Err(format!(
                "applied artifact digest mismatch for {}",
                artifact.relative_path.display()
            ));
        }
        applied_artifacts.push(json!({
            "source_path": portable_path(root, &artifact.source_path)?,
            "applied_path": portable_path(root, &target)?,
            "sha256": applied_sha256,
        }));
    }
    let manifest_path = manifests_dir.join("applied-update.json");
    let latest_path = download_root.join("latest-applied-update.json");
    let apply_dir_relative = portable_path(root, &apply_root)?;
    let manifest_path_relative = portable_path(root, &manifest_path)?;
    let applied_manifest = json!({
        "schema_version": "kyuubiki.applied-update/v1",
        "generated_at": unix_timestamp(),
        "channel": downloaded.channel,
        "target_version": downloaded.target_version,
        "apply_dir": apply_dir_relative,
        "source_download_dir": downloaded.download_dir,
        "source_download_manifest_path": downloaded.manifest_path,
        "downloaded_paths": downloaded.downloaded_paths,
        "artifacts": applied_artifacts,
    });
    fs::write(
        &manifest_path,
        serde_json::to_string_pretty(&applied_manifest).map_err(|error| error.to_string())?,
    )
    .map_err(|error| format!("failed to write {}: {error}", manifest_path.display()))?;
    fs::write(
        &latest_path,
        serde_json::to_string_pretty(&json!({
            "schema_version": "kyuubiki.applied-update-pointer/v1",
            "channel": applied_manifest["channel"],
            "target_version": applied_manifest["target_version"],
            "apply_dir": apply_dir_relative,
            "manifest_path": manifest_path_relative,
            "source_download_manifest_path": applied_manifest["source_download_manifest_path"],
        }))
        .map_err(|error| error.to_string())?,
    )
    .map_err(|error| format!("failed to write {}: {error}", latest_path.display()))?;
    Ok(AppliedUpdateRecord {
        channel: applied_manifest["channel"]
            .as_str()
            .unwrap_or("unknown")
            .to_string(),
        target_version: applied_manifest["target_version"]
            .as_str()
            .unwrap_or("unknown")
            .to_string(),
        apply_dir: apply_dir_relative,
        manifest_path: manifest_path_relative,
        source_download_manifest_path: applied_manifest["source_download_manifest_path"]
            .as_str()
            .unwrap_or("")
            .to_string(),
    })
}

pub fn latest_applied_update_record() -> Result<Option<AppliedUpdateRecord>, String> {
    let root = workspace_root();
    let config = read_update_source_config()?;
    let path = managed_path(&root, &config.download_dir, "download_dir")?
        .join("latest-applied-update.json");
    if !path.exists() {
        return Ok(None);
    }
    let contents = fs::read_to_string(&path)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    let value: Value = serde_json::from_str(&contents)
        .map_err(|error| format!("failed to parse {}: {error}", path.display()))?;
    Ok(Some(AppliedUpdateRecord {
        channel: value_string(value.get("channel"), "unknown"),
        target_version: value_string(value.get("target_version"), "unknown"),
        apply_dir: value_string(value.get("apply_dir"), ""),
        manifest_path: value_string(value.get("manifest_path"), ""),
        source_download_manifest_path: value_string(value.get("source_download_manifest_path"), ""),
    }))
}

fn update_source_config_path() -> PathBuf {
    workspace_root().join("deploy").join("update-source.json")
}

fn default_update_source_config() -> UpdateSourceConfig {
    UpdateSourceConfig {
        schema_version: UPDATE_SOURCE_SCHEMA_VERSION.to_string(),
        catalog_path: "releases/update-catalog.json".to_string(),
        artifact_root: ".".to_string(),
        download_dir: "dist/downloads".to_string(),
    }
}

pub(crate) fn validate_update_source_config(config: &UpdateSourceConfig) -> Result<(), String> {
    if config.schema_version != UPDATE_SOURCE_SCHEMA_VERSION {
        return Err(format!(
            "unsupported update source schema: {}",
            config.schema_version
        ));
    }
    relative_path(&config.catalog_path, "catalog_path", false)?;
    let download_dir = relative_path(&config.download_dir, "download_dir", false)?;
    if download_dir == Path::new(".") {
        return Err("download_dir cannot be the workspace root".to_string());
    }
    source_root_value(&config.artifact_root)?;
    Ok(())
}

fn managed_path(root: &Path, value: &str, label: &str) -> Result<PathBuf, String> {
    let relative = relative_path(value, label, true)?;
    let target = root.join(&relative);
    ensure_managed_target(root, &target)?;
    Ok(target)
}

fn ensure_managed_target(root: &Path, target: &Path) -> Result<(), String> {
    let relative = target.strip_prefix(root).map_err(|_| {
        format!(
            "managed update path escapes workspace: {}",
            target.display()
        )
    })?;
    let mut cursor = root.to_path_buf();
    for component in relative.components() {
        cursor.push(component);
        if let Ok(metadata) = fs::symlink_metadata(&cursor)
            && metadata.file_type().is_symlink()
        {
            return Err(format!(
                "managed update path crosses symlink: {}",
                cursor.display()
            ));
        }
    }
    Ok(())
}

fn source_root_value(value: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(value.trim());
    if value.trim().is_empty()
        || path
            .components()
            .any(|component| matches!(component, Component::ParentDir))
    {
        return Err(format!("artifact_root is not controlled: {value}"));
    }
    Ok(path)
}

fn source_root_path(root: &Path, value: &str) -> Result<PathBuf, String> {
    let configured = source_root_value(value)?;
    let explicit_absolute = configured.is_absolute();
    let joined = if explicit_absolute {
        configured
    } else {
        root.join(configured)
    };
    let resolved = joined
        .canonicalize()
        .map_err(|error| format!("configured artifact_root is unavailable: {error}"))?;
    if !explicit_absolute {
        let resolved_root = root
            .canonicalize()
            .map_err(|error| format!("failed to resolve workspace root: {error}"))?;
        if !resolved.starts_with(&resolved_root) {
            return Err("relative artifact_root escapes the workspace".to_string());
        }
    }
    Ok(resolved)
}

fn source_artifact_path(source_root: &Path, value: &str) -> Result<PathBuf, String> {
    let relative = relative_path(value, "catalog artifact path", false)?;
    let source = source_root.join(relative);
    let resolved = source.canonicalize().map_err(|error| {
        format!(
            "configured update source is missing {}: {error}",
            source.display()
        )
    })?;
    if !resolved.starts_with(source_root) {
        return Err(format!("catalog artifact escapes source root: {value}"));
    }
    Ok(resolved)
}

fn relative_path(value: &str, label: &str, allow_dot: bool) -> Result<PathBuf, String> {
    let trimmed = value.trim();
    let path = PathBuf::from(trimmed);
    if trimmed.is_empty() || path.is_absolute() {
        return Err(format!("{label} must be a non-empty relative path"));
    }
    for component in path.components() {
        match component {
            Component::Normal(_) => {}
            Component::CurDir if allow_dot => {}
            _ => return Err(format!("{label} is not controlled: {value}")),
        }
    }
    Ok(path)
}

fn safe_component(value: &str, label: &str) -> Result<String, String> {
    let path = relative_path(value, label, false)?;
    let mut components = path.components();
    let component = components
        .next()
        .and_then(|entry| match entry {
            Component::Normal(value) => value.to_str(),
            _ => None,
        })
        .filter(|_| components.next().is_none())
        .ok_or_else(|| format!("{label} must be one portable path component"))?;
    Ok(component.to_string())
}

fn portable_path(root: &Path, path: &Path) -> Result<String, String> {
    path.strip_prefix(root)
        .map(|relative| {
            relative
                .to_string_lossy()
                .replace(std::path::MAIN_SEPARATOR, "/")
        })
        .map_err(|_| format!("update path is not workspace-relative: {}", path.display()))
}

struct VerifiedDownloadedArtifact {
    source_path: PathBuf,
    relative_path: PathBuf,
    sha256: String,
}

fn verify_downloaded_artifacts(
    root: &Path,
    configured_download_root: &Path,
    downloaded: &DownloadedUpdateRecord,
) -> Result<Vec<VerifiedDownloadedArtifact>, String> {
    let download_root = managed_path(root, &downloaded.download_dir, "record download_dir")?;
    let expected_download_root = configured_download_root.join(format!(
        "{}-{}",
        safe_component(&downloaded.channel, "downloaded channel")?,
        safe_component(&downloaded.target_version, "downloaded target version")?
    ));
    if download_root != expected_download_root {
        return Err("download record points outside its configured version directory".to_string());
    }
    let manifest_path = managed_path(root, &downloaded.manifest_path, "download manifest")?;
    if !manifest_path.starts_with(&download_root.join("manifests")) {
        return Err("download manifest is outside the version manifest directory".to_string());
    }
    let manifest: Value = serde_json::from_str(
        &fs::read_to_string(&manifest_path)
            .map_err(|error| format!("failed to read {}: {error}", manifest_path.display()))?,
    )
    .map_err(|error| format!("invalid download manifest: {error}"))?;
    if value_string(manifest.get("schema_version"), "") != "kyuubiki.downloaded-update/v1"
        || value_string(manifest.get("channel"), "") != downloaded.channel
        || value_string(manifest.get("target_version"), "") != downloaded.target_version
    {
        return Err("download pointer and manifest identity do not match".to_string());
    }
    let expected_paths = downloaded
        .downloaded_paths
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();
    let manifest_paths = manifest
        .get("downloaded_paths")
        .and_then(Value::as_array)
        .ok_or_else(|| "download manifest is missing downloaded_paths".to_string())?
        .iter()
        .filter_map(Value::as_str)
        .map(str::to_string)
        .collect::<BTreeSet<_>>();
    if expected_paths.is_empty() || expected_paths != manifest_paths {
        return Err("download pointer path set does not match its manifest".to_string());
    }
    let artifacts = manifest
        .get("artifacts")
        .and_then(Value::as_array)
        .ok_or_else(|| "download manifest is missing verified artifacts".to_string())?;
    if artifacts.len() != expected_paths.len() {
        return Err("download manifest artifact count does not match its pointer".to_string());
    }

    let mut verified = Vec::new();
    let mut seen = BTreeSet::new();
    for artifact in artifacts {
        let path_value = value_string(artifact.get("downloaded_path"), "");
        let expected_sha256 = value_string(artifact.get("sha256"), "");
        if !expected_paths.contains(&path_value)
            || !seen.insert(path_value.clone())
            || !valid_sha256(&expected_sha256)
        {
            return Err("download manifest contains an invalid artifact record".to_string());
        }
        let source_path = managed_path(root, &path_value, "downloaded artifact")?;
        if !source_path.starts_with(&download_root) {
            return Err("downloaded artifact escapes its version directory".to_string());
        }
        let actual_sha256 = digest_path(&source_path)?;
        if actual_sha256 != expected_sha256 {
            return Err(format!("downloaded artifact digest mismatch: {path_value}"));
        }
        let relative_path = source_path
            .strip_prefix(&download_root)
            .map_err(|_| "downloaded artifact is outside its version directory".to_string())?
            .to_path_buf();
        verified.push(VerifiedDownloadedArtifact {
            source_path,
            relative_path,
            sha256: expected_sha256,
        });
    }
    Ok(verified)
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

fn nonempty_or_default(value: String, fallback: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        fallback.to_string()
    } else {
        trimmed.to_string()
    }
}

fn value_string(value: Option<&Value>, fallback: &str) -> String {
    value
        .and_then(Value::as_str)
        .unwrap_or(fallback)
        .to_string()
}

fn unix_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

fn digest_path(path: &Path) -> Result<String, String> {
    let mut hasher = Sha256::new();
    digest_entry(path, Path::new("."), &mut hasher)?;
    Ok(format!("{:x}", hasher.finalize()))
}

fn digest_entry(path: &Path, relative: &Path, hasher: &mut Sha256) -> Result<(), String> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|error| format!("failed to inspect {}: {error}", path.display()))?;
    if metadata.file_type().is_symlink() {
        return Err(format!(
            "update artifacts cannot contain symlinks: {}",
            path.display()
        ));
    }
    let relative = relative
        .to_string_lossy()
        .replace(std::path::MAIN_SEPARATOR, "/");
    hasher.update((relative.len() as u64).to_le_bytes());
    hasher.update(relative.as_bytes());
    if metadata.is_dir() {
        hasher.update(b"directory");
        let mut entries = fs::read_dir(path)
            .map_err(|error| format!("failed to read {}: {error}", path.display()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
        entries.sort_by_key(|entry| entry.file_name());
        for entry in entries {
            digest_entry(
                &entry.path(),
                &Path::new(&relative).join(entry.file_name()),
                hasher,
            )?;
        }
        return Ok(());
    }
    if !metadata.is_file() {
        return Err(format!(
            "unsupported update artifact type: {}",
            path.display()
        ));
    }
    hasher.update(b"file");
    hasher.update(metadata.len().to_le_bytes());
    let mut file = fs::File::open(path)
        .map_err(|error| format!("failed to open {}: {error}", path.display()))?;
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let count = file
            .read(&mut buffer)
            .map_err(|error| format!("failed to hash {}: {error}", path.display()))?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }
    Ok(())
}

fn copy_path(source: &Path, target: &Path) -> Result<(), String> {
    let metadata = fs::symlink_metadata(source)
        .map_err(|error| format!("failed to inspect {}: {error}", source.display()))?;
    if metadata.file_type().is_symlink() {
        return Err(format!(
            "update artifacts cannot contain symlinks: {}",
            source.display()
        ));
    }
    if let Ok(target_metadata) = fs::symlink_metadata(target)
        && target_metadata.file_type().is_symlink()
    {
        return Err(format!(
            "update target cannot be a symlink: {}",
            target.display()
        ));
    }
    if metadata.is_dir() {
        fs::create_dir_all(target)
            .map_err(|error| format!("failed to create {}: {error}", target.display()))?;
        let mut entries = fs::read_dir(source)
            .map_err(|error| format!("failed to read {}: {error}", source.display()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| error.to_string())?;
        entries.sort_by_key(|entry| entry.file_name());
        for entry in entries {
            copy_path(&entry.path(), &target.join(entry.file_name()))?;
        }
        return Ok(());
    }
    if !metadata.is_file() {
        return Err(format!(
            "unsupported update artifact type: {}",
            source.display()
        ));
    }
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
    }
    fs::copy(source, target).map_err(|error| {
        format!(
            "failed to copy {} to {}: {error}",
            source.display(),
            target.display()
        )
    })?;
    Ok(())
}
