mod analysis;
mod job;
mod operator;
mod workflow;

mod types {
    pub mod field;
    pub mod linear_results;
    pub mod linear_structural;
    pub mod plane_frame;
    pub mod rpc;
    pub mod space_structural;
}

pub use analysis::{AnalysisResult, ResultChunkKind, ResultChunkRequest, ResultChunkResponse};
pub use job::{Job, JobStatus, ProgressEvent};
pub use operator::{
    OperatorArtifactRef, OperatorDescriptor, OperatorKind, OperatorOrigin, OperatorPortDescriptor,
    OperatorRunContext, OperatorRunRequest, OperatorRunResult, OperatorSchemaRef,
    OperatorValidationProfile, OperatorValidationStatus, WorkflowDatasetAxis,
    WorkflowDatasetContract, WorkflowDatasetEncoding, WorkflowDatasetShape,
    WorkflowDatasetValueInfo,
};
pub use types::field::*;
pub use types::linear_results::*;
pub use types::linear_structural::*;
pub use types::plane_frame::*;
pub use types::rpc::*;
pub use types::space_structural::*;
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
