use crate::{
    action_capability_manifest, all_action_contracts, all_direct_fem_routes,
    engine_solver_headless_bridge_manifest, list_template_categories, list_templates,
    material_study_catalog, material_workflow_catalog,
};
use serde::Serialize;

pub const HEADLESS_SDK_SURFACE_SCHEMA_VERSION: &str = "kyuubiki.headless-sdk-surface/v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct HeadlessSdkSurfaceManifest {
    pub schema_version: &'static str,
    pub package: &'static str,
    pub crate_name: &'static str,
    pub areas: Vec<HeadlessSdkSurfaceArea>,
    pub counts: HeadlessSdkSurfaceCounts,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct HeadlessSdkSurfaceArea {
    pub id: &'static str,
    pub title: &'static str,
    pub role: &'static str,
    pub modules: &'static [&'static str],
    pub anchor_exports: &'static [&'static str],
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct HeadlessSdkSurfaceCounts {
    pub action_contracts: usize,
    pub action_capabilities: usize,
    pub direct_fem_routes: usize,
    pub engine_solver_bridge_routes: usize,
    pub executable_solver_routes: usize,
    pub benchmark_solver_routes: usize,
    pub workflow_templates: usize,
    pub template_categories: usize,
    pub material_studies: usize,
    pub material_workflows: usize,
}

pub fn headless_sdk_surface_manifest() -> HeadlessSdkSurfaceManifest {
    HeadlessSdkSurfaceManifest {
        schema_version: HEADLESS_SDK_SURFACE_SCHEMA_VERSION,
        package: "workers/rust/crates/headless-sdk",
        crate_name: "kyuubiki-headless-sdk",
        areas: headless_sdk_surface_areas(),
        counts: headless_sdk_surface_counts(),
    }
}

pub fn headless_sdk_surface_counts() -> HeadlessSdkSurfaceCounts {
    let bridge = engine_solver_headless_bridge_manifest();
    HeadlessSdkSurfaceCounts {
        action_contracts: all_action_contracts().len(),
        action_capabilities: action_capability_manifest().len(),
        direct_fem_routes: all_direct_fem_routes().len(),
        engine_solver_bridge_routes: bridge.route_count,
        executable_solver_routes: bridge.executable_solver_route_count,
        benchmark_solver_routes: bridge.benchmark_route_count,
        workflow_templates: list_templates().len(),
        template_categories: list_template_categories().len(),
        material_studies: material_study_catalog().len(),
        material_workflows: material_workflow_catalog().len(),
    }
}

pub fn find_headless_sdk_surface_area(id: &str) -> Option<HeadlessSdkSurfaceArea> {
    headless_sdk_surface_areas()
        .into_iter()
        .find(|area| area.id == id)
}

pub fn headless_sdk_surface_areas() -> Vec<HeadlessSdkSurfaceArea> {
    vec![
        HeadlessSdkSurfaceArea {
            id: "contracts",
            title: "Action contracts and capabilities",
            role: "Stable action metadata for UI-free automation and executor compatibility checks.",
            modules: &["contracts", "contracts_types", "capabilities"],
            anchor_exports: &[
                "all_action_contracts",
                "find_action_contract",
                "action_capability_manifest",
            ],
        },
        HeadlessSdkSurfaceArea {
            id: "execution",
            title: "Execution planning and dry runs",
            role: "Workflow validation, compatibility checks, executor dispatch, and dry-run reports.",
            modules: &[
                "plan",
                "run",
                "executor",
                "hybrid_executor",
                "service_executor",
            ],
            anchor_exports: &[
                "build_execution_plan",
                "run_batch_dry",
                "execute_batch_with_executor",
            ],
        },
        HeadlessSdkSurfaceArea {
            id: "direct_fem",
            title: "Direct FEM and engine solver bridge",
            role: "GUI-independent solver action to control-plane route and engine solver operator mapping.",
            modules: &["direct_fem", "engine_solver_bridge"],
            anchor_exports: &[
                "all_direct_fem_routes",
                "direct_fem_submit_route",
                "direct_fem_capability_manifest",
                "engine_solver_headless_bridge_manifest",
            ],
        },
        HeadlessSdkSurfaceArea {
            id: "templates",
            title: "Workflow templates",
            role: "Discoverable workflow documents for direct solver, mesh, orchestration, and material studies.",
            modules: &["templates", "template_search", "template_workflows"],
            anchor_exports: &[
                "list_templates",
                "search_templates",
                "build_template_document",
            ],
        },
        HeadlessSdkSurfaceArea {
            id: "operator_task",
            title: "Operator TaskIR",
            role: "Language-neutral operator task preparation, preview, readiness, and batch execution contracts.",
            modules: &["operator_task", "operator_task_readiness"],
            anchor_exports: &[
                "prepare_operator_task_payload",
                "preview_operator_task_execute_payload",
                "operator_task_error_preview",
            ],
        },
        HeadlessSdkSurfaceArea {
            id: "material_research",
            title: "Material research workflows",
            role: "Reference study builders, candidate catalogs, exploration runs, and next-round planning.",
            modules: &[
                "material_research",
                "material_structural",
                "material_thermo",
                "material_dielectric",
                "material_composite",
                "material_exploration",
            ],
            anchor_exports: &[
                "material_study_catalog",
                "build_material_exploration_run",
                "build_material_exploration_next_round_plan",
            ],
        },
        HeadlessSdkSurfaceArea {
            id: "research_artifacts",
            title: "Research artifacts and reliability",
            role: "Report extraction, optimization profiles, quality gates, retained bundles, and materialization plans.",
            modules: &[
                "material_reports",
                "material_optimization",
                "material_reliability",
                "material_research_bundle",
                "material_candidate_materialization",
            ],
            anchor_exports: &[
                "build_material_report",
                "validate_material_research_bundle",
                "build_material_candidate_materialization_plan",
            ],
        },
        HeadlessSdkSurfaceArea {
            id: "workflow_data",
            title: "Workflow data contracts",
            role: "Batch normalization, dataset preflight, workflow descriptors, and reusable execution document shapes.",
            modules: &[
                "workflow_batch",
                "workflow_dataset_preflight",
                "material_workflows",
                "material_study_execution_plan",
            ],
            anchor_exports: &[
                "normalize_workflow_document",
                "preflight_workflow_dataset_contract",
                "build_material_study_execution_plan",
            ],
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::{
        HEADLESS_SDK_SURFACE_SCHEMA_VERSION, find_headless_sdk_surface_area,
        headless_sdk_surface_areas, headless_sdk_surface_manifest,
    };
    use crate::{
        action_capability_manifest, all_action_contracts, all_direct_fem_routes,
        engine_solver_headless_bridge_manifest, list_templates, material_study_catalog,
        material_workflow_catalog,
    };
    use std::collections::BTreeSet;

    #[test]
    fn headless_sdk_surface_manifest_is_serializable_and_counted() {
        let manifest = headless_sdk_surface_manifest();
        assert_eq!(manifest.schema_version, HEADLESS_SDK_SURFACE_SCHEMA_VERSION);
        assert_eq!(manifest.package, "workers/rust/crates/headless-sdk");
        assert_eq!(
            manifest.counts.action_contracts,
            all_action_contracts().len()
        );
        assert_eq!(
            manifest.counts.action_capabilities,
            action_capability_manifest().len()
        );
        assert_eq!(
            manifest.counts.direct_fem_routes,
            all_direct_fem_routes().len()
        );
        assert_eq!(
            manifest.counts.engine_solver_bridge_routes,
            engine_solver_headless_bridge_manifest().route_count
        );
        assert_eq!(
            manifest.counts.executable_solver_routes,
            engine_solver_headless_bridge_manifest().executable_solver_route_count
        );
        assert!(manifest.counts.benchmark_solver_routes > 0);
        assert_eq!(manifest.counts.workflow_templates, list_templates().len());
        assert_eq!(
            manifest.counts.material_studies,
            material_study_catalog().len()
        );
        assert_eq!(
            manifest.counts.material_workflows,
            material_workflow_catalog().len()
        );
        serde_json::to_value(&manifest).expect("surface manifest should serialize");
    }

    #[test]
    fn headless_sdk_surface_areas_are_unique_and_cover_expected_entry_points() {
        let areas = headless_sdk_surface_areas();
        let ids = areas.iter().map(|area| area.id).collect::<BTreeSet<_>>();
        assert_eq!(ids.len(), areas.len());
        assert!(ids.contains("contracts"));
        assert!(ids.contains("execution"));
        assert!(ids.contains("direct_fem"));
        assert!(ids.contains("operator_task"));
        assert!(ids.contains("material_research"));
        assert!(ids.contains("workflow_data"));

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
    fn headless_sdk_surface_lookup_returns_owned_area_records() {
        let area = find_headless_sdk_surface_area("direct_fem").expect("direct FEM area");
        assert_eq!(area.title, "Direct FEM and engine solver bridge");
        assert!(area.anchor_exports.contains(&"direct_fem_submit_route"));
        assert!(find_headless_sdk_surface_area("missing").is_none());
    }
}
