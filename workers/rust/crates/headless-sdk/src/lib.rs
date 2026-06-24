mod capabilities;
#[cfg(test)]
mod contract_tests;
mod contracts;
mod direct_fem;
mod executor;
mod hybrid_executor;
mod material_dielectric;
mod material_exploration;
mod material_optimization;
mod material_reports;
mod material_research;
mod material_structural;
mod material_thermo;
mod plan;
mod run;
mod service_executor;
mod template_search;
#[cfg(test)]
mod template_tests;
mod template_workflows;
mod templates;
mod workflow_batch;

pub use capabilities::{
    HeadlessActionCapability, action_capability_manifest, find_action_capability,
};
pub use contracts::{
    HeadlessActionContract, HeadlessEngine, HeadlessRisk, HeadlessRuntimeStyle,
    all_action_contracts, find_action_contract,
};
pub use direct_fem::{
    DirectFemCapability, DirectFemRoute, all_direct_fem_routes, direct_fem_capability_manifest,
    direct_fem_submit_route,
};
pub use executor::{
    HeadlessExecutor, HeadlessExecutorError, HeadlessExecutorOutcome, MockHeadlessExecutor,
    collect_executor_compatibility_issues, execute_batch_with_executor, executor_supports_action,
};
pub use hybrid_executor::HybridHeadlessExecutor;
pub use material_dielectric::{
    DielectricMaterialCandidate, DielectricMaterialCandidateReport, DielectricMaterialReport,
    build_dielectric_screening_report, build_dielectric_screening_report_with_optimization,
    build_dielectric_screening_steps, dielectric_screening_candidates,
    dielectric_screening_metric_specs,
};
pub use material_exploration::{
    MATERIAL_EXPLORATION_SCHEMA_VERSION, MaterialExplorationRun, build_material_exploration_run,
    material_exploration_steps,
};
pub use material_optimization::{
    MaterialOptimizationConstraint, MaterialOptimizationProfile, MaterialOptimizationTerm,
    MaterialOptimizationWeight, less_equal_status, material_optimization_constraint,
    material_optimization_profile, material_optimization_term, material_optimization_weight,
    profile_weight,
};
pub use material_reports::{
    MaterialStudyCatalogEntry, MaterialStudyDescriptor, build_material_report,
    build_material_report_from_run, build_material_report_with_optimization,
    describe_material_study, extract_material_result_payloads, extract_result_payloads_from_run,
    find_material_study, material_study_catalog, material_study_descriptors,
};
pub use material_research::{
    MaterialResearchCandidate, MaterialResearchCandidateReport, MaterialResearchMetricSpec,
    MaterialResearchReport, build_heat_spreader_screening_report,
    build_heat_spreader_screening_report_with_optimization, build_heat_spreader_screening_steps,
    heat_spreader_screening_candidates, heat_spreader_screening_metric_specs,
};
pub use material_structural::{
    StructuralMaterialCandidate, StructuralMaterialCandidateReport, StructuralMaterialReport,
    build_structural_panel_screening_report,
    build_structural_panel_screening_report_with_optimization,
    build_structural_panel_screening_steps, structural_panel_screening_candidates,
};
pub use material_thermo::{
    ThermoMaterialCandidate, ThermoMaterialCandidateReport, ThermoMaterialReport,
    build_thermo_shield_screening_report, build_thermo_shield_screening_report_with_optimization,
    build_thermo_shield_screening_steps, thermo_shield_screening_candidates,
};
pub use plan::{
    HeadlessExecutionPlan, HeadlessPlanBinding, HeadlessPlanCompatibility,
    HeadlessPlanConfirmation, HeadlessPlanStep, build_execution_plan,
};
pub use run::{
    HeadlessBlockedConfirmation, HeadlessExecutionStepReport, HeadlessRunReport, run_batch_dry,
};
pub use service_executor::ServiceHeadlessExecutor;
pub use templates::{
    HeadlessTemplateSuggestion, build_template_document, find_template, list_template_categories,
    list_templates, search_templates, suggest_template_details, suggest_templates,
};
pub use workflow_batch::{
    HeadlessBatchSummary, HeadlessExecutionBatch, HeadlessExecutionBatchStep,
    HeadlessPolicySummary, HeadlessTemplateDescriptor, HeadlessTemplateSnapshot,
    HeadlessValidationReport, HeadlessWorkflowDocument, HeadlessWorkflowDraft,
    HeadlessWorkflowStep, normalize_workflow_document, summarize_batch, validate_batch,
};
