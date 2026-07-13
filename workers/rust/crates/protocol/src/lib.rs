mod analysis;
mod benchmark_surface;
mod job;
mod operator;
mod operator_task_ir;
mod solver_execution_capability;
mod workflow;

mod types {
    pub mod acoustic;
    pub mod acoustic_results;
    pub mod dynamic_results;
    pub mod electrostatic_plane_results;
    pub mod field;
    pub mod field_results;
    pub mod fluid_results;
    pub mod linear_results;
    pub mod linear_structural;
    pub mod nonlinear_structural;
    pub mod plane_frame;
    pub mod plane_results;
    pub mod rpc;
    pub mod rpc_descriptor;
    pub mod space_structural;
    pub mod transport_results;
}

pub use analysis::{AnalysisResult, ResultChunkKind, ResultChunkRequest, ResultChunkResponse};
pub use benchmark_surface::{
    PROTOCOL_BENCHMARK_SURFACE_SCHEMA_VERSION, ProtocolBenchmarkLane, ProtocolBenchmarkSurface,
    protocol_benchmark_surface,
};
pub use job::{Job, JobStatus, ProgressEvent};
pub use operator::{
    OperatorArtifactRef, OperatorDescriptor, OperatorKind, OperatorOrigin, OperatorPortDescriptor,
    OperatorRunContext, OperatorRunRequest, OperatorRunResult, OperatorSchemaRef,
    OperatorValidationProfile, OperatorValidationStatus, WORKFLOW_DATASET_DATA_CLASSES,
    WorkflowDatasetAxis, WorkflowDatasetContract, WorkflowDatasetEncoding, WorkflowDatasetShape,
    WorkflowDatasetValueInfo,
};
pub use operator_task_ir::{
    OPERATOR_TASK_DIGEST_FIELDS, OPERATOR_TASK_IR_SCHEMA, OperatorTaskDigestError,
    OperatorTaskExecutionPreview, OperatorTaskExecutionSummary, OperatorTaskSummaryError,
    OperatorTaskSummaryErrorCode, canonical_json, compute_operator_task_digest,
    operator_task_digest_fields, preview_operator_task_execution,
    summarize_operator_task_execution, summarize_operator_task_execution_checked,
    verify_operator_task_digest,
};
pub use solver_execution_capability::{
    SolverExecutionCapability, SolverExecutionCapabilityReport,
    check_operator_task_execution_capability,
};
pub use types::acoustic::*;
pub use types::acoustic_results::*;
pub use types::dynamic_results::*;
pub use types::electrostatic_plane_results::*;
pub use types::field::*;
pub use types::field_results::*;
pub use types::fluid_results::*;
pub use types::linear_results::*;
pub use types::linear_structural::*;
pub use types::nonlinear_structural::*;
pub use types::plane_frame::*;
pub use types::plane_results::*;
pub use types::rpc::*;
pub use types::space_structural::*;
pub use types::transport_results::*;
pub use workflow::{
    ElectrostaticHeatToThermoPlaneQuad2dWorkflowRequest,
    ElectrostaticHeatToThermoPlaneQuad2dWorkflowResult,
    ElectrostaticHeatToThermoPlaneTriangle2dWorkflowRequest,
    ElectrostaticHeatToThermoPlaneTriangle2dWorkflowResult, HeatToThermoPlaneQuad2dWorkflowRequest,
    HeatToThermoPlaneQuad2dWorkflowResult, HeatToThermoPlaneTriangle2dWorkflowRequest,
    HeatToThermoPlaneTriangle2dWorkflowResult, WorkflowArtifactLineage, WorkflowBranchDecision,
    WorkflowCachePolicy, WorkflowDefaults, WorkflowEdge, WorkflowGraph, WorkflowGraphRunRequest,
    WorkflowGraphRunResult, WorkflowNode, WorkflowNodeKind, WorkflowNodePortRef,
    WorkflowNodeRunStatus, WorkflowNodeRunTrace, WorkflowPort, WorkflowProgressEvent,
};

pub const RPC_VERSION: u8 = 1;
pub const SOLVER_RPC_PROTOCOL: &str = "kyuubiki.solver-rpc/v1";
pub const CONTROL_PLANE_PROTOCOL: &str = "kyuubiki.control-plane/http-v1";

#[cfg(test)]
mod tests;
