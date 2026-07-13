use serde::Serialize;

pub const INSTALLER_HEADLESS_SURFACE_SCHEMA_VERSION: &str =
    "kyuubiki.installer-headless-surface/v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct InstallerHeadlessSurfaceManifest {
    pub schema_version: &'static str,
    pub package: &'static str,
    pub crate_name: &'static str,
    pub entrypoints: Vec<InstallerHeadlessEntrypoint>,
    pub runtime_api: InstallerHeadlessRuntimeApi,
    pub workflow_compositions: Vec<InstallerWorkflowComposition>,
    pub benchmark_lanes: Vec<InstallerBenchmarkLane>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct InstallerHeadlessEntrypoint {
    pub id: &'static str,
    pub role: &'static str,
    pub exports: &'static [&'static str],
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct InstallerHeadlessRuntimeApi {
    pub schema_version: &'static str,
    pub api_owner: &'static str,
    pub stable_exports: &'static [&'static str],
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct InstallerWorkflowComposition {
    pub id: &'static str,
    pub stages: &'static [&'static str],
    pub evidence_export: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct InstallerBenchmarkLane {
    pub id: &'static str,
    pub target_workflow: &'static str,
    pub measured_stages: &'static [&'static str],
}

pub fn installer_headless_surface_manifest() -> InstallerHeadlessSurfaceManifest {
    InstallerHeadlessSurfaceManifest {
        schema_version: INSTALLER_HEADLESS_SURFACE_SCHEMA_VERSION,
        package: "workers/rust/crates/installer",
        crate_name: "kyuubiki-installer",
        entrypoints: vec![
            InstallerHeadlessEntrypoint {
                id: "integrity",
                role: "Component integrity, residue cleanup, and visible install rules.",
                exports: &[
                    "installation_integrity_report",
                    "repair_installation",
                    "component_integrity_protocol_report",
                ],
            },
            InstallerHeadlessEntrypoint {
                id: "remote_deploy",
                role: "SSH-shaped remote deployment planning, dry-run, journal, and artifacts.",
                exports: &[
                    "default_remote_deployment_plan",
                    "remote_deployment_dry_run",
                    "remote_deployment_journal_for_plan",
                    "remote_artifact_delivery_manifest",
                ],
            },
            InstallerHeadlessEntrypoint {
                id: "updates",
                role: "Unified update source, staged download, apply, and catalog preview.",
                exports: &[
                    "unified_update_preview",
                    "download_update",
                    "apply_downloaded_update",
                    "read_update_source_config",
                ],
            },
            InstallerHeadlessEntrypoint {
                id: "credential_storage",
                role: "Sandbox-first credential storage contract and platform backend policy.",
                exports: &["credential_storage_contract", "credential_sandbox_root"],
            },
            InstallerHeadlessEntrypoint {
                id: "embedded_runtime",
                role: "Self-host runtime inventory and release-time embedded runtime manifest.",
                exports: &[
                    "embedded_runtime_report",
                    "build_embedded_runtime_manifest",
                    "linux_desktop_dependency_plan",
                ],
            },
        ],
        runtime_api: InstallerHeadlessRuntimeApi {
            schema_version: "kyuubiki.installer-runtime-api/v1",
            api_owner: "runtime_installer",
            stable_exports: &[
                "installation_integrity_report",
                "remote_deployment_dry_run",
                "unified_update_preview",
                "credential_storage_contract",
                "embedded_runtime_report",
            ],
        },
        workflow_compositions: vec![
            InstallerWorkflowComposition {
                id: "standard_remote_deploy",
                stages: &[
                    "trust_host",
                    "plan_remote_deployment",
                    "dry_run",
                    "deliver_artifacts",
                    "journal_plan",
                ],
                evidence_export: "remote_deployment_journal_for_plan",
            },
            InstallerWorkflowComposition {
                id: "standard_update_apply",
                stages: &[
                    "read_update_source",
                    "preview_update",
                    "download_update",
                    "apply_downloaded_update",
                    "record_update",
                ],
                evidence_export: "latest_applied_update_record",
            },
            InstallerWorkflowComposition {
                id: "standard_integrity_repair",
                stages: &[
                    "scan_integrity",
                    "classify_residue",
                    "repair_installation",
                    "emit_component_protocol",
                ],
                evidence_export: "installation_integrity_report",
            },
        ],
        benchmark_lanes: vec![
            InstallerBenchmarkLane {
                id: "installer_release",
                target_workflow: "standard_update_apply",
                measured_stages: &[
                    "preview_update",
                    "download_update",
                    "apply_downloaded_update",
                ],
            },
            InstallerBenchmarkLane {
                id: "remote_deploy",
                target_workflow: "standard_remote_deploy",
                measured_stages: &["plan_remote_deployment", "dry_run", "deliver_artifacts"],
            },
            InstallerBenchmarkLane {
                id: "integrity_repair",
                target_workflow: "standard_integrity_repair",
                measured_stages: &["scan_integrity", "repair_installation"],
            },
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::{INSTALLER_HEADLESS_SURFACE_SCHEMA_VERSION, installer_headless_surface_manifest};

    #[test]
    fn installer_headless_surface_manifest_is_serializable() {
        let manifest = installer_headless_surface_manifest();

        assert_eq!(
            manifest.schema_version,
            INSTALLER_HEADLESS_SURFACE_SCHEMA_VERSION
        );
        assert_eq!(manifest.crate_name, "kyuubiki-installer");
        assert!(manifest.entrypoints.iter().any(|entry| {
            entry.id == "remote_deploy" && entry.exports.contains(&"remote_deployment_dry_run")
        }));
        assert!(manifest.entrypoints.iter().any(|entry| {
            entry.id == "credential_storage"
                && entry.exports.contains(&"credential_storage_contract")
        }));
        assert!(
            manifest
                .runtime_api
                .stable_exports
                .contains(&"remote_deployment_dry_run")
        );
        assert!(manifest.workflow_compositions.iter().any(|workflow| {
            workflow.id == "standard_update_apply"
                && workflow.stages.contains(&"apply_downloaded_update")
        }));
        assert!(manifest.benchmark_lanes.iter().any(|lane| {
            lane.id == "remote_deploy" && lane.measured_stages.contains(&"dry_run")
        }));
        serde_json::to_value(manifest).expect("installer headless manifest should serialize");
    }
}
