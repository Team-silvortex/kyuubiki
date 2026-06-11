mod agent_client;
mod auth;
mod control_plane;
mod error;
mod session;
mod solver_rpc;
mod workflow_builders;
mod workflow_contracts;

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
    workflow_axis, workflow_dataset_contract, workflow_dataset_value, workflow_edge, workflow_graph, workflow_node, workflow_port,
    workflow_schema_ref, workflow_shape,
};
pub use workflow_contracts::{
    WorkflowAxis, WorkflowDatasetContract, WorkflowDatasetValue, WorkflowDefaults, WorkflowGraphDefinition, WorkflowGraphEdge,
    WorkflowGraphNode, WorkflowGraphPort, WorkflowNodePortRef, WorkflowSchemaRef, WorkflowShape,
    WORKFLOW_DATASET_SCHEMA_VERSION, WORKFLOW_GRAPH_SCHEMA_VERSION,
};
