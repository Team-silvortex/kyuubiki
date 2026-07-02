use serde::{Deserialize, Serialize};

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
pub struct TransientHeatBar1dElementInput {
    pub id: String,
    pub node_i: usize,
    pub node_j: usize,
    pub area: f64,
    pub conductivity: f64,
    pub density: f64,
    pub specific_heat: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SolveTransientHeatBar1dRequest {
    pub nodes: Vec<HeatBar1dNodeInput>,
    pub elements: Vec<TransientHeatBar1dElementInput>,
    pub time_step: f64,
    pub steps: usize,
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
pub struct MagnetostaticBar1dNodeInput {
    pub id: String,
    pub x: f64,
    pub fix_magnetic_potential: bool,
    #[serde(default)]
    pub magnetic_potential: f64,
    #[serde(default)]
    pub magnetomotive_source: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MagnetostaticBar1dElementInput {
    pub id: String,
    pub node_i: usize,
    pub node_j: usize,
    pub area: f64,
    pub permeability: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SolveMagnetostaticBar1dRequest {
    pub nodes: Vec<MagnetostaticBar1dNodeInput>,
    pub elements: Vec<MagnetostaticBar1dElementInput>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AdvectionDiffusionBar1dNodeInput {
    pub id: String,
    pub x: f64,
    pub fix_concentration: bool,
    #[serde(default)]
    pub concentration: f64,
    #[serde(default)]
    pub source: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AdvectionDiffusionBar1dElementInput {
    pub id: String,
    pub node_i: usize,
    pub node_j: usize,
    pub area: f64,
    pub diffusivity: f64,
    pub velocity: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SolveAdvectionDiffusionBar1dRequest {
    pub nodes: Vec<AdvectionDiffusionBar1dNodeInput>,
    pub elements: Vec<AdvectionDiffusionBar1dElementInput>,
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
pub struct MagnetostaticPlaneNodeInput {
    pub id: String,
    pub x: f64,
    pub y: f64,
    pub fix_vector_potential: bool,
    #[serde(default)]
    pub vector_potential: f64,
    #[serde(default)]
    pub current_density: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MagnetostaticPlaneTriangleElementInput {
    pub id: String,
    pub node_i: usize,
    pub node_j: usize,
    pub node_k: usize,
    pub thickness: f64,
    pub permeability: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SolveMagnetostaticPlaneTriangle2dRequest {
    pub nodes: Vec<MagnetostaticPlaneNodeInput>,
    pub elements: Vec<MagnetostaticPlaneTriangleElementInput>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MagnetostaticPlaneQuadElementInput {
    pub id: String,
    pub node_i: usize,
    pub node_j: usize,
    pub node_k: usize,
    pub node_l: usize,
    pub thickness: f64,
    pub permeability: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SolveMagnetostaticPlaneQuad2dRequest {
    pub nodes: Vec<MagnetostaticPlaneNodeInput>,
    pub elements: Vec<MagnetostaticPlaneQuadElementInput>,
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
pub struct StokesFlowPlaneNodeInput {
    pub id: String,
    pub x: f64,
    pub y: f64,
    #[serde(default)]
    pub fix_velocity_x: bool,
    #[serde(default)]
    pub velocity_x: f64,
    #[serde(default)]
    pub fix_velocity_y: bool,
    #[serde(default)]
    pub velocity_y: f64,
    #[serde(default)]
    pub fix_pressure: bool,
    #[serde(default)]
    pub pressure: f64,
    #[serde(default)]
    pub body_force_x: f64,
    #[serde(default)]
    pub body_force_y: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StokesFlowPlaneQuadElementInput {
    pub id: String,
    pub node_i: usize,
    pub node_j: usize,
    pub node_k: usize,
    pub node_l: usize,
    pub thickness: f64,
    pub viscosity: f64,
    pub density: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SolveStokesFlowPlaneQuad2dRequest {
    pub nodes: Vec<StokesFlowPlaneNodeInput>,
    pub elements: Vec<StokesFlowPlaneQuadElementInput>,
}
