use crate::{
    OPERATOR_PACKAGE_MANIFEST_FILE, OPERATOR_PACKAGE_SCHEMA_VERSION, OPERATOR_SDK_API_VERSION,
};
use serde::Serialize;

pub const OPERATOR_SDK_SURFACE_SCHEMA_VERSION: &str = "kyuubiki.operator-sdk-surface/v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct OperatorSdkSurfaceManifest {
    pub schema_version: &'static str,
    pub package: &'static str,
    pub crate_name: &'static str,
    pub sdk_api_version: &'static str,
    pub language: &'static str,
    pub purpose: &'static str,
    pub manifest_file: &'static str,
    pub package_schema_version: &'static str,
    pub areas: Vec<OperatorSdkSurfaceArea>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct OperatorSdkSurfaceArea {
    pub id: &'static str,
    pub title: &'static str,
    pub role: &'static str,
    pub modules: &'static [&'static str],
    pub anchor_exports: &'static [&'static str],
}

pub fn operator_sdk_surface_manifest() -> OperatorSdkSurfaceManifest {
    OperatorSdkSurfaceManifest {
        schema_version: OPERATOR_SDK_SURFACE_SCHEMA_VERSION,
        package: "workers/rust/crates/operator-sdk",
        crate_name: "kyuubiki-operator-sdk",
        sdk_api_version: OPERATOR_SDK_API_VERSION,
        language: "rust",
        purpose: "author_and_package_extension_operators",
        manifest_file: OPERATOR_PACKAGE_MANIFEST_FILE,
        package_schema_version: OPERATOR_PACKAGE_SCHEMA_VERSION,
        areas: operator_sdk_surface_areas(),
    }
}

pub fn find_operator_sdk_surface_area(id: &str) -> Option<OperatorSdkSurfaceArea> {
    operator_sdk_surface_areas()
        .into_iter()
        .find(|area| area.id == id)
}

pub fn operator_sdk_surface_areas() -> Vec<OperatorSdkSurfaceArea> {
    vec![
        OperatorSdkSurfaceArea {
            id: "authoring",
            title: "Operator authoring",
            role: "Rust-only descriptor, port, validation, readiness, and summary-result helpers for operator authors.",
            modules: &["builder", "readiness"],
            anchor_exports: &[
                "OperatorDescriptorBuilder",
                "operator_port",
                "operator_port_with_dataset",
                "operator_summary_result",
                "operator_descriptor_readiness",
            ],
        },
        OperatorSdkSurfaceArea {
            id: "readiness",
            title: "Operator package readiness",
            role: "Pure manifest and descriptor preflight checks before any host loads dynamic code.",
            modules: &["readiness"],
            anchor_exports: &[
                "operator_descriptor_readiness",
                "operator_package_manifest_readiness",
                "operator_package_descriptor_readiness",
            ],
        },
        OperatorSdkSurfaceArea {
            id: "runtime",
            title: "In-process operator runtime",
            role: "Traits, typed JSON adapter, registry, and dispatch errors for host-owned execution.",
            modules: &["lib"],
            anchor_exports: &[
                "OperatorHandler",
                "JsonOperator",
                "OperatorRegistry",
                "OperatorSdkError",
            ],
        },
        OperatorSdkSurfaceArea {
            id: "package_manifest",
            title: "External-local package manifest",
            role: "Stable manifest schema for local Rust operator packages and their validation posture.",
            modules: &["manifest"],
            anchor_exports: &[
                "OperatorPackageManifest",
                "read_operator_package_manifest",
                "discover_operator_packages",
            ],
        },
        OperatorSdkSurfaceArea {
            id: "package_loading",
            title: "Host-mediated package loading",
            role: "Discovery-to-activation planning while keeping dynamic loading policy host-owned.",
            modules: &["loader"],
            anchor_exports: &[
                "OperatorPackageLoadPlan",
                "OperatorPackageActivator",
                "discover_and_activate_operator_packages",
            ],
        },
        OperatorSdkSurfaceArea {
            id: "platform_abi",
            title: "Cross-platform library ABI naming",
            role: "Portable dynamic library filename helpers for template manifests and installer preflight.",
            modules: &["kyuubiki-platform"],
            anchor_exports: &[
                "expand_platform_library_template",
                "current_platform_library_file_name",
                "current_platform_library_path",
            ],
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::{
        OPERATOR_SDK_SURFACE_SCHEMA_VERSION, find_operator_sdk_surface_area,
        operator_sdk_surface_areas, operator_sdk_surface_manifest,
    };
    use crate::{
        OPERATOR_PACKAGE_MANIFEST_FILE, OPERATOR_PACKAGE_SCHEMA_VERSION, OPERATOR_SDK_API_VERSION,
    };
    use std::collections::BTreeSet;

    #[test]
    fn operator_sdk_surface_manifest_is_rust_only_and_serializable() {
        let manifest = operator_sdk_surface_manifest();
        assert_eq!(manifest.schema_version, OPERATOR_SDK_SURFACE_SCHEMA_VERSION);
        assert_eq!(manifest.package, "workers/rust/crates/operator-sdk");
        assert_eq!(manifest.crate_name, "kyuubiki-operator-sdk");
        assert_eq!(manifest.sdk_api_version, OPERATOR_SDK_API_VERSION);
        assert_eq!(manifest.language, "rust");
        assert_eq!(manifest.manifest_file, OPERATOR_PACKAGE_MANIFEST_FILE);
        assert_eq!(
            manifest.package_schema_version,
            OPERATOR_PACKAGE_SCHEMA_VERSION
        );
        serde_json::to_value(&manifest).expect("operator SDK surface should serialize");
    }

    #[test]
    fn operator_sdk_surface_areas_are_unique_and_cover_extension_flow() {
        let areas = operator_sdk_surface_areas();
        let ids = areas.iter().map(|area| area.id).collect::<BTreeSet<_>>();
        assert_eq!(ids.len(), areas.len());
        assert!(ids.contains("authoring"));
        assert!(ids.contains("readiness"));
        assert!(ids.contains("runtime"));
        assert!(ids.contains("package_manifest"));
        assert!(ids.contains("package_loading"));
        assert!(ids.contains("platform_abi"));

        for area in areas {
            assert!(
                !area.modules.is_empty(),
                "surface area {} has no modules",
                area.id
            );
            assert!(
                !area.anchor_exports.is_empty(),
                "surface area {} has no anchor exports",
                area.id
            );
        }
    }

    #[test]
    fn operator_sdk_surface_lookup_returns_authoring_area() {
        let area = find_operator_sdk_surface_area("authoring").expect("authoring area");
        assert_eq!(area.title, "Operator authoring");
        assert!(area.anchor_exports.contains(&"OperatorDescriptorBuilder"));
        assert!(find_operator_sdk_surface_area("headless_control").is_none());
    }
}
