use crate::{
    HeadlessWorkflowStep, MaterialOptimizationProfile, MaterialOptimizationTerm, less_equal_status,
    material_optimization_constraint, material_optimization_profile, material_optimization_term,
    material_optimization_weight, profile_weight,
};
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
    pub metric_specs: Vec<MaterialResearchMetricSpec>,
    pub candidates: Vec<MaterialResearchCandidateReport>,
    pub winner_candidate_id: Option<String>,
    pub warnings: Vec<String>,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn heat_spreader_report_ranks_by_result_and_material_metrics() {
        let report = build_heat_spreader_screening_report(&[
            json!({ "result": { "max_temperature": 82.0, "max_heat_flux": 900.0 } }),
            json!({ "result": { "result": { "max_temperature": 64.0, "max_heat_flux": 1400.0 } } }),
            json!({ "max_temperature": 58.0, "max_heat_flux": 1800.0 }),
        ])
        .expect("report should build");

        assert_eq!(
            report.schema_version,
            "kyuubiki.material-research-report/v1"
        );
        assert_eq!(report.candidates.len(), 3);
        assert_eq!(
            report.winner_candidate_id.as_deref(),
            Some("pyrolytic_graphite_in_plane")
        );
        assert_eq!(
            report.optimization.id,
            "material.heat_spreader_screening.optimization.v1"
        );
        assert!(
            report
                .optimization
                .score_formula
                .contains("peak_temperature_c:min")
        );
        assert_eq!(report.candidates[0].rank, 1);
        assert!(report.candidates[0].score > report.candidates[1].score);
        assert_eq!(report.candidates[0].optimization_terms.len(), 3);
        assert!(
            report.candidates[0]
                .optimization_terms
                .iter()
                .any(|term| term.metric_id == "areal_mass_kg_m2" && term.weighted_score > 0.0)
        );
        assert_eq!(report.candidates[2].candidate_id, "aluminum_6061");
    }

    #[test]
    fn heat_spreader_report_keeps_missing_metric_warnings_visible() {
        let report = build_heat_spreader_screening_report(&[
            json!({ "result": { "kind": "simulated_result" } }),
            json!({ "result": { "max_temperature": 64.0 } }),
            json!({ "result": { "max_temperature": 58.0, "max_heat_flux": 1800.0 } }),
        ])
        .expect("report should tolerate incomplete early results");

        assert!(report.warning_count() >= 3);
        assert!(
            report
                .warnings
                .iter()
                .any(|warning| warning.contains("aluminum_6061 is missing peak_temperature_c"))
        );
    }

    #[test]
    fn heat_spreader_report_rejects_candidate_result_count_mismatch() {
        let error = build_heat_spreader_screening_report(&[])
            .expect_err("mismatched result count should fail");
        assert!(error.contains("expects 3 result payloads"));
    }

    impl MaterialResearchReport {
        fn warning_count(&self) -> usize {
            self.warnings.len()
        }
    }
}
