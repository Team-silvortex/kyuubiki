use crate::material_composite::CompositePanelCandidate;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompositePanelMaterialRegion {
    pub id: String,
    pub role: String,
    pub material_family: String,
    pub elements: Vec<String>,
    pub active_fields: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompositePanelInterfaceAssessment {
    pub id: String,
    pub from_region: String,
    pub to_region: String,
    pub interface_role: String,
    pub thermal_expansion_delta_1_k: f64,
    pub stiffness_ratio: f64,
    pub risk_score: f64,
    pub compatibility_score: f64,
    pub dominant_driver: String,
}

#[derive(Debug, Clone, Copy)]
struct InterfaceMaterial {
    youngs_modulus_pa: f64,
    thermal_expansion_1_k: f64,
}

pub fn composite_material_regions() -> Vec<CompositePanelMaterialRegion> {
    vec![
        CompositePanelMaterialRegion {
            id: "conductor_left".to_string(),
            role: "conductor_heat_spreader".to_string(),
            material_family: "metal".to_string(),
            elements: vec!["conductor_left".to_string()],
            active_fields: all_fields(),
        },
        CompositePanelMaterialRegion {
            id: "dielectric_core".to_string(),
            role: "electrical_isolation".to_string(),
            material_family: "dielectric".to_string(),
            elements: vec!["dielectric_core".to_string()],
            active_fields: all_fields(),
        },
        CompositePanelMaterialRegion {
            id: "substrate_right".to_string(),
            role: "mechanical_support".to_string(),
            material_family: "substrate".to_string(),
            elements: vec!["substrate_right".to_string()],
            active_fields: all_fields(),
        },
    ]
}

pub fn assess_composite_interfaces(
    candidate: &CompositePanelCandidate,
) -> Vec<CompositePanelInterfaceAssessment> {
    let conductor = conductor_material(candidate);
    let dielectric = dielectric_material(candidate);
    let substrate = substrate_material(candidate);
    vec![
        assess_interface(
            "interface.conductor_dielectric",
            "conductor_left",
            "dielectric_core",
            "electrical_isolation_bond",
            conductor,
            dielectric,
            1.10,
        ),
        assess_interface(
            "interface.dielectric_substrate",
            "dielectric_core",
            "substrate_right",
            "structural_support_bond",
            dielectric,
            substrate,
            1.0,
        ),
    ]
}

fn assess_interface(
    id: &str,
    from_region: &str,
    to_region: &str,
    interface_role: &str,
    left: InterfaceMaterial,
    right: InterfaceMaterial,
    role_factor: f64,
) -> CompositePanelInterfaceAssessment {
    let thermal_expansion_delta_1_k =
        (left.thermal_expansion_1_k - right.thermal_expansion_1_k).abs();
    let stiffness_ratio = stiffness_ratio(left.youngs_modulus_pa, right.youngs_modulus_pa);
    let cte_component = (thermal_expansion_delta_1_k / 45.0e-6).clamp(0.0, 1.0);
    let stiffness_component = (stiffness_ratio.log10() / 3.0).clamp(0.0, 1.0);
    let risk_score =
        ((0.65 * cte_component + 0.35 * stiffness_component) * role_factor).clamp(0.0, 1.0);
    CompositePanelInterfaceAssessment {
        id: id.to_string(),
        from_region: from_region.to_string(),
        to_region: to_region.to_string(),
        interface_role: interface_role.to_string(),
        thermal_expansion_delta_1_k,
        stiffness_ratio,
        risk_score,
        compatibility_score: 1.0 - risk_score,
        dominant_driver: dominant_driver(cte_component, stiffness_component),
    }
}

fn conductor_material(candidate: &CompositePanelCandidate) -> InterfaceMaterial {
    match candidate.conductor {
        "aluminum" => InterfaceMaterial {
            youngs_modulus_pa: 69.0e9,
            thermal_expansion_1_k: 23.1e-6,
        },
        _ => InterfaceMaterial {
            youngs_modulus_pa: 110.0e9,
            thermal_expansion_1_k: 17.0e-6,
        },
    }
}

fn dielectric_material(candidate: &CompositePanelCandidate) -> InterfaceMaterial {
    match candidate.dielectric {
        "alumina_96" => InterfaceMaterial {
            youngs_modulus_pa: 300.0e9,
            thermal_expansion_1_k: 7.5e-6,
        },
        "ptfe" => InterfaceMaterial {
            youngs_modulus_pa: 0.5e9,
            thermal_expansion_1_k: 120.0e-6,
        },
        _ => InterfaceMaterial {
            youngs_modulus_pa: 2.5e9,
            thermal_expansion_1_k: 45.0e-6,
        },
    }
}

fn substrate_material(candidate: &CompositePanelCandidate) -> InterfaceMaterial {
    InterfaceMaterial {
        youngs_modulus_pa: candidate.substrate_youngs_modulus_pa,
        thermal_expansion_1_k: candidate.substrate_thermal_expansion_1_k,
    }
}

fn stiffness_ratio(left: f64, right: f64) -> f64 {
    let min = left.min(right).max(f64::EPSILON);
    left.max(right) / min
}

fn dominant_driver(cte_component: f64, stiffness_component: f64) -> String {
    if cte_component >= stiffness_component {
        "thermal_expansion_mismatch".to_string()
    } else {
        "stiffness_contrast".to_string()
    }
}

fn all_fields() -> Vec<String> {
    vec![
        "electrostatic".to_string(),
        "heat".to_string(),
        "thermal_stress".to_string(),
    ]
}
