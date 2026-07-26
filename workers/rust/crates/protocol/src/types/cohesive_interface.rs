use serde::{Deserialize, Serialize};

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
pub struct CohesiveInterface2dStepResult {
    pub step: usize,
    pub local_separation: [f64; 2],
    pub local_traction: [f64; 2],
    pub local_tangent: [f64; 2],
    pub global_traction: [f64; 2],
    pub element_nodal_internal_forces: [[f64; 2]; 4],
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
