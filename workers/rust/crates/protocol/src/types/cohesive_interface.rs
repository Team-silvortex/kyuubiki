use serde::{Deserialize, Serialize};

use super::plane_frame::{PlaneQuadElementInput, PlaneTriangleElementInput};
use super::plane_results::{PlaneQuadElementResult, PlaneTriangleElementResult};
use super::space_structural::{TrussElementInput, TrussElementResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CohesiveTractionRegime {
    Compression,
    Elastic,
    ElasticOpening,
    Softening,
    UnloadingReloading,
    Failed,
}

pub type CohesiveInterface1dRegime = CohesiveTractionRegime;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SolveCohesiveInterface1dRequest {
    pub id: String,
    pub initial_stiffness: f64,
    pub compression_stiffness: f64,
    pub peak_traction: f64,
    pub failure_separation: f64,
    pub separation_history: Vec<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CohesiveInterface1dStepResult {
    pub step: usize,
    pub separation: f64,
    pub traction: f64,
    pub tangent_stiffness: f64,
    pub damage: f64,
    pub max_opening: f64,
    pub regime: CohesiveTractionRegime,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SolveCohesiveInterface1dResult {
    pub input: SolveCohesiveInterface1dRequest,
    pub onset_separation: f64,
    pub fracture_energy: f64,
    pub steps: Vec<CohesiveInterface1dStepResult>,
    pub max_traction: f64,
    pub max_damage: f64,
    pub fully_failed: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CohesiveInterface2dNodeInput {
    pub id: String,
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CohesiveInterface2dElementInput {
    pub id: String,
    pub lower_i: usize,
    pub lower_j: usize,
    pub upper_i: usize,
    pub upper_j: usize,
    pub thickness: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CohesiveInterface2dMaterialInput {
    pub normal_initial_stiffness: f64,
    pub normal_compression_stiffness: f64,
    pub normal_peak_traction: f64,
    pub normal_failure_separation: f64,
    pub shear_initial_stiffness: f64,
    pub shear_peak_traction: f64,
    pub shear_failure_separation: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CohesiveInterface2dDisplacementStepInput {
    pub nodal_displacements: Vec<[f64; 2]>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SolveCohesiveInterface2dRequest {
    pub nodes: Vec<CohesiveInterface2dNodeInput>,
    pub element: CohesiveInterface2dElementInput,
    pub material: CohesiveInterface2dMaterialInput,
    pub displacement_history: Vec<CohesiveInterface2dDisplacementStepInput>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CohesiveInterface2dIntegrationPointResult {
    pub natural_coordinate: f64,
    pub local_separation: [f64; 2],
    pub local_traction: [f64; 2],
    pub local_tangent: [f64; 2],
    pub shear_damage: f64,
    pub normal_damage: f64,
    pub max_shear_separation: f64,
    pub max_normal_opening: f64,
    pub shear_regime: CohesiveTractionRegime,
    pub normal_regime: CohesiveTractionRegime,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CohesiveInterface2dStepResult {
    pub step: usize,
    pub local_separation: [f64; 2],
    pub local_traction: [f64; 2],
    pub local_tangent: [f64; 2],
    pub global_traction: [f64; 2],
    pub element_nodal_internal_forces: [[f64; 2]; 4],
    pub element_tangent: [[f64; 8]; 8],
    pub integration_points: Vec<CohesiveInterface2dIntegrationPointResult>,
    pub shear_damage: f64,
    pub normal_damage: f64,
    pub max_shear_separation: f64,
    pub max_normal_opening: f64,
    pub shear_regime: CohesiveTractionRegime,
    pub normal_regime: CohesiveTractionRegime,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SolveCohesiveInterface2dResult {
    pub input: SolveCohesiveInterface2dRequest,
    pub interface_length: f64,
    pub interface_area: f64,
    pub local_tangent_direction: [f64; 2],
    pub local_normal_direction: [f64; 2],
    pub shear_onset_separation: f64,
    pub normal_onset_separation: f64,
    pub shear_fracture_energy: f64,
    pub normal_fracture_energy: f64,
    pub steps: Vec<CohesiveInterface2dStepResult>,
    pub max_resultant_traction: f64,
    pub max_shear_damage: f64,
    pub max_normal_damage: f64,
    pub shear_failed: bool,
    pub normal_failed: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CohesiveInterfaceMesh2dNodeInput {
    pub id: String,
    pub x: f64,
    pub y: f64,
    pub fixed: [bool; 2],
    #[serde(default)]
    pub prescribed_displacement: Option<[f64; 2]>,
    pub load: [f64; 2],
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CohesiveInterfaceMesh2dMaterialInput {
    pub id: String,
    pub properties: CohesiveInterface2dMaterialInput,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CohesiveInterfaceMesh2dElementInput {
    pub id: String,
    pub lower_i: usize,
    pub lower_j: usize,
    pub upper_i: usize,
    pub upper_j: usize,
    pub thickness: f64,
    pub material_id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CohesiveInterfaceMesh2dConnectorSpringInput {
    pub id: String,
    pub node_i: usize,
    pub node_j: usize,
    pub stiffness: [f64; 2],
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CohesiveInterfaceMesh2dConnectorSpringResult {
    pub id: String,
    pub node_i: usize,
    pub node_j: usize,
    pub relative_displacement: [f64; 2],
    pub force: [f64; 2],
    pub strain_energy: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CohesiveInterfaceMesh2dControlStepInput {
    pub load_factor: f64,
    pub prescribed_displacements: Vec<[f64; 2]>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SolveCohesiveInterfaceMesh2dRequest {
    pub id: String,
    pub nodes: Vec<CohesiveInterfaceMesh2dNodeInput>,
    pub materials: Vec<CohesiveInterfaceMesh2dMaterialInput>,
    pub elements: Vec<CohesiveInterfaceMesh2dElementInput>,
    #[serde(default)]
    pub connector_springs: Vec<CohesiveInterfaceMesh2dConnectorSpringInput>,
    #[serde(default)]
    pub host_trusses: Vec<TrussElementInput>,
    #[serde(default)]
    pub host_plane_triangles: Vec<PlaneTriangleElementInput>,
    #[serde(default)]
    pub host_plane_quads: Vec<PlaneQuadElementInput>,
    #[serde(default)]
    pub load_steps: Option<usize>,
    #[serde(default)]
    pub control_history: Option<Vec<CohesiveInterfaceMesh2dControlStepInput>>,
    #[serde(default)]
    pub max_iterations: Option<usize>,
    #[serde(default)]
    pub tolerance: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CohesiveInterfaceMesh2dNodeResult {
    pub id: String,
    pub displacement: [f64; 2],
    pub reaction: [f64; 2],
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CohesiveInterfaceMesh2dElementResult {
    pub id: String,
    pub material_id: String,
    pub local_separation: [f64; 2],
    pub local_traction: [f64; 2],
    pub max_shear_damage: f64,
    pub max_normal_damage: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CohesiveInterfaceMesh2dLoadStepResult {
    pub step: usize,
    pub load_factor: f64,
    pub iterations: usize,
    pub residual_norm: f64,
    pub converged: bool,
    pub max_displacement: f64,
    pub prescribed_displacement_norm: f64,
    pub reaction_norm: f64,
    pub max_resultant_traction: f64,
    pub max_shear_damage: f64,
    pub max_normal_damage: f64,
    pub max_connector_force: f64,
    pub max_host_truss_axial_force: f64,
    pub max_host_truss_stress: f64,
    pub max_host_plane_stress: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SolveCohesiveInterfaceMesh2dResult {
    pub input: SolveCohesiveInterfaceMesh2dRequest,
    pub nodes: Vec<CohesiveInterfaceMesh2dNodeResult>,
    pub elements: Vec<CohesiveInterfaceMesh2dElementResult>,
    pub connector_springs: Vec<CohesiveInterfaceMesh2dConnectorSpringResult>,
    pub host_trusses: Vec<TrussElementResult>,
    pub host_plane_triangles: Vec<PlaneTriangleElementResult>,
    pub host_plane_quads: Vec<PlaneQuadElementResult>,
    pub steps: Vec<CohesiveInterfaceMesh2dLoadStepResult>,
    pub converged: bool,
    pub completed_load_factor: f64,
    pub residual_norm: f64,
    pub max_displacement: f64,
    pub max_shear_damage: f64,
    pub max_normal_damage: f64,
    pub max_connector_force: f64,
    pub max_host_truss_axial_force: f64,
    pub max_host_truss_stress: f64,
    pub max_host_plane_stress: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub failure_reason: Option<String>,
}
