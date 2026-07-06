use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

use crate::{
    JobStatus, SolveElectrostaticPlaneQuad2dRequest, SolveElectrostaticPlaneQuad2dResult,
    SolveElectrostaticPlaneTriangle2dRequest, SolveElectrostaticPlaneTriangle2dResult,
    SolveHeatPlaneQuad2dRequest, SolveHeatPlaneQuad2dResult, SolveHeatPlaneTriangle2dRequest,
    SolveHeatPlaneTriangle2dResult, SolveThermalPlaneQuad2dRequest, SolveThermalPlaneQuad2dResult,
    SolveThermalPlaneTriangle2dRequest, SolveThermalPlaneTriangle2dResult, WorkflowDatasetContract,
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HeatToThermoPlaneQuad2dWorkflowRequest {
    pub heat_model: SolveHeatPlaneQuad2dRequest,
    pub thermo_seed_model: SolveThermalPlaneQuad2dRequest,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HeatToThermoPlaneQuad2dWorkflowResult {
    pub workflow_id: String,
    pub heat_result: SolveHeatPlaneQuad2dResult,
    pub bridged_model: SolveThermalPlaneQuad2dRequest,
    pub thermo_result: SolveThermalPlaneQuad2dResult,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HeatToThermoPlaneTriangle2dWorkflowRequest {
    pub heat_model: SolveHeatPlaneTriangle2dRequest,
    pub thermo_seed_model: SolveThermalPlaneTriangle2dRequest,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HeatToThermoPlaneTriangle2dWorkflowResult {
    pub workflow_id: String,
    pub heat_result: SolveHeatPlaneTriangle2dResult,
    pub bridged_model: SolveThermalPlaneTriangle2dRequest,
    pub thermo_result: SolveThermalPlaneTriangle2dResult,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ElectrostaticHeatToThermoPlaneQuad2dWorkflowRequest {
    pub electrostatic_model: SolveElectrostaticPlaneQuad2dRequest,
    pub heat_seed_model: SolveHeatPlaneQuad2dRequest,
    pub thermo_seed_model: SolveThermalPlaneQuad2dRequest,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ElectrostaticHeatToThermoPlaneQuad2dWorkflowResult {
    pub workflow_id: String,
    pub electrostatic_result: SolveElectrostaticPlaneQuad2dResult,
    pub bridged_heat_model: SolveHeatPlaneQuad2dRequest,
    pub heat_result: SolveHeatPlaneQuad2dResult,
    pub bridged_thermo_model: SolveThermalPlaneQuad2dRequest,
    pub thermo_result: SolveThermalPlaneQuad2dResult,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ElectrostaticHeatToThermoPlaneTriangle2dWorkflowRequest {
    pub electrostatic_model: SolveElectrostaticPlaneTriangle2dRequest,
    pub heat_seed_model: SolveHeatPlaneTriangle2dRequest,
    pub thermo_seed_model: SolveThermalPlaneTriangle2dRequest,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ElectrostaticHeatToThermoPlaneTriangle2dWorkflowResult {
    pub workflow_id: String,
    pub electrostatic_result: SolveElectrostaticPlaneTriangle2dResult,
    pub bridged_heat_model: SolveHeatPlaneTriangle2dRequest,
    pub heat_result: SolveHeatPlaneTriangle2dResult,
    pub bridged_thermo_model: SolveThermalPlaneTriangle2dRequest,
    pub thermo_result: SolveThermalPlaneTriangle2dResult,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowCachePolicy {
    Ephemeral,
    Cached,
    Persisted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowNodeKind {
    Input,
    Solve,
    Transform,
    Extract,
    Export,
    Condition,
    Output,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct WorkflowDefaults {
    pub cache_policy: Option<WorkflowCachePolicy>,
    pub orchestrated: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkflowPort {
    pub id: String,
    pub artifact_type: String,
    pub name: Option<String>,
    pub required: Option<bool>,
    pub cardinality: Option<String>,
    #[serde(default)]
    pub dataset_value: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkflowNode {
    pub id: String,
    pub kind: WorkflowNodeKind,
    pub operator_id: Option<String>,
    pub name: Option<String>,
    pub description: Option<String>,
    pub config: Option<Value>,
    pub cache_policy: Option<WorkflowCachePolicy>,
    pub inputs: Vec<WorkflowPort>,
    pub outputs: Vec<WorkflowPort>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkflowNodePortRef {
    pub node: String,
    pub port: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkflowEdge {
    pub id: String,
    pub from: WorkflowNodePortRef,
    pub to: WorkflowNodePortRef,
    pub artifact_type: String,
    #[serde(default)]
    pub dataset_value: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkflowGraph {
    pub schema_version: String,
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    #[serde(default)]
    pub dataset_contract: Option<WorkflowDatasetContract>,
    pub entry_nodes: Vec<String>,
    #[serde(default)]
    pub output_nodes: Vec<String>,
    #[serde(default)]
    pub defaults: WorkflowDefaults,
    pub nodes: Vec<WorkflowNode>,
    pub edges: Vec<WorkflowEdge>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkflowGraphRunRequest {
    pub graph: WorkflowGraph,
    #[serde(default)]
    pub input_artifacts: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkflowGraphRunResult {
    pub workflow_id: String,
    pub completed_nodes: Vec<String>,
    #[serde(default)]
    pub skipped_nodes: Vec<String>,
    #[serde(default)]
    pub failed_nodes: Vec<String>,
    #[serde(default)]
    pub progress_events: Vec<WorkflowProgressEvent>,
    #[serde(default)]
    pub branch_decisions: Vec<WorkflowBranchDecision>,
    #[serde(default)]
    pub node_runs: Vec<WorkflowNodeRunTrace>,
    #[serde(default)]
    pub artifact_lineage: Vec<WorkflowArtifactLineage>,
    pub artifacts: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkflowProgressEvent {
    pub stage: JobStatus,
    pub progress: f32,
    #[serde(default)]
    pub message: Option<String>,
    #[serde(default)]
    pub node_id: Option<String>,
    #[serde(default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub emitted_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkflowBranchDecision {
    pub node_id: String,
    pub chosen_output: String,
    pub predicate_result: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowNodeRunStatus {
    Completed,
    Skipped,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkflowNodeRunTrace {
    pub node_id: String,
    pub kind: WorkflowNodeKind,
    pub operator_id: Option<String>,
    pub status: WorkflowNodeRunStatus,
    #[serde(default)]
    pub consumed_artifacts: Vec<String>,
    #[serde(default)]
    pub produced_artifacts: Vec<String>,
    #[serde(default)]
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkflowArtifactLineage {
    pub artifact_key: String,
    pub node_id: String,
    pub port_id: String,
    #[serde(default)]
    pub source_artifacts: Vec<String>,
}
