mod agent_client;
mod auth;
mod control_plane;
mod error;
mod material_research_bundle;
mod material_research_bundle_validation;
mod material_workflows;
mod model_collaboration;
mod model_provider_adapters;
mod model_research_execution;
mod model_research_frontier;
mod model_research_validation;
mod operator_tasks;
mod session;
mod solver_rpc;
mod workflow_builders;
mod workflow_contract_validation;
mod workflow_contracts;
mod workflow_results;

pub use agent_client::{
    FailureClass, KyuubikiAgentClient, ResultChunkIter, RetriedStudyRunOutcome, RetryPolicy,
    StudyRunOutcome,
};
pub use auth::KyuubikiAuth;
pub use control_plane::ControlPlaneClient;
pub use error::{SdkError, SdkResult};
pub use material_research_bundle::{
    MATERIAL_RESEARCH_BUNDLE_SCHEMA_VERSION, MaterialResearchBundle,
    MaterialResearchBundleArtifactChecksums, MaterialResearchBundleReproducibility,
    MaterialResearchBundleSummary, validate_material_research_bundle,
};
pub use material_workflows::{
    MATERIAL_ENVELOPE_CATALOG_WORKFLOW_ID, MATERIAL_STUDY_EXECUTION_PLAN_SCHEMA_VERSION,
    material_study_envelope_catalog_request, material_study_envelope_input_artifacts,
    material_study_execution_plan_example, material_workflow_catalog,
};
pub use model_collaboration::{
    HeadlessModelRisk, HeadlessModelRuntime, HeadlessModelTool, MODEL_COLLABORATION_SCHEMA_VERSION,
    MODEL_HEADLESS_PLAN_SCHEMA_VERSION, MODEL_WORKFLOW_PROPOSAL_SCHEMA_VERSION,
    ModelCollaborationPolicy, ModelCollaborationRequest, ModelCollaborationSession,
    ModelHeadlessPlan, ModelHeadlessPlanStep, ModelProvider, ModelToolCall, ModelWorkflowProposal,
    build_model_collaboration_request, build_model_headless_plan, rust_headless_model_tools,
};
pub use model_provider_adapters::{
    normalize_model_response, project_model_tools, sanitize_model_context,
};
pub use model_research_execution::{
    ApprovedModelPlanStep, MODEL_PLAN_APPROVAL_SCHEMA_VERSION,
    MODEL_RESEARCH_RECEIPT_SCHEMA_VERSION, ModelActionDispatch, ModelActionDispatcher,
    ModelApprovalVerifier, ModelPlanApproval, ModelResearchExecutionReceipt,
    ModelResearchExecutionRecord, ModelResearchExecutionStatus, SessionModelActionDispatcher,
    execute_model_headless_plan,
};
pub use model_research_frontier::{
    MODEL_RESEARCH_FRONTIER_SCHEMA_VERSION, ModelFrontierVerifier, ModelReceiptVerifier,
    ModelResearchFrontier, ModelResearchFrontierEvidence, ModelResearchFrontierStage,
    advance_model_research_frontier, build_model_research_frontier_proposal,
    start_model_research_frontier,
};
pub use model_research_validation::{
    MODEL_RESEARCH_VALIDATION_REPORT_SCHEMA_VERSION, ModelResearchBundleValidation,
    ModelResearchValidationReport, ModelResearchValidationStage, ModelResearchWorkflowValidation,
    validate_model_research_frontier_result,
};
pub use operator_tasks::{
    operator_task_failure_actions, operator_task_failure_receipts, operator_task_recovery_summary,
};
pub use session::{JobRequest, JobWaitOutcome, KyuubikiSession};
pub use solver_rpc::{RpcCallOutcome, SolverRpcClient};
pub use workflow_builders::{
    workflow_axis, workflow_dataset_contract, workflow_dataset_value, workflow_defaults,
    workflow_edge, workflow_graph, workflow_node, workflow_operator_fetch_entry, workflow_port,
    workflow_schema_ref, workflow_shape,
};
pub use workflow_contracts::{
    WORKFLOW_DATASET_SCHEMA_VERSION, WORKFLOW_DISPATCH_POLICIES, WORKFLOW_GRAPH_SCHEMA_VERSION,
    WorkflowAxis, WorkflowDatasetContract, WorkflowDatasetValue, WorkflowDefaults,
    WorkflowGraphDefinition, WorkflowGraphEdge, WorkflowGraphNode, WorkflowGraphPort,
    WorkflowNodePortRef, WorkflowOperatorFetchEntry, WorkflowSchemaRef, WorkflowShape,
};
pub use workflow_results::{
    WorkflowOutputArtifact, WorkflowOutputManifest, WorkflowProgressSnapshot, WorkflowProgression,
    WorkflowRuntimeSnapshot, WorkflowValidatedArtifacts, build_workflow_output_manifest,
    normalize_workflow_progression, normalize_workflow_runtime,
    validate_workflow_result_against_graph,
};
