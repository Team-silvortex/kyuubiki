use serde::{Deserialize, Serialize};

use super::plane_frame::{SolveBucklingFrame2dRequest, SolveBucklingFrame2dResult};

pub const FRAME_2D_P_DELTA_CRITICAL_FACTOR_LIMIT_RATIO: f64 = 0.95;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NonlinearSpring1dNodeInput {
    pub id: String,
    pub x: f64,
    pub fix_x: bool,
    pub load_x: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NonlinearSpring1dElementInput {
    pub id: String,
    pub node_i: usize,
    pub node_j: usize,
    pub stiffness: f64,
    pub cubic_stiffness: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SolveNonlinearSpring1dRequest {
    pub nodes: Vec<NonlinearSpring1dNodeInput>,
    pub elements: Vec<NonlinearSpring1dElementInput>,
    #[serde(default)]
    pub load_steps: Option<usize>,
    #[serde(default)]
    pub max_iterations: Option<usize>,
    #[serde(default)]
    pub tolerance: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NonlinearSpring1dStepResult {
    pub step: usize,
    pub load_factor: f64,
    pub iterations: usize,
    pub residual_norm: f64,
    pub converged: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NonlinearSpring1dNodeResult {
    pub index: usize,
    pub id: String,
    pub x: f64,
    pub ux: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NonlinearSpring1dElementResult {
    pub index: usize,
    pub id: String,
    pub node_i: usize,
    pub node_j: usize,
    pub length: f64,
    pub extension: f64,
    pub force: f64,
    pub tangent_stiffness: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SolveNonlinearSpring1dResult {
    pub input: SolveNonlinearSpring1dRequest,
    pub nodes: Vec<NonlinearSpring1dNodeResult>,
    pub elements: Vec<NonlinearSpring1dElementResult>,
    pub steps: Vec<NonlinearSpring1dStepResult>,
    pub converged: bool,
    pub residual_norm: f64,
    pub max_displacement: f64,
    pub max_force: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SolveFrame2dPDeltaRequest {
    pub buckling: SolveBucklingFrame2dRequest,
    pub imperfection_amplitude: f64,
    #[serde(default)]
    pub kinematics: Frame2dStabilityKinematics,
    #[serde(default)]
    pub path_control: Frame2dStabilityPathControl,
    #[serde(default)]
    pub imperfection_shape: Option<Vec<f64>>,
    #[serde(default)]
    pub imperfection_mode_index: Option<usize>,
    #[serde(default)]
    pub maximum_load_factor: Option<f64>,
    #[serde(default)]
    pub load_steps: Option<usize>,
    #[serde(default)]
    pub max_iterations: Option<usize>,
    #[serde(default)]
    pub tolerance: Option<f64>,
    #[serde(default)]
    pub max_step_cutbacks: Option<usize>,
    #[serde(default)]
    pub arc_length_radius: Option<f64>,
    #[serde(default)]
    pub arc_length_load_scale: Option<f64>,
    #[serde(default)]
    pub arc_length_target_iterations: Option<usize>,
    #[serde(default)]
    pub tangent_transition_refinement_steps: Option<usize>,
    #[serde(default)]
    pub branch_switch: Frame2dBranchSwitchSelection,
    #[serde(default)]
    pub branch_switch_amplitude: Option<f64>,
    #[serde(default)]
    pub branch_switch_mode_count: Option<usize>,
    #[serde(default)]
    pub branch_switch_pairwise_combinations: bool,
    #[serde(default)]
    pub branch_continuation_steps: Option<usize>,
    #[serde(default)]
    pub branch_continuation_radius: Option<f64>,
    #[serde(default)]
    pub branch_continuation_min_radius_ratio: Option<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Frame2dStabilityKinematics {
    LinearizedPDelta,
    Corotational,
}

impl Default for Frame2dStabilityKinematics {
    fn default() -> Self {
        Self::LinearizedPDelta
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Frame2dStabilityPathControl {
    LoadControl,
    ArcLength,
}

impl Default for Frame2dStabilityPathControl {
    fn default() -> Self {
        Self::LoadControl
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Frame2dImperfectionSource {
    BucklingMode,
    ExplicitShape,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Frame2dPDeltaFailureReason {
    MaximumIterations,
    TangentSolveFailed,
    LineSearchFailed,
    CutbackLimitExhausted,
    IncrementTooSmall,
    ArcLengthConstraintSingular,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Frame2dEquilibriumPathEvent {
    LimitPointMaximum,
    LimitPointMinimum,
    BifurcationCandidate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Frame2dTangentStability {
    PositiveDefinite,
    Indefinite,
    NearSingular,
    UnassessedSizeLimit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Frame2dBranchSwitchSelection {
    Disabled,
    Positive,
    Negative,
    Both,
}

impl Default for Frame2dBranchSwitchSelection {
    fn default() -> Self {
        Self::Disabled
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Frame2dBranchDirection {
    Positive,
    Negative,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Frame2dCriticalModeResult {
    pub mode_index: usize,
    pub normalized_eigenvalue: f64,
    pub normalized_residual: f64,
    pub shape: Vec<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Frame2dBranchModeComponent {
    pub mode_index: usize,
    pub normalized_eigenvalue: Option<f64>,
    pub weight: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Frame2dBranchContinuationStepResult {
    pub step: usize,
    pub load_factor: f64,
    pub load_factor_increment: f64,
    pub iterations: usize,
    pub converged: bool,
    pub cutbacks: usize,
    pub failure_reason: Option<Frame2dPDeltaFailureReason>,
    pub failure_detail: Option<String>,
    pub residual_norm: f64,
    pub arc_length_constraint_error: f64,
    pub arc_length_radius: f64,
    pub tangent_stability: Option<Frame2dTangentStability>,
    pub tangent_negative_pivots: Option<usize>,
    pub tangent_near_zero_pivots: Option<usize>,
    #[serde(default)]
    pub tangent_negative_pivot_delta: Option<i32>,
    #[serde(default)]
    pub path_event: Option<Frame2dEquilibriumPathEvent>,
    pub displacements: Vec<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Frame2dBranchSwitchProbeResult {
    #[serde(default)]
    pub mode_index: usize,
    #[serde(default)]
    pub mode_eigenvalue: Option<f64>,
    #[serde(default)]
    pub mode_components: Vec<Frame2dBranchModeComponent>,
    #[serde(default)]
    pub mode_component_projections: Vec<f64>,
    pub direction: Frame2dBranchDirection,
    pub seed_amplitude: f64,
    pub iterations: usize,
    pub equilibrium_converged: bool,
    pub primary_equilibrium_converged: bool,
    pub distinct_branch: bool,
    pub load_factor: Option<f64>,
    pub residual_norm: Option<f64>,
    pub modal_constraint_error: Option<f64>,
    pub mode_projection: Option<f64>,
    pub displacement_distance: Option<f64>,
    pub primary_displacement_distance: Option<f64>,
    pub displacements: Option<Vec<f64>>,
    pub failure_detail: Option<String>,
    #[serde(default)]
    pub continuation_steps: Vec<Frame2dBranchContinuationStepResult>,
    #[serde(default)]
    pub continuation_converged: Option<bool>,
    #[serde(default)]
    pub continuation_failure_detail: Option<String>,
}

impl Default for Frame2dImperfectionSource {
    fn default() -> Self {
        Self::BucklingMode
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Frame2dPDeltaStepResult {
    pub step: usize,
    pub load_factor: f64,
    pub critical_factor_ratio: f64,
    #[serde(default = "default_single_iteration")]
    pub iterations: usize,
    #[serde(default = "default_true")]
    pub converged: bool,
    #[serde(default)]
    pub achieved_load_factor: Option<f64>,
    #[serde(default = "default_single_iteration")]
    pub substeps: usize,
    #[serde(default)]
    pub cutbacks: usize,
    #[serde(default)]
    pub failure_reason: Option<Frame2dPDeltaFailureReason>,
    #[serde(default)]
    pub failure_detail: Option<String>,
    #[serde(default)]
    pub arc_length_constraint_error: Option<f64>,
    #[serde(default)]
    pub arc_length_radius: Option<f64>,
    #[serde(default)]
    pub load_factor_increment: Option<f64>,
    #[serde(default)]
    pub path_event: Option<Frame2dEquilibriumPathEvent>,
    #[serde(default)]
    pub tangent_stability: Option<Frame2dTangentStability>,
    #[serde(default)]
    pub tangent_negative_pivots: Option<usize>,
    #[serde(default)]
    pub tangent_near_zero_pivots: Option<usize>,
    #[serde(default)]
    pub tangent_negative_pivot_delta: Option<i32>,
    #[serde(default)]
    pub tangent_critical_eigenvalue: Option<f64>,
    #[serde(default)]
    pub tangent_critical_mode_residual: Option<f64>,
    #[serde(default)]
    pub tangent_critical_mode: Option<Vec<f64>>,
    #[serde(default)]
    pub tangent_critical_modes: Vec<Frame2dCriticalModeResult>,
    #[serde(default)]
    pub tangent_transition_load_factor_min: Option<f64>,
    #[serde(default)]
    pub tangent_transition_load_factor_max: Option<f64>,
    #[serde(default)]
    pub tangent_transition_load_factor_width: Option<f64>,
    #[serde(default)]
    pub tangent_transition_refinements: Option<usize>,
    #[serde(default)]
    pub tangent_critical_load_factor: Option<f64>,
    #[serde(default)]
    pub branch_switch_probes: Vec<Frame2dBranchSwitchProbeResult>,
    pub residual_norm: f64,
    pub imperfection_amplification: f64,
    pub max_incremental_displacement: f64,
    pub displacements: Vec<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SolveFrame2dPDeltaResult {
    pub input: SolveFrame2dPDeltaRequest,
    pub buckling_result: SolveBucklingFrame2dResult,
    #[serde(default)]
    pub imperfection_source: Frame2dImperfectionSource,
    #[serde(default)]
    pub kinematics: Frame2dStabilityKinematics,
    #[serde(default)]
    pub path_control: Frame2dStabilityPathControl,
    pub initial_imperfection_shape: Vec<f64>,
    pub critical_factor_limit_ratio: f64,
    pub steps: Vec<Frame2dPDeltaStepResult>,
    pub final_displacements: Vec<f64>,
    pub max_imperfection_amplification: f64,
    #[serde(default = "default_true")]
    pub converged: bool,
}

fn default_single_iteration() -> usize {
    1
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContactGap1dContactInput {
    pub id: String,
    pub node: usize,
    pub gap: f64,
    pub normal_stiffness: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SolveContactGap1dRequest {
    pub nodes: Vec<NonlinearSpring1dNodeInput>,
    pub elements: Vec<NonlinearSpring1dElementInput>,
    pub contacts: Vec<ContactGap1dContactInput>,
    #[serde(default)]
    pub load_steps: Option<usize>,
    #[serde(default)]
    pub max_iterations: Option<usize>,
    #[serde(default)]
    pub tolerance: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContactGap1dContactResult {
    pub index: usize,
    pub id: String,
    pub node: usize,
    pub gap: f64,
    pub penetration: f64,
    pub force: f64,
    pub active: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SolveContactGap1dResult {
    pub input: SolveContactGap1dRequest,
    pub nodes: Vec<NonlinearSpring1dNodeResult>,
    pub elements: Vec<NonlinearSpring1dElementResult>,
    pub contacts: Vec<ContactGap1dContactResult>,
    pub steps: Vec<NonlinearSpring1dStepResult>,
    pub converged: bool,
    pub residual_norm: f64,
    pub max_displacement: f64,
    pub max_force: f64,
    pub max_contact_force: f64,
    pub active_contact_count: usize,
}
