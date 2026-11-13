//! Foreign binaries are inert delivery resources, outside the local Agent package root.
use crate::store_operator_install::verified_snapshot;
use fs2::FileExt;
use kyuubiki_operator_sdk::{OperatorPackageManifest, operator_package_manifest_readiness};
use kyuubiki_protocol::*;
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
};

pub(crate) fn variant_root(root: &Path, platform: &str) -> Result<PathBuf, String> {
    if !matches!(platform, "linux-x86_64" | "linux-aarch64") {
        return Err("operator_variant_platform_unsupported".into());
    }
    let variants = root.join("variants");
    let target = variants.join(platform);
    for path in [root, &variants, &target] {
        safe(path, true)?;
    }
    Ok(target)
}

fn safe(path: &Path, dir: bool) -> Result<(), String> {
    match fs::symlink_metadata(path) {
        Ok(m) if !m.file_type().is_symlink() && if dir { m.is_dir() } else { m.is_file() } => {
            Ok(())
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        _ => Err("operator_variant_unsafe_path".into()),
    }
}
fn options() -> OpenOptions {
    let mut options = OpenOptions::new();
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    options
}
fn dir(path: &Path) -> Result<(), String> {
    safe(path, true)?;
    fs::create_dir_all(path).map_err(|_| "operator_variant_storage_unavailable")?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o700))
            .map_err(|_| "operator_variant_storage_unavailable")?;
    }
    Ok(())
}
fn sync(path: &Path) -> Result<(), String> {
    #[cfg(unix)]
    File::open(path)
        .and_then(|f| f.sync_all())
        .map_err(|_| "operator_variant_write_failed")?;
    #[cfg(not(unix))]
    let _ = path;
    Ok(())
}
fn validate(request: &StoreOperatorInstallRequest) -> Result<(), String> {
    if !valid_operator_component(&request.package_id)
        || !matches!(request.platform.as_str(), "linux-x86_64" | "linux-aarch64")
    {
        return Err("operator_variant_invalid_request".into());
    }
    if request.files.iter().any(|f| {
        f.size_bytes > OPERATOR_DEPLOYMENT_MAX_BYTES
            || (f.path.ends_with("/kyuubiki-operator.json") && f.size_bytes > 65536)
    }) {
        return Err("operator_variant_limit".into());
    }
    Ok(())
}
fn pin(
    request: &StoreOperatorInstallRequest,
    manifest: OperatorPackageManifest,
) -> Result<ProjectOperatorPin, String> {
    if !operator_package_manifest_readiness(&manifest).ok {
        return Err("operator_variant_manifest_incompatible".into());
    }
    let hash = |name: &str| {
        request
            .files
            .iter()
            .find(|f| f.path == format!("{}/{name}", request.platform))
            .map(|f| f.sha256.clone())
            .ok_or_else(|| "operator_variant_receipt_invalid".to_string())
    };
    let pin = ProjectOperatorPin {
        package_id: request.package_id.clone(),
        package_version: request.package_version.clone(),
        manifest_sha256: hash("kyuubiki-operator.json")?,
        entrypoint_sha256: hash("operator.so")?,
        operators: manifest
            .operators
            .into_iter()
            .map(|o| ProjectOperatorEntry {
                operator_id: o.operator_id,
                kind: o.kind,
            })
            .collect(),
    };
    pin.validate()?;
    Ok(pin)
}
pub(crate) fn variant_pin_at(
    root: &Path,
    id: &str,
    platform: &str,
) -> Result<ProjectOperatorPin, String> {
    if !valid_operator_component(id) {
        return Err("operator_variant_invalid_request".into());
    }
    let packages = root.join("packages");
    let path = packages.join(id);
    safe(&packages, true)?;
    safe(&path, true)?;
    let receipt = path.join("variant.json");
    safe(&receipt, false)?;
    let mut bytes = Vec::new();
    File::open(receipt)
        .and_then(|f| f.take(16385).read_to_end(&mut bytes))
        .map_err(|_| "operator_variant_unavailable")?;
    if bytes.len() > 16384 {
        return Err("operator_variant_limit".into());
    }
    let request: StoreOperatorInstallRequest =
        serde_json::from_slice(&bytes).map_err(|_| "operator_variant_receipt_invalid")?;
    validate(&request)?;
    if request.platform != platform || request.package_id != id {
        return Err("operator_variant_receipt_mismatch".into());
    }
    let (_snapshot, manifest) = verified_snapshot(&path, &request)?;
    pin(&request, manifest)
}

pub fn stage_store_operator_variant_into(
    source: &Path,
    store: &Path,
    request: &StoreOperatorInstallRequest,
) -> Result<OperatorVariantReceipt, String> {
    validate(request)?;
    safe(source, true)?;
    let root = variant_root(store, &request.platform)?;
    let (snapshot, manifest) = verified_snapshot(&source.join(&request.platform), request)?;
    let package = pin(request, manifest)?;
    dir(store)?;
    dir(&store.join("variants"))?;
    dir(&root)?;
    let lock_path = root.join("variant.lock");
    safe(&lock_path, false)?;
    let lock = options()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(lock_path)
        .map_err(|_| "operator_variant_storage_unavailable")?;
    lock.try_lock_exclusive()
        .map_err(|_| "operator_variant_busy")?;
    let packages = root.join("packages");
    dir(&packages)?;
    let target = packages.join(&package.package_id);
    safe(&target, true)?;
    if target.exists() {
        if variant_pin_at(&root, &package.package_id, &request.platform)? != package {
            return Err("operator_variant_version_conflict".into());
        }
    } else {
        if fs::read_dir(&packages)
            .map_err(|_| "operator_variant_unavailable")?
            .take(8)
            .count()
            >= 8
        {
            return Err("operator_variant_capacity".into());
        }
        // One owned pending slot under the lock: retry after a crash cannot accumulate payloads.
        let pending = root.join("variant.pending");
        safe(&pending, true)?;
        if pending.exists() {
            fs::remove_dir_all(&pending).map_err(|_| "operator_variant_write_failed")?;
        }
        dir(&pending)?;
        let result = (|| -> Result<(), String> {
            for name in ["kyuubiki-operator.json", "operator.so"] {
                fs::copy(snapshot.0.join(name), pending.join(name))
                    .map_err(|_| "operator_variant_write_failed")?;
                File::open(pending.join(name))
                    .and_then(|f| f.sync_all())
                    .map_err(|_| "operator_variant_write_failed")?;
            }
            let mut receipt = options()
                .write(true)
                .create_new(true)
                .open(pending.join("variant.json"))
                .map_err(|_| "operator_variant_write_failed")?;
            receipt
                .write_all(
                    &serde_json::to_vec(request).map_err(|_| "operator_variant_receipt_invalid")?,
                )
                .and_then(|_| receipt.sync_all())
                .map_err(|_| "operator_variant_write_failed")?;
            sync(&pending)?;
            fs::rename(&pending, &target).map_err(|_| "operator_variant_write_failed")?;
            sync(&packages)?;
            sync(&root)
        })();
        if result.is_err() {
            let _ = fs::remove_dir_all(&pending);
        }
        result?;
    }
    Ok(OperatorVariantReceipt {
        schema_version: "kyuubiki.operator-variant/v1".into(),
        platform: request.platform.clone(),
        package,
        size_bytes: request
            .files
            .iter()
            .find(|f| f.path.ends_with("/operator.so"))
            .ok_or("operator_variant_receipt_invalid")?
            .size_bytes,
        relative_root: format!(
            "variants/{}/packages/{}",
            request.platform, request.package_id
        ),
    })
}
