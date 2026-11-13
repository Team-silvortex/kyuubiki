use super::operator_release_binary::{self as binary, Target};
use kyuubiki_operator_sdk::{
    OPERATOR_PACKAGE_DISTRIBUTION_FILE, OPERATOR_PACKAGE_DISTRIBUTION_SCHEMA_VERSION,
    OPERATOR_PACKAGE_MANIFEST_FILE, OperatorPackageDistributionArtifact,
    OperatorPackageDistributionManifest, OperatorPackageManifest,
    operator_package_manifest_readiness, validate_operator_package_distribution,
};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

const MAX_MANIFEST: u64 = 1024 * 1024;
const MAX_LIBRARY: u64 = 512 * 1024 * 1024;

#[derive(Clone, Debug)]
pub struct OperatorReleaseArtifact {
    pub rust_target: String,
    pub library_path: PathBuf,
}

/// Prepare an immutable distribution candidate; does not execute, sign, upload or approve code.
pub fn prepare_operator_distribution(
    manifest_path: &Path,
    artifacts: &[OperatorReleaseArtifact],
    output: &Path,
) -> Result<OperatorPackageDistributionManifest, String> {
    if artifacts.is_empty() || artifacts.len() > 5 {
        return Err("release requires 1..5 distinct supported targets".into());
    }
    regular_file(manifest_path, MAX_MANIFEST)?;
    let mut bytes = Vec::new();
    File::open(manifest_path)
        .map_err(|e| e.to_string())?
        .take(MAX_MANIFEST + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() as u64 > MAX_MANIFEST {
        return Err("manifest exceeds size limit".into());
    }
    let raw: serde_json::Value = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    let manifest: OperatorPackageManifest =
        serde_json::from_value(raw.clone()).map_err(|e| e.to_string())?;
    super::operator_package_store::validate_component(&manifest.package_id, "package_id")?;
    super::operator_package_store::validate_component(
        &manifest.package_version,
        "package_version",
    )?;
    // Reject metadata the execution contract would silently discard on repackaging.
    if serde_json::to_value(&manifest).map_err(|e| e.to_string())? != raw {
        return Err("manifest must explicitly contain only the supported package fields".into());
    }
    let readiness = operator_package_manifest_readiness(&manifest);
    if !readiness.ok || manifest.runtime != "rust_crate" {
        return Err(format!(
            "manifest is not release-ready: {:?}",
            readiness.issues
        ));
    }
    let mut targets = BTreeSet::new();
    let mut selected = Vec::new();
    for artifact in artifacts {
        let target = binary::target(&artifact.rust_target)?;
        if !targets.insert(target.id) {
            return Err(format!("duplicate release target {}", target.id));
        }
        regular_file(&artifact.library_path, MAX_LIBRARY)?;
        selected.push((target, artifact));
    }
    selected.sort_by_key(|(target, _)| target.id);
    let parent = output
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let parent = parent
        .canonicalize()
        .map_err(|e| format!("output parent must exist: {e}"))?;
    let name = output
        .file_name()
        .and_then(|n| n.to_str())
        .filter(|n| !n.is_empty())
        .ok_or("output requires a named directory")?;
    let output = parent.join(name);
    let lock = parent.join(format!(".{name}.operator-release.lock"));
    let _lock_file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&lock)
        .map_err(|e| format!("release output is locked: {e}"))?;
    let _lock = Cleanup {
        path: lock,
        directory: false,
    };
    // Close before the guard removes the lock file, including on Windows.
    drop(_lock_file);
    if output.symlink_metadata().is_ok() {
        return Err(
            "release output already exists; immutable candidates are never overwritten".into(),
        );
    }
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_nanos();
    let stage = parent.join(format!(".{name}.stage-{}-{nonce}", std::process::id()));
    fs::create_dir(&stage).map_err(|e| e.to_string())?;
    let _stage = Cleanup {
        path: stage.clone(),
        directory: true,
    };
    let mut distribution = OperatorPackageDistributionManifest {
        schema_version: OPERATOR_PACKAGE_DISTRIBUTION_SCHEMA_VERSION.into(),
        sdk_api_version: manifest.sdk_api_version.clone(),
        execution_abi: manifest.execution_abi.clone(),
        package_id: manifest.package_id.clone(),
        package_version: manifest.package_version.clone(),
        artifacts: vec![],
    };
    for (target, artifact) in selected {
        distribution
            .artifacts
            .push(stage_artifact(&stage, &manifest, target, artifact)?);
    }
    validate_operator_package_distribution(OPERATOR_PACKAGE_DISTRIBUTION_FILE, &distribution)
        .map_err(|e| e.to_string())?;
    write_json(
        &stage.join(OPERATOR_PACKAGE_DISTRIBUTION_FILE),
        &distribution,
    )?;
    if output.symlink_metadata().is_ok() {
        return Err("release output appeared while preparing; refusing replacement".into());
    }
    fs::rename(&stage, &output).map_err(|e| format!("failed to promote release candidate: {e}"))?;
    Ok(distribution)
}

fn stage_artifact(
    stage: &Path,
    manifest: &OperatorPackageManifest,
    target: Target,
    artifact: &OperatorReleaseArtifact,
) -> Result<OperatorPackageDistributionArtifact, String> {
    let root = stage.join(target.id);
    fs::create_dir(&root).map_err(|e| e.to_string())?;
    let library = root.join(target.filename);
    let (entrypoint_sha256, entrypoint_size_bytes) =
        copy_and_hash(&artifact.library_path, &library)?;
    // Inspect the copied bytes whose hash goes into the index, not a separately opened source.
    binary::verify(&library, target)?;
    let mut manifest = manifest.clone();
    manifest.entrypoint = target.filename.into();
    let manifest_bytes = write_json(&root.join(OPERATOR_PACKAGE_MANIFEST_FILE), &manifest)?;
    Ok(OperatorPackageDistributionArtifact {
        target: target.id.into(),
        manifest_path: format!("{}/{}", target.id, OPERATOR_PACKAGE_MANIFEST_FILE),
        manifest_sha256: format!("{:x}", Sha256::digest(&manifest_bytes)),
        manifest_size_bytes: manifest_bytes.len() as u64,
        entrypoint_path: format!("{}/{}", target.id, target.filename),
        entrypoint_sha256,
        entrypoint_size_bytes,
    })
}

fn regular_file(path: &Path, limit: u64) -> Result<u64, String> {
    let m = path
        .symlink_metadata()
        .map_err(|e| format!("cannot inspect artifact {}: {e}", path.display()))?;
    if !m.file_type().is_file() || m.len() == 0 || m.len() > limit {
        return Err(format!(
            "artifact must be a nonempty regular file within {limit} bytes"
        ));
    }
    Ok(m.len())
}

fn copy_and_hash(source: &Path, target: &Path) -> Result<(String, u64), String> {
    let expected = regular_file(source, MAX_LIBRARY)?;
    let mut input = File::open(source)
        .map_err(|e| e.to_string())?
        .take(MAX_LIBRARY + 1);
    let mut output = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(target)
        .map_err(|e| e.to_string())?;
    let mut digest = Sha256::new();
    let mut buffer = [0u8; 65536];
    let mut size = 0;
    loop {
        let count = input.read(&mut buffer).map_err(|e| e.to_string())?;
        if count == 0 {
            break;
        }
        size += count as u64;
        if size > MAX_LIBRARY {
            return Err("library grew beyond size limit".into());
        }
        output
            .write_all(&buffer[..count])
            .map_err(|e| e.to_string())?;
        digest.update(&buffer[..count]);
    }
    if size != expected {
        return Err("library changed size during packaging".into());
    }
    output.sync_all().map_err(|e| e.to_string())?;
    Ok((format!("{:x}", digest.finalize()), size))
}

fn write_json<T: Serialize>(path: &Path, value: &T) -> Result<Vec<u8>, String> {
    let mut bytes = serde_json::to_vec_pretty(value).map_err(|e| e.to_string())?;
    bytes.push(b'\n');
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|e| e.to_string())?;
    file.write_all(&bytes).map_err(|e| e.to_string())?;
    file.sync_all().map_err(|e| e.to_string())?;
    Ok(bytes)
}

struct Cleanup {
    path: PathBuf,
    directory: bool,
}
impl Drop for Cleanup {
    fn drop(&mut self) {
        if self.directory {
            let _ = fs::remove_dir_all(&self.path);
        } else {
            let _ = fs::remove_file(&self.path);
        }
    }
}
