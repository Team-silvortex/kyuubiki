mod agent_client;
mod auth;
mod control_plane;
mod error;
mod material_research_bundle;
mod material_workflows;
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
