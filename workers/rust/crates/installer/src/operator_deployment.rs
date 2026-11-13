//! Bounded, restart-resumable package ingress. Never loads executable code.
use crate::operator_deployment_lock::DeploymentLock;
use kyuubiki_operator_sdk::OperatorPackageManifest;
use kyuubiki_protocol::*;
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
    path::Path,
};

fn safe(path: &Path, directory: bool) -> Result<(), String> {
    match fs::symlink_metadata(path) {
        Ok(m)
            if !m.file_type().is_symlink() && if directory { m.is_dir() } else { m.is_file() } =>
        {
            Ok(())
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        _ => Err("deployment_unsafe_path".into()),
    }
}
fn options() -> OpenOptions {
    let mut o = OpenOptions::new();
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        o.mode(0o600);
    }
    o
}
fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn read(path: &Path, max: u64) -> Result<Vec<u8>, String> {
    safe(path, false)?;
    let mut data = Vec::new();
    File::open(path)
        .map_err(|_| "deployment_unavailable")?
        .take(max + 1)
        .read_to_end(&mut data)
        .map_err(|_| "deployment_unavailable")?;
    if data.len() as u64 > max {
        return Err("deployment_limit".into());
    }
    Ok(data)
}
fn manifest(spec: &OperatorDeploymentSpec) -> Result<OperatorPackageManifest, String> {
    manifest_for(spec, &kyuubiki_operator_sdk::current_platform_target_id())
}
fn manifest_for(
    spec: &OperatorDeploymentSpec,
    platform: &str,
) -> Result<OperatorPackageManifest, String> {
    spec.package.validate()?;
    if spec.size_bytes == 0
        || spec.size_bytes > OPERATOR_DEPLOYMENT_MAX_BYTES
        || spec.manifest_json.len() > 65536
        || spec.platform != platform
        || digest(spec.manifest_json.as_bytes()) != spec.package.manifest_sha256
    {
        return Err("deployment_invalid_spec".into());
    }
    let m: OperatorPackageManifest =
        serde_json::from_str(&spec.manifest_json).map_err(|_| "deployment_invalid_manifest")?;
    // Ingress has exactly two flat files; no archive paths, URLs or install hooks.
    if !valid_operator_component(&m.entrypoint)
        || m.entrypoint == "kyuubiki-operator.json"
        || m.package_id != spec.package.package_id
        || m.package_version != spec.package.package_version
        || m.operators
            .iter()
            .map(|o| ProjectOperatorEntry {
                operator_id: o.operator_id.clone(),
                kind: o.kind.clone(),
            })
            .collect::<Vec<_>>()
            != spec.package.operators
    {
        return Err("deployment_invalid_manifest".into());
    }
    Ok(m)
}
fn write_new(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let mut f = options()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|_| "deployment_write_failed")?;
    f.write_all(bytes)
        .and_then(|_| f.sync_all())
        .map_err(|_| "deployment_write_failed".into())
}
fn sync_dir(path: &Path) -> Result<(), String> {
    #[cfg(unix)]
    File::open(path)
        .and_then(|f| f.sync_all())
        .map_err(|_| "deployment_write_failed")?;
    #[cfg(not(unix))]
    let _ = path;
    Ok(())
}
fn pin(root: &Path, package: &ProjectOperatorPin) -> Result<Option<ProjectOperatorPin>, String> {
    let path = root.join("packages").join(&package.package_id);
    safe(&root.join("packages"), true)?;
    safe(&path, true)?;
    if !path.exists() {
        return Ok(None);
    }
    crate::project_operator::pin_at(root, &package.package_id).map(Some)
}
pub fn operator_deployment_in(
    root: &Path,
    package: &ProjectOperatorPin,
    operation: OperatorDeploymentAction,
) -> Result<OperatorDeploymentStatus, String> {
    package.validate()?;
    safe(root, true)?;
    fs::create_dir_all(root).map_err(|_| "deployment_unavailable")?;
    let root = root.canonicalize().map_err(|_| "deployment_unavailable")?;
    let uploads = root.join("deployments");
    safe(&uploads, true)?;
    fs::create_dir_all(&uploads).map_err(|_| "deployment_unavailable")?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&uploads, fs::Permissions::from_mode(0o700))
            .map_err(|_| "deployment_unavailable")?;
    }
    let lock_path = uploads.join("lock");
    safe(&lock_path, false)?;
    let lock = options()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(lock_path)
        .map_err(|_| "deployment_unavailable")?;
    let _lock = DeploymentLock::acquire(lock)?;
    let id = digest(&serde_json::to_vec(package).map_err(|_| "deployment_invalid_spec")?);
    let dir = uploads.join(id);
    safe(&dir, true)?;
    let installed = pin(&root, package)?;
    let mut result = OperatorDeploymentStatus {
        schema_version: OPERATOR_DEPLOYMENT_SCHEMA.into(),
        package: package.clone(),
        platform: kyuubiki_operator_sdk::current_platform_target_id(),
        state: if installed.as_ref().is_some_and(|p| p != package) {
            "version_conflict"
        } else if installed.is_some() {
            "installed_restart_required"
        } else {
            "missing"
        }
        .into(),
        installed,
        active: None,
        received_bytes: 0,
        total_bytes: 0,
        process_instance_id: None,
        deployment_allowed: true,
        activation_error: None,
    };
    if matches!(operation, OperatorDeploymentAction::Abort) {
        if dir.exists() {
            fs::remove_dir_all(&dir).map_err(|_| "deployment_cleanup_failed")?;
            sync_dir(&uploads)?;
        }
        return Ok(result);
    }
    // A conflict is an observation, never permission to uninstall or overwrite.
    if result.installed.is_some() {
        if result.installed.as_ref() == Some(package)
            && matches!(operation, OperatorDeploymentAction::Commit)
            && dir.exists()
        {
            fs::remove_dir_all(&dir).map_err(|_| "deployment_cleanup_failed")?;
            sync_dir(&uploads)?;
        }
        return Ok(result);
    }
    if let OperatorDeploymentAction::Begin { spec } = &operation {
        if &spec.package != package {
            return Err("deployment_identity_mismatch".into());
        }
        manifest(spec)?;
        if !dir.exists() {
            if fs::read_dir(&uploads)
                .map_err(|_| "deployment_unavailable")?
                .take(6)
                .count()
                >= 5
            {
                return Err("deployment_capacity".into());
            }
            fs::create_dir(&dir).map_err(|_| "deployment_unavailable")?;
            // Interrupted creation is unconfirmed, and can be explicitly aborted.
            write_new(
                &dir.join("spec.json"),
                &serde_json::to_vec(spec).map_err(|_| "deployment_invalid_spec")?,
            )?;
            write_new(&dir.join("data"), &[])?;
            sync_dir(&dir)?;
            sync_dir(&uploads)?;
        }
    }
    if !dir.exists() {
        return if matches!(operation, OperatorDeploymentAction::Inspect) {
            Ok(result)
        } else {
            Err("deployment_not_started".into())
        };
    }
    let spec: OperatorDeploymentSpec =
        serde_json::from_slice(&read(&dir.join("spec.json"), 131072)?)
            .map_err(|_| "deployment_corrupt")?;
    if &spec.package != package {
        return Err("deployment_identity_mismatch".into());
    }
    let m = manifest(&spec)?;
    if let OperatorDeploymentAction::Begin { spec: expected } = &operation {
        if expected != &spec {
            return Err("deployment_identity_mismatch".into());
        }
    }
    let data_path = dir.join("data");
    safe(&data_path, false)?;
    let mut data = options()
        .read(true)
        .write(true)
        .open(&data_path)
        .map_err(|_| "deployment_corrupt")?;
    let mut size = data.metadata().map_err(|_| "deployment_unavailable")?.len();
    if size > spec.size_bytes {
        return Err("deployment_corrupt".into());
    }
    if let OperatorDeploymentAction::Chunk {
        offset,
        data: chunk,
    } = &operation
    {
        if chunk.is_empty()
            || chunk.len() > OPERATOR_DEPLOYMENT_CHUNK_BYTES
            || offset
                .checked_add(chunk.len() as u64)
                .is_none_or(|end| end > spec.size_bytes)
        {
            return Err("deployment_invalid_chunk".into());
        }
        if *offset < size {
            if offset + chunk.len() as u64 > size {
                return Err("deployment_offset_conflict".into());
            }
            let mut old = vec![0; chunk.len()];
            data.seek(SeekFrom::Start(*offset))
                .and_then(|_| data.read_exact(&mut old))
                .map_err(|_| "deployment_unavailable")?;
            if &old != chunk {
                return Err("deployment_offset_conflict".into());
            }
        } else if *offset == size {
            data.seek(SeekFrom::End(0))
                .and_then(|_| data.write_all(chunk))
                .and_then(|_| data.sync_all())
                .map_err(|_| "deployment_write_failed")?;
            size += chunk.len() as u64;
        } else {
            return Err("deployment_offset_conflict".into());
        }
    }
    result.received_bytes = size;
    result.total_bytes = spec.size_bytes;
    result.state = "uploading".into();
    if matches!(operation, OperatorDeploymentAction::Commit) {
        if size != spec.size_bytes {
            return Err("deployment_incomplete".into());
        }
        drop(data);
        if digest(&read(&data_path, OPERATOR_DEPLOYMENT_MAX_BYTES)?) != package.entrypoint_sha256 {
            return Err("deployment_digest_mismatch".into());
        }
        let staged = dir.join("package");
        safe(&staged, true)?;
        if staged.exists() {
            fs::remove_dir_all(&staged).map_err(|_| "deployment_write_failed")?;
        }
        fs::create_dir(&staged).map_err(|_| "deployment_write_failed")?;
        write_new(
            &staged.join("kyuubiki-operator.json"),
            spec.manifest_json.as_bytes(),
        )?;
        fs::copy(&data_path, staged.join(&m.entrypoint)).map_err(|_| "deployment_write_failed")?;
        crate::install_operator_package_into(&staged, &root)?;
        let installed = pin(&root, package)?.ok_or("deployment_unconfirmed")?;
        if &installed != package {
            return Err("deployment_identity_mismatch".into());
        }
        result.installed = Some(installed);
        result.state = "installed_restart_required".into();
        fs::remove_dir_all(&dir).map_err(|_| "deployment_cleanup_failed")?;
        sync_dir(&uploads)?;
    }
    Ok(result)
}

/// Final remote commit rehashes the full binary; chunks never carry local paths.
pub(crate) fn export_chunk_for(
    root: &Path,
    package: &ProjectOperatorPin,
    offset: u64,
    platform: &str,
) -> Result<OperatorDeploymentChunk, String> {
    package.validate()?;
    let path = root.join("packages").join(&package.package_id);
    safe(root, true)?;
    safe(&root.join("packages"), true)?;
    safe(&path, true)?;
    let manifest_json = String::from_utf8(read(&path.join("kyuubiki-operator.json"), 65536)?)
        .map_err(|_| "deployment_invalid_manifest")?;
    let m: OperatorPackageManifest =
        serde_json::from_str(&manifest_json).map_err(|_| "deployment_invalid_manifest")?;
    if !valid_operator_component(&m.entrypoint) {
        return Err("deployment_flat_package_required".into());
    }
    let entry = path.join(&m.entrypoint);
    safe(&entry, false)?;
    let mut file = File::open(entry).map_err(|_| "deployment_unavailable")?;
    let size_bytes = file.metadata().map_err(|_| "deployment_unavailable")?.len();
    let spec = OperatorDeploymentSpec {
        package: package.clone(),
        platform: platform.into(),
        manifest_json,
        size_bytes,
    };
    manifest_for(&spec, platform)?;
    if offset > size_bytes {
        return Err("deployment_invalid_offset".into());
    }
    file.seek(SeekFrom::Start(offset))
        .map_err(|_| "deployment_unavailable")?;
    let mut data = Vec::new();
    file.take(OPERATOR_DEPLOYMENT_CHUNK_BYTES as u64)
        .read_to_end(&mut data)
        .map_err(|_| "deployment_unavailable")?;
    Ok(OperatorDeploymentChunk { spec, offset, data })
}
