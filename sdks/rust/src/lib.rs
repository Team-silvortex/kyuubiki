mod agent_client;
mod auth;
mod control_plane;
mod error;
mod session;
mod solver_rpc;
mod workflow_builders;
mod workflow_contracts;
mod workflow_results;

pub use agent_client::{
    FailureClass,
    KyuubikiAgentClient,
    ResultChunkIter,
    RetriedStudyRunOutcome,
    RetryPolicy,
    StudyRunOutcome,
};
pub use auth::KyuubikiAuth;
pub use control_plane::ControlPlaneClient;
pub use error::{SdkError, SdkResult};
pub use session::{JobRequest, JobWaitOutcome, KyuubikiSession};
pub use solver_rpc::{RpcCallOutcome, SolverRpcClient};
pub use workflow_builders::{
    workflow_axis, workflow_dataset_contract, workflow_dataset_value, workflow_defaults, workflow_edge, workflow_graph, workflow_node,
    workflow_operator_fetch_entry, workflow_port, workflow_schema_ref, workflow_shape,
};
pub use workflow_contracts::{
    WorkflowAxis, WorkflowDatasetContract, WorkflowDatasetValue, WorkflowDefaults, WorkflowGraphDefinition, WorkflowGraphEdge,
    WorkflowGraphNode, WorkflowGraphPort, WorkflowNodePortRef, WorkflowOperatorFetchEntry, WorkflowSchemaRef, WorkflowShape,
    WORKFLOW_DATASET_SCHEMA_VERSION, WORKFLOW_DISPATCH_POLICIES, WORKFLOW_GRAPH_SCHEMA_VERSION,
};
pub use workflow_results::{
    build_workflow_output_manifest, normalize_workflow_progression, normalize_workflow_runtime, validate_workflow_result_against_graph,
    WorkflowOutputArtifact, WorkflowOutputManifest, WorkflowProgressSnapshot, WorkflowProgression, WorkflowRuntimeSnapshot,
    WorkflowValidatedArtifacts,
};
