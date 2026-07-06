use crate::{
    BuiltInOperatorRegistryKind, ExternalOperatorHostConfig,
    built_in_registry_with_external_packages,
};
use kyuubiki_operator_sdk::{
    OperatorPackageActivator, OperatorPackageLoadError, OperatorPackageLoadPlan, OperatorRegistry,
};
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

struct NoopActivator;

impl OperatorPackageActivator for NoopActivator {
    fn activate_package(
        &self,
        _plan: &OperatorPackageLoadPlan,
        _registry: &mut OperatorRegistry,
    ) -> Result<(), OperatorPackageLoadError> {
        Ok(())
    }
}

#[test]
fn host_policy_rejects_non_library_entrypoint_by_default() {
    let packages_root = temp_dir("external-host-script-entrypoint");
    let package_dir = packages_root.join("operator-script");
    fs::create_dir_all(package_dir.join("target/debug")).expect("create package dir");
    fs::write(
        package_dir.join("target/debug/operator_script.sh"),
        b"#!/bin/sh\nexit 0\n",
    )
    .expect("write script entrypoint");
    write_manifest(
        &package_dir,
        package_manifest(
            "operator.script",
            "extract.script",
            "target/debug/operator_script.sh",
            "register_script",
        ),
    );

    let error = reject_package(
        &packages_root,
        "host policy should reject script entrypoint",
    );

    assert!(
        error
            .to_string()
            .contains("current-platform dynamic library")
    );
    assert!(error.to_string().contains("operator.script"));
}

#[test]
fn host_policy_rejects_malformed_package_id() {
    let packages_root = temp_dir("external-host-bad-package-id");
    let package_dir = packages_root.join("operator-bad-id");
    fs::create_dir_all(&package_dir).expect("create package dir");
    write_manifest(
        &package_dir,
        package_manifest(
            "operator/bad",
            "extract.bad",
            "target/debug/{lib_prefix}operator_bad.{lib_extension}",
            "register_bad",
        ),
    );

    let error = reject_package(
        &packages_root,
        "host policy should reject malformed package id",
    );

    assert!(
        error
            .to_string()
            .contains("package_id contains unsupported characters")
    );
}

#[test]
fn host_policy_rejects_malformed_entry_symbol() {
    let packages_root = temp_dir("external-host-bad-symbol");
    let package_dir = packages_root.join("operator-bad-symbol");
    fs::create_dir_all(&package_dir).expect("create package dir");
    write_manifest(
        &package_dir,
        package_manifest(
            "operator.bad_symbol",
            "extract.bad_symbol",
            "target/debug/{lib_prefix}operator_bad_symbol.{lib_extension}",
            "register-bad-symbol",
        ),
    );

    let error = reject_package(
        &packages_root,
        "host policy should reject malformed entry symbol",
    );

    assert!(
        error
            .to_string()
            .contains("entry_symbol contains unsupported characters")
    );
}

fn package_manifest(
    package_id: &str,
    operator_id: &str,
    entrypoint: &str,
    entry_symbol: &str,
) -> serde_json::Value {
    serde_json::json!({
        "schema_version": kyuubiki_operator_sdk::OPERATOR_PACKAGE_SCHEMA_VERSION,
        "sdk_api_version": kyuubiki_operator_sdk::OPERATOR_SDK_API_VERSION,
        "package_id": package_id,
        "package_version": "0.1.0",
        "minimum_host_version": "1.15.0",
        "validation_status": "partial",
        "validation_notes": "Engine external-operator host security fixture.",
        "runtime": "rust_crate",
        "entrypoint": entrypoint,
        "operators": [
            {
                "operator_id": operator_id,
                "kind": "extract",
                "entry_symbol": entry_symbol
            }
        ]
    })
}

fn write_manifest(package_dir: &std::path::Path, manifest: serde_json::Value) {
    fs::write(
        package_dir.join(kyuubiki_operator_sdk::OPERATOR_PACKAGE_MANIFEST_FILE),
        manifest.to_string(),
    )
    .expect("write manifest");
}

fn reject_package(
    packages_root: &std::path::Path,
    panic_message: &str,
) -> crate::ExternalOperatorHostError {
    match built_in_registry_with_external_packages(
        &ExternalOperatorHostConfig::new(BuiltInOperatorRegistryKind::Extract, packages_root),
        &NoopActivator,
    ) {
        Ok(_) => panic!("{panic_message}"),
        Err(error) => error,
    }
}

fn temp_dir(label: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("kyuubiki-engine-{label}-{unique}"));
    fs::create_dir_all(&dir).expect("create temp dir");
    dir
}
