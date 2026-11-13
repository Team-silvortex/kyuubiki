use crate::{
    ManagedOperatorPackageReceipt, install_operator_package_into, operator_release_binary,
};
use kyuubiki_operator_sdk::{
    OPERATOR_PACKAGE_MANIFEST_FILE, OperatorPackageManifest, current_platform_target_id,
};
pub use kyuubiki_protocol::StoreOperatorInstallRequest;
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

/// Rechecks the transport receipt in a private snapshot, then uses the existing native installer.
/// This never activates a package or loads a dynamic library.
pub fn install_store_operator_into(
    source: &Path,
    store: &Path,
    request: &StoreOperatorInstallRequest,
) -> Result<ManagedOperatorPackageReceipt, String> {
    if request.platform != current_platform_target_id() {
        return Err("store_platform_mismatch".into());
    }
    regular(source, true)?;
    let (snapshot, manifest) = verified_snapshot(&source.join(&request.platform), request)?;
    let receipt = install_operator_package_into(&snapshot.0, store)?;
    let expected = |name: &str| {
        request
            .files
            .iter()
            .find(|f| f.path == format!("{}/{name}", request.platform))
            .unwrap()
    };
    if receipt.package_id != request.package_id
        || receipt.package_version != request.package_version
        || receipt.manifest_sha256 != expected(OPERATOR_PACKAGE_MANIFEST_FILE).sha256
        || receipt.entrypoint_sha256 != expected(&manifest.entrypoint).sha256
        || receipt.entrypoint_size_bytes != expected(&manifest.entrypoint).size_bytes
    {
        return Err("store_install_receipt_mismatch".into());
    }
    Ok(receipt)
}

/// Verify a flat target directory without applying host-platform admission or loading code.
pub(crate) fn verified_snapshot(
    source: &Path,
    request: &StoreOperatorInstallRequest,
) -> Result<(Snapshot, OperatorPackageManifest), String> {
    let invalid = || "store_package_invalid".to_string();
    if request.schema_version != "kyuubiki.store-operator-install/v1" {
        return Err("store_platform_mismatch".into());
    }
    let triple = match request.platform.as_str() {
        "macos-aarch64" => "aarch64-apple-darwin",
        "macos-x86_64" => "x86_64-apple-darwin",
        "linux-aarch64" => "aarch64-unknown-linux-gnu",
        "linux-x86_64" => "x86_64-unknown-linux-gnu",
        "windows-x86_64" => "x86_64-pc-windows-msvc",
        _ => return Err("store_platform_mismatch".into()),
    };
    let target = operator_release_binary::target(triple)?;
    let names = [OPERATOR_PACKAGE_MANIFEST_FILE, target.filename];
    if request.files.len() != 2
        || names.iter().any(|name| {
            request
                .files
                .iter()
                .filter(|file| file.path == format!("{}/{name}", request.platform))
                .count()
                != 1
        })
    {
        return Err("store_package_layout_unsupported".into());
    }
    regular(source, true)?;
    let snapshot = Snapshot::new()?;
    for file in &request.files {
        if file.size_bytes == 0
            || file.size_bytes > 64 * 1024 * 1024
            || file.sha256.len() != 64
            || !file
                .sha256
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err(invalid());
        }
        let name = file.path.split('/').next_back().ok_or_else(invalid)?;
        if name == OPERATOR_PACKAGE_MANIFEST_FILE && file.size_bytes > 1024 * 1024 {
            return Err(invalid());
        }
        let path = source.join(name);
        regular(&path, false)?;
        let input = fs::File::open(path).map_err(|_| invalid())?;
        if !input.metadata().map_err(|_| invalid())?.is_file() {
            return Err(invalid());
        }
        let mut input = input.take(file.size_bytes + 1);
        let mut options = fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut output = options.open(snapshot.0.join(name)).map_err(|_| invalid())?;
        let mut digest = Sha256::new();
        let mut bytes = [0; 65536];
        let mut length = 0;
        loop {
            let count = input.read(&mut bytes).map_err(|_| invalid())?;
            if count == 0 {
                break;
            }
            length += count as u64;
            if length > file.size_bytes {
                return Err("store_artifact_mismatch".into());
            }
            digest.update(&bytes[..count]);
            output.write_all(&bytes[..count]).map_err(|_| invalid())?;
        }
        if length != file.size_bytes || format!("{:x}", digest.finalize()) != file.sha256 {
            return Err("store_artifact_mismatch".into());
        }
        output.sync_all().map_err(|_| invalid())?;
    }
    let value: serde_json::Value = serde_json::from_slice(
        &fs::read(snapshot.0.join(OPERATOR_PACKAGE_MANIFEST_FILE)).map_err(|_| invalid())?,
    )
    .map_err(|_| invalid())?;
    let manifest: OperatorPackageManifest =
        serde_json::from_value(value.clone()).map_err(|_| invalid())?;
    if serde_json::to_value(&manifest).map_err(|_| invalid())? != value
        || manifest.package_id != request.package_id
        || manifest.package_version != request.package_version
        || manifest.runtime != "rust_crate"
        || manifest.entrypoint != target.filename
    {
        return Err("store_manifest_mismatch".into());
    }
    operator_release_binary::verify(&snapshot.0.join(target.filename), target)
        .map_err(|_| "store_binary_mismatch".to_string())?;
    Ok((snapshot, manifest))
}
fn regular(path: &Path, dir: bool) -> Result<(), String> {
    let meta = fs::symlink_metadata(path).map_err(|_| "store_source_unavailable")?;
    if meta.file_type().is_symlink() || (dir && !meta.is_dir()) || (!dir && !meta.is_file()) {
        return Err("store_source_unsafe".into());
    }
    Ok(())
}
pub(crate) struct Snapshot(pub(crate) PathBuf);
static NEXT_SNAPSHOT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
impl Snapshot {
    fn new() -> Result<Self, String> {
        let path = std::env::temp_dir().join(format!(
            "kyuubiki-install-snapshot-{}-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_err(|e| e.to_string())?
                .as_nanos(),
            NEXT_SNAPSHOT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        let mut builder = fs::DirBuilder::new();
        #[cfg(unix)]
        {
            use std::os::unix::fs::DirBuilderExt;
            builder.mode(0o700);
        }
        builder
            .create(&path)
            .map_err(|_| "store_snapshot_unavailable")?;
        Ok(Self(path))
    }
}
impl Drop for Snapshot {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

pub fn store_operator_command(args: Vec<String>, variant: bool) -> Result<String, String> {
    let (source, root) = match args.as_slice() {
        [source] => (PathBuf::from(source), crate::operator_package_store_root()?),
        [source, flag, root] if flag == "--store-root" => {
            (PathBuf::from(source), PathBuf::from(root))
        }
        _ => return Err("expected <files-root> [--store-root path]".into()),
    };
    let mut bytes = Vec::new();
    std::io::stdin()
        .take(16385)
        .read_to_end(&mut bytes)
        .map_err(|_| "store_request_unavailable")?;
    if bytes.len() > 16384 {
        return Err("store_request_limit".into());
    }
    let request = serde_json::from_slice(&bytes).map_err(|_| "store_request_invalid")?;
    let value = if variant {
        serde_json::to_value(crate::stage_store_operator_variant_into(
            &source, &root, &request,
        )?)
    } else {
        serde_json::to_value(install_store_operator_into(&source, &root, &request)?)
    }
    .map_err(|_| "store_receipt_invalid")?;
    serde_json::to_string(&value).map_err(|_| "store_receipt_invalid".into())
}
