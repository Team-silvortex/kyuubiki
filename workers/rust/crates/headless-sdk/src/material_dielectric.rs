use crate::{
    HeadlessWorkflowStep, MaterialOptimizationProfile, MaterialOptimizationTerm,
    MaterialResearchMetricSpec, material_optimization_constraint, material_optimization_profile,
    material_optimization_term, material_optimization_weight, profile_weight,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DielectricMaterialCandidate {
    pub id: &'static str,
    pub label: &'static str,
    pub family: &'static str,
    pub relative_permittivity: f64,
    pub breakdown_field_v_m: f64,
    pub dissipation_factor: f64,
    pub density_kg_m3: f64,
    pub note: &'static str,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DielectricMaterialCandidateReport {
    pub candidate_id: String,
    pub candidate_label: String,
    pub rank: usize,
    pub score: f64,
    pub max_electric_field_v_m: Option<f64>,
    pub max_flux_density_c_m2: Option<f64>,
    pub breakdown_safety_factor: Option<f64>,
    pub areal_mass_kg_m2: f64,
    pub dielectric_loss_proxy: f64,
    pub optimization_terms: Vec<MaterialOptimizationTerm>,
    pub missing_metrics: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DielectricMaterialReport {
    pub schema_version: String,
    pub study: String,
    pub objective: String,
    pub optimization: MaterialOptimizationProfile,
    pub metric_specs: Vec<MaterialResearchMetricSpec>,
    pub candidates: Vec<DielectricMaterialCandidateReport>,
    pub winner_candidate_id: Option<String>,
    pub warnings: Vec<String>,
}

pub fn dielectric_screening_candidates() -> Vec<DielectricMaterialCandidate> {
    vec![
        DielectricMaterialCandidate {
            id: "polyimide_film",
            label: "Polyimide film",
            family: "polymer",
            relative_permittivity: 3.4,
            breakdown_field_v_m: 300.0e6,
            dissipation_factor: 0.002,
            density_kg_m3: 1420.0,
            note: "flexible high-breakdown film baseline",
        },
        DielectricMaterialCandidate {
            id: "alumina_96",
            label: "Alumina 96%",
            family: "ceramic",
            relative_permittivity: 9.8,
            breakdown_field_v_m: 130.0e6,
            dissipation_factor: 0.0002,
            density_kg_m3: 3720.0,
            note: "ceramic substrate candidate with low loss but high mass",
        },
        DielectricMaterialCandidate {
            id: "ptfe",
            label: "PTFE",
            family: "polymer",
            relative_permittivity: 2.1,
            breakdown_field_v_m: 60.0e6,
            dissipation_factor: 0.0002,
            density_kg_m3: 2200.0,
            note: "low-loss polymer candidate with weaker breakdown margin",
        },
    ]
}

pub fn dielectric_screening_metric_specs() -> Vec<MaterialResearchMetricSpec> {
    vec![
        metric(
            "max_electric_field_v_m",
            "Max electric field",
            "V/m",
            "minimize",
            0.25,
            "solver.result.max_electric_field",
        ),
        metric(
            "breakdown_safety_factor",
            "Breakdown safety factor",
            "ratio",
            "maximize",
            0.4,
            "candidate.breakdown_field_v_m / solver.result.max_electric_field",
        ),
        metric(
            "dielectric_loss_proxy",
            "Dielectric loss proxy",
            "relative",
            "minimize",
            0.2,
            "candidate.relative_permittivity * candidate.dissipation_factor",
        ),
        metric(
            "areal_mass_kg_m2",
            "Areal mass",
            "kg/m^2",
            "minimize",
            0.15,
            "candidate.density_kg_m3 * model.thickness",
        ),
        metric(
            "max_flux_density_c_m2",
            "Max electric flux density",
            "C/m^2",
            "observe",
            0.0,
            "solver.result.max_flux_density",
        ),
    ]
}

pub fn build_dielectric_screening_steps() -> Vec<HeadlessWorkflowStep> {
    dielectric_screening_candidates()
        .into_iter()
        .enumerate()
        .flat_map(|(candidate_index, candidate)| {
            let solve_step = candidate_index * 3 + 1;
            [
                HeadlessWorkflowStep::new(
                    "solve_electrostatic_plane_quad_2d",
                    json!({
                        "research": dielectric_research_metadata(&candidate),
                        "model": dielectric_quad_model(&candidate),
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

pub fn build_dielectric_screening_report(
    result_payloads: &[Value],
) -> Result<DielectricMaterialReport, String> {
    build_dielectric_screening_report_with_optimization(
        result_payloads,
        dielectric_optimization_profile(),
    )
}

pub fn build_dielectric_screening_report_with_optimization(
    result_payloads: &[Value],
    optimization: MaterialOptimizationProfile,
) -> Result<DielectricMaterialReport, String> {
    let candidates = dielectric_screening_candidates();
    if result_payloads.len() != candidates.len() {
        return Err(format!(
            "dielectric screening expects {} result payloads, received {}",
            candidates.len(),
            result_payloads.len()
        ));
    }

    let mut rows = candidates
        .iter()
        .zip(result_payloads.iter())
        .map(|(candidate, payload)| dielectric_candidate_report(candidate, payload))
        .collect::<Vec<_>>();
    apply_scores(&mut rows, &optimization);
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

    Ok(DielectricMaterialReport {
        schema_version: "kyuubiki.dielectric-material-report/v1".to_string(),
        study: "material.dielectric_screening.v1".to_string(),
        objective: "rank dielectric candidates by electric-field margin, low loss, and low mass"
            .to_string(),
        optimization,
        metric_specs: dielectric_screening_metric_specs(),
        winner_candidate_id: rows.first().map(|row| row.candidate_id.clone()),
        candidates: rows,
        warnings,
    })
}

fn dielectric_candidate_report(
    candidate: &DielectricMaterialCandidate,
    payload: &Value,
) -> DielectricMaterialCandidateReport {
    let result = descend_result_payload(payload);
    let max_electric_field_v_m = read_f64(result, "max_electric_field");
    let max_flux_density_c_m2 = read_f64(result, "max_flux_density");
    let breakdown_safety_factor = max_electric_field_v_m
        .filter(|field| *field > 0.0)
        .map(|field| candidate.breakdown_field_v_m / field);
    let mut missing_metrics = Vec::new();
    if max_electric_field_v_m.is_none() {
        missing_metrics.push("max_electric_field_v_m".to_string());
        missing_metrics.push("breakdown_safety_factor".to_string());
    }
    if max_flux_density_c_m2.is_none() {
        missing_metrics.push("max_flux_density_c_m2".to_string());
    }

    DielectricMaterialCandidateReport {
        candidate_id: candidate.id.to_string(),
        candidate_label: candidate.label.to_string(),
        rank: 0,
        score: 0.0,
        max_electric_field_v_m,
        max_flux_density_c_m2,
        breakdown_safety_factor,
        areal_mass_kg_m2: candidate.density_kg_m3 * dielectric_thickness_m(),
        dielectric_loss_proxy: candidate.relative_permittivity * candidate.dissipation_factor,
        optimization_terms: Vec::new(),
        missing_metrics,
    }
}

fn apply_scores(
    rows: &mut [DielectricMaterialCandidateReport],
    optimization: &MaterialOptimizationProfile,
) {
    let field_values = rows
        .iter()
        .filter_map(|row| row.max_electric_field_v_m)
        .collect::<Vec<_>>();
    let safety_values = rows
        .iter()
        .filter_map(|row| row.breakdown_safety_factor)
        .collect::<Vec<_>>();
    let loss_values = rows
        .iter()
        .map(|row| row.dielectric_loss_proxy)
        .collect::<Vec<_>>();
    let mass_values = rows
        .iter()
        .map(|row| row.areal_mass_kg_m2)
        .collect::<Vec<_>>();

    for row in rows {
        let field_weight = profile_weight(optimization, "max_electric_field_v_m", 0.25);
        let safety_weight = profile_weight(optimization, "breakdown_safety_factor", 0.4);
        let loss_weight = profile_weight(optimization, "dielectric_loss_proxy", 0.2);
        let mass_weight = profile_weight(optimization, "areal_mass_kg_m2", 0.15);
        let field_score = row
            .max_electric_field_v_m
            .map(|value| normalize_minimize(value, &field_values))
            .unwrap_or(0.0);
        let safety_score = row
            .breakdown_safety_factor
            .map(|value| normalize_maximize(value, &safety_values))
            .unwrap_or(0.0);
        let loss_score = normalize_minimize(row.dielectric_loss_proxy, &loss_values);
        let mass_score = normalize_minimize(row.areal_mass_kg_m2, &mass_values);
        row.optimization_terms = vec![
            material_optimization_term(
                "max_electric_field_v_m",
                "minimize",
                row.max_electric_field_v_m,
                field_score,
                field_weight,
                &less_equal_status(row.max_electric_field_v_m, 55.0e6),
            ),
            material_optimization_term(
                "breakdown_safety_factor",
                "maximize",
                row.breakdown_safety_factor,
                safety_score,
                safety_weight,
                &greater_equal_status(row.breakdown_safety_factor, 2.0),
            ),
            material_optimization_term(
                "dielectric_loss_proxy",
                "minimize",
                Some(row.dielectric_loss_proxy),
                loss_score,
                loss_weight,
                &less_equal_status(Some(row.dielectric_loss_proxy), 0.01),
            ),
            material_optimization_term(
                "areal_mass_kg_m2",
                "minimize",
                Some(row.areal_mass_kg_m2),
                mass_score,
                mass_weight,
                &less_equal_status(Some(row.areal_mass_kg_m2), 8.0),
            ),
        ];
        row.score = round_score(
            field_score * field_weight
                + safety_score * safety_weight
                + loss_score * loss_weight
                + mass_score * mass_weight,
        );
    }
}

fn dielectric_optimization_profile() -> MaterialOptimizationProfile {
    material_optimization_profile(
        "material.dielectric_screening.optimization.v1",
        "Maximize dielectric breakdown margin while minimizing electric field, loss proxy, and mass.",
        "0.25*max_electric_field_v_m:min + 0.40*breakdown_safety_factor:max + 0.20*dielectric_loss_proxy:min + 0.15*areal_mass_kg_m2:min",
        vec![
            material_optimization_weight("max_electric_field_v_m", "minimize", 0.25),
            material_optimization_weight("breakdown_safety_factor", "maximize", 0.4),
            material_optimization_weight("dielectric_loss_proxy", "minimize", 0.2),
            material_optimization_weight("areal_mass_kg_m2", "minimize", 0.15),
        ],
        vec![
            material_optimization_constraint("max_electric_field_v_m", "<=", 55.0e6, "warning"),
            material_optimization_constraint("breakdown_safety_factor", ">=", 2.0, "warning"),
            material_optimization_constraint("dielectric_loss_proxy", "<=", 0.01, "warning"),
        ],
    )
}

fn dielectric_research_metadata(candidate: &DielectricMaterialCandidate) -> Value {
    json!({
        "study": "material.dielectric_screening.v1",
        "candidate_id": candidate.id,
        "candidate_label": candidate.label,
        "family": candidate.family,
        "relative_permittivity": candidate.relative_permittivity,
        "breakdown_field_v_m": candidate.breakdown_field_v_m,
        "dissipation_factor": candidate.dissipation_factor,
        "objective": "maximize electric breakdown margin with low loss and low mass",
        "note": candidate.note,
    })
}

fn dielectric_quad_model(candidate: &DielectricMaterialCandidate) -> Value {
    json!({
        "nodes": [
            { "id": "ground_left_bottom", "x": 0.0, "y": 0.0, "fix_potential": true, "potential": 0.0, "charge_density": 0.0 },
            { "id": "drive_right_bottom", "x": 0.04, "y": 0.0, "fix_potential": true, "potential": 1200.0, "charge_density": 0.0 },
            { "id": "drive_right_top", "x": 0.04, "y": 0.02, "fix_potential": true, "potential": 1200.0, "charge_density": 0.0 },
            { "id": "ground_left_top", "x": 0.0, "y": 0.02, "fix_potential": true, "potential": 0.0, "charge_density": 0.0 }
        ],
        "elements": [{
            "id": format!("dielectric_{}", candidate.id),
            "node_i": 0,
            "node_j": 1,
            "node_k": 2,
            "node_l": 3,
            "thickness": dielectric_thickness_m(),
            "permittivity": 8.8541878128e-12 * candidate.relative_permittivity
        }]
    })
}

fn metric(
    id: &str,
    label: &str,
    unit: &str,
    objective: &str,
    weight: f64,
    source: &str,
) -> MaterialResearchMetricSpec {
    MaterialResearchMetricSpec {
        id: id.to_string(),
        label: label.to_string(),
        unit: unit.to_string(),
        objective: objective.to_string(),
        weight,
        source: source.to_string(),
    }
}

fn less_equal_status(value: Option<f64>, limit: f64) -> String {
    match value {
        Some(actual) if actual <= limit => "pass".to_string(),
        Some(_) => "violate".to_string(),
        None => "unknown".to_string(),
    }
}

fn greater_equal_status(value: Option<f64>, limit: f64) -> String {
    match value {
        Some(actual) if actual >= limit => "pass".to_string(),
        Some(_) => "violate".to_string(),
        None => "unknown".to_string(),
    }
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

fn dielectric_thickness_m() -> f64 {
    0.001
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dielectric_report_ranks_breakdown_margin_and_loss() {
        let report = build_dielectric_screening_report(&[
            json!({ "result": { "max_electric_field": 42.0e6, "max_flux_density": 1.2e-3 } }),
            json!({ "result": { "result": { "max_electric_field": 38.0e6, "max_flux_density": 3.3e-3 } } }),
            json!({ "max_electric_field": 48.0e6, "max_flux_density": 0.9e-3 }),
        ])
        .expect("report should build");

        assert_eq!(
            report.schema_version,
            "kyuubiki.dielectric-material-report/v1"
        );
        assert_eq!(report.candidates.len(), 3);
        assert_eq!(
            report.winner_candidate_id.as_deref(),
            Some("polyimide_film")
        );
        assert_eq!(
            report.optimization.id,
            "material.dielectric_screening.optimization.v1"
        );
        assert_eq!(report.candidates[0].rank, 1);
        assert_eq!(report.candidates[0].optimization_terms.len(), 4);
        assert!(
            report.candidates[0]
                .optimization_terms
                .iter()
                .any(|term| term.metric_id == "breakdown_safety_factor")
        );
    }

    #[test]
    fn dielectric_report_keeps_missing_metric_warnings_visible() {
        let report = build_dielectric_screening_report(&[
            json!({ "result": { "kind": "simulated_result" } }),
            json!({ "result": { "max_electric_field": 38.0e6 } }),
            json!({ "max_electric_field": 48.0e6, "max_flux_density": 0.9e-3 }),
        ])
        .expect("report should tolerate incomplete early results");

        assert!(report.warnings.len() >= 3);
        assert!(
            report
                .warnings
                .iter()
                .any(|warning| warning.contains("polyimide_film is missing max_electric_field_v_m"))
        );
    }
}
