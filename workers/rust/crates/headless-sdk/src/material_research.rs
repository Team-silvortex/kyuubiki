use crate::HeadlessWorkflowStep;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

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

pub fn build_heat_spreader_screening_steps() -> Vec<HeadlessWorkflowStep> {
    heat_spreader_screening_candidates()
        .into_iter()
        .enumerate()
        .flat_map(|(candidate_index, candidate)| {
            let solve_step = candidate_index * 3 + 1;
            [
                HeadlessWorkflowStep::new(
                    "solve_heat_plane_quad_2d",
                    json!({
                        "research": build_heat_spreader_research_metadata(&candidate),
                        "model": heat_spreader_quad_model(&candidate),
                    }),
                ),
                HeadlessWorkflowStep::new(
                    "job_wait",
                    json!({
                        "job_id": format!("{{{{steps.{solve_step}.result.job_id}}}}"),
                        "interval_ms": 1000,
                        "timeout_ms": 60000,
                    }),
                ),
                HeadlessWorkflowStep::new(
                    "result_fetch",
                    json!({ "job_id": format!("{{{{steps.{solve_step}.result.job_id}}}}") }),
                ),
            ]
        })
        .collect()
}

fn build_heat_spreader_research_metadata(candidate: &MaterialResearchCandidate) -> Value {
    json!({
        "study": "material.heat_spreader_screening.v1",
        "candidate_id": candidate.id,
        "candidate_label": candidate.label,
        "family": candidate.family,
        "thermal_conductivity_w_mk": candidate.thermal_conductivity_w_mk,
        "density_kg_m3": candidate.density_kg_m3,
        "objective": "minimize peak temperature and mass pressure for a thin heat spreader patch",
        "note": candidate.note,
    })
}

fn heat_spreader_quad_model(candidate: &MaterialResearchCandidate) -> Value {
    json!({
        "nodes": [
            { "id": "hot_left_bottom", "x": 0.0, "y": 0.0, "fix_temperature": true, "temperature": 95.0, "heat_load": 0.0 },
            { "id": "mid_bottom", "x": 0.05, "y": 0.0, "fix_temperature": false, "temperature": 0.0, "heat_load": 5.0 },
            { "id": "cold_right_top", "x": 0.05, "y": 0.03, "fix_temperature": true, "temperature": 35.0, "heat_load": 0.0 },
            { "id": "cold_left_top", "x": 0.0, "y": 0.03, "fix_temperature": true, "temperature": 35.0, "heat_load": 0.0 }
        ],
        "elements": [{
            "id": format!("spread_{}", candidate.id),
            "node_i": 0,
            "node_j": 1,
            "node_k": 2,
            "node_l": 3,
            "thickness": 0.0015,
            "conductivity": candidate.thermal_conductivity_w_mk
        }]
    })
}
