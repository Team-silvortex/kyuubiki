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
pub enum Frame2dImperfectionSource {
    BucklingMode,
    ExplicitShape,
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
