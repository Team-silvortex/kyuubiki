use serde::{Deserialize, Serialize};

use super::cohesive_interface::CohesiveTractionRegime;
use super::space_structural::{SolidTetra3dElementInput, SolidTetra3dElementResult};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CohesiveInterface3dMaterialInput {
    pub normal_initial_stiffness: f64,
    pub normal_compression_stiffness: f64,
    pub normal_peak_traction: f64,
    pub normal_failure_separation: f64,
    pub shear_initial_stiffness: f64,
    pub shear_peak_traction: f64,
    pub shear_failure_separation: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CohesiveInterfaceMesh3dNodeInput {
    pub id: String,
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub fixed: [bool; 3],
    #[serde(default)]
    pub prescribed_displacement: Option<[f64; 3]>,
    pub load: [f64; 3],
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CohesiveInterfaceMesh3dMaterialInput {
    pub id: String,
    pub properties: CohesiveInterface3dMaterialInput,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CohesiveInterfaceMesh3dElementInput {
    pub id: String,
    pub lower_a: usize,
    pub lower_b: usize,
    pub lower_c: usize,
    pub upper_a: usize,
    pub upper_b: usize,
    pub upper_c: usize,
    pub material_id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CohesiveInterfaceMesh3dControlStepInput {
    pub load_factor: f64,
    pub prescribed_displacements: Vec<[f64; 3]>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SolveCohesiveInterfaceMesh3dRequest {
    pub id: String,
    pub nodes: Vec<CohesiveInterfaceMesh3dNodeInput>,
    pub materials: Vec<CohesiveInterfaceMesh3dMaterialInput>,
    pub elements: Vec<CohesiveInterfaceMesh3dElementInput>,
    #[serde(default)]
    pub host_tetrahedra: Vec<SolidTetra3dElementInput>,
    #[serde(default)]
    pub load_steps: Option<usize>,
    #[serde(default)]
    pub control_history: Option<Vec<CohesiveInterfaceMesh3dControlStepInput>>,
    #[serde(default)]
    pub max_iterations: Option<usize>,
    #[serde(default)]
    pub tolerance: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CohesiveInterface3dIntegrationPointResult {
    pub barycentric_coordinates: [f64; 3],
    pub local_separation: [f64; 3],
    pub local_traction: [f64; 3],
    pub local_tangent: [f64; 3],
    pub tangential_damage: [f64; 2],
    pub normal_damage: f64,
    pub max_tangential_separation: [f64; 2],
    pub max_normal_opening: f64,
    pub regimes: [CohesiveTractionRegime; 3],
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CohesiveInterfaceMesh3dNodeResult {
    pub id: String,
    pub displacement: [f64; 3],
    pub reaction: [f64; 3],
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CohesiveInterfaceMesh3dElementResult {
    pub id: String,
    pub material_id: String,
    pub area: f64,
    pub local_tangent_1_direction: [f64; 3],
    pub local_tangent_2_direction: [f64; 3],
    pub local_normal_direction: [f64; 3],
    pub local_separation: [f64; 3],
    pub local_traction: [f64; 3],
    pub global_traction: [f64; 3],
    pub integration_points: Vec<CohesiveInterface3dIntegrationPointResult>,
    pub max_tangential_damage: f64,
    pub max_normal_damage: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CohesiveInterfaceMesh3dLoadStepResult {
    pub step: usize,
    pub load_factor: f64,
    pub iterations: usize,
    pub residual_norm: f64,
    pub converged: bool,
    pub max_displacement: f64,
    pub prescribed_displacement_norm: f64,
    pub reaction_norm: f64,
    pub max_resultant_traction: f64,
    pub max_tangential_damage: f64,
    pub max_normal_damage: f64,
    pub max_host_von_mises_stress: f64,
    #[serde(default)]
    pub tangent_non_zero_count: usize,
    #[serde(default)]
    pub tangent_fill_ratio: f64,
    #[serde(default)]
    pub linear_solver: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SolveCohesiveInterfaceMesh3dResult {
    pub input: SolveCohesiveInterfaceMesh3dRequest,
    pub nodes: Vec<CohesiveInterfaceMesh3dNodeResult>,
    pub elements: Vec<CohesiveInterfaceMesh3dElementResult>,
    pub host_tetrahedra: Vec<SolidTetra3dElementResult>,
    pub steps: Vec<CohesiveInterfaceMesh3dLoadStepResult>,
    pub converged: bool,
    pub completed_load_factor: f64,
    pub residual_norm: f64,
    pub max_displacement: f64,
    pub max_resultant_traction: f64,
    pub max_tangential_damage: f64,
    pub max_normal_damage: f64,
    pub max_host_von_mises_stress: f64,
    #[serde(default)]
    pub max_tangent_non_zero_count: usize,
    #[serde(default)]
    pub max_tangent_fill_ratio: f64,
    #[serde(default)]
    pub linear_solver_methods: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub failure_reason: Option<String>,
}
