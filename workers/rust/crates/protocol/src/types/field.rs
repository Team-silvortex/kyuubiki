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
