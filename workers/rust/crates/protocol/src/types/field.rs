use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "crate::axial_bar_input::AxialBarInput")]
pub struct SolveBarRequest {
    pub length: f64,
    pub area: f64,
    pub youngs_modulus: f64,
    pub elements: usize,
    pub tip_force: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThermalBar1dNodeInput {
    #[serde(default)]
    pub id: String,
    pub x: f64,
    pub fix_x: bool,
    pub load_x: f64,
    #[serde(default)]
    pub temperature_delta: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThermalBar1dElementInput {
    #[serde(default)]
    pub id: String,
    pub node_i: usize,
    pub node_j: usize,
    pub area: f64,
    pub youngs_modulus: f64,
    pub thermal_expansion: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SolveThermalBar1dRequest {
    #[serde(deserialize_with = "crate::graph_entity_input::deserialize_nodes")]
    pub nodes: Vec<ThermalBar1dNodeInput>,
    #[serde(deserialize_with = "crate::graph_entity_input::deserialize_elements")]
    pub elements: Vec<ThermalBar1dElementInput>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HeatBar1dNodeInput {
    #[serde(default)]
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
    #[serde(default)]
    pub id: String,
    pub node_i: usize,
    pub node_j: usize,
    pub area: f64,
    pub conductivity: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SolveHeatBar1dRequest {
    #[serde(deserialize_with = "crate::graph_entity_input::deserialize_nodes")]
    pub nodes: Vec<HeatBar1dNodeInput>,
    #[serde(deserialize_with = "crate::graph_entity_input::deserialize_elements")]
    pub elements: Vec<HeatBar1dElementInput>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TransientHeatBar1dElementInput {
    #[serde(default)]
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
    #[serde(deserialize_with = "crate::graph_entity_input::deserialize_nodes")]
    pub nodes: Vec<HeatBar1dNodeInput>,
    #[serde(deserialize_with = "crate::graph_entity_input::deserialize_elements")]
    pub elements: Vec<TransientHeatBar1dElementInput>,
    pub time_step: f64,
    pub steps: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub history_stride: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ElectrostaticBar1dNodeInput {
    #[serde(default)]
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
    #[serde(default)]
    pub id: String,
    pub node_i: usize,
    pub node_j: usize,
    pub area: f64,
    pub permittivity: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SolveElectrostaticBar1dRequest {
    #[serde(deserialize_with = "crate::graph_entity_input::deserialize_nodes")]
    pub nodes: Vec<ElectrostaticBar1dNodeInput>,
    #[serde(deserialize_with = "crate::graph_entity_input::deserialize_elements")]
    pub elements: Vec<ElectrostaticBar1dElementInput>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MagnetostaticBar1dNodeInput {
    #[serde(default)]
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
    #[serde(default)]
    pub id: String,
    pub node_i: usize,
    pub node_j: usize,
    pub area: f64,
    pub permeability: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SolveMagnetostaticBar1dRequest {
    #[serde(deserialize_with = "crate::graph_entity_input::deserialize_nodes")]
    pub nodes: Vec<MagnetostaticBar1dNodeInput>,
    #[serde(deserialize_with = "crate::graph_entity_input::deserialize_elements")]
    pub elements: Vec<MagnetostaticBar1dElementInput>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AdvectionDiffusionBar1dNodeInput {
    #[serde(default)]
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
    #[serde(default)]
    pub id: String,
    pub node_i: usize,
    pub node_j: usize,
    pub area: f64,
    pub diffusivity: f64,
    pub velocity: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SolveAdvectionDiffusionBar1dRequest {
    #[serde(
        default,
        skip_serializing_if = "AdvectionDiffusionBar1dScheme::is_galerkin"
    )]
    pub scheme: AdvectionDiffusionBar1dScheme,
    #[serde(deserialize_with = "crate::graph_entity_input::deserialize_nodes")]
    pub nodes: Vec<AdvectionDiffusionBar1dNodeInput>,
    #[serde(deserialize_with = "crate::graph_entity_input::deserialize_elements")]
    pub elements: Vec<AdvectionDiffusionBar1dElementInput>,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AdvectionDiffusionBar1dScheme {
    #[default]
    Galerkin,
    Upwind,
}

impl AdvectionDiffusionBar1dScheme {
    pub fn is_galerkin(&self) -> bool {
        *self == Self::Galerkin
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HeatPlaneNodeInput {
    #[serde(default)]
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
    #[serde(default)]
    pub id: String,
    pub node_i: usize,
    pub node_j: usize,
    pub node_k: usize,
    pub thickness: f64,
    pub conductivity: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SolveHeatPlaneTriangle2dRequest {
    #[serde(deserialize_with = "crate::graph_entity_input::deserialize_nodes")]
    pub nodes: Vec<HeatPlaneNodeInput>,
    #[serde(deserialize_with = "crate::graph_entity_input::deserialize_elements")]
    pub elements: Vec<HeatPlaneTriangleElementInput>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub contact_interfaces: Vec<crate::HeatPlaneContactInput>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ElectrostaticPlaneNodeInput {
    #[serde(default)]
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
    #[serde(default)]
    pub id: String,
    pub node_i: usize,
    pub node_j: usize,
    pub node_k: usize,
    pub thickness: f64,
    pub permittivity: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SolveElectrostaticPlaneTriangle2dRequest {
    #[serde(deserialize_with = "crate::graph_entity_input::deserialize_nodes")]
    pub nodes: Vec<ElectrostaticPlaneNodeInput>,
    #[serde(deserialize_with = "crate::graph_entity_input::deserialize_elements")]
    pub elements: Vec<ElectrostaticPlaneTriangleElementInput>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ElectrostaticPlaneQuadElementInput {
    #[serde(default)]
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
    #[serde(deserialize_with = "crate::graph_entity_input::deserialize_nodes")]
    pub nodes: Vec<ElectrostaticPlaneNodeInput>,
    #[serde(deserialize_with = "crate::graph_entity_input::deserialize_elements")]
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
    #[serde(default)]
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
    #[serde(deserialize_with = "crate::graph_entity_input::deserialize_nodes")]
    pub nodes: Vec<HeatPlaneNodeInput>,
    #[serde(deserialize_with = "crate::graph_entity_input::deserialize_elements")]
    pub elements: Vec<HeatPlaneQuadElementInput>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub contact_interfaces: Vec<crate::HeatPlaneContactInput>,
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
pub struct StokesFlowPlaneTriangleElementInput {
    pub id: String,
    pub node_i: usize,
    pub node_j: usize,
    pub node_k: usize,
    pub thickness: f64,
    pub viscosity: f64,
    pub density: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SolveStokesFlowPlaneQuad2dRequest {
    pub nodes: Vec<StokesFlowPlaneNodeInput>,
    pub elements: Vec<StokesFlowPlaneQuadElementInput>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SolveStokesFlowPlaneTriangle2dRequest {
    pub nodes: Vec<StokesFlowPlaneNodeInput>,
    pub elements: Vec<StokesFlowPlaneTriangleElementInput>,
}
