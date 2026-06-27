use crate::{SolveMagnetostaticPlaneQuad2dRequest, SolveMagnetostaticPlaneTriangle2dRequest};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MagnetostaticPlaneNodeResult {
    pub index: usize,
    pub id: String,
    pub x: f64,
    pub y: f64,
    pub vector_potential: f64,
    pub current_density: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MagnetostaticPlaneTriangleElementResult {
    pub index: usize,
    pub id: String,
    pub node_i: usize,
    pub node_j: usize,
    pub node_k: usize,
    pub area: f64,
    pub average_vector_potential: f64,
    pub vector_potential_gradient_x: f64,
    pub vector_potential_gradient_y: f64,
    pub magnetic_field_strength_x: f64,
    pub magnetic_field_strength_y: f64,
    pub magnetic_field_strength_magnitude: f64,
    pub magnetic_flux_density_x: f64,
    pub magnetic_flux_density_y: f64,
    pub magnetic_flux_density_magnitude: f64,
    pub stored_energy: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SolveMagnetostaticPlaneTriangle2dResult {
    pub input: SolveMagnetostaticPlaneTriangle2dRequest,
    pub nodes: Vec<MagnetostaticPlaneNodeResult>,
    pub elements: Vec<MagnetostaticPlaneTriangleElementResult>,
    pub max_vector_potential: f64,
    pub max_magnetic_field_strength: f64,
    pub max_flux_density: f64,
    pub total_stored_energy: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MagnetostaticPlaneQuadElementResult {
    pub index: usize,
    pub id: String,
    pub node_i: usize,
    pub node_j: usize,
    pub node_k: usize,
    pub node_l: usize,
    pub area: f64,
    pub average_vector_potential: f64,
    pub vector_potential_gradient_x: f64,
    pub vector_potential_gradient_y: f64,
    pub magnetic_field_strength_x: f64,
    pub magnetic_field_strength_y: f64,
    pub magnetic_field_strength_magnitude: f64,
    pub magnetic_flux_density_x: f64,
    pub magnetic_flux_density_y: f64,
    pub magnetic_flux_density_magnitude: f64,
    pub stored_energy: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SolveMagnetostaticPlaneQuad2dResult {
    pub input: SolveMagnetostaticPlaneQuad2dRequest,
    pub nodes: Vec<MagnetostaticPlaneNodeResult>,
    pub elements: Vec<MagnetostaticPlaneQuadElementResult>,
    pub max_vector_potential: f64,
    pub max_magnetic_field_strength: f64,
    pub max_flux_density: f64,
    pub total_stored_energy: f64,
}
