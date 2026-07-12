use crate::{SolveStokesFlowPlaneQuad2dRequest, SolveStokesFlowPlaneTriangle2dRequest};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StokesFlowPlaneNodeResult {
    pub index: usize,
    pub id: String,
    pub x: f64,
    pub y: f64,
    pub velocity_x: f64,
    pub velocity_y: f64,
    pub velocity_magnitude: f64,
    pub pressure: f64,
    pub body_force_x: f64,
    pub body_force_y: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StokesFlowPlaneQuadElementResult {
    pub index: usize,
    pub id: String,
    pub node_i: usize,
    pub node_j: usize,
    pub node_k: usize,
    pub node_l: usize,
    pub area: f64,
    pub average_velocity_x: f64,
    pub average_velocity_y: f64,
    pub average_velocity_magnitude: f64,
    pub average_pressure: f64,
    pub velocity_gradient_x: f64,
    pub velocity_gradient_y: f64,
    pub shear_rate: f64,
    pub max_viscous_shear_stress: f64,
    pub divergence_error: f64,
    pub reynolds_number: f64,
    pub viscous_dissipation: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StokesFlowPlaneTriangleElementResult {
    pub index: usize,
    pub id: String,
    pub node_i: usize,
    pub node_j: usize,
    pub node_k: usize,
    pub area: f64,
    pub average_velocity_x: f64,
    pub average_velocity_y: f64,
    pub average_velocity_magnitude: f64,
    pub average_pressure: f64,
    pub velocity_gradient_x: f64,
    pub velocity_gradient_y: f64,
    pub shear_rate: f64,
    pub max_viscous_shear_stress: f64,
    pub divergence_error: f64,
    pub reynolds_number: f64,
    pub viscous_dissipation: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SolveStokesFlowPlaneQuad2dResult {
    pub input: SolveStokesFlowPlaneQuad2dRequest,
    pub nodes: Vec<StokesFlowPlaneNodeResult>,
    pub elements: Vec<StokesFlowPlaneQuadElementResult>,
    pub max_velocity: f64,
    pub max_pressure: f64,
    pub pressure_drop: f64,
    pub max_divergence_error: f64,
    pub max_reynolds_number: f64,
    pub max_shear_rate: f64,
    pub max_viscous_shear_stress: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SolveStokesFlowPlaneTriangle2dResult {
    pub input: SolveStokesFlowPlaneTriangle2dRequest,
    pub nodes: Vec<StokesFlowPlaneNodeResult>,
    pub elements: Vec<StokesFlowPlaneTriangleElementResult>,
    pub max_velocity: f64,
    pub max_pressure: f64,
    pub pressure_drop: f64,
    pub max_divergence_error: f64,
    pub max_reynolds_number: f64,
    pub max_shear_rate: f64,
    pub max_viscous_shear_stress: f64,
}
