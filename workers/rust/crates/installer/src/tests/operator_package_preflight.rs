use std::fs;
use std::path::{Path, PathBuf};

use crate::{
    operator_package_preflight, operator_package_preflight_json,
    write_operator_package_preflight_json,
};

#[test]
fn operator_package_preflight_json_reports_admission_summary() {
    let root = unique_temp_dir("operator-package-preflight");
    write_operator_package_manifest(
        &root,
        "accepted",
        "lab.accepted",
        "1.15.0",
        "target/liblab_accepted.dylib",
    );
    write_operator_package_manifest(
        &root,
        "future",
        "lab.future",
        "9.0.0",
        "target/liblab_future.dylib",
    );

    let report = operator_package_preflight_json(&root).unwrap();
    let payload: serde_json::Value = serde_json::from_str(&report).unwrap();
    assert_eq!(
        payload["schema_version"],
        "kyuubiki.operator-package-preflight/v1"
    );
    assert_eq!(payload["registry_kind"], "extract");
    assert_eq!(payload["safety"]["loads_dynamic_libraries"], false);
    assert_eq!(payload["accepted_package_count"], 1);
    assert_eq!(payload["rejected_package_count"], 1);
    assert_eq!(
        payload["accepted_packages"][0]["package_id"],
        "lab.accepted"
    );
    assert_eq!(
        payload["accepted_packages"][0]["validation_status"],
        "partial"
    );
    assert_eq!(payload["rejected_packages"][0]["package_id"], "lab.future");
    assert!(
        payload["rejected_packages"][0]["reason"]
            .as_str()
            .unwrap()
            .contains("minimum_host_version")
    );

    let _ = fs::remove_dir_all(root);
}

#[test]
fn operator_package_preflight_outcome_can_gate_rejected_packages() {
    let root = unique_temp_dir("operator-package-preflight-gate");
    write_operator_package_manifest(
        &root,
        "accepted",
        "lab.accepted",
        "1.15.0",
        "target/liblab_accepted.dylib",
    );
    write_operator_package_manifest(
        &root,
        "future",
        "lab.future",
        "9.0.0",
        "target/liblab_future.dylib",
    );

    let outcome = operator_package_preflight(&root).unwrap();
    assert_eq!(outcome.accepted_package_count, 1);
    assert_eq!(outcome.rejected_package_count, 1);
    assert!(
        outcome
            .ensure_no_rejections()
            .unwrap_err()
            .contains("rejected 1 package")
    );

    let _ = fs::remove_dir_all(root);
}

#[test]
fn operator_package_preflight_json_can_be_written_to_report_file() {
    let root = unique_temp_dir("operator-package-preflight-out");
    write_operator_package_manifest(
        &root,
        "accepted",
        "lab.accepted",
        "1.15.0",
        "target/liblab_accepted.dylib",
    );
    let output_path = root.join("reports").join("preflight.json");

    let written_path = write_operator_package_preflight_json(&root, &output_path).unwrap();
    let payload: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&written_path).unwrap()).unwrap();
    assert_eq!(written_path, output_path);
    assert_eq!(payload["accepted_package_count"], 1);
    assert_eq!(payload["rejected_package_count"], 0);
    assert_eq!(
        payload["accepted_packages"][0]["package_id"],
        "lab.accepted"
    );

    let _ = fs::remove_dir_all(root);
}

fn unique_temp_dir(name: &str) -> PathBuf {
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!("kyuubiki-{name}-{nonce}"));
    fs::create_dir_all(&path).unwrap();
    path
}

fn write_operator_package_manifest(
    root: &Path,
    directory: &str,
    package_id: &str,
    minimum_host_version: &str,
    entrypoint: &str,
) {
    let package_root = root.join(directory);
    fs::create_dir_all(&package_root).unwrap();
    let manifest = format!(
        r#"{{
  "schema_version": "kyuubiki.operator-package/v1",
  "sdk_api_version": "kyuubiki.operator-sdk/v1",
  "package_id": "{package_id}",
  "package_version": "0.1.0",
  "minimum_host_version": "{minimum_host_version}",
  "validation_status": "partial",
  "validation_notes": "fixture package for installer preflight tests",
  "runtime": "rust_crate",
  "entrypoint": "{entrypoint}",
  "operators": [
    {{
      "operator_id": "{package_id}.solve",
      "kind": "solver",
      "entry_symbol": "register_operator"
    }}
  ]
}}"#
    );
    fs::write(package_root.join("kyuubiki-operator.json"), manifest).unwrap();
}
