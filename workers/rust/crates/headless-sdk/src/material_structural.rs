use crate::material_card_refs::built_in_material_card_ref;
use crate::{
    HeadlessWorkflowStep, MaterialCardReference, MaterialOptimizationProfile,
    MaterialOptimizationTerm, MaterialResearchMetricSpec, less_equal_status,
    material_optimization_constraint, material_optimization_profile, material_optimization_term,
    material_optimization_weight, profile_weight,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StructuralMaterialCandidate {
    pub id: &'static str,
    pub label: &'static str,
    pub family: &'static str,
    pub density_kg_m3: f64,
    pub youngs_modulus_pa: f64,
    pub poisson_ratio: f64,
    pub yield_strength_pa: f64,
    pub note: &'static str,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StructuralMaterialCandidateReport {
    pub candidate_id: String,
    pub candidate_label: String,
    pub rank: usize,
    pub score: f64,
    pub max_stress_pa: Option<f64>,
    pub max_displacement_m: Option<f64>,
    pub areal_mass_kg_m2: f64,
    pub specific_stiffness_m2_s2: f64,
    pub yield_safety_factor: Option<f64>,
    pub optimization_terms: Vec<MaterialOptimizationTerm>,
    pub missing_metrics: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StructuralMaterialReport {
    pub schema_version: String,
    pub study: String,
    pub objective: String,
    pub optimization: MaterialOptimizationProfile,
    pub metric_specs: Vec<MaterialResearchMetricSpec>,
    pub material_card_refs: Vec<MaterialCardReference>,
    pub candidates: Vec<StructuralMaterialCandidateReport>,
    pub winner_candidate_id: Option<String>,
    pub warnings: Vec<String>,
}

pub fn structural_panel_screening_candidates() -> Vec<StructuralMaterialCandidate> {
    vec![
        StructuralMaterialCandidate {
            id: "aluminum_7075_t6",
            label: "Aluminum 7075-T6",
            family: "metal",
            density_kg_m3: 2810.0,
            youngs_modulus_pa: 71.7e9,
            poisson_ratio: 0.33,
            yield_strength_pa: 503.0e6,
            note: "light aerospace baseline with strong yield margin",
        },
        StructuralMaterialCandidate {
            id: "steel_4130_normalized",
            label: "Steel 4130 normalized",
            family: "steel",
            density_kg_m3: 7850.0,
            youngs_modulus_pa: 205.0e9,
            poisson_ratio: 0.29,
            yield_strength_pa: 435.0e6,
            note: "stiff durable baseline with mass penalty",
        },
        StructuralMaterialCandidate {
            id: "carbon_fiber_quasi_iso",
            label: "Carbon fiber quasi-isotropic",
            family: "composite",
            density_kg_m3: 1600.0,
            youngs_modulus_pa: 70.0e9,
            poisson_ratio: 0.28,
            yield_strength_pa: 600.0e6,
            note: "low-mass composite candidate for stiffness-per-mass studies",
        },
    ]
}

pub fn structural_panel_metric_specs() -> Vec<MaterialResearchMetricSpec> {
    vec![
        metric(
            "max_stress_pa",
            "Max equivalent stress",
            "Pa",
            "minimize",
            0.3,
            "solver.result.max_stress",
        ),
        metric(
            "max_displacement_m",
            "Max displacement",
            "m",
            "minimize",
            0.25,
            "solver.result.max_displacement",
        ),
        metric(
            "areal_mass_kg_m2",
            "Areal mass",
            "kg/m^2",
            "minimize",
            0.2,
            "candidate.density_kg_m3 * model.thickness",
        ),
        metric(
            "specific_stiffness_m2_s2",
            "Specific stiffness",
            "m^2/s^2",
            "maximize",
            0.15,
            "candidate.youngs_modulus_pa / candidate.density_kg_m3",
        ),
        metric(
            "yield_safety_factor",
            "Yield safety factor",
            "ratio",
            "maximize",
            0.1,
            "candidate.yield_strength_pa / solver.result.max_stress",
        ),
    ]
}

pub fn build_structural_panel_screening_steps() -> Vec<HeadlessWorkflowStep> {
    structural_panel_screening_candidates()
        .into_iter()
        .enumerate()
        .flat_map(|(candidate_index, candidate)| {
            let solve_step = candidate_index * 3 + 1;
            [
                HeadlessWorkflowStep::new(
                    "solve_plane_quad_2d",
                    json!({
                        "research": structural_research_metadata(&candidate),
                        "model": structural_quad_model(&candidate),
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

pub fn build_structural_panel_screening_report(
    result_payloads: &[Value],
) -> Result<StructuralMaterialReport, String> {
    build_structural_panel_screening_report_with_optimization(
        result_payloads,
        structural_panel_optimization_profile(),
    )
}

pub fn build_structural_panel_screening_report_with_optimization(
    result_payloads: &[Value],
    optimization: MaterialOptimizationProfile,
) -> Result<StructuralMaterialReport, String> {
    let candidates = structural_panel_screening_candidates();
    if result_payloads.len() != candidates.len() {
        return Err(format!(
            "structural panel screening expects {} result payloads, received {}",
            candidates.len(),
            result_payloads.len()
        ));
    }

    let mut rows = candidates
        .iter()
        .zip(result_payloads.iter())
        .map(|(candidate, payload)| structural_candidate_report(candidate, payload))
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

    Ok(StructuralMaterialReport {
        schema_version: "kyuubiki.structural-material-report/v1".to_string(),
        study: "material.structural_panel_screening.v1".to_string(),
        objective:
            "rank lightweight structural panel candidates by stress, deflection, mass, stiffness efficiency, and yield margin"
                .to_string(),
        optimization,
        metric_specs: structural_panel_metric_specs(),
        material_card_refs: structural_material_card_refs(),
        winner_candidate_id: rows.first().map(|row| row.candidate_id.clone()),
        candidates: rows,
        warnings,
    })
}

fn structural_material_card_refs() -> Vec<MaterialCardReference> {
    structural_panel_screening_candidates()
        .iter()
        .map(|candidate| {
            built_in_material_card_ref(
                candidate.id,
                structural_material_card_confidence(candidate),
                "room-temperature scalar structural screening values",
                "kyuubiki built-in structural-panel screening fixture",
            )
        })
        .collect()
}

fn structural_candidate_report(
    candidate: &StructuralMaterialCandidate,
    payload: &Value,
) -> StructuralMaterialCandidateReport {
    let result = descend_result_payload(payload);
    let max_stress_pa = read_f64(result, "max_stress");
    let max_displacement_m = read_f64(result, "max_displacement");
    let yield_safety_factor = max_stress_pa
        .filter(|stress| *stress > 0.0)
        .map(|stress| candidate.yield_strength_pa / stress);
    let mut missing_metrics = Vec::new();
    if max_stress_pa.is_none() {
        missing_metrics.push("max_stress_pa".to_string());
        missing_metrics.push("yield_safety_factor".to_string());
    }
    if max_displacement_m.is_none() {
        missing_metrics.push("max_displacement_m".to_string());
    }

    StructuralMaterialCandidateReport {
        candidate_id: candidate.id.to_string(),
        candidate_label: candidate.label.to_string(),
        rank: 0,
        score: 0.0,
        max_stress_pa,
        max_displacement_m,
        areal_mass_kg_m2: candidate.density_kg_m3 * structural_panel_thickness_m(),
        specific_stiffness_m2_s2: candidate.youngs_modulus_pa / candidate.density_kg_m3,
        yield_safety_factor,
        optimization_terms: Vec::new(),
        missing_metrics,
    }
}

fn apply_scores(
    rows: &mut [StructuralMaterialCandidateReport],
    optimization: &MaterialOptimizationProfile,
) {
    let stress_values = rows
        .iter()
        .filter_map(|row| row.max_stress_pa)
        .collect::<Vec<_>>();
    let displacement_values = rows
        .iter()
        .filter_map(|row| row.max_displacement_m)
        .collect::<Vec<_>>();
    let mass_values = rows
        .iter()
        .map(|row| row.areal_mass_kg_m2)
        .collect::<Vec<_>>();
    let stiffness_values = rows
        .iter()
        .map(|row| row.specific_stiffness_m2_s2)
        .collect::<Vec<_>>();
    let safety_values = rows
        .iter()
        .filter_map(|row| row.yield_safety_factor)
        .collect::<Vec<_>>();

    for row in rows {
        let stress_weight = profile_weight(optimization, "max_stress_pa", 0.3);
        let displacement_weight = profile_weight(optimization, "max_displacement_m", 0.25);
        let mass_weight = profile_weight(optimization, "areal_mass_kg_m2", 0.2);
        let stiffness_weight = profile_weight(optimization, "specific_stiffness_m2_s2", 0.15);
        let safety_weight = profile_weight(optimization, "yield_safety_factor", 0.1);
        let stress_score = row
            .max_stress_pa
            .map(|value| normalize_minimize(value, &stress_values))
            .unwrap_or(0.0);
        let displacement_score = row
            .max_displacement_m
            .map(|value| normalize_minimize(value, &displacement_values))
            .unwrap_or(0.0);
        let mass_score = normalize_minimize(row.areal_mass_kg_m2, &mass_values);
        let stiffness_score = normalize_maximize(row.specific_stiffness_m2_s2, &stiffness_values);
        let safety_score = row
            .yield_safety_factor
            .map(|value| normalize_maximize(value, &safety_values))
            .unwrap_or(0.0);
        row.optimization_terms = vec![
            material_optimization_term(
                "max_stress_pa",
                "minimize",
                row.max_stress_pa,
                stress_score,
                stress_weight,
                &less_equal_status(row.max_stress_pa, 300.0e6),
            ),
            material_optimization_term(
                "max_displacement_m",
                "minimize",
                row.max_displacement_m,
                displacement_score,
                displacement_weight,
                &less_equal_status(row.max_displacement_m, 0.001),
            ),
            material_optimization_term(
                "areal_mass_kg_m2",
                "minimize",
                Some(row.areal_mass_kg_m2),
                mass_score,
                mass_weight,
                &less_equal_status(Some(row.areal_mass_kg_m2), 18.0),
            ),
            material_optimization_term(
                "specific_stiffness_m2_s2",
                "maximize",
                Some(row.specific_stiffness_m2_s2),
                stiffness_score,
                stiffness_weight,
                "observe",
            ),
            material_optimization_term(
                "yield_safety_factor",
                "maximize",
                row.yield_safety_factor,
                safety_score,
                safety_weight,
                safety_status(row.yield_safety_factor),
            ),
        ];
        row.score = round_score(
            stress_score * stress_weight
                + displacement_score * displacement_weight
                + mass_score * mass_weight
                + stiffness_score * stiffness_weight
                + safety_score * safety_weight,
        );
    }
}

fn structural_panel_optimization_profile() -> MaterialOptimizationProfile {
    material_optimization_profile(
        "material.structural_panel_screening.optimization.v1",
        "Minimize stress, deflection, and mass while preserving stiffness efficiency and yield margin.",
        "0.30*max_stress_pa:min + 0.25*max_displacement_m:min + 0.20*areal_mass_kg_m2:min + 0.15*specific_stiffness_m2_s2:max + 0.10*yield_safety_factor:max",
        vec![
            material_optimization_weight("max_stress_pa", "minimize", 0.3),
            material_optimization_weight("max_displacement_m", "minimize", 0.25),
            material_optimization_weight("areal_mass_kg_m2", "minimize", 0.2),
            material_optimization_weight("specific_stiffness_m2_s2", "maximize", 0.15),
            material_optimization_weight("yield_safety_factor", "maximize", 0.1),
        ],
        vec![
            material_optimization_constraint("max_stress_pa", "<=", 300.0e6, "warning"),
            material_optimization_constraint("max_displacement_m", "<=", 0.001, "warning"),
            material_optimization_constraint("areal_mass_kg_m2", "<=", 18.0, "warning"),
            material_optimization_constraint("yield_safety_factor", ">=", 1.5, "warning"),
        ],
    )
}

fn structural_research_metadata(candidate: &StructuralMaterialCandidate) -> Value {
    json!({
        "study": "material.structural_panel_screening.v1",
        "candidate_id": candidate.id,
        "candidate_label": candidate.label,
        "family": candidate.family,
        "density_kg_m3": candidate.density_kg_m3,
        "youngs_modulus_pa": candidate.youngs_modulus_pa,
        "yield_strength_pa": candidate.yield_strength_pa,
        "objective": "minimize panel stress, deflection, and mass while retaining yield margin",
        "note": candidate.note,
    })
}

fn structural_quad_model(candidate: &StructuralMaterialCandidate) -> Value {
    json!({
        "nodes": [
            { "id": "fixed_left_bottom", "x": 0.0, "y": 0.0, "fix_x": true, "fix_y": true, "load_x": 0.0, "load_y": 0.0 },
            { "id": "free_right_bottom", "x": 0.12, "y": 0.0, "fix_x": false, "fix_y": false, "load_x": 0.0, "load_y": -450.0 },
            { "id": "free_right_top", "x": 0.12, "y": 0.05, "fix_x": false, "fix_y": false, "load_x": 0.0, "load_y": -450.0 },
            { "id": "fixed_left_top", "x": 0.0, "y": 0.05, "fix_x": true, "fix_y": true, "load_x": 0.0, "load_y": 0.0 }
        ],
        "elements": [{
            "id": format!("panel_{}", candidate.id),
            "node_i": 0,
            "node_j": 1,
            "node_k": 2,
            "node_l": 3,
            "thickness": structural_panel_thickness_m(),
            "youngs_modulus": candidate.youngs_modulus_pa,
            "poisson_ratio": candidate.poisson_ratio
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

fn safety_status(value: Option<f64>) -> &'static str {
    match value {
        Some(actual) if actual >= 1.5 => "pass",
        Some(_) => "violate",
        None => "unknown",
    }
}

fn round_score(value: f64) -> f64 {
    (value * 1000.0).round() / 1000.0
}

fn structural_panel_thickness_m() -> f64 {
    0.002
}

fn structural_material_card_confidence(candidate: &StructuralMaterialCandidate) -> &'static str {
    match candidate.id {
        "aluminum_7075_t6" | "steel_4130_normalized" => "medium",
        _ => "low",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn structural_report_ranks_panel_candidates() {
        let report = build_structural_panel_screening_report(&[
            json!({ "result": { "max_stress": 210.0e6, "max_displacement": 0.0009 } }),
            json!({ "result": { "max_stress": 160.0e6, "max_displacement": 0.00042 } }),
            json!({ "max_stress": 120.0e6, "max_displacement": 0.00055 }),
        ])
        .expect("report should build");

        assert_eq!(
            report.schema_version,
            "kyuubiki.structural-material-report/v1"
        );
        assert_eq!(report.candidates.len(), 3);
        assert_eq!(report.material_card_refs.len(), 3);
        assert!(
            report
                .material_card_refs
                .iter()
                .any(|reference| reference.material_card_id
                    == "kyuubiki.material_card.carbon_fiber_quasi_iso.v1"
                    && reference.confidence == "low")
        );
        assert_eq!(
            report.winner_candidate_id.as_deref(),
            Some("carbon_fiber_quasi_iso")
        );
        assert_eq!(report.candidates[0].rank, 1);
        assert_eq!(report.candidates[0].optimization_terms.len(), 5);
        assert!(
            report.candidates[0]
                .optimization_terms
                .iter()
                .any(|term| term.metric_id == "yield_safety_factor")
        );
    }

    #[test]
    fn structural_report_keeps_missing_metric_warnings_visible() {
        let report = build_structural_panel_screening_report(&[
            json!({ "result": { "kind": "simulated_result" } }),
            json!({ "result": { "max_stress": 160.0e6 } }),
            json!({ "max_stress": 120.0e6, "max_displacement": 0.00055 }),
        ])
        .expect("report should tolerate incomplete early results");

        assert!(report.warnings.len() >= 3);
        assert!(
            report
                .warnings
                .iter()
                .any(|warning| warning.contains("aluminum_7075_t6 is missing max_stress_pa"))
        );
    }
}
