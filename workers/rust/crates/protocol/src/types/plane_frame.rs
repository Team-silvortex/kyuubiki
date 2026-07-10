use serde::{Deserialize, Serialize};

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
pub struct ModalFrame2dElementInput {
    pub id: String,
    pub node_i: usize,
    pub node_j: usize,
    pub area: f64,
    pub youngs_modulus: f64,
    pub moment_of_inertia: f64,
    pub section_modulus: f64,
    pub density: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SolveModalFrame2dRequest {
    pub nodes: Vec<Frame2dNodeInput>,
    pub elements: Vec<ModalFrame2dElementInput>,
    #[serde(default)]
    pub mode_count: Option<usize>,
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
pub struct ModalFrame2dModeResult {
    pub index: usize,
    pub eigenvalue_rad_s_squared: f64,
    pub natural_frequency_rad_s: f64,
    pub natural_frequency_hz: f64,
    pub period_s: f64,
    pub participation_norm: f64,
    pub shape: Vec<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SolveModalFrame2dResult {
    pub input: SolveModalFrame2dRequest,
    pub modes: Vec<ModalFrame2dModeResult>,
    pub free_dofs: Vec<usize>,
    pub total_mass: f64,
    pub min_frequency_hz: f64,
    pub max_frequency_hz: f64,
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
