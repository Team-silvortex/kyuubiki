use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MaterialResearchCandidate {
    pub id: &'static str,
    pub label: &'static str,
    pub family: &'static str,
    pub thermal_conductivity_w_mk: f64,
    pub density_kg_m3: f64,
    pub note: &'static str,
}

pub fn heat_spreader_screening_candidates() -> Vec<MaterialResearchCandidate> {
    vec![
        MaterialResearchCandidate {
            id: "aluminum_6061",
            label: "Aluminum 6061",
            family: "metal",
            thermal_conductivity_w_mk: 167.0,
            density_kg_m3: 2700.0,
            note: "balanced lightweight baseline",
        },
        MaterialResearchCandidate {
            id: "copper_c110",
            label: "Copper C110",
            family: "metal",
            thermal_conductivity_w_mk: 385.0,
            density_kg_m3: 8960.0,
            note: "high-conductivity heavy baseline",
        },
        MaterialResearchCandidate {
            id: "pyrolytic_graphite_in_plane",
            label: "Pyrolytic graphite, in-plane",
            family: "carbon",
            thermal_conductivity_w_mk: 1500.0,
            density_kg_m3: 2200.0,
            note: "anisotropic high-spreading candidate",
        },
    ]
}
