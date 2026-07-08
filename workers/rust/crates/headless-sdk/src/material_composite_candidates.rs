use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompositePanelCandidate {
    pub id: &'static str,
    pub label: &'static str,
    pub conductor: &'static str,
    pub dielectric: &'static str,
    pub substrate: &'static str,
    pub conductor_conductivity_w_mk: f64,
    pub dielectric_relative_permittivity: f64,
    pub dielectric_breakdown_field_v_m: f64,
    pub substrate_youngs_modulus_pa: f64,
    pub substrate_thermal_expansion_1_k: f64,
    pub areal_mass_kg_m2: f64,
}

pub fn composite_panel_candidates() -> Vec<CompositePanelCandidate> {
    vec![
        CompositePanelCandidate {
            id: "copper_polyimide_aluminum",
            label: "Copper / Polyimide / Aluminum",
            conductor: "copper",
            dielectric: "polyimide",
            substrate: "aluminum_6061",
            conductor_conductivity_w_mk: 390.0,
            dielectric_relative_permittivity: 3.4,
            dielectric_breakdown_field_v_m: 300.0e6,
            substrate_youngs_modulus_pa: 68.9e9,
            substrate_thermal_expansion_1_k: 23.6e-6,
            areal_mass_kg_m2: 2.85,
        },
        CompositePanelCandidate {
            id: "aluminum_alumina_aluminum",
            label: "Aluminum / Alumina / Aluminum",
            conductor: "aluminum",
            dielectric: "alumina_96",
            substrate: "aluminum_6061",
            conductor_conductivity_w_mk: 167.0,
            dielectric_relative_permittivity: 9.8,
            dielectric_breakdown_field_v_m: 130.0e6,
            substrate_youngs_modulus_pa: 68.9e9,
            substrate_thermal_expansion_1_k: 23.6e-6,
            areal_mass_kg_m2: 3.7,
        },
        CompositePanelCandidate {
            id: "copper_ptfe_glass_epoxy",
            label: "Copper / PTFE / Glass epoxy",
            conductor: "copper",
            dielectric: "ptfe",
            substrate: "glass_epoxy",
            conductor_conductivity_w_mk: 390.0,
            dielectric_relative_permittivity: 2.1,
            dielectric_breakdown_field_v_m: 60.0e6,
            substrate_youngs_modulus_pa: 22.0e9,
            substrate_thermal_expansion_1_k: 14.0e-6,
            areal_mass_kg_m2: 2.25,
        },
    ]
}
