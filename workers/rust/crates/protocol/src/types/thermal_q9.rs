use serde::{Deserialize, Serialize};

pub const THERMAL_Q9_RECT_SCHEMA: &str = "kyuubiki.thermal-plane-q9-rect/v1";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ThermalQ9Node {
    pub id: String,
    pub x: f64,
    pub y: f64,
    pub load_x: f64,
    pub load_y: f64,
    pub fix_x: bool,
    pub fix_y: bool,
    pub temperature_delta: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ThermalQ9Element {
    pub id: String,
    /// Row-major 3x3 tensor grid, bottom-left to top-right, not perimeter order.
    pub nodes: [usize; 9],
    pub thickness: f64,
    pub youngs_modulus: f64,
    pub poisson_ratio: f64,
    pub thermal_expansion: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SolveThermalQ9RectRequest {
    pub schema_version: String,
    pub nodes: Vec<ThermalQ9Node>,
    pub elements: Vec<ThermalQ9Element>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ThermalQ9ElementResult {
    pub id: String,
    pub nodes: [usize; 9],
    pub area: f64,
    pub mean_stress: [f64; 3],
    pub mean_mechanical_strain: [f64; 3],
    pub strain_energy_j: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SolveThermalQ9RectResult {
    pub schema_version: String,
    pub nodes: Vec<super::plane_results::ThermalPlaneNodeResult>,
    pub elements: Vec<ThermalQ9ElementResult>,
    pub total_strain_energy_j: f64,
    pub solver_iterations: usize,
    pub solver_matrix_non_zero_count: usize,
    pub solver_residual_norm: f64,
    pub solver_rhs_norm: f64,
}
