use serde::{Deserialize, Serialize};

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
pub struct TransientSpring1dNodeInput {
    pub id: String,
    pub x: f64,
    pub fix_x: bool,
    pub load_x: f64,
    pub mass: f64,
    #[serde(default)]
    pub initial_displacement: f64,
    #[serde(default)]
    pub initial_velocity: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TransientSpring1dElementInput {
    pub id: String,
    pub node_i: usize,
    pub node_j: usize,
    pub stiffness: f64,
    #[serde(default)]
    pub damping: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SolveTransientSpring1dRequest {
    pub nodes: Vec<TransientSpring1dNodeInput>,
    pub elements: Vec<TransientSpring1dElementInput>,
    pub time_step: f64,
    pub steps: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SolveHarmonicSpring1dRequest {
    pub nodes: Vec<TransientSpring1dNodeInput>,
    pub elements: Vec<TransientSpring1dElementInput>,
    pub frequencies_hz: Vec<f64>,
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
