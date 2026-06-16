use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{Value, json};

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

pub(crate) fn current_update_catalog_path(root: &Path) -> PathBuf {
    let config = read_update_source_config().unwrap_or_else(|_| default_update_source_config());
    resolve_source_path(root, &config.catalog_path)
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
    Ok(UpdateSourceConfig {
        schema_version: value_string(value.get("schema_version"), UPDATE_SOURCE_SCHEMA_VERSION),
        catalog_path: value_string(value.get("catalog_path"), "releases/update-catalog.json"),
        artifact_root: value_string(value.get("artifact_root"), "."),
        download_dir: value_string(value.get("download_dir"), "dist/downloads"),
    })
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
    let latest_path =
        resolve_source_path(&root, &config.download_dir).join("latest-downloaded-update.json");
    let source_root = resolve_source_path(&root, &config.artifact_root);
    let target_root = resolve_source_path(&root, &config.download_dir)
        .join(format!("{}-{}", plan.target_channel, plan.target_version));
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

    fs::create_dir_all(&target_root)
        .map_err(|error| format!("failed to create {}: {error}", target_root.display()))?;
    let manifests_dir = target_root.join("manifests");
    fs::create_dir_all(&manifests_dir)
        .map_err(|error| format!("failed to create {}: {error}", manifests_dir.display()))?;

    let mut downloaded_paths = Vec::new();
    for artifact in artifacts {
        let source = resolve_artifact_path(&source_root, &artifact.path);
        if !source.exists() {
            return Err(format!(
                "configured update source is missing {}",
                source.display()
            ));
        }
        let file_name = source
            .file_name()
            .and_then(|value| value.to_str())
            .ok_or_else(|| format!("invalid artifact path {}", source.display()))?;
        let target = target_root
            .join(&artifact.product)
            .join(&artifact.kind)
            .join(file_name);
        copy_path(&source, &target)?;
        downloaded_paths.push(target.display().to_string());
    }

    let manifest_path = manifests_dir.join("downloaded-update.json");
    fs::write(
        &manifest_path,
        serde_json::to_string_pretty(&json!({
            "schema_version": "kyuubiki.downloaded-update/v1",
            "generated_at": unix_timestamp(),
            "channel": plan.target_channel,
            "target_version": plan.target_version,
            "platform": platform_key,
            "download_dir": target_root.display().to_string(),
            "source": {
                "catalog_path": config.catalog_path,
                "artifact_root": config.artifact_root,
            },
            "downloaded_paths": downloaded_paths,
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
            "download_dir": target_root.display().to_string(),
            "manifest_path": manifest_path.display().to_string(),
            "downloaded_paths": downloaded_paths,
        }))
        .map_err(|error| error.to_string())?,
    )
    .map_err(|error| format!("failed to write {}: {error}", latest_path.display()))?;

    Ok(DownloadedUpdateRecord {
        channel: plan.target_channel,
        target_version: plan.target_version,
        download_dir: target_root.display().to_string(),
        manifest_path: manifest_path.display().to_string(),
        downloaded_paths,
    })
}

pub fn latest_downloaded_update_record() -> Result<Option<DownloadedUpdateRecord>, String> {
    let config = read_update_source_config().unwrap_or_else(|_| default_update_source_config());
    let path = resolve_source_path(&workspace_root(), &config.download_dir)
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
    let config = read_update_source_config().unwrap_or_else(|_| default_update_source_config());
    let root = workspace_root();
    let apply_root = resolve_source_path(&root, &config.download_dir)
        .join("applied")
        .join(format!(
            "{}-{}",
            downloaded.channel, downloaded.target_version
        ));
    let manifests_dir = apply_root.join("manifests");
    fs::create_dir_all(&manifests_dir)
        .map_err(|error| format!("failed to create {}: {error}", manifests_dir.display()))?;
    let manifest_path = manifests_dir.join("applied-update.json");
    let latest_path =
        resolve_source_path(&root, &config.download_dir).join("latest-applied-update.json");
    let applied_manifest = json!({
        "schema_version": "kyuubiki.applied-update/v1",
        "generated_at": unix_timestamp(),
        "channel": downloaded.channel,
        "target_version": downloaded.target_version,
        "apply_dir": apply_root.display().to_string(),
        "source_download_dir": downloaded.download_dir,
        "source_download_manifest_path": downloaded.manifest_path,
        "downloaded_paths": downloaded.downloaded_paths,
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
            "apply_dir": apply_root.display().to_string(),
            "manifest_path": manifest_path.display().to_string(),
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
        apply_dir: apply_root.display().to_string(),
        manifest_path: manifest_path.display().to_string(),
        source_download_manifest_path: applied_manifest["source_download_manifest_path"]
            .as_str()
            .unwrap_or("")
            .to_string(),
    })
}

pub fn latest_applied_update_record() -> Result<Option<AppliedUpdateRecord>, String> {
    let config = read_update_source_config().unwrap_or_else(|_| default_update_source_config());
    let path = resolve_source_path(&workspace_root(), &config.download_dir)
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

fn resolve_source_path(root: &Path, value: &str) -> PathBuf {
    let path = PathBuf::from(value);
    if path.is_absolute() {
        path
    } else {
        root.join(value)
    }
}

fn resolve_artifact_path(source_root: &Path, value: &str) -> PathBuf {
    let path = PathBuf::from(value);
    if path.is_absolute() {
        path
    } else {
        source_root.join(value)
    }
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

fn copy_path(source: &Path, target: &Path) -> Result<(), String> {
    if source.is_dir() {
        fs::create_dir_all(target)
            .map_err(|error| format!("failed to create {}: {error}", target.display()))?;
        for entry in fs::read_dir(source)
            .map_err(|error| format!("failed to read {}: {error}", source.display()))?
        {
            let entry = entry.map_err(|error| error.to_string())?;
            copy_path(&entry.path(), &target.join(entry.file_name()))?;
        }
        return Ok(());
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
