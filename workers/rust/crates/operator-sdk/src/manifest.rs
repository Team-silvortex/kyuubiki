use kyuubiki_protocol::OperatorValidationStatus;
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};
use std::fs;
use std::path::{Path, PathBuf};

pub const OPERATOR_PACKAGE_SCHEMA_VERSION: &str = "kyuubiki.operator-package/v1";
pub const OPERATOR_SDK_API_VERSION: &str = "kyuubiki.operator-sdk/v1";
pub const OPERATOR_PACKAGE_MANIFEST_FILE: &str = "kyuubiki-operator.json";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OperatorPackageManifest {
    pub schema_version: String,
    pub sdk_api_version: String,
    pub package_id: String,
    pub package_version: String,
    pub minimum_host_version: String,
    pub validation_status: OperatorValidationStatus,
    pub validation_notes: String,
    pub runtime: String,
    pub entrypoint: String,
    #[serde(default)]
    pub operators: Vec<OperatorPackageOperatorEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OperatorPackageOperatorEntry {
    pub operator_id: String,
    pub kind: String,
    pub entry_symbol: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveredOperatorPackage {
    pub package_root: PathBuf,
    pub manifest_path: PathBuf,
    pub manifest: OperatorPackageManifest,
}

#[derive(Debug)]
pub enum OperatorManifestError {
    Io { path: PathBuf, message: String },
    Decode { path: PathBuf, message: String },
    Invalid { path: PathBuf, message: String },
}

impl Display for OperatorManifestError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io { path, message } => {
                write!(f, "failed to access {}: {message}", path.display())
            }
            Self::Decode { path, message } => {
                write!(f, "failed to decode {}: {message}", path.display())
            }
            Self::Invalid { path, message } => {
                write!(f, "invalid operator manifest {}: {message}", path.display())
            }
        }
    }
}

impl std::error::Error for OperatorManifestError {}

pub fn read_operator_package_manifest(
    manifest_path: impl AsRef<Path>,
) -> Result<OperatorPackageManifest, OperatorManifestError> {
    let manifest_path = manifest_path.as_ref().to_path_buf();
    let content =
        fs::read_to_string(&manifest_path).map_err(|error| OperatorManifestError::Io {
            path: manifest_path.clone(),
            message: error.to_string(),
        })?;
    let manifest: OperatorPackageManifest =
        serde_json::from_str(&content).map_err(|error| OperatorManifestError::Decode {
            path: manifest_path.clone(),
            message: error.to_string(),
        })?;
    validate_operator_package_manifest(&manifest_path, &manifest)?;
    Ok(manifest)
}

pub fn discover_operator_packages(
    root: impl AsRef<Path>,
) -> Result<Vec<DiscoveredOperatorPackage>, OperatorManifestError> {
    let root = root.as_ref().to_path_buf();
    let mut discovered = Vec::new();
    let entries = fs::read_dir(&root).map_err(|error| OperatorManifestError::Io {
        path: root.clone(),
        message: error.to_string(),
    })?;

    for entry in entries {
        let entry = entry.map_err(|error| OperatorManifestError::Io {
            path: root.clone(),
            message: error.to_string(),
        })?;
        let package_root = entry.path();
        if !package_root.is_dir() {
            continue;
        }
        let manifest_path = package_root.join(OPERATOR_PACKAGE_MANIFEST_FILE);
        if !manifest_path.is_file() {
            continue;
        }
        let manifest = read_operator_package_manifest(&manifest_path)?;
        discovered.push(DiscoveredOperatorPackage {
            package_root,
            manifest_path,
            manifest,
        });
    }

    discovered.sort_by(|left, right| {
        left.manifest
            .package_id
            .cmp(&right.manifest.package_id)
            .then(
                left.manifest
                    .package_version
                    .cmp(&right.manifest.package_version),
            )
    });
    Ok(discovered)
}

fn validate_operator_package_manifest(
    manifest_path: &Path,
    manifest: &OperatorPackageManifest,
) -> Result<(), OperatorManifestError> {
    if manifest.schema_version != OPERATOR_PACKAGE_SCHEMA_VERSION {
        return Err(OperatorManifestError::Invalid {
            path: manifest_path.to_path_buf(),
            message: format!(
                "expected schema_version {} but found {}",
                OPERATOR_PACKAGE_SCHEMA_VERSION, manifest.schema_version
            ),
        });
    }
    if manifest.sdk_api_version != OPERATOR_SDK_API_VERSION {
        return Err(OperatorManifestError::Invalid {
            path: manifest_path.to_path_buf(),
            message: format!(
                "expected sdk_api_version {} but found {}",
                OPERATOR_SDK_API_VERSION, manifest.sdk_api_version
            ),
        });
    }
    if manifest.package_id.trim().is_empty() {
        return Err(OperatorManifestError::Invalid {
            path: manifest_path.to_path_buf(),
            message: "package_id must not be empty".to_string(),
        });
    }
    if manifest.package_version.trim().is_empty() {
        return Err(OperatorManifestError::Invalid {
            path: manifest_path.to_path_buf(),
            message: "package_version must not be empty".to_string(),
        });
    }
    if manifest.minimum_host_version.trim().is_empty() {
        return Err(OperatorManifestError::Invalid {
            path: manifest_path.to_path_buf(),
            message: "minimum_host_version must not be empty".to_string(),
        });
    }
    if manifest.validation_notes.trim().is_empty() {
        return Err(OperatorManifestError::Invalid {
            path: manifest_path.to_path_buf(),
            message: "validation_notes must not be empty".to_string(),
        });
    }
    if manifest.entrypoint.trim().is_empty() {
        return Err(OperatorManifestError::Invalid {
            path: manifest_path.to_path_buf(),
            message: "entrypoint must not be empty".to_string(),
        });
    }
    if manifest.runtime.trim().is_empty() {
        return Err(OperatorManifestError::Invalid {
            path: manifest_path.to_path_buf(),
            message: "runtime must not be empty".to_string(),
        });
    }
    if manifest.operators.is_empty() {
        return Err(OperatorManifestError::Invalid {
            path: manifest_path.to_path_buf(),
            message: "operators must contain at least one entry".to_string(),
        });
    }
    for operator in &manifest.operators {
        if operator.operator_id.trim().is_empty() {
            return Err(OperatorManifestError::Invalid {
                path: manifest_path.to_path_buf(),
                message: "operator_id must not be empty".to_string(),
            });
        }
        if operator.entry_symbol.trim().is_empty() {
            return Err(OperatorManifestError::Invalid {
                path: manifest_path.to_path_buf(),
                message: format!(
                    "operator {} entry_symbol must not be empty",
                    operator.operator_id
                ),
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        OPERATOR_PACKAGE_MANIFEST_FILE, OPERATOR_PACKAGE_SCHEMA_VERSION,
        discover_operator_packages, read_operator_package_manifest,
    };
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn reads_valid_operator_package_manifest() {
        let dir = temp_dir("manifest-read");
        let manifest_path = dir.join(OPERATOR_PACKAGE_MANIFEST_FILE);
        fs::write(
            &manifest_path,
            serde_json::json!({
                "schema_version": OPERATOR_PACKAGE_SCHEMA_VERSION,
                "sdk_api_version": super::OPERATOR_SDK_API_VERSION,
                "package_id": "operator.example.peak_field",
                "package_version": "0.1.0",
                "minimum_host_version": "1.15.0",
                "validation_status": "partial",
                "validation_notes": "Manifest parser smoke fixture.",
                "runtime": "rust_crate",
                "entrypoint": "target/debug/liboperator_example_peak_field.dylib",
                "operators": [
                    {
                        "operator_id": "extract.electrostatic_peak_field",
                        "kind": "extract",
                        "entry_symbol": "register_operator"
                    }
                ]
            })
            .to_string(),
        )
        .expect("write manifest");

        let manifest =
            read_operator_package_manifest(&manifest_path).expect("manifest should parse");
        assert_eq!(manifest.package_id, "operator.example.peak_field");
        assert_eq!(manifest.operators.len(), 1);
    }

    #[test]
    fn discovers_operator_packages_from_directory() {
        let root = temp_dir("manifest-discover");
        let alpha = root.join("alpha");
        let beta = root.join("beta");
        fs::create_dir_all(&alpha).expect("create alpha");
        fs::create_dir_all(&beta).expect("create beta");

        fs::write(
            alpha.join(OPERATOR_PACKAGE_MANIFEST_FILE),
            serde_json::json!({
                "schema_version": OPERATOR_PACKAGE_SCHEMA_VERSION,
                "sdk_api_version": super::OPERATOR_SDK_API_VERSION,
                "package_id": "operator.alpha",
                "package_version": "0.1.0",
                "minimum_host_version": "1.15.0",
                "validation_status": "partial",
                "validation_notes": "Discovery smoke fixture.",
                "runtime": "rust_crate",
                "entrypoint": "target/debug/liboperator_alpha.dylib",
                "operators": [
                    {
                        "operator_id": "extract.alpha",
                        "kind": "extract",
                        "entry_symbol": "register_operator"
                    }
                ]
            })
            .to_string(),
        )
        .expect("write alpha");
        fs::write(
            beta.join(OPERATOR_PACKAGE_MANIFEST_FILE),
            serde_json::json!({
                "schema_version": OPERATOR_PACKAGE_SCHEMA_VERSION,
                "sdk_api_version": super::OPERATOR_SDK_API_VERSION,
                "package_id": "operator.beta",
                "package_version": "0.2.0",
                "minimum_host_version": "1.15.0",
                "validation_status": "partial",
                "validation_notes": "Discovery smoke fixture.",
                "runtime": "rust_crate",
                "entrypoint": "target/debug/liboperator_beta.dylib",
                "operators": [
                    {
                        "operator_id": "extract.beta",
                        "kind": "extract",
                        "entry_symbol": "register_operator"
                    }
                ]
            })
            .to_string(),
        )
        .expect("write beta");

        let packages = discover_operator_packages(&root).expect("packages should discover");
        assert_eq!(packages.len(), 2);
        assert_eq!(packages[0].manifest.package_id, "operator.alpha");
        assert_eq!(packages[1].manifest.package_id, "operator.beta");
    }

    #[test]
    fn rejects_manifest_with_wrong_sdk_api_version() {
        let dir = temp_dir("manifest-sdk-version");
        let manifest_path = dir.join(OPERATOR_PACKAGE_MANIFEST_FILE);
        fs::write(
            &manifest_path,
            serde_json::json!({
                "schema_version": OPERATOR_PACKAGE_SCHEMA_VERSION,
                "sdk_api_version": "kyuubiki.operator-sdk/v0",
                "package_id": "operator.bad",
                "package_version": "0.1.0",
                "minimum_host_version": "1.15.0",
                "validation_status": "partial",
                "validation_notes": "Bad SDK version fixture.",
                "runtime": "rust_crate",
                "entrypoint": "target/debug/liboperator_bad.dylib",
                "operators": [
                    {
                        "operator_id": "extract.bad",
                        "kind": "extract",
                        "entry_symbol": "register_bad"
                    }
                ]
            })
            .to_string(),
        )
        .expect("write manifest");

        let error = read_operator_package_manifest(&manifest_path).expect_err("manifest rejects");
        assert!(error.to_string().contains("sdk_api_version"));
    }

    #[test]
    fn rejects_manifest_without_validation_notes() {
        let dir = temp_dir("manifest-validation-notes");
        let manifest_path = dir.join(OPERATOR_PACKAGE_MANIFEST_FILE);
        fs::write(
            &manifest_path,
            serde_json::json!({
                "schema_version": OPERATOR_PACKAGE_SCHEMA_VERSION,
                "sdk_api_version": super::OPERATOR_SDK_API_VERSION,
                "package_id": "operator.bad",
                "package_version": "0.1.0",
                "minimum_host_version": "1.15.0",
                "validation_status": "partial",
                "validation_notes": "",
                "runtime": "rust_crate",
                "entrypoint": "target/debug/liboperator_bad.dylib",
                "operators": [
                    {
                        "operator_id": "extract.bad",
                        "kind": "extract",
                        "entry_symbol": "register_bad"
                    }
                ]
            })
            .to_string(),
        )
        .expect("write manifest");

        let error = read_operator_package_manifest(&manifest_path).expect_err("manifest rejects");
        assert!(error.to_string().contains("validation_notes"));
    }

    fn temp_dir(label: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("kyuubiki-{label}-{unique}"));
        fs::create_dir_all(&dir).expect("create temp dir");
        dir
    }
}
