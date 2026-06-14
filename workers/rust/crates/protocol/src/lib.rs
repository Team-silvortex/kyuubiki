use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

pub const RPC_VERSION: u8 = 1;
pub const SOLVER_RPC_PROTOCOL: &str = "kyuubiki.solver-rpc/v1";
pub const CONTROL_PLANE_PROTOCOL: &str = "kyuubiki.control-plane/http-v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OperatorKind {
    Solver,
    Transform,
    Extract,
    Export,
    WorkflowBridge,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OperatorOrigin {
    BuiltIn,
    ExternalLocal,
    ExternalRemote,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OperatorSchemaRef {
    pub schema: String,
    pub version: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OperatorValidationStatus {
    Verified,
    Partial,
    Unverified,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OperatorPortDescriptor {
    pub id: String,
    pub artifact_type: String,
    pub description: String,
    #[serde(default)]
    pub dataset_value: Option<String>,
    #[serde(default)]
    pub schema_ref: Option<OperatorSchemaRef>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OperatorValidationProfile {
    pub baseline_status: OperatorValidationStatus,
    #[serde(default)]
    pub baseline_cases: Vec<String>,
    #[serde(default)]
    pub smoke_paths: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowDatasetEncoding {
    Json,
    JsonLines,
    F64Le,
    F32Le,
    I64Le,
    I32Le,
    U8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkflowDatasetAxis {
    pub id: String,
    pub label: Option<String>,
    pub size: Option<u64>,
    pub semantic: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct WorkflowDatasetShape {
    #[serde(default)]
    pub axes: Vec<WorkflowDatasetAxis>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkflowDatasetValueInfo {
    pub id: String,
    pub data_class: String,
    pub element_type: String,
    pub shape: WorkflowDatasetShape,
    pub semantic_type: Option<String>,
    pub unit: Option<String>,
    pub encoding: Option<WorkflowDatasetEncoding>,
    pub schema_ref: Option<OperatorSchemaRef>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct WorkflowDatasetContract {
    pub id: String,
    pub version: String,
    #[serde(default)]
    pub values: Vec<WorkflowDatasetValueInfo>,
    #[serde(default)]
    pub metadata: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OperatorDescriptor {
    pub id: String,
    pub version: String,
    pub domain: String,
    pub family: String,
    pub kind: OperatorKind,
    pub summary: String,
    pub capability_tags: Vec<String>,
    pub origin: OperatorOrigin,
    pub input_schema: OperatorSchemaRef,
    pub output_schema: OperatorSchemaRef,
    #[serde(default)]
    pub inputs: Vec<OperatorPortDescriptor>,
    #[serde(default)]
    pub outputs: Vec<OperatorPortDescriptor>,
    pub validation: OperatorValidationProfile,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct OperatorRunContext {
    pub orchestrated: bool,
    pub project_id: Option<String>,
    pub model_id: Option<String>,
    pub workflow_run_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OperatorRunRequest {
    pub operator_id: String,
    pub input: Value,
    #[serde(default)]
    pub context: OperatorRunContext,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OperatorArtifactRef {
    pub kind: String,
    pub id: String,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OperatorRunResult {
    pub operator_id: String,
    pub summary: Value,
    #[serde(default)]
    pub artifacts: Vec<OperatorArtifactRef>,
}

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
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkflowArtifactLineage {
    pub artifact_key: String,
    pub node_id: String,
    pub port_id: String,
    #[serde(default)]
    pub source_artifacts: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobStatus {
    Queued,
    Preprocessing,
    Partitioning,
    Solving,
    Postprocessing,
    Completed,
    Failed,
    Cancelled,
}

impl JobStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::Preprocessing => "preprocessing",
            Self::Partitioning => "partitioning",
            Self::Solving => "solving",
            Self::Postprocessing => "postprocessing",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Job {
    pub job_id: String,
    pub project_id: String,
    pub simulation_case_id: String,
    pub status: JobStatus,
    pub progress: f32,
    pub residual: Option<f64>,
    pub iteration: Option<u64>,
    pub worker_id: Option<String>,
}

impl Job {
    pub fn new(
        job_id: impl Into<String>,
        project_id: impl Into<String>,
        simulation_case_id: impl Into<String>,
    ) -> Self {
        Self {
            job_id: job_id.into(),
            project_id: project_id.into(),
            simulation_case_id: simulation_case_id.into(),
            status: JobStatus::Queued,
            progress: 0.0,
            residual: None,
            iteration: None,
            worker_id: None,
        }
    }

    pub fn apply_progress(&mut self, event: &ProgressEvent) {
        self.status = event.stage;
        self.progress = event.progress;
        self.residual = event.residual;
        self.iteration = event.iteration;
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProgressEvent {
    pub job_id: String,
    pub stage: JobStatus,
    pub progress: f32,
    pub residual: Option<f64>,
    pub iteration: Option<u64>,
    pub peak_memory: Option<u64>,
    pub message: Option<String>,
}

impl ProgressEvent {
    pub fn new(job_id: impl Into<String>, stage: JobStatus, progress: f32) -> Self {
        Self {
            job_id: job_id.into(),
            stage,
            progress,
            residual: None,
            iteration: None,
            peak_memory: None,
            message: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SolveBarRequest {
    pub length: f64,
    pub area: f64,
    pub youngs_modulus: f64,
    pub elements: usize,
    pub tip_force: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThermalBar1dNodeInput {
    pub id: String,
    pub x: f64,
    pub fix_x: bool,
    pub load_x: f64,
    #[serde(default)]
    pub temperature_delta: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThermalBar1dElementInput {
    pub id: String,
    pub node_i: usize,
    pub node_j: usize,
    pub area: f64,
    pub youngs_modulus: f64,
    pub thermal_expansion: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SolveThermalBar1dRequest {
    pub nodes: Vec<ThermalBar1dNodeInput>,
    pub elements: Vec<ThermalBar1dElementInput>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HeatBar1dNodeInput {
    pub id: String,
    pub x: f64,
    pub fix_temperature: bool,
    #[serde(default)]
    pub temperature: f64,
    #[serde(default)]
    pub heat_load: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HeatBar1dElementInput {
    pub id: String,
    pub node_i: usize,
    pub node_j: usize,
    pub area: f64,
    pub conductivity: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SolveHeatBar1dRequest {
    pub nodes: Vec<HeatBar1dNodeInput>,
    pub elements: Vec<HeatBar1dElementInput>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ElectrostaticBar1dNodeInput {
    pub id: String,
    pub x: f64,
    pub fix_potential: bool,
    #[serde(default)]
    pub potential: f64,
    #[serde(default)]
    pub charge_density: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ElectrostaticBar1dElementInput {
    pub id: String,
    pub node_i: usize,
    pub node_j: usize,
    pub area: f64,
    pub permittivity: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SolveElectrostaticBar1dRequest {
    pub nodes: Vec<ElectrostaticBar1dNodeInput>,
    pub elements: Vec<ElectrostaticBar1dElementInput>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HeatPlaneNodeInput {
    pub id: String,
    pub x: f64,
    pub y: f64,
    pub fix_temperature: bool,
    #[serde(default)]
    pub temperature: f64,
    #[serde(default)]
    pub heat_load: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HeatPlaneTriangleElementInput {
    pub id: String,
    pub node_i: usize,
    pub node_j: usize,
    pub node_k: usize,
    pub thickness: f64,
    pub conductivity: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SolveHeatPlaneTriangle2dRequest {
    pub nodes: Vec<HeatPlaneNodeInput>,
    pub elements: Vec<HeatPlaneTriangleElementInput>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ElectrostaticPlaneNodeInput {
    pub id: String,
    pub x: f64,
    pub y: f64,
    pub fix_potential: bool,
    #[serde(default)]
    pub potential: f64,
    #[serde(default)]
    pub charge_density: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ElectrostaticPlaneTriangleElementInput {
    pub id: String,
    pub node_i: usize,
    pub node_j: usize,
    pub node_k: usize,
    pub thickness: f64,
    pub permittivity: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SolveElectrostaticPlaneTriangle2dRequest {
    pub nodes: Vec<ElectrostaticPlaneNodeInput>,
    pub elements: Vec<ElectrostaticPlaneTriangleElementInput>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ElectrostaticPlaneQuadElementInput {
    pub id: String,
    pub node_i: usize,
    pub node_j: usize,
    pub node_k: usize,
    pub node_l: usize,
    pub thickness: f64,
    pub permittivity: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SolveElectrostaticPlaneQuad2dRequest {
    pub nodes: Vec<ElectrostaticPlaneNodeInput>,
    pub elements: Vec<ElectrostaticPlaneQuadElementInput>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HeatPlaneQuadElementInput {
    pub id: String,
    pub node_i: usize,
    pub node_j: usize,
    pub node_k: usize,
    pub node_l: usize,
    pub thickness: f64,
    pub conductivity: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SolveHeatPlaneQuad2dRequest {
    pub nodes: Vec<HeatPlaneNodeInput>,
    pub elements: Vec<HeatPlaneQuadElementInput>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThermalTruss2dNodeInput {
    pub id: String,
    pub x: f64,
    pub y: f64,
    pub fix_x: bool,
    pub fix_y: bool,
    pub load_x: f64,
    pub load_y: f64,
    #[serde(default)]
    pub temperature_delta: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThermalTruss2dElementInput {
    pub id: String,
    pub node_i: usize,
    pub node_j: usize,
    pub area: f64,
    pub youngs_modulus: f64,
    pub thermal_expansion: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SolveThermalTruss2dRequest {
    pub nodes: Vec<ThermalTruss2dNodeInput>,
    pub elements: Vec<ThermalTruss2dElementInput>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThermalTruss3dNodeInput {
    pub id: String,
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub fix_x: bool,
    pub fix_y: bool,
    pub fix_z: bool,
    pub load_x: f64,
    pub load_y: f64,
    pub load_z: f64,
    #[serde(default)]
    pub temperature_delta: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThermalTruss3dElementInput {
    pub id: String,
    pub node_i: usize,
    pub node_j: usize,
    pub area: f64,
    pub youngs_modulus: f64,
    pub thermal_expansion: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SolveThermalTruss3dRequest {
    pub nodes: Vec<ThermalTruss3dNodeInput>,
    pub elements: Vec<ThermalTruss3dElementInput>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Spring1dNodeInput {
    pub id: String,
    pub x: f64,
    pub fix_x: bool,
    pub load_x: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Spring1dElementInput {
    pub id: String,
    pub node_i: usize,
    pub node_j: usize,
    pub stiffness: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SolveSpring1dRequest {
    pub nodes: Vec<Spring1dNodeInput>,
    pub elements: Vec<Spring1dElementInput>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Spring2dNodeInput {
    pub id: String,
    pub x: f64,
    pub y: f64,
    pub fix_x: bool,
    pub fix_y: bool,
    pub load_x: f64,
    pub load_y: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Spring2dElementInput {
    pub id: String,
    pub node_i: usize,
    pub node_j: usize,
    pub stiffness: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SolveSpring2dRequest {
    pub nodes: Vec<Spring2dNodeInput>,
    pub elements: Vec<Spring2dElementInput>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Spring3dNodeInput {
    pub id: String,
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub fix_x: bool,
    pub fix_y: bool,
    pub fix_z: bool,
    pub load_x: f64,
    pub load_y: f64,
    pub load_z: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Spring3dElementInput {
    pub id: String,
    pub node_i: usize,
    pub node_j: usize,
    pub stiffness: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SolveSpring3dRequest {
    pub nodes: Vec<Spring3dNodeInput>,
    pub elements: Vec<Spring3dElementInput>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Beam1dNodeInput {
    pub id: String,
    pub x: f64,
    pub fix_y: bool,
    pub fix_rz: bool,
    pub load_y: f64,
    pub moment_z: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Beam1dElementInput {
    pub id: String,
    pub node_i: usize,
    pub node_j: usize,
    pub youngs_modulus: f64,
    pub moment_of_inertia: f64,
    pub section_modulus: f64,
    #[serde(default)]
    pub distributed_load_y: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SolveBeam1dRequest {
    pub nodes: Vec<Beam1dNodeInput>,
    pub elements: Vec<Beam1dElementInput>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThermalBeam1dNodeInput {
    pub id: String,
    pub x: f64,
    pub fix_y: bool,
    pub fix_rz: bool,
    pub load_y: f64,
    pub moment_z: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThermalBeam1dElementInput {
    pub id: String,
    pub node_i: usize,
    pub node_j: usize,
    pub youngs_modulus: f64,
    pub moment_of_inertia: f64,
    pub section_modulus: f64,
    pub thermal_expansion: f64,
    pub section_depth: f64,
    #[serde(default)]
    pub distributed_load_y: f64,
    #[serde(default)]
    pub temperature_gradient_y: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SolveThermalBeam1dRequest {
    pub nodes: Vec<ThermalBeam1dNodeInput>,
    pub elements: Vec<ThermalBeam1dElementInput>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Torsion1dNodeInput {
    pub id: String,
    pub x: f64,
    pub fix_rz: bool,
    pub torque_z: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Torsion1dElementInput {
    pub id: String,
    pub node_i: usize,
    pub node_j: usize,
    pub shear_modulus: f64,
    pub polar_moment: f64,
    pub section_modulus: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SolveTorsion1dRequest {
    pub nodes: Vec<Torsion1dNodeInput>,
    pub elements: Vec<Torsion1dElementInput>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NodeResult {
    pub index: usize,
    pub x: f64,
    pub displacement: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ElementResult {
    pub index: usize,
    pub x1: f64,
    pub x2: f64,
    pub strain: f64,
    pub stress: f64,
    pub axial_force: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SolveBarResult {
    pub input: SolveBarRequest,
    pub nodes: Vec<NodeResult>,
    pub elements: Vec<ElementResult>,
    pub tip_displacement: f64,
    pub reaction_force: f64,
    pub max_displacement: f64,
    pub max_stress: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThermalBar1dNodeResult {
    pub index: usize,
    pub id: String,
    pub x: f64,
    pub ux: f64,
    pub temperature_delta: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThermalBar1dElementResult {
    pub index: usize,
    pub id: String,
    pub node_i: usize,
    pub node_j: usize,
    pub length: f64,
    pub average_temperature_delta: f64,
    pub thermal_strain: f64,
    pub mechanical_strain: f64,
    pub total_strain: f64,
    pub stress: f64,
    pub axial_force: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SolveThermalBar1dResult {
    pub input: SolveThermalBar1dRequest,
    pub nodes: Vec<ThermalBar1dNodeResult>,
    pub elements: Vec<ThermalBar1dElementResult>,
    pub max_displacement: f64,
    pub max_stress: f64,
    pub max_axial_force: f64,
    pub max_temperature_delta: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HeatBar1dNodeResult {
    pub index: usize,
    pub id: String,
    pub x: f64,
    pub temperature: f64,
    pub heat_load: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HeatBar1dElementResult {
    pub index: usize,
    pub id: String,
    pub node_i: usize,
    pub node_j: usize,
    pub length: f64,
    pub average_temperature: f64,
    pub temperature_gradient: f64,
    pub heat_flux: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SolveHeatBar1dResult {
    pub input: SolveHeatBar1dRequest,
    pub nodes: Vec<HeatBar1dNodeResult>,
    pub elements: Vec<HeatBar1dElementResult>,
    pub max_temperature: f64,
    pub max_heat_flux: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ElectrostaticBar1dNodeResult {
    pub index: usize,
    pub id: String,
    pub x: f64,
    pub potential: f64,
    pub charge_density: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ElectrostaticBar1dElementResult {
    pub index: usize,
    pub id: String,
    pub node_i: usize,
    pub node_j: usize,
    pub length: f64,
    pub average_potential: f64,
    pub potential_gradient: f64,
    pub electric_field: f64,
    pub electric_flux_density: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SolveElectrostaticBar1dResult {
    pub input: SolveElectrostaticBar1dRequest,
    pub nodes: Vec<ElectrostaticBar1dNodeResult>,
    pub elements: Vec<ElectrostaticBar1dElementResult>,
    pub max_potential: f64,
    pub max_electric_field: f64,
    pub max_flux_density: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HeatPlaneNodeResult {
    pub index: usize,
    pub id: String,
    pub x: f64,
    pub y: f64,
    pub temperature: f64,
    pub heat_load: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HeatPlaneTriangleElementResult {
    pub index: usize,
    pub id: String,
    pub node_i: usize,
    pub node_j: usize,
    pub node_k: usize,
    pub area: f64,
    pub average_temperature: f64,
    pub temperature_gradient_x: f64,
    pub temperature_gradient_y: f64,
    pub heat_flux_x: f64,
    pub heat_flux_y: f64,
    pub heat_flux_magnitude: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SolveHeatPlaneTriangle2dResult {
    pub input: SolveHeatPlaneTriangle2dRequest,
    pub nodes: Vec<HeatPlaneNodeResult>,
    pub elements: Vec<HeatPlaneTriangleElementResult>,
    pub max_temperature: f64,
    pub max_heat_flux: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ElectrostaticPlaneNodeResult {
    pub index: usize,
    pub id: String,
    pub x: f64,
    pub y: f64,
    pub potential: f64,
    pub charge_density: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ElectrostaticPlaneTriangleElementResult {
    pub index: usize,
    pub id: String,
    pub node_i: usize,
    pub node_j: usize,
    pub node_k: usize,
    pub area: f64,
    pub average_potential: f64,
    pub potential_gradient_x: f64,
    pub potential_gradient_y: f64,
    pub electric_field_x: f64,
    pub electric_field_y: f64,
    pub electric_field_magnitude: f64,
    pub electric_flux_density_x: f64,
    pub electric_flux_density_y: f64,
    pub electric_flux_density_magnitude: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SolveElectrostaticPlaneTriangle2dResult {
    pub input: SolveElectrostaticPlaneTriangle2dRequest,
    pub nodes: Vec<ElectrostaticPlaneNodeResult>,
    pub elements: Vec<ElectrostaticPlaneTriangleElementResult>,
    pub max_potential: f64,
    pub max_electric_field: f64,
    pub max_flux_density: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ElectrostaticPlaneQuadElementResult {
    pub index: usize,
    pub id: String,
    pub node_i: usize,
    pub node_j: usize,
    pub node_k: usize,
    pub node_l: usize,
    pub area: f64,
    pub average_potential: f64,
    pub potential_gradient_x: f64,
    pub potential_gradient_y: f64,
    pub electric_field_x: f64,
    pub electric_field_y: f64,
    pub electric_field_magnitude: f64,
    pub electric_flux_density_x: f64,
    pub electric_flux_density_y: f64,
    pub electric_flux_density_magnitude: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SolveElectrostaticPlaneQuad2dResult {
    pub input: SolveElectrostaticPlaneQuad2dRequest,
    pub nodes: Vec<ElectrostaticPlaneNodeResult>,
    pub elements: Vec<ElectrostaticPlaneQuadElementResult>,
    pub max_potential: f64,
    pub max_electric_field: f64,
    pub max_flux_density: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HeatPlaneQuadElementResult {
    pub index: usize,
    pub id: String,
    pub node_i: usize,
    pub node_j: usize,
    pub node_k: usize,
    pub node_l: usize,
    pub area: f64,
    pub average_temperature: f64,
    pub temperature_gradient_x: f64,
    pub temperature_gradient_y: f64,
    pub heat_flux_x: f64,
    pub heat_flux_y: f64,
    pub heat_flux_magnitude: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SolveHeatPlaneQuad2dResult {
    pub input: SolveHeatPlaneQuad2dRequest,
    pub nodes: Vec<HeatPlaneNodeResult>,
    pub elements: Vec<HeatPlaneQuadElementResult>,
    pub max_temperature: f64,
    pub max_heat_flux: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThermalTruss2dNodeResult {
    pub index: usize,
    pub id: String,
    pub x: f64,
    pub y: f64,
    pub ux: f64,
    pub uy: f64,
    pub temperature_delta: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThermalTruss2dElementResult {
    pub index: usize,
    pub id: String,
    pub node_i: usize,
    pub node_j: usize,
    pub length: f64,
    pub average_temperature_delta: f64,
    pub thermal_strain: f64,
    pub mechanical_strain: f64,
    pub total_strain: f64,
    pub stress: f64,
    pub axial_force: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SolveThermalTruss2dResult {
    pub input: SolveThermalTruss2dRequest,
    pub nodes: Vec<ThermalTruss2dNodeResult>,
    pub elements: Vec<ThermalTruss2dElementResult>,
    pub max_displacement: f64,
    pub max_stress: f64,
    pub max_axial_force: f64,
    pub max_temperature_delta: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThermalTruss3dNodeResult {
    pub index: usize,
    pub id: String,
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub ux: f64,
    pub uy: f64,
    pub uz: f64,
    pub temperature_delta: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThermalTruss3dElementResult {
    pub index: usize,
    pub id: String,
    pub node_i: usize,
    pub node_j: usize,
    pub length: f64,
    pub average_temperature_delta: f64,
    pub thermal_strain: f64,
    pub mechanical_strain: f64,
    pub total_strain: f64,
    pub stress: f64,
    pub axial_force: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SolveThermalTruss3dResult {
    pub input: SolveThermalTruss3dRequest,
    pub nodes: Vec<ThermalTruss3dNodeResult>,
    pub elements: Vec<ThermalTruss3dElementResult>,
    pub max_displacement: f64,
    pub max_stress: f64,
    pub max_axial_force: f64,
    pub max_temperature_delta: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Spring1dNodeResult {
    pub index: usize,
    pub id: String,
    pub x: f64,
    pub ux: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Spring1dElementResult {
    pub index: usize,
    pub id: String,
    pub node_i: usize,
    pub node_j: usize,
    pub length: f64,
    pub extension: f64,
    pub force: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SolveSpring1dResult {
    pub input: SolveSpring1dRequest,
    pub nodes: Vec<Spring1dNodeResult>,
    pub elements: Vec<Spring1dElementResult>,
    pub max_displacement: f64,
    pub max_force: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Spring2dNodeResult {
    pub index: usize,
    pub id: String,
    pub x: f64,
    pub y: f64,
    pub ux: f64,
    pub uy: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Spring2dElementResult {
    pub index: usize,
    pub id: String,
    pub node_i: usize,
    pub node_j: usize,
    pub length: f64,
    pub extension: f64,
    pub force: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SolveSpring2dResult {
    pub input: SolveSpring2dRequest,
    pub nodes: Vec<Spring2dNodeResult>,
    pub elements: Vec<Spring2dElementResult>,
    pub max_displacement: f64,
    pub max_force: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Spring3dNodeResult {
    pub index: usize,
    pub id: String,
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub ux: f64,
    pub uy: f64,
    pub uz: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Spring3dElementResult {
    pub index: usize,
    pub id: String,
    pub node_i: usize,
    pub node_j: usize,
    pub length: f64,
    pub extension: f64,
    pub force: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SolveSpring3dResult {
    pub input: SolveSpring3dRequest,
    pub nodes: Vec<Spring3dNodeResult>,
    pub elements: Vec<Spring3dElementResult>,
    pub max_displacement: f64,
    pub max_force: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Beam1dNodeResult {
    pub index: usize,
    pub id: String,
    pub x: f64,
    pub uy: f64,
    pub rz: f64,
    pub displacement_magnitude: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Beam1dElementResult {
    pub index: usize,
    pub id: String,
    pub node_i: usize,
    pub node_j: usize,
    pub length: f64,
    pub shear_force_i: f64,
    pub moment_i: f64,
    pub shear_force_j: f64,
    pub moment_j: f64,
    pub max_bending_stress: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SolveBeam1dResult {
    pub input: SolveBeam1dRequest,
    pub nodes: Vec<Beam1dNodeResult>,
    pub elements: Vec<Beam1dElementResult>,
    pub max_displacement: f64,
    pub max_rotation: f64,
    pub max_moment: f64,
    pub max_stress: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThermalBeam1dNodeResult {
    pub index: usize,
    pub id: String,
    pub x: f64,
    pub uy: f64,
    pub rz: f64,
    pub displacement_magnitude: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThermalBeam1dElementResult {
    pub index: usize,
    pub id: String,
    pub node_i: usize,
    pub node_j: usize,
    pub length: f64,
    pub temperature_gradient_y: f64,
    pub thermal_curvature: f64,
    pub shear_force_i: f64,
    pub moment_i: f64,
    pub shear_force_j: f64,
    pub moment_j: f64,
    pub max_bending_stress: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SolveThermalBeam1dResult {
    pub input: SolveThermalBeam1dRequest,
    pub nodes: Vec<ThermalBeam1dNodeResult>,
    pub elements: Vec<ThermalBeam1dElementResult>,
    pub max_displacement: f64,
    pub max_rotation: f64,
    pub max_moment: f64,
    pub max_stress: f64,
    pub max_temperature_gradient: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Torsion1dNodeResult {
    pub index: usize,
    pub id: String,
    pub x: f64,
    pub rz: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Torsion1dElementResult {
    pub index: usize,
    pub id: String,
    pub node_i: usize,
    pub node_j: usize,
    pub length: f64,
    pub twist: f64,
    pub torque: f64,
    pub shear_stress: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SolveTorsion1dResult {
    pub input: SolveTorsion1dRequest,
    pub nodes: Vec<Torsion1dNodeResult>,
    pub elements: Vec<Torsion1dElementResult>,
    pub max_rotation: f64,
    pub max_torque: f64,
    pub max_stress: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TransportDescriptor {
    pub kind: String,
    pub framing: Option<String>,
    pub encoding: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CapabilityDescriptor {
    pub id: String,
    pub role: String,
    pub methods: Vec<RpcMethod>,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClusterPeerDescriptor {
    pub address: String,
    pub status: String,
    pub failure_count: u32,
    pub last_seen_unix_s: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentClusterDescriptor {
    pub cluster_id: Option<String>,
    pub runtime_mode: String,
    pub headless: bool,
    pub cluster_size: usize,
    pub health_score: u8,
    pub peers: Vec<ClusterPeerDescriptor>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RuntimeAuthorityDescriptor {
    pub control_mode: String,
    pub authority_mode: String,
    pub orchestrator_id: Option<String>,
    pub orchestrator_session_id: Option<String>,
    pub accepts_multi_orchestrator_binding: bool,
    pub agent_library_replication: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RpcProtocolDescriptor {
    pub name: String,
    pub rpc_version: u8,
    pub transport: TransportDescriptor,
    pub methods: Vec<RpcMethod>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentDescriptor {
    pub program: String,
    pub role: String,
    pub protocol: RpcProtocolDescriptor,
    pub capabilities: Vec<CapabilityDescriptor>,
    pub deployment_modes: Vec<String>,
    pub runtime: AgentClusterDescriptor,
    pub authority: RuntimeAuthorityDescriptor,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RpcMethod {
    #[serde(rename = "ping")]
    Ping,
    #[serde(rename = "describe_agent")]
    DescribeAgent,
    #[serde(rename = "solve_bar_1d")]
    SolveBar1d,
    #[serde(rename = "solve_thermal_bar_1d")]
    SolveThermalBar1d,
    #[serde(rename = "solve_heat_bar_1d")]
    SolveHeatBar1d,
    #[serde(rename = "solve_electrostatic_bar_1d")]
    SolveElectrostaticBar1d,
    #[serde(rename = "solve_electrostatic_plane_triangle_2d")]
    SolveElectrostaticPlaneTriangle2d,
    #[serde(rename = "solve_electrostatic_plane_quad_2d")]
    SolveElectrostaticPlaneQuad2d,
    #[serde(rename = "solve_heat_plane_triangle_2d")]
    SolveHeatPlaneTriangle2d,
    #[serde(rename = "solve_heat_plane_quad_2d")]
    SolveHeatPlaneQuad2d,
    #[serde(rename = "solve_thermal_truss_2d")]
    SolveThermalTruss2d,
    #[serde(rename = "solve_thermal_truss_3d")]
    SolveThermalTruss3d,
    #[serde(rename = "solve_spring_1d")]
    SolveSpring1d,
    #[serde(rename = "solve_spring_2d")]
    SolveSpring2d,
    #[serde(rename = "solve_spring_3d")]
    SolveSpring3d,
    #[serde(rename = "solve_beam_1d")]
    SolveBeam1d,
    #[serde(rename = "solve_thermal_beam_1d")]
    SolveThermalBeam1d,
    #[serde(rename = "solve_torsion_1d")]
    SolveTorsion1d,
    #[serde(rename = "solve_truss_2d")]
    SolveTruss2d,
    #[serde(rename = "solve_truss_3d")]
    SolveTruss3d,
    #[serde(rename = "solve_frame_3d")]
    SolveFrame3d,
    #[serde(rename = "solve_plane_triangle_2d")]
    SolvePlaneTriangle2d,
    #[serde(rename = "solve_thermal_plane_triangle_2d")]
    SolveThermalPlaneTriangle2d,
    #[serde(rename = "solve_plane_quad_2d")]
    SolvePlaneQuad2d,
    #[serde(rename = "solve_thermal_plane_quad_2d")]
    SolveThermalPlaneQuad2d,
    #[serde(rename = "solve_frame_2d")]
    SolveFrame2d,
    #[serde(rename = "solve_thermal_frame_2d")]
    SolveThermalFrame2d,
    #[serde(rename = "solve_thermal_frame_3d")]
    SolveThermalFrame3d,
    #[serde(rename = "cancel_job")]
    CancelJob,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RpcRequest {
    pub rpc_version: u8,
    pub id: String,
    pub method: RpcMethod,
    pub params: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RpcError {
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RpcProgress {
    pub rpc_version: u8,
    pub id: String,
    pub event: String,
    pub progress: ProgressEvent,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RpcResponse {
    pub rpc_version: u8,
    pub id: String,
    pub ok: bool,
    pub result: Option<Value>,
    pub error: Option<RpcError>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CancelJobRequest {
    pub job_id: String,
}

impl RpcProgress {
    pub fn new(id: impl Into<String>, progress: ProgressEvent) -> Self {
        Self {
            rpc_version: RPC_VERSION,
            id: id.into(),
            event: "progress".to_string(),
            progress,
        }
    }

    pub fn heartbeat(id: impl Into<String>, progress: ProgressEvent) -> Self {
        Self {
            rpc_version: RPC_VERSION,
            id: id.into(),
            event: "heartbeat".to_string(),
            progress,
        }
    }
}

impl RpcResponse {
    pub fn success(id: impl Into<String>, result: Value) -> Self {
        Self {
            rpc_version: RPC_VERSION,
            id: id.into(),
            ok: true,
            result: Some(result),
            error: None,
        }
    }

    pub fn error(
        id: impl Into<String>,
        code: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            rpc_version: RPC_VERSION,
            id: id.into(),
            ok: false,
            result: None,
            error: Some(RpcError {
                code: code.into(),
                message: message.into(),
            }),
        }
    }
}

impl RpcProtocolDescriptor {
    pub fn solver_agent_default() -> Self {
        Self {
            name: SOLVER_RPC_PROTOCOL.to_string(),
            rpc_version: RPC_VERSION,
            transport: TransportDescriptor {
                kind: "tcp".to_string(),
                framing: Some("length_prefixed_u32".to_string()),
                encoding: "json".to_string(),
            },
            methods: vec![
                RpcMethod::Ping,
                RpcMethod::DescribeAgent,
                RpcMethod::SolveBar1d,
                RpcMethod::SolveThermalBar1d,
                RpcMethod::SolveHeatBar1d,
                RpcMethod::SolveElectrostaticBar1d,
                RpcMethod::SolveElectrostaticPlaneTriangle2d,
                RpcMethod::SolveElectrostaticPlaneQuad2d,
                RpcMethod::SolveHeatPlaneTriangle2d,
                RpcMethod::SolveHeatPlaneQuad2d,
                RpcMethod::SolveThermalTruss2d,
                RpcMethod::SolveThermalTruss3d,
                RpcMethod::SolveSpring1d,
                RpcMethod::SolveSpring2d,
                RpcMethod::SolveSpring3d,
                RpcMethod::SolveBeam1d,
                RpcMethod::SolveThermalBeam1d,
                RpcMethod::SolveTorsion1d,
                RpcMethod::SolveTruss2d,
                RpcMethod::SolveTruss3d,
                RpcMethod::SolveFrame3d,
                RpcMethod::SolvePlaneTriangle2d,
                RpcMethod::SolveThermalPlaneTriangle2d,
                RpcMethod::SolvePlaneQuad2d,
                RpcMethod::SolveThermalPlaneQuad2d,
                RpcMethod::SolveFrame2d,
                RpcMethod::SolveThermalFrame2d,
                RpcMethod::SolveThermalFrame3d,
                RpcMethod::CancelJob,
            ],
        }
    }
}

impl AgentDescriptor {
    pub fn solver_agent_default() -> Self {
        Self {
            program: "kyuubiki-rust-agent".to_string(),
            role: "solver_agent".to_string(),
            protocol: RpcProtocolDescriptor::solver_agent_default(),
            capabilities: vec![
                CapabilityDescriptor {
                    id: "bar-1d".to_string(),
                    role: "solver".to_string(),
                    methods: vec![RpcMethod::SolveBar1d],
                    tags: vec!["bar".to_string(), "cpu".to_string()],
                },
                CapabilityDescriptor {
                    id: "thermal-bar-1d".to_string(),
                    role: "solver".to_string(),
                    methods: vec![RpcMethod::SolveThermalBar1d],
                    tags: vec![
                        "bar".to_string(),
                        "thermal".to_string(),
                        "line".to_string(),
                        "cpu".to_string(),
                    ],
                },
                CapabilityDescriptor {
                    id: "heat-bar-1d".to_string(),
                    role: "solver".to_string(),
                    methods: vec![RpcMethod::SolveHeatBar1d],
                    tags: vec![
                        "heat".to_string(),
                        "bar".to_string(),
                        "line".to_string(),
                        "cpu".to_string(),
                    ],
                },
                CapabilityDescriptor {
                    id: "electrostatic-bar-1d".to_string(),
                    role: "solver".to_string(),
                    methods: vec![RpcMethod::SolveElectrostaticBar1d],
                    tags: vec![
                        "electromagnetic".to_string(),
                        "electrostatic".to_string(),
                        "bar".to_string(),
                        "line".to_string(),
                        "cpu".to_string(),
                    ],
                },
                CapabilityDescriptor {
                    id: "electrostatic-plane-triangle-2d".to_string(),
                    role: "solver".to_string(),
                    methods: vec![RpcMethod::SolveElectrostaticPlaneTriangle2d],
                    tags: vec![
                        "electromagnetic".to_string(),
                        "electrostatic".to_string(),
                        "plane".to_string(),
                        "triangle".to_string(),
                        "2d".to_string(),
                        "cpu".to_string(),
                    ],
                },
                CapabilityDescriptor {
                    id: "electrostatic-plane-quad-2d".to_string(),
                    role: "solver".to_string(),
                    methods: vec![RpcMethod::SolveElectrostaticPlaneQuad2d],
                    tags: vec![
                        "electromagnetic".to_string(),
                        "electrostatic".to_string(),
                        "plane".to_string(),
                        "quad".to_string(),
                        "2d".to_string(),
                        "cpu".to_string(),
                    ],
                },
                CapabilityDescriptor {
                    id: "heat-plane-triangle-2d".to_string(),
                    role: "solver".to_string(),
                    methods: vec![RpcMethod::SolveHeatPlaneTriangle2d],
                    tags: vec![
                        "heat".to_string(),
                        "plane".to_string(),
                        "mesh".to_string(),
                        "cpu".to_string(),
                    ],
                },
                CapabilityDescriptor {
                    id: "heat-plane-quad-2d".to_string(),
                    role: "solver".to_string(),
                    methods: vec![RpcMethod::SolveHeatPlaneQuad2d],
                    tags: vec![
                        "heat".to_string(),
                        "plane".to_string(),
                        "mesh".to_string(),
                        "quad".to_string(),
                        "cpu".to_string(),
                    ],
                },
                CapabilityDescriptor {
                    id: "thermal-truss-2d".to_string(),
                    role: "solver".to_string(),
                    methods: vec![RpcMethod::SolveThermalTruss2d],
                    tags: vec![
                        "truss".to_string(),
                        "thermal".to_string(),
                        "plane".to_string(),
                        "cpu".to_string(),
                    ],
                },
                CapabilityDescriptor {
                    id: "thermal-truss-3d".to_string(),
                    role: "solver".to_string(),
                    methods: vec![RpcMethod::SolveThermalTruss3d],
                    tags: vec![
                        "truss".to_string(),
                        "thermal".to_string(),
                        "space".to_string(),
                        "cpu".to_string(),
                    ],
                },
                CapabilityDescriptor {
                    id: "spring-1d".to_string(),
                    role: "solver".to_string(),
                    methods: vec![RpcMethod::SolveSpring1d],
                    tags: vec![
                        "spring".to_string(),
                        "line".to_string(),
                        "support".to_string(),
                        "cpu".to_string(),
                    ],
                },
                CapabilityDescriptor {
                    id: "spring-2d".to_string(),
                    role: "solver".to_string(),
                    methods: vec![RpcMethod::SolveSpring2d],
                    tags: vec![
                        "spring".to_string(),
                        "plane".to_string(),
                        "support".to_string(),
                        "cpu".to_string(),
                    ],
                },
                CapabilityDescriptor {
                    id: "spring-3d".to_string(),
                    role: "solver".to_string(),
                    methods: vec![RpcMethod::SolveSpring3d],
                    tags: vec![
                        "spring".to_string(),
                        "space".to_string(),
                        "support".to_string(),
                        "cpu".to_string(),
                    ],
                },
                CapabilityDescriptor {
                    id: "beam-1d".to_string(),
                    role: "solver".to_string(),
                    methods: vec![RpcMethod::SolveBeam1d],
                    tags: vec![
                        "beam".to_string(),
                        "bending".to_string(),
                        "line".to_string(),
                        "cpu".to_string(),
                    ],
                },
                CapabilityDescriptor {
                    id: "thermal-beam-1d".to_string(),
                    role: "solver".to_string(),
                    methods: vec![RpcMethod::SolveThermalBeam1d],
                    tags: vec![
                        "beam".to_string(),
                        "thermal".to_string(),
                        "bending".to_string(),
                        "line".to_string(),
                        "cpu".to_string(),
                    ],
                },
                CapabilityDescriptor {
                    id: "thermal-frame-2d".to_string(),
                    role: "solver".to_string(),
                    methods: vec![RpcMethod::SolveThermalFrame2d],
                    tags: vec![
                        "frame".to_string(),
                        "thermal".to_string(),
                        "beam".to_string(),
                        "bending".to_string(),
                        "cpu".to_string(),
                    ],
                },
                CapabilityDescriptor {
                    id: "thermal-frame-3d".to_string(),
                    role: "solver".to_string(),
                    methods: vec![RpcMethod::SolveThermalFrame3d],
                    tags: vec![
                        "frame".to_string(),
                        "space".to_string(),
                        "thermal".to_string(),
                        "beam".to_string(),
                        "bending".to_string(),
                        "cpu".to_string(),
                    ],
                },
                CapabilityDescriptor {
                    id: "torsion-1d".to_string(),
                    role: "solver".to_string(),
                    methods: vec![RpcMethod::SolveTorsion1d],
                    tags: vec![
                        "torsion".to_string(),
                        "shaft".to_string(),
                        "line".to_string(),
                        "cpu".to_string(),
                    ],
                },
                CapabilityDescriptor {
                    id: "truss-2d".to_string(),
                    role: "solver".to_string(),
                    methods: vec![RpcMethod::SolveTruss2d],
                    tags: vec!["truss".to_string(), "cpu".to_string()],
                },
                CapabilityDescriptor {
                    id: "truss-3d".to_string(),
                    role: "solver".to_string(),
                    methods: vec![RpcMethod::SolveTruss3d],
                    tags: vec!["truss".to_string(), "space".to_string(), "cpu".to_string()],
                },
                CapabilityDescriptor {
                    id: "frame-3d".to_string(),
                    role: "solver".to_string(),
                    methods: vec![RpcMethod::SolveFrame3d],
                    tags: vec![
                        "frame".to_string(),
                        "space".to_string(),
                        "beam".to_string(),
                        "bending".to_string(),
                        "cpu".to_string(),
                    ],
                },
                CapabilityDescriptor {
                    id: "plane-triangle-2d".to_string(),
                    role: "solver".to_string(),
                    methods: vec![RpcMethod::SolvePlaneTriangle2d],
                    tags: vec!["plane".to_string(), "mesh".to_string(), "cpu".to_string()],
                },
                CapabilityDescriptor {
                    id: "thermal-plane-triangle-2d".to_string(),
                    role: "solver".to_string(),
                    methods: vec![RpcMethod::SolveThermalPlaneTriangle2d],
                    tags: vec![
                        "plane".to_string(),
                        "thermal".to_string(),
                        "mesh".to_string(),
                        "cpu".to_string(),
                    ],
                },
                CapabilityDescriptor {
                    id: "plane-quad-2d".to_string(),
                    role: "solver".to_string(),
                    methods: vec![RpcMethod::SolvePlaneQuad2d],
                    tags: vec![
                        "plane".to_string(),
                        "mesh".to_string(),
                        "quad".to_string(),
                        "cpu".to_string(),
                    ],
                },
                CapabilityDescriptor {
                    id: "thermal-plane-quad-2d".to_string(),
                    role: "solver".to_string(),
                    methods: vec![RpcMethod::SolveThermalPlaneQuad2d],
                    tags: vec![
                        "plane".to_string(),
                        "thermal".to_string(),
                        "mesh".to_string(),
                        "quad".to_string(),
                        "cpu".to_string(),
                    ],
                },
                CapabilityDescriptor {
                    id: "frame-2d".to_string(),
                    role: "solver".to_string(),
                    methods: vec![RpcMethod::SolveFrame2d],
                    tags: vec![
                        "frame".to_string(),
                        "beam".to_string(),
                        "bending".to_string(),
                        "cpu".to_string(),
                    ],
                },
                CapabilityDescriptor {
                    id: "control".to_string(),
                    role: "runtime".to_string(),
                    methods: vec![
                        RpcMethod::Ping,
                        RpcMethod::DescribeAgent,
                        RpcMethod::CancelJob,
                    ],
                    tags: vec!["control".to_string(), "general".to_string()],
                },
            ],
            deployment_modes: vec![
                "local".to_string(),
                "cloud".to_string(),
                "distributed".to_string(),
            ],
            runtime: AgentClusterDescriptor {
                cluster_id: None,
                runtime_mode: "standalone".to_string(),
                headless: true,
                cluster_size: 1,
                health_score: 100,
                peers: vec![],
            },
            authority: RuntimeAuthorityDescriptor {
                control_mode: "standalone".to_string(),
                authority_mode: "self_directed".to_string(),
                orchestrator_id: None,
                orchestrator_session_id: None,
                accepts_multi_orchestrator_binding: false,
                agent_library_replication: "central_fetch".to_string(),
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrussNodeInput {
    pub id: String,
    pub x: f64,
    pub y: f64,
    pub fix_x: bool,
    pub fix_y: bool,
    pub load_x: f64,
    pub load_y: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrussElementInput {
    pub id: String,
    pub node_i: usize,
    pub node_j: usize,
    pub area: f64,
    pub youngs_modulus: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SolveTruss2dRequest {
    pub nodes: Vec<TrussNodeInput>,
    pub elements: Vec<TrussElementInput>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrussNodeResult {
    pub index: usize,
    pub id: String,
    pub x: f64,
    pub y: f64,
    pub ux: f64,
    pub uy: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrussElementResult {
    pub index: usize,
    pub id: String,
    pub node_i: usize,
    pub node_j: usize,
    pub length: f64,
    pub strain: f64,
    pub stress: f64,
    pub axial_force: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SolveTruss2dResult {
    pub input: SolveTruss2dRequest,
    pub nodes: Vec<TrussNodeResult>,
    pub elements: Vec<TrussElementResult>,
    pub max_displacement: f64,
    pub max_stress: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Truss3dNodeInput {
    pub id: String,
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub fix_x: bool,
    pub fix_y: bool,
    pub fix_z: bool,
    pub load_x: f64,
    pub load_y: f64,
    pub load_z: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Truss3dElementInput {
    pub id: String,
    pub node_i: usize,
    pub node_j: usize,
    pub area: f64,
    pub youngs_modulus: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SolveTruss3dRequest {
    pub nodes: Vec<Truss3dNodeInput>,
    pub elements: Vec<Truss3dElementInput>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Truss3dNodeResult {
    pub index: usize,
    pub id: String,
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub ux: f64,
    pub uy: f64,
    pub uz: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Truss3dElementResult {
    pub index: usize,
    pub id: String,
    pub node_i: usize,
    pub node_j: usize,
    pub length: f64,
    pub strain: f64,
    pub stress: f64,
    pub axial_force: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SolveTruss3dResult {
    pub input: SolveTruss3dRequest,
    pub nodes: Vec<Truss3dNodeResult>,
    pub elements: Vec<Truss3dElementResult>,
    pub max_displacement: f64,
    pub max_stress: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Frame3dNodeInput {
    pub id: String,
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub fix_x: bool,
    pub fix_y: bool,
    pub fix_z: bool,
    pub fix_rx: bool,
    pub fix_ry: bool,
    pub fix_rz: bool,
    pub load_x: f64,
    pub load_y: f64,
    pub load_z: f64,
    pub moment_x: f64,
    pub moment_y: f64,
    pub moment_z: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Frame3dElementInput {
    pub id: String,
    pub node_i: usize,
    pub node_j: usize,
    pub area: f64,
    pub youngs_modulus: f64,
    pub shear_modulus: f64,
    pub torsion_constant: f64,
    pub moment_of_inertia_y: f64,
    pub moment_of_inertia_z: f64,
    pub section_modulus_y: f64,
    pub section_modulus_z: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SolveFrame3dRequest {
    pub nodes: Vec<Frame3dNodeInput>,
    pub elements: Vec<Frame3dElementInput>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Frame3dNodeResult {
    pub index: usize,
    pub id: String,
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub ux: f64,
    pub uy: f64,
    pub uz: f64,
    pub rx: f64,
    pub ry: f64,
    pub rz: f64,
    pub displacement_magnitude: f64,
    pub rotation_magnitude: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Frame3dElementResult {
    pub index: usize,
    pub id: String,
    pub node_i: usize,
    pub node_j: usize,
    pub length: f64,
    pub axial_force_i: f64,
    pub shear_force_y_i: f64,
    pub shear_force_z_i: f64,
    pub torsion_i: f64,
    pub moment_y_i: f64,
    pub moment_z_i: f64,
    pub axial_force_j: f64,
    pub shear_force_y_j: f64,
    pub shear_force_z_j: f64,
    pub torsion_j: f64,
    pub moment_y_j: f64,
    pub moment_z_j: f64,
    pub axial_stress: f64,
    pub max_bending_stress: f64,
    pub max_combined_stress: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SolveFrame3dResult {
    pub input: SolveFrame3dRequest,
    pub nodes: Vec<Frame3dNodeResult>,
    pub elements: Vec<Frame3dElementResult>,
    pub max_displacement: f64,
    pub max_rotation: f64,
    pub max_moment: f64,
    pub max_stress: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlaneNodeInput {
    pub id: String,
    pub x: f64,
    pub y: f64,
    pub fix_x: bool,
    pub fix_y: bool,
    pub load_x: f64,
    pub load_y: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThermalPlaneNodeInput {
    pub id: String,
    pub x: f64,
    pub y: f64,
    pub fix_x: bool,
    pub fix_y: bool,
    pub load_x: f64,
    pub load_y: f64,
    #[serde(default)]
    pub temperature_delta: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlaneTriangleElementInput {
    pub id: String,
    pub node_i: usize,
    pub node_j: usize,
    pub node_k: usize,
    pub thickness: f64,
    pub youngs_modulus: f64,
    pub poisson_ratio: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SolvePlaneTriangle2dRequest {
    pub nodes: Vec<PlaneNodeInput>,
    pub elements: Vec<PlaneTriangleElementInput>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThermalPlaneTriangleElementInput {
    pub id: String,
    pub node_i: usize,
    pub node_j: usize,
    pub node_k: usize,
    pub thickness: f64,
    pub youngs_modulus: f64,
    pub poisson_ratio: f64,
    pub thermal_expansion: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SolveThermalPlaneTriangle2dRequest {
    pub nodes: Vec<ThermalPlaneNodeInput>,
    pub elements: Vec<ThermalPlaneTriangleElementInput>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlaneQuadElementInput {
    pub id: String,
    pub node_i: usize,
    pub node_j: usize,
    pub node_k: usize,
    pub node_l: usize,
    pub thickness: f64,
    pub youngs_modulus: f64,
    pub poisson_ratio: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SolvePlaneQuad2dRequest {
    pub nodes: Vec<PlaneNodeInput>,
    pub elements: Vec<PlaneQuadElementInput>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThermalPlaneQuadElementInput {
    pub id: String,
    pub node_i: usize,
    pub node_j: usize,
    pub node_k: usize,
    pub node_l: usize,
    pub thickness: f64,
    pub youngs_modulus: f64,
    pub poisson_ratio: f64,
    pub thermal_expansion: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SolveThermalPlaneQuad2dRequest {
    pub nodes: Vec<ThermalPlaneNodeInput>,
    pub elements: Vec<ThermalPlaneQuadElementInput>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Frame2dNodeInput {
    pub id: String,
    pub x: f64,
    pub y: f64,
    pub fix_x: bool,
    pub fix_y: bool,
    pub fix_rz: bool,
    pub load_x: f64,
    pub load_y: f64,
    pub moment_z: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Frame2dElementInput {
    pub id: String,
    pub node_i: usize,
    pub node_j: usize,
    pub area: f64,
    pub youngs_modulus: f64,
    pub moment_of_inertia: f64,
    pub section_modulus: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SolveFrame2dRequest {
    pub nodes: Vec<Frame2dNodeInput>,
    pub elements: Vec<Frame2dElementInput>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThermalFrame2dNodeInput {
    pub id: String,
    pub x: f64,
    pub y: f64,
    pub fix_x: bool,
    pub fix_y: bool,
    pub fix_rz: bool,
    pub load_x: f64,
    pub load_y: f64,
    pub moment_z: f64,
    #[serde(default)]
    pub temperature_delta: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThermalFrame2dElementInput {
    pub id: String,
    pub node_i: usize,
    pub node_j: usize,
    pub area: f64,
    pub youngs_modulus: f64,
    pub moment_of_inertia: f64,
    pub section_modulus: f64,
    pub thermal_expansion: f64,
    pub section_depth: f64,
    #[serde(default)]
    pub temperature_gradient_y: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SolveThermalFrame2dRequest {
    pub nodes: Vec<ThermalFrame2dNodeInput>,
    pub elements: Vec<ThermalFrame2dElementInput>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThermalFrame3dNodeInput {
    pub id: String,
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub fix_x: bool,
    pub fix_y: bool,
    pub fix_z: bool,
    pub fix_rx: bool,
    pub fix_ry: bool,
    pub fix_rz: bool,
    pub load_x: f64,
    pub load_y: f64,
    pub load_z: f64,
    pub moment_x: f64,
    pub moment_y: f64,
    pub moment_z: f64,
    #[serde(default)]
    pub temperature_delta: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThermalFrame3dElementInput {
    pub id: String,
    pub node_i: usize,
    pub node_j: usize,
    pub area: f64,
    pub youngs_modulus: f64,
    pub shear_modulus: f64,
    pub torsion_constant: f64,
    pub moment_of_inertia_y: f64,
    pub moment_of_inertia_z: f64,
    pub section_modulus_y: f64,
    pub section_modulus_z: f64,
    pub thermal_expansion: f64,
    pub section_depth_y: f64,
    pub section_depth_z: f64,
    #[serde(default)]
    pub temperature_gradient_y: f64,
    #[serde(default)]
    pub temperature_gradient_z: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SolveThermalFrame3dRequest {
    pub nodes: Vec<ThermalFrame3dNodeInput>,
    pub elements: Vec<ThermalFrame3dElementInput>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlaneNodeResult {
    pub index: usize,
    pub id: String,
    pub x: f64,
    pub y: f64,
    pub ux: f64,
    pub uy: f64,
    pub displacement_magnitude: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThermalPlaneNodeResult {
    pub index: usize,
    pub id: String,
    pub x: f64,
    pub y: f64,
    pub ux: f64,
    pub uy: f64,
    pub displacement_magnitude: f64,
    pub temperature_delta: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlaneTriangleElementResult {
    pub index: usize,
    pub id: String,
    pub node_i: usize,
    pub node_j: usize,
    pub node_k: usize,
    pub area: f64,
    pub strain_x: f64,
    pub strain_y: f64,
    pub gamma_xy: f64,
    pub stress_x: f64,
    pub stress_y: f64,
    pub tau_xy: f64,
    pub principal_stress_1: f64,
    pub principal_stress_2: f64,
    pub max_in_plane_shear: f64,
    pub von_mises: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SolvePlaneTriangle2dResult {
    pub input: SolvePlaneTriangle2dRequest,
    pub nodes: Vec<PlaneNodeResult>,
    pub elements: Vec<PlaneTriangleElementResult>,
    pub max_displacement: f64,
    pub max_stress: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThermalPlaneTriangleElementResult {
    pub index: usize,
    pub id: String,
    pub node_i: usize,
    pub node_j: usize,
    pub node_k: usize,
    pub area: f64,
    pub average_temperature_delta: f64,
    pub thermal_strain: f64,
    pub mechanical_strain_x: f64,
    pub mechanical_strain_y: f64,
    pub total_strain_x: f64,
    pub total_strain_y: f64,
    pub gamma_xy: f64,
    pub stress_x: f64,
    pub stress_y: f64,
    pub tau_xy: f64,
    pub principal_stress_1: f64,
    pub principal_stress_2: f64,
    pub max_in_plane_shear: f64,
    pub von_mises: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SolveThermalPlaneTriangle2dResult {
    pub input: SolveThermalPlaneTriangle2dRequest,
    pub nodes: Vec<ThermalPlaneNodeResult>,
    pub elements: Vec<ThermalPlaneTriangleElementResult>,
    pub max_displacement: f64,
    pub max_stress: f64,
    pub max_temperature_delta: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlaneQuadElementResult {
    pub index: usize,
    pub id: String,
    pub node_i: usize,
    pub node_j: usize,
    pub node_k: usize,
    pub node_l: usize,
    pub area: f64,
    pub strain_x: f64,
    pub strain_y: f64,
    pub gamma_xy: f64,
    pub stress_x: f64,
    pub stress_y: f64,
    pub tau_xy: f64,
    pub principal_stress_1: f64,
    pub principal_stress_2: f64,
    pub max_in_plane_shear: f64,
    pub von_mises: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SolvePlaneQuad2dResult {
    pub input: SolvePlaneQuad2dRequest,
    pub nodes: Vec<PlaneNodeResult>,
    pub elements: Vec<PlaneQuadElementResult>,
    pub max_displacement: f64,
    pub max_stress: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThermalPlaneQuadElementResult {
    pub index: usize,
    pub id: String,
    pub node_i: usize,
    pub node_j: usize,
    pub node_k: usize,
    pub node_l: usize,
    pub area: f64,
    pub average_temperature_delta: f64,
    pub thermal_strain: f64,
    pub mechanical_strain_x: f64,
    pub mechanical_strain_y: f64,
    pub total_strain_x: f64,
    pub total_strain_y: f64,
    pub gamma_xy: f64,
    pub stress_x: f64,
    pub stress_y: f64,
    pub tau_xy: f64,
    pub principal_stress_1: f64,
    pub principal_stress_2: f64,
    pub max_in_plane_shear: f64,
    pub von_mises: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SolveThermalPlaneQuad2dResult {
    pub input: SolveThermalPlaneQuad2dRequest,
    pub nodes: Vec<ThermalPlaneNodeResult>,
    pub elements: Vec<ThermalPlaneQuadElementResult>,
    pub max_displacement: f64,
    pub max_stress: f64,
    pub max_temperature_delta: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Frame2dNodeResult {
    pub index: usize,
    pub id: String,
    pub x: f64,
    pub y: f64,
    pub ux: f64,
    pub uy: f64,
    pub rz: f64,
    pub displacement_magnitude: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Frame2dElementResult {
    pub index: usize,
    pub id: String,
    pub node_i: usize,
    pub node_j: usize,
    pub length: f64,
    pub axial_force_i: f64,
    pub shear_force_i: f64,
    pub moment_i: f64,
    pub axial_force_j: f64,
    pub shear_force_j: f64,
    pub moment_j: f64,
    pub axial_stress: f64,
    pub max_bending_stress: f64,
    pub max_combined_stress: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SolveFrame2dResult {
    pub input: SolveFrame2dRequest,
    pub nodes: Vec<Frame2dNodeResult>,
    pub elements: Vec<Frame2dElementResult>,
    pub max_displacement: f64,
    pub max_rotation: f64,
    pub max_moment: f64,
    pub max_stress: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThermalFrame2dNodeResult {
    pub index: usize,
    pub id: String,
    pub x: f64,
    pub y: f64,
    pub ux: f64,
    pub uy: f64,
    pub rz: f64,
    pub displacement_magnitude: f64,
    pub temperature_delta: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThermalFrame2dElementResult {
    pub index: usize,
    pub id: String,
    pub node_i: usize,
    pub node_j: usize,
    pub length: f64,
    pub average_temperature_delta: f64,
    pub thermal_strain: f64,
    pub mechanical_strain: f64,
    pub total_strain: f64,
    pub temperature_gradient_y: f64,
    pub thermal_curvature: f64,
    pub axial_force_i: f64,
    pub shear_force_i: f64,
    pub moment_i: f64,
    pub axial_force_j: f64,
    pub shear_force_j: f64,
    pub moment_j: f64,
    pub axial_stress: f64,
    pub max_bending_stress: f64,
    pub max_combined_stress: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SolveThermalFrame2dResult {
    pub input: SolveThermalFrame2dRequest,
    pub nodes: Vec<ThermalFrame2dNodeResult>,
    pub elements: Vec<ThermalFrame2dElementResult>,
    pub max_displacement: f64,
    pub max_rotation: f64,
    pub max_moment: f64,
    pub max_stress: f64,
    pub max_axial_force: f64,
    pub max_temperature_delta: f64,
    pub max_temperature_gradient: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThermalFrame3dNodeResult {
    pub index: usize,
    pub id: String,
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub ux: f64,
    pub uy: f64,
    pub uz: f64,
    pub rx: f64,
    pub ry: f64,
    pub rz: f64,
    pub displacement_magnitude: f64,
    pub rotation_magnitude: f64,
    pub temperature_delta: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThermalFrame3dElementResult {
    pub index: usize,
    pub id: String,
    pub node_i: usize,
    pub node_j: usize,
    pub length: f64,
    pub average_temperature_delta: f64,
    pub thermal_strain: f64,
    pub mechanical_strain: f64,
    pub total_strain: f64,
    pub temperature_gradient_y: f64,
    pub temperature_gradient_z: f64,
    pub thermal_curvature_y: f64,
    pub thermal_curvature_z: f64,
    pub axial_force_i: f64,
    pub shear_force_y_i: f64,
    pub shear_force_z_i: f64,
    pub torsion_i: f64,
    pub moment_y_i: f64,
    pub moment_z_i: f64,
    pub axial_force_j: f64,
    pub shear_force_y_j: f64,
    pub shear_force_z_j: f64,
    pub torsion_j: f64,
    pub moment_y_j: f64,
    pub moment_z_j: f64,
    pub axial_stress: f64,
    pub max_bending_stress: f64,
    pub max_combined_stress: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SolveThermalFrame3dResult {
    pub input: SolveThermalFrame3dRequest,
    pub nodes: Vec<ThermalFrame3dNodeResult>,
    pub elements: Vec<ThermalFrame3dElementResult>,
    pub max_displacement: f64,
    pub max_rotation: f64,
    pub max_moment: f64,
    pub max_stress: f64,
    pub max_axial_force: f64,
    pub max_temperature_delta: f64,
    pub max_temperature_gradient: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AnalysisResult {
    Bar1d(SolveBarResult),
    ThermalBar1d(SolveThermalBar1dResult),
    HeatBar1d(SolveHeatBar1dResult),
    ElectrostaticBar1d(SolveElectrostaticBar1dResult),
    ElectrostaticPlaneTriangle2d(SolveElectrostaticPlaneTriangle2dResult),
    ElectrostaticPlaneQuad2d(SolveElectrostaticPlaneQuad2dResult),
    HeatPlaneTriangle2d(SolveHeatPlaneTriangle2dResult),
    HeatPlaneQuad2d(SolveHeatPlaneQuad2dResult),
    ThermalTruss2d(SolveThermalTruss2dResult),
    ThermalTruss3d(SolveThermalTruss3dResult),
    Spring1d(SolveSpring1dResult),
    Spring2d(SolveSpring2dResult),
    Spring3d(SolveSpring3dResult),
    Beam1d(SolveBeam1dResult),
    ThermalBeam1d(SolveThermalBeam1dResult),
    Torsion1d(SolveTorsion1dResult),
    Truss2d(SolveTruss2dResult),
    Truss3d(SolveTruss3dResult),
    Frame3d(SolveFrame3dResult),
    PlaneTriangle2d(SolvePlaneTriangle2dResult),
    ThermalPlaneTriangle2d(SolveThermalPlaneTriangle2dResult),
    PlaneQuad2d(SolvePlaneQuad2dResult),
    ThermalPlaneQuad2d(SolveThermalPlaneQuad2dResult),
    Frame2d(SolveFrame2dResult),
    ThermalFrame2d(SolveThermalFrame2dResult),
    ThermalFrame3d(SolveThermalFrame3dResult),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResultChunkKind {
    Nodes,
    Elements,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResultChunkRequest {
    pub kind: ResultChunkKind,
    pub offset: usize,
    pub limit: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResultChunkResponse {
    pub kind: ResultChunkKind,
    pub offset: usize,
    pub limit: usize,
    pub returned: usize,
    pub total: usize,
    pub items: Vec<Value>,
}

#[cfg(test)]
mod tests {
    use super::{
        AgentDescriptor, Beam1dElementInput, Beam1dNodeInput, ElectrostaticBar1dElementInput,
        ElectrostaticBar1dNodeInput, ElectrostaticPlaneNodeInput,
        ElectrostaticPlaneQuadElementInput, ElectrostaticPlaneTriangleElementInput,
        Frame2dElementInput, Frame2dNodeInput, Frame3dElementInput, Frame3dNodeInput,
        HeatBar1dElementInput, HeatBar1dNodeInput, HeatPlaneNodeInput, HeatPlaneNodeResult,
        HeatPlaneQuadElementInput, HeatPlaneQuadElementResult, HeatPlaneTriangleElementInput,
        HeatToThermoPlaneQuad2dWorkflowRequest, HeatToThermoPlaneQuad2dWorkflowResult, Job,
        JobStatus, OperatorArtifactRef, OperatorDescriptor, OperatorKind, OperatorOrigin,
        OperatorPortDescriptor, OperatorRunRequest, OperatorRunResult, OperatorSchemaRef,
        OperatorValidationProfile, OperatorValidationStatus, PlaneQuadElementInput, ProgressEvent,
        RPC_VERSION, RpcMethod, RpcProgress, RpcRequest, RpcResponse, SolveBarRequest,
        SolveBeam1dRequest, SolveElectrostaticBar1dRequest, SolveElectrostaticPlaneQuad2dRequest,
        SolveElectrostaticPlaneTriangle2dRequest, SolveFrame2dRequest, SolveFrame3dRequest,
        SolveHeatBar1dRequest, SolveHeatPlaneQuad2dRequest, SolveHeatPlaneQuad2dResult,
        SolveHeatPlaneTriangle2dRequest, SolvePlaneQuad2dRequest, SolvePlaneTriangle2dRequest,
        SolveSpring1dRequest, SolveSpring2dRequest, SolveSpring3dRequest, SolveThermalBar1dRequest,
        SolveThermalBeam1dRequest, SolveThermalFrame2dRequest, SolveThermalFrame3dRequest,
        SolveThermalPlaneQuad2dRequest, SolveThermalPlaneQuad2dResult,
        SolveThermalPlaneTriangle2dRequest, SolveThermalTruss2dRequest, SolveTorsion1dRequest,
        SolveTruss3dRequest, Spring1dElementInput, Spring1dNodeInput, Spring2dElementInput,
        Spring2dNodeInput, Spring3dElementInput, Spring3dNodeInput, ThermalBar1dElementInput,
        ThermalBar1dNodeInput, ThermalBeam1dElementInput, ThermalBeam1dNodeInput,
        ThermalFrame2dElementInput, ThermalFrame2dNodeInput, ThermalFrame3dElementInput,
        ThermalFrame3dNodeInput, ThermalPlaneNodeInput, ThermalPlaneNodeResult,
        ThermalPlaneQuadElementInput, ThermalPlaneQuadElementResult,
        ThermalPlaneTriangleElementInput, ThermalTruss2dElementInput, ThermalTruss2dNodeInput,
        Torsion1dElementInput, Torsion1dNodeInput, WorkflowCachePolicy, WorkflowDatasetAxis,
        WorkflowDatasetContract, WorkflowDatasetEncoding, WorkflowDatasetShape,
        WorkflowDatasetValueInfo, WorkflowDefaults, WorkflowEdge, WorkflowGraph,
        WorkflowGraphRunRequest, WorkflowGraphRunResult, WorkflowNode, WorkflowNodeKind,
        WorkflowNodePortRef, WorkflowPort,
    };

    #[test]
    fn applies_progress_to_job() {
        let mut job = Job::new("job-1", "project-1", "case-1");
        let mut event = ProgressEvent::new("job-1", JobStatus::Solving, 0.5);
        event.iteration = Some(12);
        event.residual = Some(1.0e-4);

        job.apply_progress(&event);

        assert_eq!(job.status, JobStatus::Solving);
        assert_eq!(job.progress, 0.5);
        assert_eq!(job.iteration, Some(12));
        assert_eq!(job.residual, Some(1.0e-4));
    }

    #[test]
    fn exposes_lowercase_status_names() {
        assert_eq!(JobStatus::Solving.as_str(), "solving");
        assert_eq!(JobStatus::Completed.as_str(), "completed");
    }

    #[test]
    fn serializes_rpc_round_trip() {
        let request = RpcRequest {
            rpc_version: RPC_VERSION,
            id: "rpc-1".to_string(),
            method: RpcMethod::SolveBar1d,
            params: serde_json::to_value(SolveBarRequest {
                length: 1.0,
                area: 0.01,
                youngs_modulus: 210.0e9,
                elements: 3,
                tip_force: 1000.0,
            })
            .expect("request params should serialize"),
        };

        let json = serde_json::to_string(&request).expect("request should serialize");
        let decoded: RpcRequest = serde_json::from_str(&json).expect("request should decode");

        assert_eq!(decoded.method, RpcMethod::SolveBar1d);
        assert_eq!(decoded.rpc_version, RPC_VERSION);
        assert_eq!(decoded.id, "rpc-1");
        let params: SolveBarRequest = serde_json::from_value(decoded.params).expect("params");
        assert_eq!(params.elements, 3);
    }

    #[test]
    fn serializes_operator_descriptor_round_trip() {
        let descriptor = OperatorDescriptor {
            id: "solve.frame_3d".to_string(),
            version: "1.0.0".to_string(),
            domain: "mechanical".to_string(),
            family: "frame_3d".to_string(),
            kind: OperatorKind::Solver,
            summary: "Solve a 3D frame model with six-DOF nodes.".to_string(),
            capability_tags: vec![
                "verified".to_string(),
                "mechanical".to_string(),
                "frame".to_string(),
            ],
            origin: OperatorOrigin::BuiltIn,
            input_schema: OperatorSchemaRef {
                schema: "kyuubiki.operator.solve.frame_3d.input".to_string(),
                version: "1".to_string(),
            },
            output_schema: OperatorSchemaRef {
                schema: "kyuubiki.operator.solve.frame_3d.output".to_string(),
                version: "1".to_string(),
            },
            inputs: vec![OperatorPortDescriptor {
                id: "model".to_string(),
                artifact_type: "model/frame_3d".to_string(),
                description: "Frame model payload".to_string(),
                dataset_value: Some("frame_model".to_string()),
                schema_ref: Some(OperatorSchemaRef {
                    schema: "kyuubiki.operator.solve.frame_3d.input".to_string(),
                    version: "1".to_string(),
                }),
            }],
            outputs: vec![OperatorPortDescriptor {
                id: "result".to_string(),
                artifact_type: "result/frame_3d".to_string(),
                description: "Frame solve result".to_string(),
                dataset_value: Some("frame_result".to_string()),
                schema_ref: Some(OperatorSchemaRef {
                    schema: "kyuubiki.operator.solve.frame_3d.output".to_string(),
                    version: "1".to_string(),
                }),
            }],
            validation: OperatorValidationProfile {
                baseline_status: OperatorValidationStatus::Verified,
                baseline_cases: vec!["frame_3d_baseline".to_string()],
                smoke_paths: vec!["workflow_graph".to_string(), "orchestrated_api".to_string()],
            },
        };

        let json = serde_json::to_string(&descriptor).expect("descriptor should serialize");
        let decoded: OperatorDescriptor =
            serde_json::from_str(&json).expect("descriptor should decode");

        assert_eq!(decoded.id, "solve.frame_3d");
        assert_eq!(decoded.kind, OperatorKind::Solver);
        assert_eq!(decoded.origin, OperatorOrigin::BuiltIn);
        assert_eq!(decoded.inputs.len(), 1);
        assert_eq!(
            decoded.validation.baseline_status,
            OperatorValidationStatus::Verified
        );
    }

    #[test]
    fn serializes_operator_run_request_and_result_round_trip() {
        let request = OperatorRunRequest {
            operator_id: "extract.result_summary".to_string(),
            input: serde_json::json!({
                "job_id": "job-42",
                "result_kind": "frame_3d"
            }),
            context: super::OperatorRunContext {
                orchestrated: true,
                project_id: Some("project-1".to_string()),
                model_id: Some("model-7".to_string()),
                workflow_run_id: Some("run-9".to_string()),
            },
        };

        let json = serde_json::to_string(&request).expect("request should serialize");
        let decoded: OperatorRunRequest =
            serde_json::from_str(&json).expect("request should decode");
        assert_eq!(decoded.operator_id, "extract.result_summary");
        assert_eq!(decoded.context.project_id.as_deref(), Some("project-1"));

        let result = OperatorRunResult {
            operator_id: decoded.operator_id,
            summary: serde_json::json!({
                "max_stress": 1.26e5,
                "max_displacement": 5.3e-7
            }),
            artifacts: vec![OperatorArtifactRef {
                kind: "result_chunk".to_string(),
                id: "chunk-1".to_string(),
                label: "Primary summary".to_string(),
            }],
        };

        let json = serde_json::to_string(&result).expect("result should serialize");
        let decoded: OperatorRunResult = serde_json::from_str(&json).expect("result should decode");
        assert_eq!(decoded.artifacts.len(), 1);
        assert_eq!(decoded.artifacts[0].kind, "result_chunk");
    }

    #[test]
    fn serializes_heat_to_thermo_plane_quad_workflow_round_trip() {
        let request = HeatToThermoPlaneQuad2dWorkflowRequest {
            heat_model: SolveHeatPlaneQuad2dRequest {
                nodes: vec![
                    HeatPlaneNodeInput {
                        id: "h0".to_string(),
                        x: 0.0,
                        y: 0.0,
                        fix_temperature: true,
                        temperature: 100.0,
                        heat_load: 0.0,
                    },
                    HeatPlaneNodeInput {
                        id: "h1".to_string(),
                        x: 1.0,
                        y: 0.0,
                        fix_temperature: true,
                        temperature: 20.0,
                        heat_load: 0.0,
                    },
                ],
                elements: vec![HeatPlaneQuadElementInput {
                    id: "hq0".to_string(),
                    node_i: 0,
                    node_j: 1,
                    node_k: 1,
                    node_l: 0,
                    thickness: 0.02,
                    conductivity: 45.0,
                }],
            },
            thermo_seed_model: SolveThermalPlaneQuad2dRequest {
                nodes: vec![
                    ThermalPlaneNodeInput {
                        id: "n0".to_string(),
                        x: 0.0,
                        y: 0.0,
                        fix_x: true,
                        fix_y: true,
                        load_x: 0.0,
                        load_y: 0.0,
                        temperature_delta: 0.0,
                    },
                    ThermalPlaneNodeInput {
                        id: "n1".to_string(),
                        x: 1.0,
                        y: 0.0,
                        fix_x: true,
                        fix_y: true,
                        load_x: 0.0,
                        load_y: 0.0,
                        temperature_delta: 0.0,
                    },
                ],
                elements: vec![ThermalPlaneQuadElementInput {
                    id: "tq0".to_string(),
                    node_i: 0,
                    node_j: 1,
                    node_k: 1,
                    node_l: 0,
                    thickness: 0.02,
                    youngs_modulus: 70.0e9,
                    poisson_ratio: 0.33,
                    thermal_expansion: 11.0e-6,
                }],
            },
        };

        let json = serde_json::to_string(&request).expect("workflow request should serialize");
        let decoded: HeatToThermoPlaneQuad2dWorkflowRequest =
            serde_json::from_str(&json).expect("workflow request should decode");
        assert_eq!(decoded.heat_model.nodes.len(), 2);

        let result = HeatToThermoPlaneQuad2dWorkflowResult {
            workflow_id: "workflow.heat-to-thermo-quad-2d".to_string(),
            heat_result: SolveHeatPlaneQuad2dResult {
                input: decoded.heat_model.clone(),
                nodes: vec![HeatPlaneNodeResult {
                    index: 0,
                    id: "h0".to_string(),
                    x: 0.0,
                    y: 0.0,
                    temperature: 100.0,
                    heat_load: 0.0,
                }],
                elements: vec![HeatPlaneQuadElementResult {
                    index: 0,
                    id: "hq0".to_string(),
                    node_i: 0,
                    node_j: 1,
                    node_k: 1,
                    node_l: 0,
                    area: 0.02,
                    average_temperature: 60.0,
                    temperature_gradient_x: -40.0,
                    temperature_gradient_y: 0.0,
                    heat_flux_x: 1800.0,
                    heat_flux_y: 0.0,
                    heat_flux_magnitude: 1800.0,
                }],
                max_temperature: 100.0,
                max_heat_flux: 1800.0,
            },
            bridged_model: decoded.thermo_seed_model.clone(),
            thermo_result: SolveThermalPlaneQuad2dResult {
                input: decoded.thermo_seed_model,
                nodes: vec![ThermalPlaneNodeResult {
                    index: 0,
                    id: "n0".to_string(),
                    x: 0.0,
                    y: 0.0,
                    ux: 0.0,
                    uy: 0.0,
                    displacement_magnitude: 0.0,
                    temperature_delta: 80.0,
                }],
                elements: vec![ThermalPlaneQuadElementResult {
                    index: 0,
                    id: "tq0".to_string(),
                    node_i: 0,
                    node_j: 1,
                    node_k: 1,
                    node_l: 0,
                    area: 0.02,
                    average_temperature_delta: 80.0,
                    thermal_strain: 8.8e-4,
                    mechanical_strain_x: 0.0,
                    mechanical_strain_y: 0.0,
                    total_strain_x: 0.0,
                    total_strain_y: 0.0,
                    gamma_xy: 0.0,
                    stress_x: -1.0,
                    stress_y: -1.0,
                    tau_xy: 0.0,
                    principal_stress_1: -1.0,
                    principal_stress_2: -1.0,
                    max_in_plane_shear: 0.0,
                    von_mises: 1.0,
                }],
                max_displacement: 0.0,
                max_stress: 1.0,
                max_temperature_delta: 80.0,
            },
        };

        let json = serde_json::to_string(&result).expect("workflow result should serialize");
        let decoded: HeatToThermoPlaneQuad2dWorkflowResult =
            serde_json::from_str(&json).expect("workflow result should decode");
        assert_eq!(decoded.workflow_id, "workflow.heat-to-thermo-quad-2d");
        assert_eq!(decoded.thermo_result.max_temperature_delta, 80.0);
    }

    #[test]
    fn serializes_workflow_graph_run_request_round_trip() {
        let dataset_contract = WorkflowDatasetContract {
            id: "dataset.heat_to_thermo_quad/v1".to_string(),
            version: "1.0.0".to_string(),
            values: vec![
                WorkflowDatasetValueInfo {
                    id: "heat_model".to_string(),
                    data_class: "study_model".to_string(),
                    element_type: "json_object".to_string(),
                    shape: WorkflowDatasetShape {
                        axes: vec![WorkflowDatasetAxis {
                            id: "elements".to_string(),
                            label: Some("quad elements".to_string()),
                            size: None,
                            semantic: Some("mesh_element".to_string()),
                        }],
                    },
                    semantic_type: Some("study_model/heat_plane_quad_2d".to_string()),
                    unit: None,
                    encoding: Some(WorkflowDatasetEncoding::Json),
                    schema_ref: Some(OperatorSchemaRef {
                        schema: "kyuubiki.operator.solve.heat_plane_quad_2d.input".to_string(),
                        version: "1".to_string(),
                    }),
                },
                WorkflowDatasetValueInfo {
                    id: "thermo_result".to_string(),
                    data_class: "result".to_string(),
                    element_type: "json_object".to_string(),
                    shape: WorkflowDatasetShape::default(),
                    semantic_type: Some("result/thermal_plane_quad_2d".to_string()),
                    unit: None,
                    encoding: Some(WorkflowDatasetEncoding::Json),
                    schema_ref: Some(OperatorSchemaRef {
                        schema: "kyuubiki.operator.solve.thermal_plane_quad_2d.output".to_string(),
                        version: "1".to_string(),
                    }),
                },
            ],
            metadata: std::collections::BTreeMap::from([(
                "philosophy".to_string(),
                "onnx_like_cross_operator_contract".to_string(),
            )]),
        };

        let graph = WorkflowGraph {
            schema_version: "kyuubiki.workflow-graph/v1".to_string(),
            id: "workflow.heat-to-thermo-quad-2d".to_string(),
            name: "Heat to thermo-mechanical quad".to_string(),
            version: "1.0.0".to_string(),
            description: Some("Reference headless graph".to_string()),
            dataset_contract: Some(dataset_contract),
            entry_nodes: vec!["heat_model".to_string()],
            output_nodes: vec!["thermo_summary".to_string()],
            defaults: WorkflowDefaults {
                cache_policy: Some(WorkflowCachePolicy::Cached),
                orchestrated: Some(true),
            },
            nodes: vec![
                WorkflowNode {
                    id: "heat_model".to_string(),
                    kind: WorkflowNodeKind::Input,
                    operator_id: None,
                    name: Some("Heat input".to_string()),
                    description: None,
                    config: None,
                    cache_policy: None,
                    inputs: vec![],
                    outputs: vec![WorkflowPort {
                        id: "model".to_string(),
                        artifact_type: "study_model/heat_plane_quad_2d".to_string(),
                        name: None,
                        required: None,
                        cardinality: None,
                        dataset_value: Some("heat_model".to_string()),
                    }],
                },
                WorkflowNode {
                    id: "thermo_summary".to_string(),
                    kind: WorkflowNodeKind::Output,
                    operator_id: None,
                    name: Some("Thermo summary".to_string()),
                    description: None,
                    config: None,
                    cache_policy: None,
                    inputs: vec![WorkflowPort {
                        id: "result".to_string(),
                        artifact_type: "result/thermal_plane_quad_2d".to_string(),
                        name: None,
                        required: None,
                        cardinality: None,
                        dataset_value: Some("thermo_result".to_string()),
                    }],
                    outputs: vec![],
                },
            ],
            edges: vec![WorkflowEdge {
                id: "edge-1".to_string(),
                from: WorkflowNodePortRef {
                    node: "heat_model".to_string(),
                    port: "model".to_string(),
                },
                to: WorkflowNodePortRef {
                    node: "thermo_summary".to_string(),
                    port: "result".to_string(),
                },
                artifact_type: "result/thermal_plane_quad_2d".to_string(),
                dataset_value: Some("thermo_result".to_string()),
            }],
        };

        let request = WorkflowGraphRunRequest {
            graph,
            input_artifacts: std::collections::BTreeMap::from([(
                "heat_model".to_string(),
                serde_json::json!({"kind": "heat_plane_quad_2d"}),
            )]),
        };

        let json =
            serde_json::to_string(&request).expect("workflow graph request should serialize");
        let decoded: WorkflowGraphRunRequest =
            serde_json::from_str(&json).expect("workflow graph request should decode");
        assert_eq!(decoded.graph.id, "workflow.heat-to-thermo-quad-2d");
        assert_eq!(decoded.input_artifacts.len(), 1);
        assert_eq!(
            decoded
                .graph
                .dataset_contract
                .as_ref()
                .expect("dataset contract")
                .values
                .len(),
            2
        );

        let result = WorkflowGraphRunResult {
            workflow_id: decoded.graph.id,
            completed_nodes: vec!["heat_model".to_string(), "thermo_summary".to_string()],
            skipped_nodes: vec![],
            progress_events: vec![],
            branch_decisions: vec![],
            node_runs: vec![],
            artifact_lineage: vec![],
            artifacts: std::collections::BTreeMap::from([(
                "thermo_summary.result".to_string(),
                serde_json::json!({"max_stress": 123.0}),
            )]),
        };
        let json = serde_json::to_string(&result).expect("workflow graph result should serialize");
        let decoded: WorkflowGraphRunResult =
            serde_json::from_str(&json).expect("workflow graph result should decode");
        assert_eq!(decoded.completed_nodes.len(), 2);
        assert_eq!(decoded.skipped_nodes.len(), 0);
        assert_eq!(decoded.progress_events.len(), 0);
        assert_eq!(decoded.node_runs.len(), 0);
    }

    #[test]
    fn serializes_thermal_bar_1d_rpc_round_trip() {
        let request = RpcRequest {
            rpc_version: RPC_VERSION,
            id: "rpc-thermal-bar".to_string(),
            method: RpcMethod::SolveThermalBar1d,
            params: serde_json::to_value(SolveThermalBar1dRequest {
                nodes: vec![
                    ThermalBar1dNodeInput {
                        id: "n0".to_string(),
                        x: 0.0,
                        fix_x: true,
                        load_x: 0.0,
                        temperature_delta: 30.0,
                    },
                    ThermalBar1dNodeInput {
                        id: "n1".to_string(),
                        x: 1.0,
                        fix_x: false,
                        load_x: 0.0,
                        temperature_delta: 30.0,
                    },
                ],
                elements: vec![ThermalBar1dElementInput {
                    id: "tb0".to_string(),
                    node_i: 0,
                    node_j: 1,
                    area: 0.01,
                    youngs_modulus: 210.0e9,
                    thermal_expansion: 12.0e-6,
                }],
            })
            .expect("request params should serialize"),
        };

        let json = serde_json::to_string(&request).expect("request should serialize");
        let decoded: RpcRequest = serde_json::from_str(&json).expect("request should decode");

        assert_eq!(decoded.method, RpcMethod::SolveThermalBar1d);
        assert_eq!(decoded.id, "rpc-thermal-bar");
    }

    #[test]
    fn serializes_heat_bar_1d_rpc_round_trip() {
        let request = RpcRequest {
            rpc_version: RPC_VERSION,
            id: "rpc-heat-bar".to_string(),
            method: RpcMethod::SolveHeatBar1d,
            params: serde_json::to_value(SolveHeatBar1dRequest {
                nodes: vec![
                    HeatBar1dNodeInput {
                        id: "n0".to_string(),
                        x: 0.0,
                        fix_temperature: true,
                        temperature: 100.0,
                        heat_load: 0.0,
                    },
                    HeatBar1dNodeInput {
                        id: "n1".to_string(),
                        x: 1.0,
                        fix_temperature: true,
                        temperature: 0.0,
                        heat_load: 0.0,
                    },
                ],
                elements: vec![HeatBar1dElementInput {
                    id: "h0".to_string(),
                    node_i: 0,
                    node_j: 1,
                    area: 0.01,
                    conductivity: 45.0,
                }],
            })
            .expect("request params should serialize"),
        };

        let json = serde_json::to_string(&request).expect("request should serialize");
        let decoded: RpcRequest = serde_json::from_str(&json).expect("request should decode");

        assert_eq!(decoded.method, RpcMethod::SolveHeatBar1d);
        assert_eq!(decoded.id, "rpc-heat-bar");
    }

    #[test]
    fn serializes_electrostatic_bar_1d_rpc_round_trip() {
        let request = RpcRequest {
            rpc_version: RPC_VERSION,
            id: "rpc-electrostatic-bar".to_string(),
            method: RpcMethod::SolveElectrostaticBar1d,
            params: serde_json::to_value(SolveElectrostaticBar1dRequest {
                nodes: vec![
                    ElectrostaticBar1dNodeInput {
                        id: "n0".to_string(),
                        x: 0.0,
                        fix_potential: true,
                        potential: 10.0,
                        charge_density: 0.0,
                    },
                    ElectrostaticBar1dNodeInput {
                        id: "n1".to_string(),
                        x: 1.0,
                        fix_potential: true,
                        potential: 0.0,
                        charge_density: 0.0,
                    },
                ],
                elements: vec![ElectrostaticBar1dElementInput {
                    id: "eb0".to_string(),
                    node_i: 0,
                    node_j: 1,
                    area: 0.02,
                    permittivity: 8.854e-12,
                }],
            })
            .expect("request params should serialize"),
        };

        let json = serde_json::to_string(&request).expect("request should serialize");
        let decoded: RpcRequest = serde_json::from_str(&json).expect("request should decode");

        assert_eq!(decoded.method, RpcMethod::SolveElectrostaticBar1d);
        assert_eq!(decoded.id, "rpc-electrostatic-bar");
    }

    #[test]
    fn serializes_electrostatic_plane_triangle_2d_rpc_round_trip() {
        let request = RpcRequest {
            rpc_version: RPC_VERSION,
            id: "rpc-electrostatic-plane-triangle".to_string(),
            method: RpcMethod::SolveElectrostaticPlaneTriangle2d,
            params: serde_json::to_value(SolveElectrostaticPlaneTriangle2dRequest {
                nodes: vec![
                    ElectrostaticPlaneNodeInput {
                        id: "n0".to_string(),
                        x: 0.0,
                        y: 0.0,
                        fix_potential: true,
                        potential: 10.0,
                        charge_density: 0.0,
                    },
                    ElectrostaticPlaneNodeInput {
                        id: "n1".to_string(),
                        x: 1.0,
                        y: 0.0,
                        fix_potential: true,
                        potential: 0.0,
                        charge_density: 0.0,
                    },
                    ElectrostaticPlaneNodeInput {
                        id: "n2".to_string(),
                        x: 0.0,
                        y: 1.0,
                        fix_potential: true,
                        potential: 10.0,
                        charge_density: 0.0,
                    },
                ],
                elements: vec![ElectrostaticPlaneTriangleElementInput {
                    id: "ep0".to_string(),
                    node_i: 0,
                    node_j: 1,
                    node_k: 2,
                    thickness: 0.05,
                    permittivity: 2.0,
                }],
            })
            .expect("request params should serialize"),
        };

        let json = serde_json::to_string(&request).expect("request should serialize");
        let decoded: RpcRequest = serde_json::from_str(&json).expect("request should decode");

        assert_eq!(decoded.method, RpcMethod::SolveElectrostaticPlaneTriangle2d);
        assert_eq!(decoded.id, "rpc-electrostatic-plane-triangle");
    }

    #[test]
    fn serializes_electrostatic_plane_quad_2d_rpc_round_trip() {
        let request = RpcRequest {
            rpc_version: RPC_VERSION,
            id: "rpc-electrostatic-plane-quad".to_string(),
            method: RpcMethod::SolveElectrostaticPlaneQuad2d,
            params: serde_json::to_value(SolveElectrostaticPlaneQuad2dRequest {
                nodes: vec![
                    ElectrostaticPlaneNodeInput {
                        id: "n0".to_string(),
                        x: 0.0,
                        y: 0.0,
                        fix_potential: true,
                        potential: 10.0,
                        charge_density: 0.0,
                    },
                    ElectrostaticPlaneNodeInput {
                        id: "n1".to_string(),
                        x: 1.0,
                        y: 0.0,
                        fix_potential: true,
                        potential: 0.0,
                        charge_density: 0.0,
                    },
                    ElectrostaticPlaneNodeInput {
                        id: "n2".to_string(),
                        x: 1.0,
                        y: 1.0,
                        fix_potential: true,
                        potential: 0.0,
                        charge_density: 0.0,
                    },
                    ElectrostaticPlaneNodeInput {
                        id: "n3".to_string(),
                        x: 0.0,
                        y: 1.0,
                        fix_potential: true,
                        potential: 10.0,
                        charge_density: 0.0,
                    },
                ],
                elements: vec![ElectrostaticPlaneQuadElementInput {
                    id: "epq0".to_string(),
                    node_i: 0,
                    node_j: 1,
                    node_k: 2,
                    node_l: 3,
                    thickness: 0.05,
                    permittivity: 2.0,
                }],
            })
            .expect("request params should serialize"),
        };

        let json = serde_json::to_string(&request).expect("request should serialize");
        let decoded: RpcRequest = serde_json::from_str(&json).expect("request should decode");

        assert_eq!(decoded.method, RpcMethod::SolveElectrostaticPlaneQuad2d);
        assert_eq!(decoded.id, "rpc-electrostatic-plane-quad");
    }

    #[test]
    fn serializes_heat_plane_triangle_2d_rpc_round_trip() {
        let request = RpcRequest {
            rpc_version: RPC_VERSION,
            id: "rpc-heat-plane-triangle".to_string(),
            method: RpcMethod::SolveHeatPlaneTriangle2d,
            params: serde_json::to_value(SolveHeatPlaneTriangle2dRequest {
                nodes: vec![
                    HeatPlaneNodeInput {
                        id: "n0".to_string(),
                        x: 0.0,
                        y: 0.0,
                        fix_temperature: true,
                        temperature: 100.0,
                        heat_load: 0.0,
                    },
                    HeatPlaneNodeInput {
                        id: "n1".to_string(),
                        x: 1.0,
                        y: 0.0,
                        fix_temperature: true,
                        temperature: 20.0,
                        heat_load: 0.0,
                    },
                    HeatPlaneNodeInput {
                        id: "n2".to_string(),
                        x: 0.0,
                        y: 1.0,
                        fix_temperature: false,
                        temperature: 0.0,
                        heat_load: 0.0,
                    },
                ],
                elements: vec![HeatPlaneTriangleElementInput {
                    id: "hpt0".to_string(),
                    node_i: 0,
                    node_j: 1,
                    node_k: 2,
                    thickness: 0.02,
                    conductivity: 45.0,
                }],
            })
            .expect("request params should serialize"),
        };

        let json = serde_json::to_string(&request).expect("request should serialize");
        let decoded: RpcRequest = serde_json::from_str(&json).expect("request should decode");

        assert_eq!(decoded.method, RpcMethod::SolveHeatPlaneTriangle2d);
        assert_eq!(decoded.id, "rpc-heat-plane-triangle");
    }

    #[test]
    fn serializes_heat_plane_quad_2d_rpc_round_trip() {
        let request = RpcRequest {
            rpc_version: RPC_VERSION,
            id: "rpc-heat-plane-quad".to_string(),
            method: RpcMethod::SolveHeatPlaneQuad2d,
            params: serde_json::to_value(SolveHeatPlaneQuad2dRequest {
                nodes: vec![
                    HeatPlaneNodeInput {
                        id: "n0".to_string(),
                        x: 0.0,
                        y: 0.0,
                        fix_temperature: true,
                        temperature: 100.0,
                        heat_load: 0.0,
                    },
                    HeatPlaneNodeInput {
                        id: "n1".to_string(),
                        x: 1.0,
                        y: 0.0,
                        fix_temperature: true,
                        temperature: 20.0,
                        heat_load: 0.0,
                    },
                    HeatPlaneNodeInput {
                        id: "n2".to_string(),
                        x: 1.0,
                        y: 1.0,
                        fix_temperature: false,
                        temperature: 0.0,
                        heat_load: 0.0,
                    },
                    HeatPlaneNodeInput {
                        id: "n3".to_string(),
                        x: 0.0,
                        y: 1.0,
                        fix_temperature: false,
                        temperature: 0.0,
                        heat_load: 0.0,
                    },
                ],
                elements: vec![HeatPlaneQuadElementInput {
                    id: "hpq0".to_string(),
                    node_i: 0,
                    node_j: 1,
                    node_k: 2,
                    node_l: 3,
                    thickness: 0.02,
                    conductivity: 45.0,
                }],
            })
            .expect("request params should serialize"),
        };

        let json = serde_json::to_string(&request).expect("request should serialize");
        let decoded: RpcRequest = serde_json::from_str(&json).expect("request should decode");

        assert_eq!(decoded.method, RpcMethod::SolveHeatPlaneQuad2d);
        assert_eq!(decoded.id, "rpc-heat-plane-quad");
    }

    #[test]
    fn serializes_thermal_truss_2d_rpc_round_trip() {
        let request = RpcRequest {
            rpc_version: RPC_VERSION,
            id: "rpc-thermal-truss-2d".to_string(),
            method: RpcMethod::SolveThermalTruss2d,
            params: serde_json::to_value(SolveThermalTruss2dRequest {
                nodes: vec![
                    ThermalTruss2dNodeInput {
                        id: "n0".to_string(),
                        x: 0.0,
                        y: 0.0,
                        fix_x: true,
                        fix_y: true,
                        load_x: 0.0,
                        load_y: 0.0,
                        temperature_delta: 25.0,
                    },
                    ThermalTruss2dNodeInput {
                        id: "n1".to_string(),
                        x: 1.0,
                        y: 0.0,
                        fix_x: false,
                        fix_y: false,
                        load_x: 0.0,
                        load_y: 0.0,
                        temperature_delta: 25.0,
                    },
                ],
                elements: vec![ThermalTruss2dElementInput {
                    id: "tt0".to_string(),
                    node_i: 0,
                    node_j: 1,
                    area: 0.01,
                    youngs_modulus: 210.0e9,
                    thermal_expansion: 12.0e-6,
                }],
            })
            .expect("request params should serialize"),
        };

        let json = serde_json::to_string(&request).expect("request should serialize");
        let decoded: RpcRequest = serde_json::from_str(&json).expect("request should decode");

        assert_eq!(decoded.method, RpcMethod::SolveThermalTruss2d);
        assert_eq!(decoded.id, "rpc-thermal-truss-2d");
    }

    #[test]
    fn serializes_plane_triangle_rpc_round_trip() {
        let request = RpcRequest {
            rpc_version: RPC_VERSION,
            id: "rpc-plane".to_string(),
            method: RpcMethod::SolvePlaneTriangle2d,
            params: serde_json::to_value(SolvePlaneTriangle2dRequest {
                nodes: vec![],
                elements: vec![],
            })
            .expect("request params should serialize"),
        };

        let json = serde_json::to_string(&request).expect("request should serialize");
        let decoded: RpcRequest = serde_json::from_str(&json).expect("request should decode");

        assert_eq!(decoded.method, RpcMethod::SolvePlaneTriangle2d);
        assert_eq!(decoded.id, "rpc-plane");
    }

    #[test]
    fn serializes_plane_quad_rpc_round_trip() {
        let request = RpcRequest {
            rpc_version: RPC_VERSION,
            id: "rpc-plane-quad".to_string(),
            method: RpcMethod::SolvePlaneQuad2d,
            params: serde_json::to_value(SolvePlaneQuad2dRequest {
                nodes: vec![],
                elements: vec![PlaneQuadElementInput {
                    id: "q0".to_string(),
                    node_i: 0,
                    node_j: 1,
                    node_k: 2,
                    node_l: 3,
                    thickness: 0.02,
                    youngs_modulus: 70.0e9,
                    poisson_ratio: 0.33,
                }],
            })
            .expect("request params should serialize"),
        };

        let json = serde_json::to_string(&request).expect("request should serialize");
        let decoded: RpcRequest = serde_json::from_str(&json).expect("request should decode");

        assert_eq!(decoded.method, RpcMethod::SolvePlaneQuad2d);
        assert_eq!(decoded.id, "rpc-plane-quad");
    }

    #[test]
    fn serializes_thermal_plane_triangle_2d_rpc_round_trip() {
        let request = RpcRequest {
            rpc_version: RPC_VERSION,
            id: "rpc-thermal-plane-triangle".to_string(),
            method: RpcMethod::SolveThermalPlaneTriangle2d,
            params: serde_json::to_value(SolveThermalPlaneTriangle2dRequest {
                nodes: vec![
                    ThermalPlaneNodeInput {
                        id: "n0".to_string(),
                        x: 0.0,
                        y: 0.0,
                        fix_x: true,
                        fix_y: true,
                        load_x: 0.0,
                        load_y: 0.0,
                        temperature_delta: 40.0,
                    },
                    ThermalPlaneNodeInput {
                        id: "n1".to_string(),
                        x: 1.0,
                        y: 0.0,
                        fix_x: false,
                        fix_y: true,
                        load_x: 0.0,
                        load_y: 0.0,
                        temperature_delta: 40.0,
                    },
                    ThermalPlaneNodeInput {
                        id: "n2".to_string(),
                        x: 0.0,
                        y: 1.0,
                        fix_x: true,
                        fix_y: false,
                        load_x: 0.0,
                        load_y: 0.0,
                        temperature_delta: 40.0,
                    },
                ],
                elements: vec![ThermalPlaneTriangleElementInput {
                    id: "tp0".to_string(),
                    node_i: 0,
                    node_j: 1,
                    node_k: 2,
                    thickness: 0.02,
                    youngs_modulus: 70.0e9,
                    poisson_ratio: 0.33,
                    thermal_expansion: 12.0e-6,
                }],
            })
            .expect("request params should serialize"),
        };

        let json = serde_json::to_string(&request).expect("request should serialize");
        let decoded: RpcRequest = serde_json::from_str(&json).expect("request should decode");

        assert_eq!(decoded.method, RpcMethod::SolveThermalPlaneTriangle2d);
        assert_eq!(decoded.id, "rpc-thermal-plane-triangle");
    }

    #[test]
    fn serializes_thermal_plane_quad_2d_rpc_round_trip() {
        let request = RpcRequest {
            rpc_version: RPC_VERSION,
            id: "rpc-thermal-plane-quad".to_string(),
            method: RpcMethod::SolveThermalPlaneQuad2d,
            params: serde_json::to_value(SolveThermalPlaneQuad2dRequest {
                nodes: vec![
                    ThermalPlaneNodeInput {
                        id: "n0".to_string(),
                        x: 0.0,
                        y: 0.0,
                        fix_x: true,
                        fix_y: true,
                        load_x: 0.0,
                        load_y: 0.0,
                        temperature_delta: 25.0,
                    },
                    ThermalPlaneNodeInput {
                        id: "n1".to_string(),
                        x: 1.0,
                        y: 0.0,
                        fix_x: false,
                        fix_y: true,
                        load_x: 0.0,
                        load_y: 0.0,
                        temperature_delta: 30.0,
                    },
                    ThermalPlaneNodeInput {
                        id: "n2".to_string(),
                        x: 1.0,
                        y: 1.0,
                        fix_x: false,
                        fix_y: false,
                        load_x: 0.0,
                        load_y: 0.0,
                        temperature_delta: 35.0,
                    },
                    ThermalPlaneNodeInput {
                        id: "n3".to_string(),
                        x: 0.0,
                        y: 1.0,
                        fix_x: true,
                        fix_y: false,
                        load_x: 0.0,
                        load_y: 0.0,
                        temperature_delta: 40.0,
                    },
                ],
                elements: vec![ThermalPlaneQuadElementInput {
                    id: "tq0".to_string(),
                    node_i: 0,
                    node_j: 1,
                    node_k: 2,
                    node_l: 3,
                    thickness: 0.02,
                    youngs_modulus: 70.0e9,
                    poisson_ratio: 0.33,
                    thermal_expansion: 11.0e-6,
                }],
            })
            .expect("request params should serialize"),
        };

        let json = serde_json::to_string(&request).expect("request should serialize");
        let decoded: RpcRequest = serde_json::from_str(&json).expect("request should decode");

        assert_eq!(decoded.method, RpcMethod::SolveThermalPlaneQuad2d);
        assert_eq!(decoded.id, "rpc-thermal-plane-quad");
    }

    #[test]
    fn serializes_frame_2d_rpc_round_trip() {
        let request = RpcRequest {
            rpc_version: RPC_VERSION,
            id: "rpc-frame-2d".to_string(),
            method: RpcMethod::SolveFrame2d,
            params: serde_json::to_value(SolveFrame2dRequest {
                nodes: vec![
                    Frame2dNodeInput {
                        id: "n0".to_string(),
                        x: 0.0,
                        y: 0.0,
                        fix_x: true,
                        fix_y: true,
                        fix_rz: true,
                        load_x: 0.0,
                        load_y: 0.0,
                        moment_z: 0.0,
                    },
                    Frame2dNodeInput {
                        id: "n1".to_string(),
                        x: 1.0,
                        y: 0.0,
                        fix_x: false,
                        fix_y: false,
                        fix_rz: false,
                        load_x: 0.0,
                        load_y: -1000.0,
                        moment_z: 0.0,
                    },
                ],
                elements: vec![Frame2dElementInput {
                    id: "f0".to_string(),
                    node_i: 0,
                    node_j: 1,
                    area: 0.01,
                    youngs_modulus: 210.0e9,
                    moment_of_inertia: 8.0e-6,
                    section_modulus: 1.6e-4,
                }],
            })
            .expect("request params should serialize"),
        };

        let json = serde_json::to_string(&request).expect("request should serialize");
        let decoded: RpcRequest = serde_json::from_str(&json).expect("request should decode");

        assert_eq!(decoded.method, RpcMethod::SolveFrame2d);
        assert_eq!(decoded.id, "rpc-frame-2d");
    }

    #[test]
    fn serializes_frame_3d_rpc_round_trip() {
        let request = RpcRequest {
            rpc_version: RPC_VERSION,
            id: "rpc-frame-3d".to_string(),
            method: RpcMethod::SolveFrame3d,
            params: serde_json::to_value(SolveFrame3dRequest {
                nodes: vec![
                    Frame3dNodeInput {
                        id: "n0".to_string(),
                        x: 0.0,
                        y: 0.0,
                        z: 0.0,
                        fix_x: true,
                        fix_y: true,
                        fix_z: true,
                        fix_rx: true,
                        fix_ry: true,
                        fix_rz: true,
                        load_x: 0.0,
                        load_y: 0.0,
                        load_z: 0.0,
                        moment_x: 0.0,
                        moment_y: 0.0,
                        moment_z: 0.0,
                    },
                    Frame3dNodeInput {
                        id: "n1".to_string(),
                        x: 1.0,
                        y: 0.0,
                        z: 0.0,
                        fix_x: false,
                        fix_y: false,
                        fix_z: false,
                        fix_rx: false,
                        fix_ry: false,
                        fix_rz: false,
                        load_x: 0.0,
                        load_y: -1000.0,
                        load_z: 0.0,
                        moment_x: 0.0,
                        moment_y: 0.0,
                        moment_z: 0.0,
                    },
                ],
                elements: vec![Frame3dElementInput {
                    id: "f0".to_string(),
                    node_i: 0,
                    node_j: 1,
                    area: 0.01,
                    youngs_modulus: 210.0e9,
                    shear_modulus: 80.0e9,
                    torsion_constant: 2.0e-6,
                    moment_of_inertia_y: 8.0e-6,
                    moment_of_inertia_z: 6.0e-6,
                    section_modulus_y: 1.6e-4,
                    section_modulus_z: 1.3e-4,
                }],
            })
            .expect("request params should serialize"),
        };

        let json = serde_json::to_string(&request).expect("request should serialize");
        let decoded: RpcRequest = serde_json::from_str(&json).expect("request should decode");

        assert_eq!(decoded.method, RpcMethod::SolveFrame3d);
        assert_eq!(decoded.id, "rpc-frame-3d");
    }

    #[test]
    fn serializes_thermal_frame_2d_rpc_round_trip() {
        let request = RpcRequest {
            rpc_version: RPC_VERSION,
            id: "rpc-thermal-frame-2d".to_string(),
            method: RpcMethod::SolveThermalFrame2d,
            params: serde_json::to_value(SolveThermalFrame2dRequest {
                nodes: vec![
                    ThermalFrame2dNodeInput {
                        id: "n0".to_string(),
                        x: 0.0,
                        y: 0.0,
                        fix_x: true,
                        fix_y: true,
                        fix_rz: true,
                        load_x: 0.0,
                        load_y: 0.0,
                        moment_z: 0.0,
                        temperature_delta: 35.0,
                    },
                    ThermalFrame2dNodeInput {
                        id: "n1".to_string(),
                        x: 2.0,
                        y: 0.0,
                        fix_x: true,
                        fix_y: true,
                        fix_rz: true,
                        load_x: 0.0,
                        load_y: 0.0,
                        moment_z: 0.0,
                        temperature_delta: 35.0,
                    },
                ],
                elements: vec![ThermalFrame2dElementInput {
                    id: "tf0".to_string(),
                    node_i: 0,
                    node_j: 1,
                    area: 0.02,
                    youngs_modulus: 210.0e9,
                    moment_of_inertia: 8.0e-6,
                    section_modulus: 1.6e-4,
                    thermal_expansion: 12.0e-6,
                    section_depth: 0.2,
                    temperature_gradient_y: 30.0,
                }],
            })
            .expect("request params should serialize"),
        };

        let json = serde_json::to_string(&request).expect("request should serialize");
        let decoded: RpcRequest = serde_json::from_str(&json).expect("request should decode");

        assert_eq!(decoded.method, RpcMethod::SolveThermalFrame2d);
        assert_eq!(decoded.id, "rpc-thermal-frame-2d");
    }

    #[test]
    fn serializes_thermal_frame_3d_rpc_round_trip() {
        let request = RpcRequest {
            rpc_version: RPC_VERSION,
            id: "rpc-thermal-frame-3d".to_string(),
            method: RpcMethod::SolveThermalFrame3d,
            params: serde_json::to_value(SolveThermalFrame3dRequest {
                nodes: vec![
                    ThermalFrame3dNodeInput {
                        id: "n0".to_string(),
                        x: 0.0,
                        y: 0.0,
                        z: 0.0,
                        fix_x: true,
                        fix_y: true,
                        fix_z: true,
                        fix_rx: true,
                        fix_ry: true,
                        fix_rz: true,
                        load_x: 0.0,
                        load_y: 0.0,
                        load_z: 0.0,
                        moment_x: 0.0,
                        moment_y: 0.0,
                        moment_z: 0.0,
                        temperature_delta: 35.0,
                    },
                    ThermalFrame3dNodeInput {
                        id: "n1".to_string(),
                        x: 2.0,
                        y: 0.0,
                        z: 0.0,
                        fix_x: true,
                        fix_y: true,
                        fix_z: true,
                        fix_rx: true,
                        fix_ry: true,
                        fix_rz: true,
                        load_x: 0.0,
                        load_y: 0.0,
                        load_z: 0.0,
                        moment_x: 0.0,
                        moment_y: 0.0,
                        moment_z: 0.0,
                        temperature_delta: 35.0,
                    },
                ],
                elements: vec![ThermalFrame3dElementInput {
                    id: "tf0".to_string(),
                    node_i: 0,
                    node_j: 1,
                    area: 0.02,
                    youngs_modulus: 210.0e9,
                    shear_modulus: 80.0e9,
                    torsion_constant: 5.0e-6,
                    moment_of_inertia_y: 8.0e-6,
                    moment_of_inertia_z: 8.0e-6,
                    section_modulus_y: 1.6e-4,
                    section_modulus_z: 1.6e-4,
                    thermal_expansion: 12.0e-6,
                    section_depth_y: 0.2,
                    section_depth_z: 0.2,
                    temperature_gradient_y: 30.0,
                    temperature_gradient_z: 25.0,
                }],
            })
            .expect("request params should serialize"),
        };

        let json = serde_json::to_string(&request).expect("request should serialize");
        let decoded: RpcRequest = serde_json::from_str(&json).expect("request should decode");

        assert_eq!(decoded.method, RpcMethod::SolveThermalFrame3d);
        assert_eq!(decoded.id, "rpc-thermal-frame-3d");
    }

    #[test]
    fn serializes_beam_1d_rpc_round_trip() {
        let request = RpcRequest {
            rpc_version: RPC_VERSION,
            id: "rpc-beam-1d".to_string(),
            method: RpcMethod::SolveBeam1d,
            params: serde_json::to_value(SolveBeam1dRequest {
                nodes: vec![
                    Beam1dNodeInput {
                        id: "n0".to_string(),
                        x: 0.0,
                        fix_y: true,
                        fix_rz: true,
                        load_y: 0.0,
                        moment_z: 0.0,
                    },
                    Beam1dNodeInput {
                        id: "n1".to_string(),
                        x: 2.0,
                        fix_y: false,
                        fix_rz: false,
                        load_y: -1000.0,
                        moment_z: 0.0,
                    },
                ],
                elements: vec![Beam1dElementInput {
                    id: "b0".to_string(),
                    node_i: 0,
                    node_j: 1,
                    youngs_modulus: 210.0e9,
                    moment_of_inertia: 8.0e-6,
                    section_modulus: 1.6e-4,
                    distributed_load_y: 0.0,
                }],
            })
            .expect("request params should serialize"),
        };

        let json = serde_json::to_string(&request).expect("request should serialize");
        let decoded: RpcRequest = serde_json::from_str(&json).expect("request should decode");

        assert_eq!(decoded.method, RpcMethod::SolveBeam1d);
        assert_eq!(decoded.id, "rpc-beam-1d");
    }

    #[test]
    fn serializes_thermal_beam_1d_rpc_round_trip() {
        let request = RpcRequest {
            rpc_version: RPC_VERSION,
            id: "rpc-thermal-beam-1d".to_string(),
            method: RpcMethod::SolveThermalBeam1d,
            params: serde_json::to_value(SolveThermalBeam1dRequest {
                nodes: vec![
                    ThermalBeam1dNodeInput {
                        id: "n0".to_string(),
                        x: 0.0,
                        fix_y: true,
                        fix_rz: true,
                        load_y: 0.0,
                        moment_z: 0.0,
                    },
                    ThermalBeam1dNodeInput {
                        id: "n1".to_string(),
                        x: 2.0,
                        fix_y: true,
                        fix_rz: true,
                        load_y: 0.0,
                        moment_z: 0.0,
                    },
                ],
                elements: vec![ThermalBeam1dElementInput {
                    id: "tb0".to_string(),
                    node_i: 0,
                    node_j: 1,
                    youngs_modulus: 210.0e9,
                    moment_of_inertia: 8.0e-6,
                    section_modulus: 1.6e-4,
                    thermal_expansion: 12.0e-6,
                    section_depth: 0.2,
                    distributed_load_y: 0.0,
                    temperature_gradient_y: 40.0,
                }],
            })
            .expect("request params should serialize"),
        };

        let json = serde_json::to_string(&request).expect("request should serialize");
        let decoded: RpcRequest = serde_json::from_str(&json).expect("request should decode");

        assert_eq!(decoded.method, RpcMethod::SolveThermalBeam1d);
        assert_eq!(decoded.id, "rpc-thermal-beam-1d");
    }

    #[test]
    fn serializes_torsion_1d_rpc_round_trip() {
        let request = RpcRequest {
            rpc_version: RPC_VERSION,
            id: "rpc-torsion-1d".to_string(),
            method: RpcMethod::SolveTorsion1d,
            params: serde_json::to_value(SolveTorsion1dRequest {
                nodes: vec![
                    Torsion1dNodeInput {
                        id: "n0".to_string(),
                        x: 0.0,
                        fix_rz: true,
                        torque_z: 0.0,
                    },
                    Torsion1dNodeInput {
                        id: "n1".to_string(),
                        x: 1.5,
                        fix_rz: false,
                        torque_z: 500.0,
                    },
                ],
                elements: vec![Torsion1dElementInput {
                    id: "t0".to_string(),
                    node_i: 0,
                    node_j: 1,
                    shear_modulus: 80.0e9,
                    polar_moment: 3.0e-6,
                    section_modulus: 2.0e-4,
                }],
            })
            .expect("request params should serialize"),
        };

        let json = serde_json::to_string(&request).expect("request should serialize");
        let decoded: RpcRequest = serde_json::from_str(&json).expect("request should decode");

        assert_eq!(decoded.method, RpcMethod::SolveTorsion1d);
        assert_eq!(decoded.id, "rpc-torsion-1d");
    }

    #[test]
    fn serializes_spring_1d_rpc_round_trip() {
        let request = RpcRequest {
            rpc_version: RPC_VERSION,
            id: "rpc-spring-1d".to_string(),
            method: RpcMethod::SolveSpring1d,
            params: serde_json::to_value(SolveSpring1dRequest {
                nodes: vec![
                    Spring1dNodeInput {
                        id: "n0".to_string(),
                        x: 0.0,
                        fix_x: true,
                        load_x: 0.0,
                    },
                    Spring1dNodeInput {
                        id: "n1".to_string(),
                        x: 1.0,
                        fix_x: false,
                        load_x: 1000.0,
                    },
                ],
                elements: vec![Spring1dElementInput {
                    id: "s0".to_string(),
                    node_i: 0,
                    node_j: 1,
                    stiffness: 25_000.0,
                }],
            })
            .expect("request params should serialize"),
        };

        let json = serde_json::to_string(&request).expect("request should serialize");
        let decoded: RpcRequest = serde_json::from_str(&json).expect("request should decode");

        assert_eq!(decoded.method, RpcMethod::SolveSpring1d);
        assert_eq!(decoded.id, "rpc-spring-1d");
    }

    #[test]
    fn serializes_spring_2d_rpc_round_trip() {
        let request = RpcRequest {
            rpc_version: RPC_VERSION,
            id: "rpc-spring-2d".to_string(),
            method: RpcMethod::SolveSpring2d,
            params: serde_json::to_value(SolveSpring2dRequest {
                nodes: vec![
                    Spring2dNodeInput {
                        id: "n0".to_string(),
                        x: 0.0,
                        y: 0.0,
                        fix_x: true,
                        fix_y: true,
                        load_x: 0.0,
                        load_y: 0.0,
                    },
                    Spring2dNodeInput {
                        id: "n1".to_string(),
                        x: 1.0,
                        y: 1.0,
                        fix_x: false,
                        fix_y: false,
                        load_x: 1000.0,
                        load_y: -500.0,
                    },
                ],
                elements: vec![Spring2dElementInput {
                    id: "s0".to_string(),
                    node_i: 0,
                    node_j: 1,
                    stiffness: 25_000.0,
                }],
            })
            .expect("request params should serialize"),
        };

        let json = serde_json::to_string(&request).expect("request should serialize");
        let decoded: RpcRequest = serde_json::from_str(&json).expect("request should decode");

        assert_eq!(decoded.method, RpcMethod::SolveSpring2d);
        assert_eq!(decoded.id, "rpc-spring-2d");
    }

    #[test]
    fn serializes_spring_3d_rpc_round_trip() {
        let request = RpcRequest {
            rpc_version: RPC_VERSION,
            id: "rpc-spring-3d".to_string(),
            method: RpcMethod::SolveSpring3d,
            params: serde_json::to_value(SolveSpring3dRequest {
                nodes: vec![
                    Spring3dNodeInput {
                        id: "n0".to_string(),
                        x: 0.0,
                        y: 0.0,
                        z: 0.0,
                        fix_x: true,
                        fix_y: true,
                        fix_z: true,
                        load_x: 0.0,
                        load_y: 0.0,
                        load_z: 0.0,
                    },
                    Spring3dNodeInput {
                        id: "n1".to_string(),
                        x: 1.0,
                        y: 1.0,
                        z: 1.0,
                        fix_x: false,
                        fix_y: false,
                        fix_z: false,
                        load_x: 1000.0,
                        load_y: -500.0,
                        load_z: 250.0,
                    },
                ],
                elements: vec![Spring3dElementInput {
                    id: "s0".to_string(),
                    node_i: 0,
                    node_j: 1,
                    stiffness: 25_000.0,
                }],
            })
            .expect("request params should serialize"),
        };

        let json = serde_json::to_string(&request).expect("request should serialize");
        let decoded: RpcRequest = serde_json::from_str(&json).expect("request should decode");

        assert_eq!(decoded.method, RpcMethod::SolveSpring3d);
        assert_eq!(decoded.id, "rpc-spring-3d");
    }

    #[test]
    fn serializes_truss_3d_rpc_round_trip() {
        let request = RpcRequest {
            rpc_version: RPC_VERSION,
            id: "rpc-truss-3d".to_string(),
            method: RpcMethod::SolveTruss3d,
            params: serde_json::to_value(SolveTruss3dRequest {
                nodes: vec![],
                elements: vec![],
            })
            .expect("request params should serialize"),
        };

        let json = serde_json::to_string(&request).expect("request should serialize");
        let decoded: RpcRequest = serde_json::from_str(&json).expect("request should decode");

        assert_eq!(decoded.method, RpcMethod::SolveTruss3d);
        assert_eq!(decoded.id, "rpc-truss-3d");
    }

    #[test]
    fn builds_error_responses() {
        let response = RpcResponse::error("rpc-1", "invalid_request", "unsupported method");

        assert!(!response.ok);
        assert!(response.result.is_none());
        assert_eq!(response.rpc_version, 1);
        assert_eq!(response.id, "rpc-1");
        assert_eq!(
            response.error.expect("error payload").code,
            "invalid_request"
        );
    }

    #[test]
    fn serializes_agent_descriptor_round_trip() {
        let descriptor = AgentDescriptor::solver_agent_default();

        let json = serde_json::to_string(&descriptor).expect("descriptor should serialize");
        let decoded: AgentDescriptor =
            serde_json::from_str(&json).expect("descriptor should decode");

        assert_eq!(decoded.program, "kyuubiki-rust-agent");
        assert_eq!(decoded.protocol.rpc_version, RPC_VERSION);
        assert!(decoded.protocol.methods.contains(&RpcMethod::DescribeAgent));
        assert_eq!(decoded.authority.control_mode, "standalone");
        assert_eq!(decoded.authority.authority_mode, "self_directed");
    }

    #[test]
    fn serializes_progress_frames() {
        let progress = RpcProgress::new(
            "rpc-1",
            ProgressEvent::new("job-1", JobStatus::Solving, 0.5),
        );

        let json = serde_json::to_string(&progress).expect("progress should serialize");
        let decoded: RpcProgress = serde_json::from_str(&json).expect("progress should decode");

        assert_eq!(decoded.id, "rpc-1");
        assert_eq!(decoded.event, "progress");
        assert_eq!(decoded.progress.job_id, "job-1");
    }
}
