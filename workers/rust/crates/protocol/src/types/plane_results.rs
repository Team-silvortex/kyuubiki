use serde::{Deserialize, Serialize};

use super::plane_frame::{
    SolvePlaneQuad2dRequest, SolvePlaneTriangle2dRequest, SolveThermalPlaneQuad2dRequest,
    SolveThermalPlaneTriangle2dRequest,
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlaneNodeResult {
    pub index: usize,
    pub id: String,
    pub x: f64,
    pub y: f64,
    pub ux: f64,
    pub uy: f64,
    pub displacement_magnitude: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThermalPlaneNodeResult {
    pub index: usize,
    pub id: String,
    pub x: f64,
    pub y: f64,
    pub ux: f64,
    pub uy: f64,
    pub displacement_magnitude: f64,
    pub temperature_delta: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlaneTriangleElementResult {
    pub index: usize,
    pub id: String,
    pub node_i: usize,
    pub node_j: usize,
    pub node_k: usize,
    pub area: f64,
    pub strain_x: f64,
    pub strain_y: f64,
    pub gamma_xy: f64,
    pub stress_x: f64,
    pub stress_y: f64,
    pub tau_xy: f64,
    pub principal_stress_1: f64,
    pub principal_stress_2: f64,
    pub max_in_plane_shear: f64,
    pub von_mises: f64,
    pub strain_energy_density: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SolvePlaneTriangle2dResult {
    pub input: SolvePlaneTriangle2dRequest,
    pub nodes: Vec<PlaneNodeResult>,
    pub elements: Vec<PlaneTriangleElementResult>,
    pub max_displacement: f64,
    pub max_stress: f64,
    pub total_strain_energy: f64,
    pub max_strain_energy_density: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThermalPlaneTriangleElementResult {
    pub index: usize,
    pub id: String,
    pub node_i: usize,
    pub node_j: usize,
    pub node_k: usize,
    pub area: f64,
    pub average_temperature_delta: f64,
    pub thermal_strain: f64,
    pub mechanical_strain_x: f64,
    pub mechanical_strain_y: f64,
    pub total_strain_x: f64,
    pub total_strain_y: f64,
    pub gamma_xy: f64,
    pub stress_x: f64,
    pub stress_y: f64,
    pub tau_xy: f64,
    pub principal_stress_1: f64,
    pub principal_stress_2: f64,
    pub max_in_plane_shear: f64,
    pub von_mises: f64,
    pub strain_energy_density: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SolveThermalPlaneTriangle2dResult {
    pub input: SolveThermalPlaneTriangle2dRequest,
    pub nodes: Vec<ThermalPlaneNodeResult>,
    pub elements: Vec<ThermalPlaneTriangleElementResult>,
    pub max_displacement: f64,
    pub max_stress: f64,
    pub max_temperature_delta: f64,
    pub total_strain_energy: f64,
    pub max_strain_energy_density: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlaneQuadElementResult {
    pub index: usize,
    pub id: String,
    pub node_i: usize,
    pub node_j: usize,
    pub node_k: usize,
    pub node_l: usize,
    pub area: f64,
    pub strain_x: f64,
    pub strain_y: f64,
    pub gamma_xy: f64,
    pub stress_x: f64,
    pub stress_y: f64,
    pub tau_xy: f64,
    pub principal_stress_1: f64,
    pub principal_stress_2: f64,
    pub max_in_plane_shear: f64,
    pub von_mises: f64,
    pub strain_energy_density: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SolvePlaneQuad2dResult {
    pub input: SolvePlaneQuad2dRequest,
    pub nodes: Vec<PlaneNodeResult>,
    pub elements: Vec<PlaneQuadElementResult>,
    pub max_displacement: f64,
    pub max_stress: f64,
    pub total_strain_energy: f64,
    pub max_strain_energy_density: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThermalPlaneQuadElementResult {
    pub index: usize,
    pub id: String,
    pub node_i: usize,
    pub node_j: usize,
    pub node_k: usize,
    pub node_l: usize,
    pub area: f64,
    pub average_temperature_delta: f64,
    pub thermal_strain: f64,
    pub mechanical_strain_x: f64,
    pub mechanical_strain_y: f64,
    pub total_strain_x: f64,
    pub total_strain_y: f64,
    pub gamma_xy: f64,
    pub stress_x: f64,
    pub stress_y: f64,
    pub tau_xy: f64,
    pub principal_stress_1: f64,
    pub principal_stress_2: f64,
    pub max_in_plane_shear: f64,
    pub von_mises: f64,
    pub strain_energy_density: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SolveThermalPlaneQuad2dResult {
    pub input: SolveThermalPlaneQuad2dRequest,
    pub nodes: Vec<ThermalPlaneNodeResult>,
    pub elements: Vec<ThermalPlaneQuadElementResult>,
    pub max_displacement: f64,
    pub max_stress: f64,
    pub max_temperature_delta: f64,
    pub total_strain_energy: f64,
    pub max_strain_energy_density: f64,
}
