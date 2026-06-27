use serde::{Deserialize, Serialize};

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
