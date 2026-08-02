use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ElectricConductionPlaneNodeInput {
    pub id: String,
    pub x: f64,
    pub y: f64,
    pub fix_electric_potential: bool,
    #[serde(default)]
    pub electric_potential_v: f64,
    #[serde(default)]
    pub current_source_a: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ElectricConductionPlaneQuadElementInput {
    pub id: String,
    pub node_i: usize,
    pub node_j: usize,
    pub node_k: usize,
    pub node_l: usize,
    pub thickness: f64,
    pub electrical_conductivity_s_m: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SolveElectricConductionPlaneQuad2dRequest {
    pub nodes: Vec<ElectricConductionPlaneNodeInput>,
    pub elements: Vec<ElectricConductionPlaneQuadElementInput>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ElectricConductionPlaneNodeResult {
    pub index: usize,
    pub id: String,
    pub x: f64,
    pub y: f64,
    pub electric_potential_v: f64,
    pub current_source_a: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ElectricConductionPlaneQuadElementResult {
    pub index: usize,
    pub id: String,
    pub node_i: usize,
    pub node_j: usize,
    pub node_k: usize,
    pub node_l: usize,
    pub area_m2: f64,
    pub average_electric_potential_v: f64,
    pub electric_potential_gradient_x_v_m: f64,
    pub electric_potential_gradient_y_v_m: f64,
    pub electric_field_x_v_m: f64,
    pub electric_field_y_v_m: f64,
    pub electric_field_magnitude_v_m: f64,
    pub current_density_x_a_m2: f64,
    pub current_density_y_a_m2: f64,
    pub current_density_magnitude_a_m2: f64,
    pub volumetric_joule_heating_w_m3: f64,
    pub joule_power_w: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SolveElectricConductionPlaneQuad2dResult {
    pub input: SolveElectricConductionPlaneQuad2dRequest,
    pub nodes: Vec<ElectricConductionPlaneNodeResult>,
    pub elements: Vec<ElectricConductionPlaneQuadElementResult>,
    pub max_electric_potential_v: f64,
    pub max_electric_field_v_m: f64,
    pub max_current_density_a_m2: f64,
    pub total_joule_power_w: f64,
}
