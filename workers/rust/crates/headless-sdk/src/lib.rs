mod capabilities;
#[cfg(test)]
mod contract_tests;
mod contracts;
mod contracts_types;
mod coupled_workflows;
mod direct_fem;
mod engine_solver_bridge;
mod execution_authority;
mod execution_observability;
mod executor;
mod hybrid_executor;
mod material_candidate_drafts;
mod material_candidate_materialization;
#[cfg(test)]
mod material_candidate_materialization_tests;
mod material_candidate_rerun;
#[cfg(test)]
mod material_candidate_rerun_tests;
mod material_candidate_review;
mod material_candidate_review_batches;
mod material_card_refs;
mod material_composite;
mod material_composite_algebraic_validation;
mod material_composite_candidates;
mod material_composite_convergence_regime;
mod material_composite_current;
mod material_composite_electrothermal;
mod material_composite_evidence;
mod material_composite_feedback;
mod material_composite_heat_validation;
mod material_composite_interfaces;
mod material_composite_joule;
mod material_composite_materialization;
#[cfg(test)]
mod material_composite_materialization_tests;
mod material_composite_materialized_report;
mod material_composite_models;
mod material_composite_quality;
mod material_composite_stress_recovery;
mod material_composite_structural_validation;
#[cfg(test)]
mod material_composite_tests;
mod material_composite_thermal_expansion;
mod material_composite_validation;
mod material_dielectric;
mod material_envelope_workflow;
mod material_exploration;
mod material_exploration_objectives;
mod material_exploration_risk;
#[cfg(test)]
mod material_exploration_tests;
mod material_optimization;
mod material_reliability;
#[cfg(test)]
mod material_reliability_tests;
mod material_reports;
mod material_research;
mod material_research_bundle;
mod material_research_candidates;
#[cfg(test)]
mod material_research_tests;
#[cfg(test)]
mod material_research_validation_flow_tests;
mod material_structural;
mod material_study_execution_plan;
mod material_thermo;
mod material_workflows;
mod model_collaboration;
#[cfg(test)]
mod model_collaboration_tests;
mod model_provider_adapters;
mod operator_task;
mod operator_task_provenance;
mod operator_task_readiness;
mod operator_task_security;
#[cfg(test)]
mod operator_task_tests;
mod operator_task_validation;
#[cfg(test)]
mod operator_task_validation_tests;
mod plan;
mod run;
mod service_executor;
mod service_executor_artifact;
mod service_executor_health;
mod service_executor_http;
mod service_executor_job_wait;
mod service_executor_library;
mod service_executor_solve;
mod surface;
mod template_search;
#[cfg(test)]
mod template_tests;
mod template_workflows;
mod templates;
mod workflow_batch;
mod workflow_dataset_preflight;

pub use capabilities::{
    HeadlessActionCapability, action_capability_manifest, find_action_capability,
};
pub use contracts::{all_action_contracts, find_action_contract};
pub use contracts_types::{
    HeadlessActionContract, HeadlessEngine, HeadlessRisk, HeadlessRuntimeStyle,
};
pub use coupled_workflows::{
    CoupledWorkflowCatalogEntry, coupled_workflow_catalog, find_coupled_workflow,
    search_coupled_workflows,
};
pub use direct_fem::{
    DirectFemCapability, DirectFemRoute, all_direct_fem_routes, direct_fem_capability_manifest,
    direct_fem_submit_route,
};
pub use engine_solver_bridge::{
    ENGINE_SOLVER_HEADLESS_BRIDGE_SCHEMA_VERSION, EngineSolverHeadlessBridgeManifest,
    EngineSolverHeadlessBridgeRoute, engine_solver_headless_bridge_manifest,
};
pub use execution_authority::{
    EXECUTION_AUTHORITY_SCHEMA_VERSION, ExecutionAuthority, validate_execution_authority,
};
pub use execution_observability::{
    HEADLESS_EXECUTION_SUMMARY_SCHEMA_VERSION, HEADLESS_FAILURE_RECEIPT_SCHEMA_VERSION,
    HeadlessExecutionSummary, HeadlessFailureReceipt, HeadlessJobTimeline,
};
pub use executor::{
    HeadlessExecutor, HeadlessExecutorError, HeadlessExecutorOutcome, MockHeadlessExecutor,
    collect_executor_compatibility_issues, execute_batch_with_executor, executor_supports_action,
};
pub use hybrid_executor::HybridHeadlessExecutor;
pub use material_candidate_materialization::build_material_candidate_materialization_plan;
pub use material_candidate_rerun::{
    build_materialized_candidate_report, build_materialized_candidate_steps,
    materialized_candidate_study,
};
pub use material_candidate_review::{
    apply_material_candidate_review_decision, build_material_candidate_materialization_request,
};
pub use material_composite::{
    CompositePanelCandidateReport, CompositePanelReport, build_composite_panel_report,
    build_composite_panel_steps, composite_panel_metric_specs,
};
pub use material_composite_algebraic_validation::{
    COMPOSITE_THERMAL_ALGEBRAIC_VALIDATION_SCHEMA_VERSION, CompositeThermalAlgebraicSample,
    CompositeThermalAlgebraicSeries, CompositeThermalAlgebraicValidation,
    composite_thermal_algebraic_series, composite_thermal_algebraic_validation,
    missing_thermal_algebraic_validation,
};
pub use material_composite_candidates::{CompositePanelCandidate, composite_panel_candidates};
pub use material_composite_convergence_regime::{
    COMPOSITE_THERMAL_CONVERGENCE_REGIME_SCHEMA_VERSION, CompositeConvergenceMetricAssessment,
    CompositeThermalConvergenceRegimeAssessment, composite_thermal_convergence_regime,
    missing_thermal_convergence_regime,
};
pub use material_composite_current::{
    COMPOSITE_CURRENT_TO_HEAT_PROJECTION_SCHEMA_VERSION, CompositeCurrentConductionFeedbackSpec,
    CompositeCurrentConductionRegionSpec, CompositeCurrentRegionProjection,
    CompositeCurrentToHeatProjection, project_composite_solved_current_to_heat,
    temperature_adjusted_composite_current_request,
};
pub use material_composite_electrothermal::{
    COMPOSITE_ELECTROTHERMAL_LOSS_SCHEMA_VERSION, COMPOSITE_HEAT_TO_THERMAL_SCHEMA_VERSION,
    CompositeDielectricLossSpec, CompositeElectrothermalLossProjection,
    CompositeHeatToThermalProjection, distribute_composite_dielectric_heat_load,
    project_composite_dielectric_loss_to_heat, project_composite_heat_to_thermal,
};
pub use material_composite_feedback::{
    COMPOSITE_ELECTROTHERMAL_FEEDBACK_SCHEMA_VERSION, CompositeElectrothermalFeedbackConvergence,
    CompositeElectrothermalFeedbackIteration, CompositeElectrothermalFeedbackSpec,
    CompositeThermalConductivityFeedbackIteration, CompositeThermalConductivityFeedbackModel,
    apply_composite_dielectric_permittivity, assess_composite_electrothermal_feedback,
    composite_dielectric_mean_temperature, composite_feedback_iteration_converged,
    composite_feedback_relative_change, composite_heat_element_mean_temperature,
    temperature_adjusted_composite_heat_request, temperature_adjusted_composite_loss_spec,
};
pub use material_composite_heat_validation::{
    COMPOSITE_HEAT_CROSS_VALIDATION_SCHEMA_VERSION, COMPOSITE_HEAT_MESH_CONVERGENCE_SCHEMA_VERSION,
    COMPOSITE_HEAT_REFINEMENT_LEVELS, CompositeHeatCrossValidation, CompositeHeatMeshConvergence,
    CompositeHeatMeshConvergenceSample, composite_heat_cross_validation,
    composite_heat_cross_validation_for_distributed_load,
    composite_heat_cross_validation_for_regional_loads, composite_heat_mesh_convergence,
    composite_heat_mesh_convergence_for_distributed_load,
    composite_heat_mesh_convergence_for_regional_loads, composite_heat_refinement_requests,
    composite_heat_refinement_requests_for_distributed_load,
    composite_heat_refinement_requests_for_regional_loads,
};
pub use material_composite_interfaces::{
    CompositePanelInterfaceAssessment, CompositePanelMaterialRegion, composite_material_regions,
};
pub use material_composite_joule::{
    COMPOSITE_JOULE_HEATING_PROJECTION_SCHEMA_VERSION, CompositeJouleHeatingProjection,
    CompositeJouleHeatingRegionProjection, CompositeJouleHeatingRegionSpec,
    CompositeJouleHeatingSpec, project_composite_joule_heating_to_heat,
};
pub use material_composite_materialization::build_composite_materialized_candidate_steps;
pub use material_composite_materialized_report::build_composite_materialized_candidate_report;
pub use material_composite_stress_recovery::{
    COMPOSITE_THERMAL_INTERFACE_GRADING_SCHEMA_VERSION,
    COMPOSITE_THERMAL_STRESS_RECOVERY_SCHEMA_VERSION, CompositeThermalInterfaceGradingAssessment,
    CompositeThermalRecoveredStressStatistics, CompositeThermalStressRecovery,
    CompositeThermalStressRecoverySample, composite_thermal_interface_graded_stress_recovery,
    composite_thermal_interface_grading_assessment, composite_thermal_recovered_stress_statistics,
    composite_thermal_stress_recovery,
};
pub use material_composite_structural_validation::{
    COMPOSITE_THERMAL_CONSTRAINT_SENSITIVITY_SCHEMA_VERSION,
    COMPOSITE_THERMAL_MESH_CONVERGENCE_SCHEMA_VERSION, COMPOSITE_THERMAL_REFINEMENT_LEVELS,
    CompositeThermalConstraintSensitivity, CompositeThermalMeshConvergence,
    CompositeThermalMeshSample, composite_thermal_constraint_sensitivity,
    composite_thermal_interface_graded_mesh_convergence,
    composite_thermal_interface_graded_refinement_requests, composite_thermal_mesh_convergence,
    composite_thermal_refinement_requests, composite_thermal_regularized_mesh_convergence,
    composite_thermal_regularized_refinement_requests,
};
pub use material_composite_thermal_expansion::{
    COMPOSITE_THERMAL_EXPANSION_PROJECTION_SCHEMA_VERSION, CompositeThermalExpansionFeedbackSpec,
    CompositeThermalExpansionProjection, CompositeThermalExpansionRegionSpec,
    CompositeThermalExpansionRegionUpdate, project_composite_temperature_dependent_expansion,
};
pub use material_composite_validation::{
    COMPOSITE_ELECTROSTATIC_CROSS_VALIDATION_SCHEMA_VERSION,
    COMPOSITE_ELECTROSTATIC_MESH_CONVERGENCE_SCHEMA_VERSION,
    COMPOSITE_ELECTROSTATIC_REFINEMENT_LEVELS, CompositeElectrostaticCrossValidation,
    CompositeElectrostaticMeshConvergence, CompositeElectrostaticMeshConvergenceSample,
    composite_electrostatic_cross_validation,
    composite_electrostatic_cross_validation_for_dielectric,
    composite_electrostatic_mesh_convergence,
    composite_electrostatic_mesh_convergence_for_dielectric,
    composite_electrostatic_refinement_requests,
    composite_electrostatic_refinement_requests_for_dielectric,
};
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
    MaterialReliabilitySummary, MaterialRepairHint, SUMMARY_TOLERANCE_VALIDATION_CONTRACT,
    gate_status, material_evidence_ref, material_model_assumption, material_quality_gate,
    material_reliability_summary, material_validation_quality_gate,
    material_validation_repair_hint,
};
pub use material_reports::{
    MaterialStudyCatalogEntry, MaterialStudyDescriptor, build_material_report,
    build_material_report_from_run, build_material_report_with_optimization,
    describe_material_study, extract_material_result_payloads, extract_result_payloads_from_run,
    find_material_study, material_study_catalog, material_study_descriptors,
    supported_material_report_study_ids, validate_material_report_compatibility,
};
pub use material_research::{
    MaterialCardReference, MaterialResearchCandidateReport, MaterialResearchMetricSpec,
    MaterialResearchReport, build_heat_spreader_materialized_candidate_report,
    build_heat_spreader_materialized_candidate_steps, build_heat_spreader_screening_report,
    build_heat_spreader_screening_report_with_optimization, build_heat_spreader_screening_steps,
    heat_spreader_screening_metric_specs,
};
pub use material_research_bundle::{
    MATERIAL_RESEARCH_BUNDLE_SCHEMA_VERSION, MaterialResearchBundle,
    MaterialResearchBundleArtifactChecksums, MaterialResearchBundleMaterialCardRef,
    MaterialResearchBundleReproducibility, MaterialResearchBundleSummary,
    validate_material_research_bundle,
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
pub use model_collaboration::{
    MODEL_COLLABORATION_SCHEMA_VERSION, MODEL_PROPOSAL_COMPILATION_SCHEMA_VERSION,
    MODEL_WORKFLOW_PROPOSAL_SCHEMA_VERSION, ModelCollaborationError, ModelCollaborationPolicy,
    ModelCollaborationRequest, ModelCollaborationSession, ModelCollaborationTool,
    ModelProposalCompilation, ModelProvider, ModelToolCall, ModelWorkflowProposal,
    build_model_collaboration_request, compile_model_proposal, model_collaboration_tools,
    sanitize_model_context,
};
pub use model_provider_adapters::{normalize_model_response, project_model_tools};
pub use operator_task::{
    OPERATOR_TASK_EXECUTE_ACTION, OPERATOR_TASK_PREPARE_ACTION, is_operator_task_execute_action,
    is_operator_task_prepare_action, operator_task_error_preview, prepare_operator_task_payload,
    preview_operator_task_execute_payload,
};
pub use operator_task_provenance::{
    HEADLESS_OPERATOR_TASK_PROVENANCE_SCHEMA_VERSION, operator_task_provenance_profile,
};
pub use operator_task_security::{
    HEADLESS_OPERATOR_TASK_SECURITY_SCHEMA_VERSION, operator_task_security_profile,
};
pub use operator_task_validation::{
    HeadlessOperatorTaskValidationReport, validate_operator_task_for_agent,
    validate_operator_task_for_builtin_agent,
};
pub use plan::{
    HeadlessExecutionPlan, HeadlessPlanBinding, HeadlessPlanCompatibility,
    HeadlessPlanConfirmation, HeadlessPlanStep, build_execution_plan,
};
pub use run::{
    HeadlessBlockedConfirmation, HeadlessExecutionStepReport, HeadlessRunReport, run_batch_dry,
};
pub use service_executor::{ServiceHeadlessExecutor, service_executor_supports_action};
pub use surface::{
    HEADLESS_SDK_SURFACE_SCHEMA_VERSION, HeadlessSdkSurfaceArea, HeadlessSdkSurfaceCounts,
    HeadlessSdkSurfaceManifest, find_headless_sdk_surface_area, headless_sdk_surface_areas,
    headless_sdk_surface_counts, headless_sdk_surface_manifest,
};
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
pub use workflow_dataset_preflight::{
    HeadlessWorkflowDatasetPreflightReport, preflight_workflow_dataset_contract,
};
