use crate::{SolveHarmonicSpring1dRequest, SolveTransientSpring1dRequest};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TransientSpring1dNodeResult {
    pub index: usize,
    pub id: String,
    pub x: f64,
    pub ux: f64,
    pub vx: f64,
    pub ax: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TransientSpring1dElementResult {
    pub index: usize,
    pub id: String,
    pub node_i: usize,
    pub node_j: usize,
    pub extension: f64,
    pub relative_velocity: f64,
    pub spring_force: f64,
    pub damping_force: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TransientSpring1dStepResult {
    pub step: usize,
    pub time: f64,
    pub max_displacement: f64,
    pub max_velocity: f64,
    pub kinetic_energy: f64,
    pub strain_energy: f64,
    pub displacements: Vec<f64>,
    pub velocities: Vec<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SolveTransientSpring1dResult {
    pub input: SolveTransientSpring1dRequest,
    pub nodes: Vec<TransientSpring1dNodeResult>,
    pub elements: Vec<TransientSpring1dElementResult>,
    pub history: Vec<TransientSpring1dStepResult>,
    pub final_time: f64,
    pub max_displacement: f64,
    pub max_velocity: f64,
    pub max_force: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HarmonicSpring1dNodeResponse {
    pub index: usize,
    pub id: String,
    pub displacement_amplitude: f64,
    pub displacement_phase_deg: f64,
    pub velocity_amplitude: f64,
    pub acceleration_amplitude: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HarmonicSpring1dElementResponse {
    pub index: usize,
    pub id: String,
    pub node_i: usize,
    pub node_j: usize,
    pub extension_amplitude: f64,
    pub force_amplitude: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HarmonicSpring1dFrequencyResult {
    pub frequency_hz: f64,
    pub angular_frequency: f64,
    pub nodes: Vec<HarmonicSpring1dNodeResponse>,
    pub elements: Vec<HarmonicSpring1dElementResponse>,
    pub max_displacement: f64,
    pub max_velocity: f64,
    pub max_acceleration: f64,
    pub max_force: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SolveHarmonicSpring1dResult {
    pub input: SolveHarmonicSpring1dRequest,
    pub frequencies: Vec<HarmonicSpring1dFrequencyResult>,
    pub max_displacement: f64,
    pub max_velocity: f64,
    pub max_acceleration: f64,
    pub max_force: f64,
    pub peak_frequency_hz: f64,
}
