use crate::material_composite_candidates::CompositePanelCandidate;
use serde_json::{Value, json};

pub(crate) fn composite_research_metadata(candidate: &CompositePanelCandidate) -> Value {
    json!({
        "study": "material.composite_thermo_electric_panel.v1",
        "candidate_id": candidate.id,
        "candidate_label": candidate.label,
        "materials": {
            "conductor": candidate.conductor,
            "dielectric": candidate.dielectric,
            "substrate": candidate.substrate
        },
        "coupling": "electrostatic_to_heat_to_thermal_stress"
    })
}

pub(crate) fn electrostatic_model(candidate: &CompositePanelCandidate) -> Value {
    json!({
        "nodes": panel_nodes("potential"),
        "elements": [
            quad("conductor_left", 0, 1, 5, 4, 1.0),
            quad("dielectric_core", 1, 2, 6, 5, candidate.dielectric_relative_permittivity),
            quad("substrate_right", 2, 3, 7, 6, 4.2)
        ]
    })
}

pub(crate) fn heat_model(candidate: &CompositePanelCandidate) -> Value {
    json!({
        "nodes": panel_nodes("temperature"),
        "elements": [
            heat_quad("conductor_left", 0, 1, 5, 4, candidate.conductor_conductivity_w_mk),
            heat_quad("dielectric_core", 1, 2, 6, 5, 0.25),
            heat_quad("substrate_right", 2, 3, 7, 6, 160.0)
        ]
    })
}

pub(crate) fn thermal_model(candidate: &CompositePanelCandidate) -> Value {
    json!({
        "nodes": panel_nodes("thermal"),
        "elements": [
            thermal_quad("conductor_left", 0, 1, 5, 4, 110.0e9, 17.0e-6),
            thermal_quad("dielectric_core", 1, 2, 6, 5, 2.5e9, 45.0e-6),
            thermal_quad("substrate_right", 2, 3, 7, 6, candidate.substrate_youngs_modulus_pa, candidate.substrate_thermal_expansion_1_k)
        ]
    })
}

fn panel_nodes(kind: &str) -> Vec<Value> {
    let coords = [
        (0.0, 0.0),
        (0.03, 0.0),
        (0.06, 0.0),
        (0.09, 0.0),
        (0.0, 0.03),
        (0.03, 0.03),
        (0.06, 0.03),
        (0.09, 0.03),
    ];
    coords
        .iter()
        .enumerate()
        .map(|(index, (x, y))| match kind {
            "potential" => json!({
                "id": format!("n{index}"),
                "x": x,
                "y": y,
                "fix_potential": matches!(index, 0 | 3 | 4 | 7),
                "potential": if matches!(index, 3 | 7) { 900.0 } else { 0.0 },
                "charge_density": 0.0
            }),
            "temperature" => json!({
                "id": format!("n{index}"),
                "x": x,
                "y": y,
                "fix_temperature": matches!(index, 3 | 7),
                "temperature": 35.0,
                "heat_load": if matches!(index, 1 | 5) { 0.01 } else { 0.0 }
            }),
            _ => json!({
                "id": format!("n{index}"),
                "x": x,
                "y": y,
                "fix_x": matches!(index, 0 | 4),
                "fix_y": matches!(index, 0 | 4),
                "load_x": 0.0,
                "load_y": 0.0,
                "temperature_delta": if matches!(index, 1 | 5) { 95.0 } else { 45.0 }
            }),
        })
        .collect()
}

fn quad(id: &str, i: usize, j: usize, k: usize, l: usize, permittivity: f64) -> Value {
    json!({
        "id": id,
        "node_i": i,
        "node_j": j,
        "node_k": k,
        "node_l": l,
        "thickness": 0.001,
        "permittivity": permittivity
    })
}

fn heat_quad(id: &str, i: usize, j: usize, k: usize, l: usize, conductivity: f64) -> Value {
    json!({
        "id": id,
        "node_i": i,
        "node_j": j,
        "node_k": k,
        "node_l": l,
        "thickness": 0.001,
        "conductivity": conductivity
    })
}

fn thermal_quad(
    id: &str,
    i: usize,
    j: usize,
    k: usize,
    l: usize,
    youngs: f64,
    expansion: f64,
) -> Value {
    json!({
        "id": id,
        "node_i": i,
        "node_j": j,
        "node_k": k,
        "node_l": l,
        "thickness": 0.001,
        "youngs_modulus": youngs,
        "poisson_ratio": 0.32,
        "thermal_expansion": expansion
    })
}
