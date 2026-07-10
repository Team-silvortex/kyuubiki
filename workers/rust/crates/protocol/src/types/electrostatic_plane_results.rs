use crate::{SolveElectrostaticPlaneQuad2dRequest, SolveElectrostaticPlaneTriangle2dRequest};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ElectrostaticPlaneNodeResult {
    pub index: usize,
    pub id: String,
    pub x: f64,
    pub y: f64,
    pub potential: f64,
    pub charge_density: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ElectrostaticPlaneTriangleElementResult {
    pub index: usize,
    pub id: String,
    pub node_i: usize,
    pub node_j: usize,
    pub node_k: usize,
    pub area: f64,
    pub average_potential: f64,
    pub potential_gradient_x: f64,
    pub potential_gradient_y: f64,
    pub electric_field_x: f64,
    pub electric_field_y: f64,
    pub electric_field_magnitude: f64,
    pub electric_flux_density_x: f64,
    pub electric_flux_density_y: f64,
    pub electric_flux_density_magnitude: f64,
    pub electric_energy_density: f64,
    pub stored_energy: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SolveElectrostaticPlaneTriangle2dResult {
    pub input: SolveElectrostaticPlaneTriangle2dRequest,
    pub nodes: Vec<ElectrostaticPlaneNodeResult>,
    pub elements: Vec<ElectrostaticPlaneTriangleElementResult>,
    pub max_potential: f64,
    pub max_electric_field: f64,
    pub max_flux_density: f64,
    pub max_electric_energy_density: f64,
    pub total_stored_energy: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ElectrostaticPlaneQuadElementResult {
    pub index: usize,
    pub id: String,
    pub node_i: usize,
    pub node_j: usize,
    pub node_k: usize,
    pub node_l: usize,
    pub area: f64,
    pub average_potential: f64,
    pub potential_gradient_x: f64,
    pub potential_gradient_y: f64,
    pub electric_field_x: f64,
    pub electric_field_y: f64,
    pub electric_field_magnitude: f64,
    pub electric_flux_density_x: f64,
    pub electric_flux_density_y: f64,
    pub electric_flux_density_magnitude: f64,
    pub electric_energy_density: f64,
    pub stored_energy: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SolveElectrostaticPlaneQuad2dResult {
    pub input: SolveElectrostaticPlaneQuad2dRequest,
    pub nodes: Vec<ElectrostaticPlaneNodeResult>,
    pub elements: Vec<ElectrostaticPlaneQuadElementResult>,
    pub max_potential: f64,
    pub max_electric_field: f64,
    pub max_flux_density: f64,
    pub max_electric_energy_density: f64,
    pub total_stored_energy: f64,
}
