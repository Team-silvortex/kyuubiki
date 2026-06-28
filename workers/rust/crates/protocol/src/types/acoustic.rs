use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AcousticBar1dNodeInput {
    pub id: String,
    pub x: f64,
    #[serde(default)]
    pub fix_pressure: bool,
    #[serde(default)]
    pub pressure: f64,
    #[serde(default)]
    pub volume_velocity_source: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AcousticBar1dElementInput {
    pub id: String,
    pub node_i: usize,
    pub node_j: usize,
    pub area: f64,
    pub density: f64,
    pub bulk_modulus: f64,
    #[serde(default)]
    pub damping_ratio: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SolveAcousticBar1dRequest {
    pub frequency_hz: f64,
    pub nodes: Vec<AcousticBar1dNodeInput>,
    pub elements: Vec<AcousticBar1dElementInput>,
}
