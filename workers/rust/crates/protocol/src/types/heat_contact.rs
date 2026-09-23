use serde::{Deserialize, Serialize};

/// A zero-thickness, finite-resistance thermal bond between two matching
/// boundary edges. The two sides must have independent node indices.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HeatPlaneContactInput {
    pub id: String,
    pub side_a: [usize; 2],
    pub side_b: [usize; 2],
    /// Area-specific resistance in m^2 K/W, finite and strictly positive.
    /// Area uses edge length times the common adjacent element thickness.
    pub thermal_resistance_m2_k_w: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HeatPlaneContactResult {
    pub index: usize,
    pub id: String,
    pub side_a: [usize; 2],
    /// Reordered if necessary so endpoints correspond to side_a.
    pub side_b: [usize; 2],
    pub thermal_resistance_m2_k_w: f64,
    pub area_m2: f64,
    pub average_temperature_jump_k: f64,
    pub max_abs_temperature_jump_k: f64,
    /// Signed A-to-B flow, not an extra heat source or dissipated power.
    pub heat_flow_a_to_b_w: f64,
    pub heat_flux_a_to_b_w_m2: f64,
    /// Consistent edge-integrated flow at the paired endpoints. The other
    /// side receives exactly the opposite residual contributions.
    pub nodal_heat_flow_a_to_b_w: [f64; 2],
}
