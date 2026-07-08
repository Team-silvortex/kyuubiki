use crate::material_research_candidates::{
    MaterialResearchCandidate, heat_spreader_screening_candidates,
};
use crate::{
    HeadlessWorkflowStep, MaterialOptimizationProfile, MaterialOptimizationTerm,
    MaterialReliabilityEnvelope, less_equal_status, material_evidence_ref,
    material_model_assumption, material_optimization_constraint, material_optimization_profile,
    material_optimization_term, material_optimization_weight, material_quality_gate,
    profile_weight,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MaterialResearchMetricSpec {
    pub id: String,
    pub label: String,
    pub unit: String,
    pub objective: String,
    pub weight: f64,
    pub source: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MaterialResearchCandidateReport {
    pub candidate_id: String,
    pub candidate_label: String,
    pub material_card_id: String,
    pub material_card_confidence: String,
    pub rank: usize,
    pub score: f64,
    pub peak_temperature_c: Option<f64>,
    pub peak_heat_flux_w_m2: Option<f64>,
    pub areal_mass_kg_m2: f64,
    pub conductivity_density_ratio: f64,
    pub optimization_terms: Vec<MaterialOptimizationTerm>,
    pub missing_metrics: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MaterialResearchReport {
    pub schema_version: String,
    pub study: String,
    pub objective: String,
    pub optimization: MaterialOptimizationProfile,
    pub reliability: MaterialReliabilityEnvelope,
    pub metric_specs: Vec<MaterialResearchMetricSpec>,
    pub candidates: Vec<MaterialResearchCandidateReport>,
    pub winner_candidate_id: Option<String>,
    pub warnings: Vec<String>,
}

pub fn heat_spreader_screening_metric_specs() -> Vec<MaterialResearchMetricSpec> {
    vec![
        MaterialResearchMetricSpec {
            id: "peak_temperature_c".to_string(),
            label: "Peak temperature".to_string(),
            unit: "degC".to_string(),
            objective: "minimize".to_string(),
            weight: 0.55,
            source: "solver.result.max_temperature".to_string(),
        },
        MaterialResearchMetricSpec {
            id: "peak_heat_flux_w_m2".to_string(),
            label: "Peak heat flux".to_string(),
            unit: "W/m^2".to_string(),
            objective: "observe".to_string(),
            weight: 0.0,
            source: "solver.result.max_heat_flux".to_string(),
        },
        MaterialResearchMetricSpec {
            id: "areal_mass_kg_m2".to_string(),
            label: "Areal mass".to_string(),
            unit: "kg/m^2".to_string(),
            objective: "minimize".to_string(),
            weight: 0.3,
            source: "candidate.density_kg_m3 * model.thickness".to_string(),
        },
        MaterialResearchMetricSpec {
            id: "conductivity_density_ratio".to_string(),
            label: "Conductivity-to-density ratio".to_string(),
            unit: "W*m^2/(m*K*kg)".to_string(),
            objective: "maximize".to_string(),
            weight: 0.15,
            source: "candidate.thermal_conductivity_w_mk / candidate.density_kg_m3".to_string(),
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

pub fn build_heat_spreader_screening_report(
    result_payloads: &[Value],
) -> Result<MaterialResearchReport, String> {
    build_heat_spreader_screening_report_with_optimization(
        result_payloads,
        heat_spreader_optimization_profile(),
    )
}

pub fn build_heat_spreader_screening_report_with_optimization(
    result_payloads: &[Value],
    optimization: MaterialOptimizationProfile,
) -> Result<MaterialResearchReport, String> {
    let candidates = heat_spreader_screening_candidates();
    if result_payloads.len() != candidates.len() {
        return Err(format!(
            "heat spreader screening expects {} result payloads, received {}",
            candidates.len(),
            result_payloads.len()
        ));
    }

    let mut rows = candidates
        .iter()
        .zip(result_payloads.iter())
        .map(|(candidate, payload)| build_candidate_report(candidate, payload))
        .collect::<Vec<_>>();

    apply_screening_scores(&mut rows, &optimization);
    rows.sort_by(|left, right| {
        right
            .score
            .partial_cmp(&left.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    for (index, row) in rows.iter_mut().enumerate() {
        row.rank = index + 1;
    }

    let warnings = rows
        .iter()
        .flat_map(|row| {
            row.missing_metrics.iter().map(|metric| {
                format!(
                    "{} is missing {}; ranking used remaining weighted metrics",
                    row.candidate_id, metric
                )
            })
        })
        .collect::<Vec<_>>();

    Ok(MaterialResearchReport {
        schema_version: "kyuubiki.material-research-report/v1".to_string(),
        study: "material.heat_spreader_screening.v1".to_string(),
        objective:
            "rank thin heat-spreader candidates by lower peak temperature and lower areal mass"
                .to_string(),
        optimization,
        reliability: build_heat_spreader_reliability_envelope(&rows),
        metric_specs: heat_spreader_screening_metric_specs(),
        winner_candidate_id: rows.first().map(|row| row.candidate_id.clone()),
        candidates: rows,
        warnings,
    })
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
        "material_card_id": material_card_id(candidate),
        "unit_system": "SI",
        "parameter_scope": "room-temperature scalar conductivity and density screening values",
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

fn build_candidate_report(
    candidate: &MaterialResearchCandidate,
    payload: &Value,
) -> MaterialResearchCandidateReport {
    let result = descend_result_payload(payload);
    let peak_temperature_c = read_f64(result, "max_temperature");
    let peak_heat_flux_w_m2 = read_f64(result, "max_heat_flux");
    let areal_mass_kg_m2 = candidate.density_kg_m3 * heat_spreader_thickness_m();
    let conductivity_density_ratio = candidate.thermal_conductivity_w_mk / candidate.density_kg_m3;
    let mut missing_metrics = Vec::new();
    if peak_temperature_c.is_none() {
        missing_metrics.push("peak_temperature_c".to_string());
    }
    if peak_heat_flux_w_m2.is_none() {
        missing_metrics.push("peak_heat_flux_w_m2".to_string());
    }

    MaterialResearchCandidateReport {
        candidate_id: candidate.id.to_string(),
        candidate_label: candidate.label.to_string(),
        material_card_id: material_card_id(candidate),
        material_card_confidence: material_card_confidence(candidate).to_string(),
        rank: 0,
        score: 0.0,
        peak_temperature_c,
        peak_heat_flux_w_m2,
        areal_mass_kg_m2,
        conductivity_density_ratio,
        optimization_terms: Vec::new(),
        missing_metrics,
    }
}

fn apply_screening_scores(
    rows: &mut [MaterialResearchCandidateReport],
    optimization: &MaterialOptimizationProfile,
) {
    let temperature_values = rows
        .iter()
        .filter_map(|row| row.peak_temperature_c)
        .collect::<Vec<_>>();
    let mass_values = rows
        .iter()
        .map(|row| row.areal_mass_kg_m2)
        .collect::<Vec<_>>();
    let ratio_values = rows
        .iter()
        .map(|row| row.conductivity_density_ratio)
        .collect::<Vec<_>>();

    for row in rows {
        let temperature_weight = profile_weight(optimization, "peak_temperature_c", 0.55);
        let mass_weight = profile_weight(optimization, "areal_mass_kg_m2", 0.3);
        let ratio_weight = profile_weight(optimization, "conductivity_density_ratio", 0.15);
        let temperature_score = row
            .peak_temperature_c
            .map(|value| normalize_minimize(value, &temperature_values))
            .unwrap_or(0.0);
        let mass_score = normalize_minimize(row.areal_mass_kg_m2, &mass_values);
        let ratio_score = normalize_maximize(row.conductivity_density_ratio, &ratio_values);
        row.optimization_terms = vec![
            material_optimization_term(
                "peak_temperature_c",
                "minimize",
                row.peak_temperature_c,
                temperature_score,
                temperature_weight,
                &constraint_status(row.peak_temperature_c, 70.0),
            ),
            material_optimization_term(
                "areal_mass_kg_m2",
                "minimize",
                Some(row.areal_mass_kg_m2),
                mass_score,
                mass_weight,
                &constraint_status(Some(row.areal_mass_kg_m2), 8.0),
            ),
            material_optimization_term(
                "conductivity_density_ratio",
                "maximize",
                Some(row.conductivity_density_ratio),
                ratio_score,
                ratio_weight,
                "observe",
            ),
        ];
        row.score = round_score(
            temperature_score * temperature_weight
                + mass_score * mass_weight
                + ratio_score * ratio_weight,
        );
    }
}

fn heat_spreader_optimization_profile() -> MaterialOptimizationProfile {
    material_optimization_profile(
        "material.heat_spreader_screening.optimization.v1",
        "Minimize peak temperature and areal mass while preserving high conductivity-density efficiency.",
        "0.55*peak_temperature_c:min + 0.30*areal_mass_kg_m2:min + 0.15*conductivity_density_ratio:max",
        vec![
            material_optimization_weight("peak_temperature_c", "minimize", 0.55),
            material_optimization_weight("areal_mass_kg_m2", "minimize", 0.3),
            material_optimization_weight("conductivity_density_ratio", "maximize", 0.15),
        ],
        vec![
            material_optimization_constraint("peak_temperature_c", "<=", 70.0, "warning"),
            material_optimization_constraint("areal_mass_kg_m2", "<=", 8.0, "warning"),
        ],
    )
}

fn build_heat_spreader_reliability_envelope(
    rows: &[MaterialResearchCandidateReport],
) -> MaterialReliabilityEnvelope {
    let max_temperature = rows
        .iter()
        .filter_map(|row| row.peak_temperature_c)
        .fold(None, |current: Option<f64>, value| {
            Some(current.map_or(value, |max| max.max(value)))
        });
    let max_areal_mass = rows
        .iter()
        .map(|row| row.areal_mass_kg_m2)
        .fold(None, |current: Option<f64>, value| {
            Some(current.map_or(value, |max| max.max(value)))
        });

    MaterialReliabilityEnvelope {
        schema_version: "kyuubiki.material-reliability-envelope/v1".to_string(),
        posture: "screening_only".to_string(),
        material_card_version: "kyuubiki.material-cards.heat-spreader.v1".to_string(),
        unit_system: "SI".to_string(),
        evidence_refs: heat_spreader_evidence_refs(),
        model_assumptions: heat_spreader_model_assumptions(),
        quality_gates: vec![
            material_quality_gate(
                "gate.peak_temperature.warning",
                "Peak temperature warning gate",
                "peak_temperature_c",
                "<=",
                70.0,
                max_temperature,
                "At least one candidate should keep the screening peak at or below the warning limit.",
            ),
            material_quality_gate(
                "gate.areal_mass.warning",
                "Areal mass warning gate",
                "areal_mass_kg_m2",
                "<=",
                8.0,
                max_areal_mass,
                "Screening candidates should remain light enough for thin spreader use.",
            ),
            material_quality_gate(
                "gate.result_completeness",
                "Result payload completeness",
                "peak_temperature_c",
                ">=",
                rows.len() as f64,
                Some(
                    rows.iter()
                        .filter(|row| row.peak_temperature_c.is_some())
                        .count() as f64,
                ),
                "Every candidate should expose a peak-temperature result before ranking is trusted.",
            ),
        ],
        limitations: vec![
            "Current material cards use scalar room-temperature screening values, not temperature-dependent material curves.".to_string(),
            "Pyrolytic graphite is represented by its in-plane conductivity only; through-plane anisotropy is not resolved in this first-pass model.".to_string(),
            "The quad model is a deterministic ranking fixture, not a CAD-derived production mesh or validated package geometry.".to_string(),
            "Use this report to shortlist candidates, then rerun with richer geometry, boundary conditions, and external benchmark comparison.".to_string(),
        ],
    }
}

fn heat_spreader_evidence_refs() -> Vec<crate::MaterialEvidenceRef> {
    vec![
        material_evidence_ref(
            "mat.aluminum_6061.screening",
            "Aluminum 6061 screening card",
            "material_card",
            "Kyuubiki built-in screening value set",
            "medium",
            "Representative conductivity and density values for early ranking.",
        ),
        material_evidence_ref(
            "mat.copper_c110.screening",
            "Copper C110 screening card",
            "material_card",
            "Kyuubiki built-in screening value set",
            "medium",
            "Representative high-conductivity metal baseline.",
        ),
        material_evidence_ref(
            "mat.pyrolytic_graphite.in_plane.screening",
            "Pyrolytic graphite in-plane screening card",
            "material_card",
            "Kyuubiki built-in screening value set",
            "low",
            "Anisotropic material simplified to an in-plane scalar value.",
        ),
    ]
}

fn heat_spreader_model_assumptions() -> Vec<crate::MaterialModelAssumption> {
    vec![
        material_model_assumption(
            "model.geometry",
            "Thin rectangular heat spreader",
            "50 mm x 30 mm x 1.5 mm quad fixture",
            "Keeps candidates comparable but does not represent a finished product geometry.",
        ),
        material_model_assumption(
            "model.boundary",
            "Fixed hot/cold boundary temperatures",
            "95 C hot-side anchor and 35 C cold-side anchors",
            "Ranks spreading behavior under a stable thermal contrast.",
        ),
        material_model_assumption(
            "model.material",
            "Linear scalar conductivity",
            "conductivity is constant over the solve",
            "Fast for screening, insufficient for final material qualification.",
        ),
    ]
}

fn material_card_id(candidate: &MaterialResearchCandidate) -> String {
    format!("kyuubiki.material_card.{}.v1", candidate.id)
}

fn material_card_confidence(candidate: &MaterialResearchCandidate) -> &'static str {
    match candidate.id {
        "pyrolytic_graphite_in_plane" => "low",
        _ => "medium",
    }
}

fn constraint_status(value: Option<f64>, limit: f64) -> String {
    less_equal_status(value, limit)
}

fn descend_result_payload(payload: &Value) -> &Value {
    let mut current = payload;
    for _ in 0..4 {
        let Some(next) = current.get("result") else {
            break;
        };
        current = next;
    }
    current
}

fn read_f64(payload: &Value, key: &str) -> Option<f64> {
    payload.get(key).and_then(Value::as_f64)
}

fn normalize_minimize(value: f64, values: &[f64]) -> f64 {
    let (min, max) = value_range(values);
    if (max - min).abs() < f64::EPSILON {
        return 1.0;
    }
    (max - value) / (max - min)
}

fn normalize_maximize(value: f64, values: &[f64]) -> f64 {
    let (min, max) = value_range(values);
    if (max - min).abs() < f64::EPSILON {
        return 1.0;
    }
    (value - min) / (max - min)
}

fn value_range(values: &[f64]) -> (f64, f64) {
    values
        .iter()
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(min, max), value| {
            (min.min(*value), max.max(*value))
        })
}

fn round_score(value: f64) -> f64 {
    (value * 1000.0).round() / 1000.0
}

fn heat_spreader_thickness_m() -> f64 {
    0.0015
}
