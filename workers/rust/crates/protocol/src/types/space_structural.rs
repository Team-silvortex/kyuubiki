use serde::{Deserialize, Serialize};

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
pub struct ModalFrame3dElementInput {
    pub id: String,
    pub node_i: usize,
    pub node_j: usize,
    pub area: f64,
    pub youngs_modulus: f64,
    pub shear_modulus: f64,
    pub torsion_constant: f64,
    pub moment_of_inertia_y: f64,
    pub moment_of_inertia_z: f64,
    pub density: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SolveModalFrame3dRequest {
    pub nodes: Vec<Frame3dNodeInput>,
    pub elements: Vec<ModalFrame3dElementInput>,
    pub mode_count: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModalFrame3dModeResult {
    pub index: usize,
    pub eigenvalue_rad_s_squared: f64,
    pub natural_frequency_rad_s: f64,
    pub natural_frequency_hz: f64,
    pub period_s: f64,
    pub participation_norm: f64,
    pub shape: Vec<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SolveModalFrame3dResult {
    pub input: SolveModalFrame3dRequest,
    pub modes: Vec<ModalFrame3dModeResult>,
    pub free_dofs: Vec<usize>,
    pub total_mass: f64,
    pub min_frequency_hz: f64,
    pub max_frequency_hz: f64,
}
