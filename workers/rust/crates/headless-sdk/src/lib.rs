mod capabilities;
#[cfg(test)]
mod contract_tests;
mod contracts;
mod contracts_types;
mod direct_fem;
mod executor;
mod hybrid_executor;
mod material_candidate_drafts;
mod material_candidate_materialization;
#[cfg(test)]
mod material_candidate_materialization_tests;
mod material_candidate_review;
mod material_composite;
mod material_composite_candidates;
mod material_composite_interfaces;
mod material_composite_materialization;
#[cfg(test)]
mod material_composite_materialization_tests;
mod material_composite_materialized_report;
mod material_composite_models;
#[cfg(test)]
mod material_composite_tests;
mod material_dielectric;
mod material_envelope_workflow;
mod material_exploration;
mod material_exploration_objectives;
#[cfg(test)]
mod material_exploration_tests;
mod material_optimization;
mod material_reliability;
mod material_reports;
mod material_research;
mod material_research_candidates;
#[cfg(test)]
mod material_research_tests;
mod material_structural;
mod material_study_execution_plan;
mod material_thermo;
mod material_workflows;
mod operator_task;
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
pub use contracts::{all_action_contracts, find_action_contract};
pub use contracts_types::{
    HeadlessActionContract, HeadlessEngine, HeadlessRisk, HeadlessRuntimeStyle,
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
pub use material_candidate_materialization::build_material_candidate_materialization_plan;
pub use material_candidate_review::{
    apply_material_candidate_review_decision, build_material_candidate_materialization_request,
};
pub use material_composite::{
    CompositePanelCandidateReport, CompositePanelReport, build_composite_panel_report,
    build_composite_panel_steps, composite_panel_metric_specs,
};
pub use material_composite_candidates::{CompositePanelCandidate, composite_panel_candidates};
pub use material_composite_interfaces::{
    CompositePanelInterfaceAssessment, CompositePanelMaterialRegion, composite_material_regions,
};
pub use material_composite_materialization::build_composite_materialized_candidate_steps;
pub use material_composite_materialized_report::build_composite_materialized_candidate_report;
pub use material_dielectric::{
    DielectricMaterialCandidate, DielectricMaterialCandidateReport, DielectricMaterialReport,
    build_dielectric_screening_report, build_dielectric_screening_report_with_optimization,
    build_dielectric_screening_steps, dielectric_screening_candidates,
    dielectric_screening_metric_specs,
};
pub use material_exploration::{
    MATERIAL_EXPLORATION_CHAIN_SCHEMA_VERSION,
    MATERIAL_EXPLORATION_NEXT_ROUND_EXECUTION_SCHEMA_VERSION,
    MATERIAL_EXPLORATION_NEXT_ROUND_SCHEMA_VERSION, MATERIAL_EXPLORATION_SCHEMA_VERSION,
    MaterialExplorationNextRoundExecutionPlan, MaterialExplorationNextRoundPlan,
    MaterialExplorationRiskMitigationHint, MaterialExplorationRun,
    build_material_exploration_next_round_execution_plan,
    build_material_exploration_next_round_plan, build_material_exploration_run,
    build_material_exploration_run_for_iteration, material_exploration_steps,
};
pub use material_optimization::{
    MaterialOptimizationConstraint, MaterialOptimizationProfile, MaterialOptimizationTerm,
    MaterialOptimizationWeight, less_equal_status, material_optimization_constraint,
    material_optimization_profile, material_optimization_term, material_optimization_weight,
    profile_weight,
};
pub use material_reliability::{
    MaterialEvidenceRef, MaterialModelAssumption, MaterialQualityGate, MaterialReliabilityEnvelope,
    MaterialReliabilitySummary, gate_status, material_evidence_ref, material_model_assumption,
    material_quality_gate, material_reliability_summary,
};
pub use material_reports::{
    MaterialStudyCatalogEntry, MaterialStudyDescriptor, build_material_report,
    build_material_report_from_run, build_material_report_with_optimization,
    describe_material_study, extract_material_result_payloads, extract_result_payloads_from_run,
    find_material_study, material_study_catalog, material_study_descriptors,
};
pub use material_research::{
    MaterialResearchCandidateReport, MaterialResearchMetricSpec, MaterialResearchReport,
    build_heat_spreader_screening_report, build_heat_spreader_screening_report_with_optimization,
    build_heat_spreader_screening_steps, heat_spreader_screening_metric_specs,
};
pub use material_research_candidates::{
    MaterialResearchCandidate, heat_spreader_screening_candidates,
};
pub use material_structural::{
    StructuralMaterialCandidate, StructuralMaterialCandidateReport, StructuralMaterialReport,
    build_structural_panel_screening_report,
    build_structural_panel_screening_report_with_optimization,
    build_structural_panel_screening_steps, structural_panel_screening_candidates,
};
pub use material_study_execution_plan::{
    MATERIAL_STUDY_EXECUTION_PLAN_SCHEMA_VERSION, MaterialStudyExecutionPlan,
    build_material_study_execution_plan,
};
pub use material_thermo::{
    ThermoMaterialCandidate, ThermoMaterialCandidateReport, ThermoMaterialReport,
    build_thermo_shield_screening_report, build_thermo_shield_screening_report_with_optimization,
    build_thermo_shield_screening_steps, thermo_shield_screening_candidates,
};
pub use material_workflows::{
    MaterialWorkflowCatalogEntry, MaterialWorkflowDescriptor, find_material_workflow,
    material_workflow_catalog, material_workflow_descriptors, search_material_workflow_templates,
};
pub use operator_task::{
    OPERATOR_TASK_EXECUTE_ACTION, OPERATOR_TASK_PREPARE_ACTION, is_operator_task_execute_action,
    is_operator_task_prepare_action, operator_task_error_preview, prepare_operator_task_payload,
    preview_operator_task_execute_payload,
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
