use crate::SolveAcousticBar1dRequest;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AcousticBar1dNodeResult {
    pub index: usize,
    pub id: String,
    pub x: f64,
    pub pressure: f64,
    pub sound_pressure_level_db: f64,
    pub volume_velocity_source: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AcousticBar1dElementResult {
    pub index: usize,
    pub id: String,
    pub node_i: usize,
    pub node_j: usize,
    pub length: f64,
    pub area: f64,
    pub density: f64,
    pub bulk_modulus: f64,
    pub speed_of_sound: f64,
    pub wave_number: f64,
    pub pressure_gradient: f64,
    pub particle_velocity: f64,
    pub acoustic_intensity: f64,
    pub damping_loss: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SolveAcousticBar1dResult {
    pub input: SolveAcousticBar1dRequest,
    pub nodes: Vec<AcousticBar1dNodeResult>,
    pub elements: Vec<AcousticBar1dElementResult>,
    pub frequency_hz: f64,
    pub angular_frequency: f64,
    pub max_pressure: f64,
    pub max_sound_pressure_level_db: f64,
    pub max_particle_velocity: f64,
    pub max_acoustic_intensity: f64,
    pub total_damping_loss: f64,
}
