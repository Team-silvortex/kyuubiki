use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

use crate::{
    JobStatus, SolveElectrostaticPlaneQuad2dRequest, SolveElectrostaticPlaneQuad2dResult,
    SolveElectrostaticPlaneTriangle2dRequest, SolveElectrostaticPlaneTriangle2dResult,
    SolveHeatPlaneQuad2dRequest, SolveHeatPlaneQuad2dResult, SolveHeatPlaneTriangle2dRequest,
    SolveHeatPlaneTriangle2dResult, SolveMagnetostaticPlaneQuad2dRequest,
    SolveMagnetostaticPlaneQuad2dResult, SolveThermalPlaneQuad2dRequest,
    SolveThermalPlaneQuad2dResult, SolveThermalPlaneTriangle2dRequest,
    SolveThermalPlaneTriangle2dResult, WorkflowDatasetContract,
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

/// Stable headless contract for the magnetic-field, heat, and thermal-stress chain.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MagnetostaticHeatToThermoPlaneQuad2dWorkflowRequest {
    pub magnetostatic_model: SolveMagnetostaticPlaneQuad2dRequest,
    pub heat_seed_model: SolveHeatPlaneQuad2dRequest,
    pub thermo_seed_model: SolveThermalPlaneQuad2dRequest,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MagnetostaticHeatToThermoPlaneQuad2dWorkflowResult {
    pub workflow_id: String,
    pub magnetostatic_result: SolveMagnetostaticPlaneQuad2dResult,
    pub bridged_heat_model: SolveHeatPlaneQuad2dRequest,
    pub heat_result: SolveHeatPlaneQuad2dResult,
    pub bridged_thermo_model: SolveThermalPlaneQuad2dRequest,
    pub thermo_result: SolveThermalPlaneQuad2dResult,
}

/// Identifies a built-in coupled workflow route for discovery and batch dispatch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CoupledWorkflowKind {
    #[serde(rename = "heat_to_thermo_plane_quad_2d")]
    HeatToThermoPlaneQuad2d,
    #[serde(rename = "electrostatic_heat_to_thermo_plane_quad_2d")]
    ElectrostaticHeatToThermoPlaneQuad2d,
    #[serde(rename = "electrostatic_heat_to_thermo_plane_triangle_2d")]
    ElectrostaticHeatToThermoPlaneTriangle2d,
    #[serde(rename = "magnetostatic_heat_to_thermo_plane_quad_2d")]
    MagnetostaticHeatToThermoPlaneQuad2d,
}

/// Static discovery metadata for one member of the coupled-workflow series.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CoupledWorkflowDescriptor {
    pub kind: CoupledWorkflowKind,
    pub id: &'static str,
    pub source_artifact_type: &'static str,
    pub result_artifact_type: &'static str,
    pub domains: &'static [&'static str],
    pub bridge_operator_ids: &'static [&'static str],
}

const COUPLED_WORKFLOW_DESCRIPTORS: &[CoupledWorkflowDescriptor] = &[
    CoupledWorkflowDescriptor {
        kind: CoupledWorkflowKind::HeatToThermoPlaneQuad2d,
        id: "workflow.heat-to-thermo-quad-2d",
        source_artifact_type: "study_model/heat_plane_quad_2d",
        result_artifact_type: "result/thermal_plane_quad_2d",
        domains: &["thermal", "thermo"],
        bridge_operator_ids: &["bridge.temperature_field_to_thermo_quad_2d"],
    },
    CoupledWorkflowDescriptor {
        kind: CoupledWorkflowKind::ElectrostaticHeatToThermoPlaneQuad2d,
        id: "workflow.electrostatic-heat-to-thermo-quad-2d",
        source_artifact_type: "study_model/electrostatic_plane_quad_2d",
        result_artifact_type: "result/thermal_plane_quad_2d",
        domains: &["electrostatic", "thermal", "thermo"],
        bridge_operator_ids: &[
            "bridge.electrostatic_field_to_heat_quad_2d",
            "bridge.temperature_field_to_thermo_quad_2d",
        ],
    },
    CoupledWorkflowDescriptor {
        kind: CoupledWorkflowKind::ElectrostaticHeatToThermoPlaneTriangle2d,
        id: "workflow.electrostatic-heat-to-thermo-triangle-2d",
        source_artifact_type: "study_model/electrostatic_plane_triangle_2d",
        result_artifact_type: "result/thermal_plane_triangle_2d",
        domains: &["electrostatic", "thermal", "thermo"],
        bridge_operator_ids: &[
            "bridge.electrostatic_field_to_heat_triangle_2d",
            "bridge.temperature_field_to_thermo_triangle_2d",
        ],
    },
    CoupledWorkflowDescriptor {
        kind: CoupledWorkflowKind::MagnetostaticHeatToThermoPlaneQuad2d,
        id: "workflow.magnetostatic-heat-to-thermo-quad-2d",
        source_artifact_type: "study_model/magnetostatic_plane_quad_2d",
        result_artifact_type: "result/thermal_plane_quad_2d",
        domains: &["magnetostatic", "thermal", "thermo"],
        bridge_operator_ids: &[
            "bridge.magnetostatic_field_to_heat_quad_2d",
            "bridge.temperature_field_to_thermo_quad_2d",
        ],
    },
];

const SUPPORTED_COUPLED_WORKFLOW_KINDS: &[CoupledWorkflowKind] = &[
    CoupledWorkflowKind::HeatToThermoPlaneQuad2d,
    CoupledWorkflowKind::ElectrostaticHeatToThermoPlaneQuad2d,
    CoupledWorkflowKind::ElectrostaticHeatToThermoPlaneTriangle2d,
    CoupledWorkflowKind::MagnetostaticHeatToThermoPlaneQuad2d,
];

/// Returns the protocol-owned discovery catalog for coupled-workflow routes.
pub fn coupled_workflow_descriptors() -> &'static [CoupledWorkflowDescriptor] {
    COUPLED_WORKFLOW_DESCRIPTORS
}

/// Returns stable coupled-workflow kinds in catalog order.
pub fn supported_coupled_workflow_kinds() -> &'static [CoupledWorkflowKind] {
    SUPPORTED_COUPLED_WORKFLOW_KINDS
}

/// A tagged request envelope for the supported coupled-workflow series.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "request", rename_all = "snake_case")]
pub enum CoupledWorkflowRequest {
    #[serde(rename = "heat_to_thermo_plane_quad_2d")]
    HeatToThermoPlaneQuad2d(HeatToThermoPlaneQuad2dWorkflowRequest),
    #[serde(rename = "electrostatic_heat_to_thermo_plane_quad_2d")]
    ElectrostaticHeatToThermoPlaneQuad2d(ElectrostaticHeatToThermoPlaneQuad2dWorkflowRequest),
    #[serde(rename = "electrostatic_heat_to_thermo_plane_triangle_2d")]
    ElectrostaticHeatToThermoPlaneTriangle2d(
        ElectrostaticHeatToThermoPlaneTriangle2dWorkflowRequest,
    ),
    #[serde(rename = "magnetostatic_heat_to_thermo_plane_quad_2d")]
    MagnetostaticHeatToThermoPlaneQuad2d(MagnetostaticHeatToThermoPlaneQuad2dWorkflowRequest),
}

/// A tagged result envelope paired with [`CoupledWorkflowRequest`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "result", rename_all = "snake_case")]
pub enum CoupledWorkflowResult {
    #[serde(rename = "heat_to_thermo_plane_quad_2d")]
    HeatToThermoPlaneQuad2d(HeatToThermoPlaneQuad2dWorkflowResult),
    #[serde(rename = "electrostatic_heat_to_thermo_plane_quad_2d")]
    ElectrostaticHeatToThermoPlaneQuad2d(ElectrostaticHeatToThermoPlaneQuad2dWorkflowResult),
    #[serde(rename = "electrostatic_heat_to_thermo_plane_triangle_2d")]
    ElectrostaticHeatToThermoPlaneTriangle2d(
        ElectrostaticHeatToThermoPlaneTriangle2dWorkflowResult,
    ),
    #[serde(rename = "magnetostatic_heat_to_thermo_plane_quad_2d")]
    MagnetostaticHeatToThermoPlaneQuad2d(MagnetostaticHeatToThermoPlaneQuad2dWorkflowResult),
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
