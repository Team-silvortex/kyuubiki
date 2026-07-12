use std::fs;
use std::path::{Path, PathBuf};

use kyuubiki_engine::{
    BuiltInOperatorRegistryKind, ExternalOperatorHostConfig, preflight_external_operator_packages,
};
use serde_json::{Value, json};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperatorPackagePreflightOutcome {
    pub json: String,
    pub accepted_package_count: usize,
    pub rejected_package_count: usize,
}

impl OperatorPackagePreflightOutcome {
    pub fn ensure_no_rejections(&self) -> Result<(), String> {
        if self.rejected_package_count == 0 {
            return Ok(());
        }
        Err(format!(
            "operator package preflight rejected {} package(s)",
            self.rejected_package_count
        ))
    }
}

pub fn operator_package_preflight(
    packages_root: &Path,
) -> Result<OperatorPackagePreflightOutcome, String> {
    let config =
        ExternalOperatorHostConfig::new(BuiltInOperatorRegistryKind::Extract, packages_root);
    let report =
        preflight_external_operator_packages(&config).map_err(|error| error.to_string())?;
    let accepted_package_count = report.accepted_packages.len();
    let rejected_package_count = report.rejected_packages.len();
    let payload = json!({
        "schema_version": "kyuubiki.operator-package-preflight/v1",
        "registry_kind": registry_kind_label(report.registry_kind),
        "packages_root": report.packages_root,
        "host_version": report.host_version,
        "accepted_package_count": accepted_package_count,
        "rejected_package_count": rejected_package_count,
        "accepted_packages": report.accepted_packages.into_iter().map(|package| {
            json!({
                "package_id": package.package_id,
                "package_version": package.package_version,
                "sdk_api_version": package.sdk_api_version,
                "minimum_host_version": package.minimum_host_version,
                "validation_status": validation_status_value(package.validation_status),
                "validation_notes": package.validation_notes,
                "runtime": package.runtime,
                "operator_ids": package.operator_ids,
                "entrypoint_path": package.entrypoint_path,
            })
        }).collect::<Vec<_>>(),
        "rejected_packages": report.rejected_packages.into_iter().map(|package| {
            json!({
                "package_id": package.package_id,
                "manifest_path": package.manifest_path,
                "reason": package.reason,
            })
        }).collect::<Vec<_>>(),
        "package_readiness": report.package_readiness.into_iter().map(|package| {
            json!({
                "package_id": package.package_id,
                "manifest_path": package.manifest_path,
                "ok": package.ok,
                "issues": package.issues,
            })
        }).collect::<Vec<_>>(),
        "safety": {
            "mode": "read_only_preflight",
            "loads_dynamic_libraries": false,
            "mutates_runtime_registry": false,
        }
    });
    let json = serde_json::to_string_pretty(&payload).map_err(|error| error.to_string())?;
    Ok(OperatorPackagePreflightOutcome {
        json,
        accepted_package_count,
        rejected_package_count,
    })
}

pub fn operator_package_preflight_json(packages_root: &Path) -> Result<String, String> {
    operator_package_preflight(packages_root).map(|outcome| outcome.json)
}

pub fn write_operator_package_preflight_json(
    packages_root: &Path,
    output_path: &Path,
) -> Result<PathBuf, String> {
    let outcome = operator_package_preflight(packages_root)?;
    write_operator_package_preflight_outcome(&outcome, output_path)
}

pub fn write_operator_package_preflight_outcome(
    outcome: &OperatorPackagePreflightOutcome,
    output_path: &Path,
) -> Result<PathBuf, String> {
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
    }
    fs::write(output_path, format!("{}\n", outcome.json))
        .map_err(|error| format!("failed to write {}: {error}", output_path.display()))?;
    Ok(output_path.to_path_buf())
}

fn registry_kind_label(kind: BuiltInOperatorRegistryKind) -> &'static str {
    match kind {
        BuiltInOperatorRegistryKind::Extract => "extract",
        BuiltInOperatorRegistryKind::Export => "export",
        BuiltInOperatorRegistryKind::Transform => "transform",
    }
}

fn validation_status_value(status: impl serde::Serialize) -> Value {
    serde_json::to_value(status).expect("operator validation status should serialize")
}
