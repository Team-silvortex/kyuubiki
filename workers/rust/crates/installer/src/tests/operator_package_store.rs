use crate::{
    MANAGED_OPERATOR_PACKAGE_RECEIPT_SCHEMA_VERSION, install_operator_package_into,
    managed_operator_package_status_in, uninstall_operator_package_from,
    verify_managed_operator_package,
};
use kyuubiki_operator_sdk::OPERATOR_PACKAGE_MANIFEST_FILE;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn installs_idempotently_verifies_and_prunes_operator_package_store() {
    let root = temp_dir("operator-package-store-lifecycle");
    let _cleanup = Cleanup(root.clone());
    let source = root.join("source");
    let store = root.join("managed");
    write_package(&source, "operator.store.alpha", "0.1.0", b"operator-alpha");

    let installed = install_operator_package_into(&source, &store).expect("install package");
    let repeated = install_operator_package_into(&source, &store).expect("idempotent install");
    assert_eq!(installed, repeated);
    assert_eq!(
        installed.schema_version,
        MANAGED_OPERATOR_PACKAGE_RECEIPT_SCHEMA_VERSION
    );
    let package_root = store.join(&installed.relative_root);
    assert_eq!(
        verify_managed_operator_package(&package_root).expect("verify installed package"),
        installed
    );
    let status = managed_operator_package_status_in(&store).expect("read package status");
    assert_eq!(status.installed_package_count, 1);
    assert_eq!(status.installed_packages, vec![installed.clone()]);

    let removal = uninstall_operator_package_from(&store, &installed.package_id)
        .expect("uninstall managed package");
    assert!(removal.removed);
    assert_eq!(removal.receipt_verified, Some(true));
    assert!(removal.receipt_error.is_none());
    assert!(removal.store_pruned);
    assert!(!store.exists());
}

#[test]
fn uninstalling_absent_package_is_idempotent_and_leaves_no_store() {
    let root = temp_dir("operator-package-store-absent-removal");
    let _cleanup = Cleanup(root.clone());
    let store = root.join("managed");

    let removal = uninstall_operator_package_from(&store, "operator.store.absent")
        .expect("remove absent managed package");

    assert!(!removal.removed);
    assert_eq!(removal.receipt_verified, None);
    assert!(removal.store_pruned);
    assert!(!store.exists());
}

#[test]
fn uninstall_recovers_from_corrupt_receipt_and_reports_it() {
    let root = temp_dir("operator-package-store-corrupt-receipt");
    let _cleanup = Cleanup(root.clone());
    let source = root.join("source");
    let store = root.join("managed");
    write_package(
        &source,
        "operator.store.corrupt",
        "0.1.0",
        b"operator-corrupt",
    );
    let receipt = install_operator_package_into(&source, &store).expect("install package");
    fs::write(
        store
            .join(&receipt.relative_root)
            .join("kyuubiki-managed-install.json"),
        b"{broken",
    )
    .expect("corrupt managed receipt");

    let removal = uninstall_operator_package_from(&store, &receipt.package_id)
        .expect("remove package with corrupt receipt");

    assert!(removal.removed);
    assert_eq!(removal.receipt_verified, Some(false));
    assert!(removal.receipt_error.is_some());
    assert!(removal.store_pruned);
    assert!(!store.exists());
}

#[test]
fn uninstall_recovers_from_partial_file_target() {
    let root = temp_dir("operator-package-store-partial-target");
    let _cleanup = Cleanup(root.clone());
    let store = root.join("managed");
    fs::create_dir_all(store.join("packages")).expect("create partial store");
    fs::write(store.join("packages/operator.store.partial"), b"partial")
        .expect("write partial target");

    let removal = uninstall_operator_package_from(&store, "operator.store.partial")
        .expect("remove partial package target");

    assert!(removal.removed);
    assert_eq!(removal.receipt_verified, Some(false));
    assert!(removal.store_pruned);
    assert!(!store.exists());
}

#[test]
fn rejects_path_semantic_and_windows_reserved_package_ids() {
    let root = temp_dir("operator-package-store-unsafe-id");
    let _cleanup = Cleanup(root.clone());
    let store = root.join("managed");

    for package_id in [".", "..", ".hidden", "trailing.", "CON", "com1.plugin"] {
        let error = uninstall_operator_package_from(&store, package_id)
            .expect_err("unsafe package id must fail before store creation");
        assert!(error.contains("safe portable path component"));
    }
    assert!(!store.exists());
}

#[test]
fn rejects_different_content_for_an_installed_package_identity() {
    let root = temp_dir("operator-package-store-replacement");
    let _cleanup = Cleanup(root.clone());
    let first = root.join("first");
    let second = root.join("second");
    let store = root.join("managed");
    write_package(&first, "operator.store.beta", "0.1.0", b"first");
    write_package(&second, "operator.store.beta", "0.1.0", b"second");
    install_operator_package_into(&first, &store).expect("install first package");

    let error = install_operator_package_into(&second, &store)
        .expect_err("different package content must not overwrite active package");

    assert!(error.contains("already installed with different content"));
}

#[test]
fn detects_tampering_in_managed_entrypoint() {
    let root = temp_dir("operator-package-store-tamper");
    let _cleanup = Cleanup(root.clone());
    let source = root.join("source");
    let store = root.join("managed");
    write_package(&source, "operator.store.gamma", "0.1.0", b"trusted");
    let receipt = install_operator_package_into(&source, &store).expect("install package");
    let package_root = store.join(&receipt.relative_root);
    fs::write(
        package_root.join(&receipt.entrypoint_relative_path),
        b"tampered",
    )
    .expect("tamper managed entrypoint");

    let error = verify_managed_operator_package(&package_root)
        .expect_err("tampered package must fail verification");

    assert!(error.contains("entrypoint integrity mismatch"));
}

#[test]
fn detects_unexpected_managed_package_files() {
    let root = temp_dir("operator-package-store-extra-file");
    let _cleanup = Cleanup(root.clone());
    let source = root.join("source");
    let store = root.join("managed");
    write_package(&source, "operator.store.extra", "0.1.0", b"trusted");
    let receipt = install_operator_package_into(&source, &store).expect("install package");
    let package_root = store.join(&receipt.relative_root);
    fs::write(package_root.join("unexpected.bin"), b"unexpected").expect("add unexpected file");

    let error = verify_managed_operator_package(&package_root)
        .expect_err("unexpected managed content must fail verification");

    assert!(error.contains("unexpected file"));
    let removal = uninstall_operator_package_from(&store, &receipt.package_id)
        .expect("remove package with unexpected content");
    assert!(removal.removed);
    assert!(removal.store_pruned);
}

#[cfg(unix)]
#[test]
fn rejects_source_entrypoint_symlink() {
    use std::os::unix::fs::symlink;

    let root = temp_dir("operator-package-store-symlink");
    let _cleanup = Cleanup(root.clone());
    let source = root.join("source");
    let store = root.join("managed");
    write_package(&source, "operator.store.link", "0.1.0", b"inside");
    let entrypoint = source.join(entrypoint_relative());
    let outside = root.join(dynamic_library_name("outside"));
    fs::write(&outside, b"outside").expect("write outside entrypoint");
    fs::remove_file(&entrypoint).expect("remove regular entrypoint");
    symlink(&outside, &entrypoint).expect("create entrypoint symlink");

    let error = install_operator_package_into(&source, &store)
        .expect_err("symlink entrypoint must be rejected");

    assert!(error.contains("entrypoint must not be a symlink"));
}

fn write_package(root: &Path, package_id: &str, version: &str, payload: &[u8]) {
    let entrypoint = root.join(entrypoint_relative());
    fs::create_dir_all(entrypoint.parent().expect("entrypoint parent"))
        .expect("create package entrypoint dir");
    fs::write(&entrypoint, payload).expect("write package entrypoint");
    fs::write(
        root.join(OPERATOR_PACKAGE_MANIFEST_FILE),
        serde_json::json!({
            "schema_version": kyuubiki_operator_sdk::OPERATOR_PACKAGE_SCHEMA_VERSION,
            "sdk_api_version": kyuubiki_operator_sdk::OPERATOR_SDK_API_VERSION,
            "execution_abi": kyuubiki_operator_sdk::OPERATOR_JSON_ABI_SCHEMA_VERSION,
            "package_id": package_id,
            "package_version": version,
            "minimum_host_version": "1.15.0",
            "validation_status": "partial",
            "validation_notes": "Installer managed package lifecycle fixture.",
            "runtime": "rust_crate",
            "entrypoint": "target/debug/{lib_prefix}managed_operator.{lib_extension}",
            "operators": [{
                "operator_id": "extract.managed_operator",
                "kind": "extract",
                "entry_symbol": "run_managed_operator_json"
            }]
        })
        .to_string(),
    )
    .expect("write package manifest");
}

fn entrypoint_relative() -> PathBuf {
    PathBuf::from("target/debug").join(dynamic_library_name("managed_operator"))
}

fn dynamic_library_name(stem: &str) -> String {
    format!(
        "{}{stem}{}",
        std::env::consts::DLL_PREFIX,
        std::env::consts::DLL_SUFFIX
    )
}

fn temp_dir(label: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    std::env::temp_dir().join(format!("kyuubiki-{label}-{nonce}"))
}

struct Cleanup(PathBuf);

impl Drop for Cleanup {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
