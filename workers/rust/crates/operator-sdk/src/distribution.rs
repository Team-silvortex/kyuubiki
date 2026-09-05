use crate::{
    OPERATOR_JSON_ABI_SCHEMA_VERSION, OPERATOR_PACKAGE_MANIFEST_FILE, OPERATOR_SDK_API_VERSION,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fmt::{Display, Formatter};
use std::fs;
use std::path::{Component, Path, PathBuf};

pub const OPERATOR_PACKAGE_DISTRIBUTION_SCHEMA_VERSION: &str =
    "kyuubiki.operator-package-distribution/v1";
pub const OPERATOR_PACKAGE_DISTRIBUTION_FILE: &str = "kyuubiki-operator-distribution.json";

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct OperatorPackageDistributionManifest {
    pub schema_version: String,
    pub sdk_api_version: String,
    pub execution_abi: String,
    pub package_id: String,
    pub package_version: String,
    pub artifacts: Vec<OperatorPackageDistributionArtifact>,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct OperatorPackageDistributionArtifact {
    pub target: String,
    pub manifest_path: String,
    pub manifest_sha256: String,
    pub manifest_size_bytes: u64,
    pub entrypoint_path: String,
    pub entrypoint_sha256: String,
    pub entrypoint_size_bytes: u64,
}

#[derive(Debug)]
pub enum OperatorDistributionError {
    Io { path: PathBuf, message: String },
    Decode { path: PathBuf, message: String },
    Invalid { path: PathBuf, message: String },
}

impl Display for OperatorDistributionError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io { path, message } => {
                write!(formatter, "failed to access {}: {message}", path.display())
            }
            Self::Decode { path, message } => {
                write!(formatter, "failed to decode {}: {message}", path.display())
            }
            Self::Invalid { path, message } => {
                write!(
                    formatter,
                    "invalid operator distribution {}: {message}",
                    path.display()
                )
            }
        }
    }
}

impl std::error::Error for OperatorDistributionError {}

pub fn read_operator_package_distribution(
    path: impl AsRef<Path>,
) -> Result<OperatorPackageDistributionManifest, OperatorDistributionError> {
    let path = path.as_ref().to_path_buf();
    let content = fs::read_to_string(&path).map_err(|error| OperatorDistributionError::Io {
        path: path.clone(),
        message: error.to_string(),
    })?;
    let manifest =
        serde_json::from_str(&content).map_err(|error| OperatorDistributionError::Decode {
            path: path.clone(),
            message: error.to_string(),
        })?;
    validate_operator_package_distribution(&path, &manifest)?;
    Ok(manifest)
}

pub fn validate_operator_package_distribution(
    path: impl AsRef<Path>,
    manifest: &OperatorPackageDistributionManifest,
) -> Result<(), OperatorDistributionError> {
    let path = path.as_ref();
    if manifest.schema_version != OPERATOR_PACKAGE_DISTRIBUTION_SCHEMA_VERSION {
        return invalid(path, "unsupported schema_version");
    }
    if manifest.sdk_api_version != OPERATOR_SDK_API_VERSION {
        return invalid(path, "unsupported sdk_api_version");
    }
    if manifest.execution_abi != OPERATOR_JSON_ABI_SCHEMA_VERSION {
        return invalid(path, "unsupported execution_abi");
    }
    validate_token(path, &manifest.package_id, "package_id")?;
    validate_token(path, &manifest.package_version, "package_version")?;
    if manifest.artifacts.is_empty() {
        return invalid(path, "artifacts must contain at least one target");
    }

    let mut targets = BTreeSet::new();
    for artifact in &manifest.artifacts {
        validate_target(path, &artifact.target)?;
        if !targets.insert(artifact.target.as_str()) {
            return invalid(path, &format!("duplicate target {}", artifact.target));
        }
        validate_artifact_path(
            path,
            &artifact.target,
            &artifact.manifest_path,
            "manifest_path",
        )?;
        if Path::new(&artifact.manifest_path)
            .file_name()
            .and_then(|name| name.to_str())
            != Some(OPERATOR_PACKAGE_MANIFEST_FILE)
        {
            return invalid(
                path,
                &format!("manifest_path must end with {OPERATOR_PACKAGE_MANIFEST_FILE}"),
            );
        }
        validate_artifact_path(
            path,
            &artifact.target,
            &artifact.entrypoint_path,
            "entrypoint_path",
        )?;
        if artifact.manifest_path == artifact.entrypoint_path {
            return invalid(path, "manifest_path and entrypoint_path must be distinct");
        }
        validate_digest(path, &artifact.manifest_sha256, "manifest_sha256")?;
        validate_digest(path, &artifact.entrypoint_sha256, "entrypoint_sha256")?;
        if artifact.manifest_size_bytes == 0 || artifact.entrypoint_size_bytes == 0 {
            return invalid(path, "artifact sizes must be greater than zero");
        }
    }
    Ok(())
}

pub fn operator_distribution_artifact_for_target<'a>(
    manifest: &'a OperatorPackageDistributionManifest,
    target: &str,
) -> Option<&'a OperatorPackageDistributionArtifact> {
    manifest
        .artifacts
        .iter()
        .find(|artifact| artifact.target == target)
}

fn validate_token(path: &Path, value: &str, label: &str) -> Result<(), OperatorDistributionError> {
    if value.is_empty()
        || value.len() > 128
        || !value
            .as_bytes()
            .first()
            .is_some_and(u8::is_ascii_alphanumeric)
        || !value
            .as_bytes()
            .last()
            .is_some_and(u8::is_ascii_alphanumeric)
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
    {
        return invalid(path, &format!("{label} is not a safe portable token"));
    }
    Ok(())
}

fn validate_target(path: &Path, target: &str) -> Result<(), OperatorDistributionError> {
    let Some((os, architecture)) = target.split_once('-') else {
        return invalid(path, "target must use the os-architecture form");
    };
    if !matches!(os, "macos" | "linux" | "windows")
        || architecture.is_empty()
        || !architecture
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
    {
        return invalid(path, &format!("unsupported portable target {target}"));
    }
    Ok(())
}

fn validate_artifact_path(
    distribution_path: &Path,
    target: &str,
    value: &str,
    label: &str,
) -> Result<(), OperatorDistributionError> {
    if value.is_empty() || value.contains('\\') {
        return invalid(
            distribution_path,
            &format!("{label} must be a non-empty portable relative path"),
        );
    }
    let path = Path::new(value);
    if path.is_absolute()
        || path.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        return invalid(
            distribution_path,
            &format!("{label} escapes its distribution"),
        );
    }
    let first = path.components().next();
    if first != Some(Component::Normal(target.as_ref())) {
        return invalid(
            distribution_path,
            &format!("{label} must be rooted under target directory {target}"),
        );
    }
    Ok(())
}

fn validate_digest(
    path: &Path,
    digest: &str,
    label: &str,
) -> Result<(), OperatorDistributionError> {
    if digest.len() != 64
        || !digest
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return invalid(path, &format!("{label} must be lowercase SHA-256 hex"));
    }
    Ok(())
}

fn invalid<T>(path: &Path, message: &str) -> Result<T, OperatorDistributionError> {
    Err(OperatorDistributionError::Invalid {
        path: path.to_path_buf(),
        message: message.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_manifest() -> OperatorPackageDistributionManifest {
        OperatorPackageDistributionManifest {
            schema_version: OPERATOR_PACKAGE_DISTRIBUTION_SCHEMA_VERSION.to_string(),
            sdk_api_version: OPERATOR_SDK_API_VERSION.to_string(),
            execution_abi: OPERATOR_JSON_ABI_SCHEMA_VERSION.to_string(),
            package_id: "operator.example.peak_field".to_string(),
            package_version: "0.1.0".to_string(),
            artifacts: vec![OperatorPackageDistributionArtifact {
                target: "linux-x86_64".to_string(),
                manifest_path: "linux-x86_64/kyuubiki-operator.json".to_string(),
                manifest_sha256: "a".repeat(64),
                manifest_size_bytes: 512,
                entrypoint_path: "linux-x86_64/liboperator_example.so".to_string(),
                entrypoint_sha256: "b".repeat(64),
                entrypoint_size_bytes: 4096,
            }],
        }
    }

    #[test]
    fn validates_and_selects_platform_artifact() {
        let manifest = valid_manifest();
        validate_operator_package_distribution("distribution.json", &manifest)
            .expect("distribution should be valid");
        let artifact = operator_distribution_artifact_for_target(&manifest, "linux-x86_64")
            .expect("target artifact");
        assert_eq!(artifact.entrypoint_size_bytes, 4096);
        assert!(operator_distribution_artifact_for_target(&manifest, "macos-aarch64").is_none());
    }

    #[test]
    fn rejects_duplicate_targets_and_path_escape() {
        let mut duplicate = valid_manifest();
        duplicate.artifacts.push(duplicate.artifacts[0].clone());
        assert!(validate_operator_package_distribution("distribution.json", &duplicate).is_err());

        let mut escaped = valid_manifest();
        escaped.artifacts[0].entrypoint_path = "../outside.so".to_string();
        assert!(validate_operator_package_distribution("distribution.json", &escaped).is_err());
    }

    #[test]
    fn rejects_noncanonical_digest_and_target() {
        let mut manifest = valid_manifest();
        manifest.artifacts[0].manifest_sha256 = "A".repeat(64);
        assert!(validate_operator_package_distribution("distribution.json", &manifest).is_err());

        let mut manifest = valid_manifest();
        manifest.artifacts[0].target = "Linux-x86_64".to_string();
        assert!(validate_operator_package_distribution("distribution.json", &manifest).is_err());
    }
}
