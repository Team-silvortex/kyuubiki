use super::*;
use std::fs;

struct Sandbox(PathBuf);
impl Sandbox {
    fn new() -> Self {
        let mut bytes = [0; 32];
        SystemRandom::new().fill(&mut bytes).unwrap();
        Self(std::env::temp_dir().join(format!(
            "kyuubiki-store-storage-{:x}",
            Sha256::digest(bytes)
        )))
    }
}
impl Drop for Sandbox {
    fn drop(&mut self) {
        if self.0.exists() {
            fs::remove_dir_all(&self.0).unwrap();
        }
    }
}
fn record() -> Record {
    serde_json::from_value(serde_json::json!({
        "format":"kyuubiki.store-download/v1","id":"a".repeat(64),"issuer":"https://issuer.test","subject":"researcher",
        "selection":{"kind":"operator","asset_id":"demo","version":"1.0.0","platform":"macos-aarch64","key_id":"test","key_sha256":"b".repeat(64)},
        "files":[{"path":"macos-aarch64/manifest.json","sha256":"c".repeat(64),"size_bytes":10,"media_type":"application/json",
            "signature":{"algorithm":"RS256","scheme":"silvortex-sha256-rs256-v1","signed_payload":"sha256_ascii","key_id":"test","value":"test-only"}}],
        "quota":{"limit":1,"used":0,"remaining":1,"unlimited":false,"reset_at":1},"metered":true,
        "state":"prepared","attempted":false,"replay_until":null,"grant_id":null,"outcome":null,"error":null
    })).unwrap()
}
#[test]
fn durable_record_replace_lock_and_unknown_outcome_removal_guard() {
    let disk = Sandbox::new();
    let store = storage::Store::open(disk.0.clone()).unwrap();
    assert_eq!(
        storage::Store::open(disk.0.clone())
            .err()
            .map(|e| e.to_owned()),
        Some("download_busy".into())
    );
    let mut receipt = record();
    store.save(&receipt).unwrap();
    receipt.attempted = true;
    receipt.replay_until = Some(now() + 600);
    receipt.state = "authorizing".into();
    store.save(&receipt).unwrap();
    assert!(store.read(&receipt.id).unwrap().attempted);
    assert!(!disk.0.join(&receipt.id).join("record.next").exists());
    assert_eq!(store.remove(&receipt), Err("authorization_outcome_unknown"));
    drop(store);
    let reopened = storage::Store::open(disk.0.clone()).unwrap();
    assert_eq!(reopened.read(&receipt.id).unwrap().state, "authorizing");
    let previous = disk.0.join(&receipt.id).join("files");
    storage::directory(&previous).unwrap();
    fs::write(previous.join("old"), b"uncommitted staged file").unwrap();
    reopened.reset_partial(&receipt.id).unwrap();
    assert!(
        !previous.exists(),
        "a retry must not retain a second package copy"
    );
}

#[test]
fn active_download_never_publishes_terminal_before_cleanup_unlock() {
    let disk = Sandbox::new();
    let mut record = record();
    let active = Active {
        id: record.id.clone(),
        cancel: Arc::new(AtomicBool::new(false)),
        received: Arc::new(AtomicU64::new(10)),
    };
    for state in ["downloaded", "failed", "cancelled"] {
        record.state = state.into();
        record.error = Some("download_connection_failed".into());
        let busy = download_view(&record, Some(&active), &disk.0);
        assert_eq!(busy.state, "downloading");
        assert!(busy.error.is_none() && busy.directory.is_none());
        assert_eq!(download_view(&record, None, &disk.0).state, state);
    }
    active.cancel.store(true, Ordering::Relaxed);
    assert_eq!(
        download_view(&record, Some(&active), &disk.0).state,
        "cancelling"
    );
}
#[test]
fn paths_sizes_and_file_directory_aliases_are_bounded() {
    let receipt = record();
    let file = &receipt.files[0];
    for path in ["a/../b", "a//b", "a/", "a/COM1.json", "/etc/passwd", "a\\b"] {
        let mut invalid = file.clone();
        invalid.path = path.into();
        assert!(validate_files(&[invalid]).is_err());
    }
    let mut alias = file.clone();
    alias.path = file.path.to_ascii_uppercase();
    assert!(validate_files(&[file.clone(), alias]).is_err());
    let mut alias = file.clone();
    alias.path = "macos-aarch64".into();
    assert!(validate_files(&[file.clone(), alias]).is_err());
    let mut large = file.clone();
    large.size_bytes = MAX_FILE + 1;
    assert!(validate_files(&[large]).is_err());
}

#[test]
fn installation_handoff_requires_complete_download_and_exact_receipt() {
    let mut record = record();
    assert_eq!(
        install::handoff(&record).err(),
        Some("operator_download_required")
    );
    record.state = "downloaded".into();
    record.selection.platform = catalog::host_platform();
    let name = if cfg!(target_os = "macos") {
        "operator.dylib"
    } else if cfg!(windows) {
        "operator.dll"
    } else {
        "operator.so"
    };
    record.files[0].path = format!("{}/kyuubiki-operator.json", record.selection.platform);
    assert_eq!(
        install::handoff(&record).err(),
        Some("store_package_layout_unsupported")
    );
    let mut binary = record.files[0].clone();
    binary.path = format!("{}/{name}", record.selection.platform);
    record.files.push(binary);
    assert_eq!(install::handoff(&record).unwrap().files.len(), 2);
    let receipt = serde_json::json!({"schema_version":"kyuubiki.managed-operator-package-receipt/v1", "package_id":"demo","package_version":"1.0.0",
        "sdk_api_version":"kyuubiki.operator-sdk/v1","execution_abi":"kyuubiki.operator-json-c/v1","runtime":"rust_crate","relative_root":"packages/demo",
        "entrypoint_relative_path":name,"manifest_sha256":"c".repeat(64),"entrypoint_sha256":"c".repeat(64),"entrypoint_size_bytes":10});
    assert_eq!(install::verify_receipt(&record, receipt.clone()), Ok(()));
    let mut alias = receipt.clone();
    alias["entrypoint_relative_path"] = serde_json::json!("kyuubiki-operator.json");
    assert_eq!(
        install::verify_receipt(&record, alias),
        Err("invalid_install_receipt")
    );
    for key in [
        "package_id",
        "package_version",
        "manifest_sha256",
        "entrypoint_sha256",
        "relative_root",
        "entrypoint_relative_path",
        "execution_abi",
    ] {
        let mut bad = receipt.clone();
        bad[key] = serde_json::json!("different");
        assert_eq!(
            install::verify_receipt(&record, bad),
            Err("invalid_install_receipt")
        );
    }
}
#[cfg(unix)]
#[test]
fn symlinked_record_or_partial_cannot_escape_managed_directory() {
    use std::os::unix::fs::{PermissionsExt, symlink};
    let disk = Sandbox::new();
    let outside = Sandbox::new();
    fs::create_dir(&outside.0).unwrap();
    let sentinel = outside.0.join("keep");
    fs::write(&sentinel, b"do not touch").unwrap();
    let store = storage::Store::open(disk.0.clone()).unwrap();
    let receipt = record();
    store.save(&receipt).unwrap();
    assert_eq!(
        fs::metadata(&disk.0).unwrap().permissions().mode() & 0o777,
        0o700
    );
    assert_eq!(
        fs::metadata(disk.0.join(&receipt.id).join("record.json"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o600
    );
    let partial = disk.0.join(&receipt.id).join("partial");
    symlink(&outside.0, &partial).unwrap();
    assert_eq!(
        store.clear_partial(&receipt.id),
        Err("unsafe_download_storage")
    );
    assert!(store.reset_partial(&receipt.id).is_err());
    fs::remove_file(&partial).unwrap();
    let metadata = disk.0.join(&receipt.id).join("record.json");
    fs::remove_file(&metadata).unwrap();
    symlink(&sentinel, &metadata).unwrap();
    assert_eq!(
        store.read(&receipt.id).err(),
        Some("unsafe_download_storage")
    );
    assert!(store.save(&receipt).is_err());
    assert_eq!(fs::read(&sentinel).unwrap(), b"do not touch");
}
