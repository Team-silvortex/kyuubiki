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
pub struct ElectricConductionContactInput {
    pub id: String,
    pub node_i: usize,
    pub node_j: usize,
    pub contact_resistance_ohm: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ElectricConductionTerminalInput {
    pub id: String,
    pub node: usize,
    pub external_potential_v: f64,
    pub impedance_ohm: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SolveElectricConductionPlaneQuad2dRequest {
    pub nodes: Vec<ElectricConductionPlaneNodeInput>,
    pub elements: Vec<ElectricConductionPlaneQuadElementInput>,
    #[serde(default)]
    pub contact_interfaces: Vec<ElectricConductionContactInput>,
    #[serde(default)]
    pub terminals: Vec<ElectricConductionTerminalInput>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ElectricConductionPlaneNodeResult {
    pub index: usize,
    pub id: String,
    pub x: f64,
    pub y: f64,
    pub fix_electric_potential: bool,
    pub electric_potential_v: f64,
    pub current_source_a: f64,
    pub reaction_current_a: f64,
    pub net_injected_current_a: f64,
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
    pub rms_electric_field_magnitude_v_m: f64,
    pub peak_electric_field_magnitude_v_m: f64,
    pub current_density_x_a_m2: f64,
    pub current_density_y_a_m2: f64,
    pub current_density_magnitude_a_m2: f64,
    pub rms_current_density_magnitude_a_m2: f64,
    pub peak_current_density_magnitude_a_m2: f64,
    pub volumetric_joule_heating_w_m3: f64,
    pub joule_power_w: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ElectricConductionContactResult {
    pub index: usize,
    pub id: String,
    pub node_i: usize,
    pub node_j: usize,
    pub contact_resistance_ohm: f64,
    pub voltage_drop_v: f64,
    pub current_i_to_j_a: f64,
    pub joule_power_w: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ElectricConductionTerminalResult {
    pub index: usize,
    pub id: String,
    pub node: usize,
    pub external_potential_v: f64,
    pub node_potential_v: f64,
    pub impedance_ohm: f64,
    pub current_into_domain_a: f64,
    pub impedance_joule_power_w: f64,
    pub power_delivered_to_domain_w: f64,
    pub source_power_w: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SolveElectricConductionPlaneQuad2dResult {
    pub input: SolveElectricConductionPlaneQuad2dRequest,
    pub nodes: Vec<ElectricConductionPlaneNodeResult>,
    pub elements: Vec<ElectricConductionPlaneQuadElementResult>,
    pub contact_interfaces: Vec<ElectricConductionContactResult>,
    pub terminals: Vec<ElectricConductionTerminalResult>,
    pub max_electric_potential_v: f64,
    pub max_electric_field_v_m: f64,
    pub max_current_density_a_m2: f64,
    pub total_injected_current_a: f64,
    pub total_extracted_current_a: f64,
    pub current_balance_relative_error: f64,
    pub max_free_current_residual_a: f64,
    pub free_current_residual_relative_error: f64,
    pub total_electrical_input_power_w: f64,
    pub total_bulk_joule_power_w: f64,
    pub total_contact_joule_power_w: f64,
    pub total_joule_power_w: f64,
    pub power_balance_relative_error: f64,
    pub total_terminal_impedance_power_w: f64,
    pub total_source_power_w: f64,
    pub total_dissipated_power_w: f64,
    pub source_power_balance_relative_error: f64,
}
