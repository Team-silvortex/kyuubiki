use super::{Result, StoreDownloads, catalog::host_platform, owner, storage, transport, types::*};
use crate::native_installer::binary as installer_binary;
use crate::store_identity::StoreIdentity;
use kyuubiki_protocol::{StoreInstallFile, StoreOperatorInstallRequest};
use serde_json::Value;
use std::path::Path;
#[cfg(not(test))]
use std::path::PathBuf;

impl StoreDownloads {
    pub(super) fn install(&self, identity: &StoreIdentity, id: &str) -> Result<()> {
        let session = identity.store_lease()?;
        let store = storage::Store::open(self.root()?)?;
        let mut record = store.read(id)?;
        owner(&record, &session)?;
        let request = handoff(&record)?;
        // Recheck recall/signature and immutable bytes before every installation, without a grant POST.
        let (files, _, _, _) = transport::preflight(&session, &record.selection, true)?;
        if files != record.files {
            return Err("artifact_mismatch");
        }
        #[cfg(not(test))]
        let (binary, install_root) = (installer_binary()?, None::<PathBuf>);
        #[cfg(test)]
        let (binary, install_root) = match &self.test_installer {
            Some((binary, root)) => (binary.clone(), Some(root.clone())),
            None => (installer_binary()?, None),
        };
        let root = store.path(id)?.join("files");
        storage::regular(&root, true)?;
        session.check()?;
        record.installation = Some("installing".into());
        record.error = None;
        store.save(&record)?;
        let result = invoke_installer(&binary, &root, &request, install_root.as_deref())
            .and_then(|value| verify_receipt(&record, value));
        // A logout cannot undo an already committed local installation. Persist its outcome for
        // the original owner, but the caller must authenticate again before viewing it.
        record.installation = Some(
            if result.is_ok() {
                "installed"
            } else {
                "install_failed"
            }
            .into(),
        );
        record.error = result.as_ref().err().map(|e| (*e).into());
        store.save(&record)?;
        result
    }
}
pub(super) fn handoff(record: &Record) -> Result<StoreOperatorInstallRequest> {
    if record.state != "downloaded" || record.selection.kind != "operator" {
        return Err("operator_download_required");
    }
    if record.selection.platform != host_platform() {
        return Err("store_platform_mismatch");
    }
    let expected = match record.selection.platform.as_str() {
        "macos-aarch64" | "macos-x86_64" => "operator.dylib",
        "linux-aarch64" | "linux-x86_64" => "operator.so",
        "windows-x86_64" => "operator.dll",
        _ => return Err("store_platform_mismatch"),
    };
    if record.files.len() != 2
        || ["kyuubiki-operator.json", expected].iter().any(|name| {
            record
                .files
                .iter()
                .filter(|f| f.path == format!("{}/{name}", record.selection.platform))
                .count()
                != 1
        })
    {
        return Err("store_package_layout_unsupported");
    }
    Ok(StoreOperatorInstallRequest {
        schema_version: "kyuubiki.store-operator-install/v1".into(),
        package_id: record.selection.asset_id.clone(),
        package_version: record.selection.version.clone(),
        platform: record.selection.platform.clone(),
        files: record
            .files
            .iter()
            .map(|f| StoreInstallFile {
                path: f.path.clone(),
                sha256: f.sha256.clone(),
                size_bytes: f.size_bytes,
            })
            .collect(),
    })
}
pub(super) fn invoke_installer(
    binary: &Path,
    source: &Path,
    request: &StoreOperatorInstallRequest,
    install_root: Option<&Path>,
) -> Result<Value> {
    crate::native_installer::invoke(
        binary,
        "install-store-operator",
        Some(source),
        &serde_json::to_value(request).map_err(|_| "invalid_install_request")?,
        install_root,
    )
}
pub(super) fn verify_receipt(record: &Record, value: Value) -> Result<()> {
    let matches =
        |key: &str, expected: &str| value.get(key).and_then(Value::as_str) == Some(expected);
    let entry = value
        .get("entrypoint_relative_path")
        .and_then(Value::as_str)
        .ok_or("invalid_install_receipt")?;
    if !["operator.dylib", "operator.so", "operator.dll"].contains(&entry) {
        return Err("invalid_install_receipt");
    }
    let file = record
        .files
        .iter()
        .find(|f| f.path == format!("{}/{entry}", record.selection.platform))
        .ok_or("invalid_install_receipt")?;
    let manifest = record
        .files
        .iter()
        .find(|f| f.path == format!("{}/kyuubiki-operator.json", record.selection.platform))
        .ok_or("invalid_install_receipt")?;
    if !matches(
        "schema_version",
        "kyuubiki.managed-operator-package-receipt/v1",
    ) || !matches("package_id", &record.selection.asset_id)
        || !matches("package_version", &record.selection.version)
        || !matches("sdk_api_version", "kyuubiki.operator-sdk/v1")
        || !matches("execution_abi", "kyuubiki.operator-json-c/v1")
        || !matches("runtime", "rust_crate")
        || !matches(
            "relative_root",
            &format!("packages/{}", record.selection.asset_id),
        )
        || !matches("manifest_sha256", &manifest.sha256)
        || !matches("entrypoint_sha256", &file.sha256)
        || value.get("entrypoint_size_bytes").and_then(Value::as_u64) != Some(file.size_bytes)
    {
        return Err("invalid_install_receipt");
    }
    Ok(())
}
