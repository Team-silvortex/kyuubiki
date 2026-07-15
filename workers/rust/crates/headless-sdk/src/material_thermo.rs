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
pub struct ThermoMaterialCandidate {
    pub id: &'static str,
    pub label: &'static str,
    pub family: &'static str,
    pub density_kg_m3: f64,
    pub youngs_modulus_pa: f64,
    pub poisson_ratio: f64,
    pub thermal_expansion_1_k: f64,
    pub note: &'static str,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThermoMaterialCandidateReport {
    pub candidate_id: String,
    pub candidate_label: String,
    pub rank: usize,
    pub score: f64,
    pub max_stress_pa: Option<f64>,
    pub max_displacement_m: Option<f64>,
    pub max_temperature_delta_k: Option<f64>,
    pub areal_mass_kg_m2: f64,
    pub optimization_terms: Vec<MaterialOptimizationTerm>,
    pub missing_metrics: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThermoMaterialReport {
    pub schema_version: String,
    pub study: String,
    pub objective: String,
    pub optimization: MaterialOptimizationProfile,
    pub metric_specs: Vec<MaterialResearchMetricSpec>,
    pub material_card_refs: Vec<MaterialCardReference>,
    pub candidates: Vec<ThermoMaterialCandidateReport>,
    pub winner_candidate_id: Option<String>,
    pub warnings: Vec<String>,
}

pub fn thermo_shield_screening_candidates() -> Vec<ThermoMaterialCandidate> {
    vec![
        ThermoMaterialCandidate {
            id: "aluminum_6061_t6",
            label: "Aluminum 6061-T6",
            family: "metal",
            density_kg_m3: 2700.0,
            youngs_modulus_pa: 68.9e9,
            poisson_ratio: 0.33,
            thermal_expansion_1_k: 23.6e-6,
            note: "lightweight baseline with high thermal expansion",
        },
        ThermoMaterialCandidate {
            id: "titanium_grade_5",
            label: "Titanium Grade 5",
            family: "metal",
            density_kg_m3: 4430.0,
            youngs_modulus_pa: 113.8e9,
            poisson_ratio: 0.342,
            thermal_expansion_1_k: 8.6e-6,
            note: "lower thermal expansion with moderate mass penalty",
        },
        ThermoMaterialCandidate {
            id: "invar_36",
            label: "Invar 36",
            family: "alloy",
            density_kg_m3: 8050.0,
            youngs_modulus_pa: 141.0e9,
            poisson_ratio: 0.29,
            thermal_expansion_1_k: 1.2e-6,
            note: "very low expansion but heavy",
        },
    ]
}

pub fn thermo_shield_metric_specs() -> Vec<MaterialResearchMetricSpec> {
    vec![
        metric(
            "max_stress_pa",
            "Max von Mises stress",
            "Pa",
            "minimize",
            0.45,
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
            0.25,
            "candidate.density_kg_m3 * model.thickness",
        ),
        metric(
            "max_temperature_delta_k",
            "Max temperature delta",
            "K",
            "observe",
            0.0,
            "solver.result.max_temperature_delta",
        ),
        metric(
            "thermal_expansion_1_k",
            "Thermal expansion",
            "1/K",
            "minimize",
            0.05,
            "candidate.thermal_expansion_1_k",
        ),
    ]
}

pub fn build_thermo_shield_screening_steps() -> Vec<HeadlessWorkflowStep> {
    thermo_shield_screening_candidates()
        .into_iter()
        .enumerate()
        .flat_map(|(candidate_index, candidate)| {
            let solve_step = candidate_index * 3 + 1;
            [
                HeadlessWorkflowStep::new(
                    "solve_thermal_plane_quad_2d",
                    json!({
                        "research": thermo_research_metadata(&candidate),
                        "model": thermo_quad_model(&candidate),
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

pub fn build_thermo_shield_screening_report(
    result_payloads: &[Value],
) -> Result<ThermoMaterialReport, String> {
    build_thermo_shield_screening_report_with_optimization(
        result_payloads,
        thermo_shield_optimization_profile(),
    )
}

pub fn build_thermo_shield_screening_report_with_optimization(
    result_payloads: &[Value],
    optimization: MaterialOptimizationProfile,
) -> Result<ThermoMaterialReport, String> {
    let candidates = thermo_shield_screening_candidates();
    if result_payloads.len() != candidates.len() {
        return Err(format!(
            "thermo shield screening expects {} result payloads, received {}",
            candidates.len(),
            result_payloads.len()
        ));
    }

    let mut rows = candidates
        .iter()
        .zip(result_payloads.iter())
        .map(|(candidate, payload)| thermo_candidate_report(candidate, payload))
        .collect::<Vec<_>>();
    apply_scores(&mut rows, &candidates, &optimization);
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

    Ok(ThermoMaterialReport {
        schema_version: "kyuubiki.thermo-material-report/v1".to_string(),
        study: "material.thermo_shield_screening.v1".to_string(),
        objective:
            "rank constrained thermo-mechanical shield candidates by stress, displacement, and mass"
                .to_string(),
        optimization,
        metric_specs: thermo_shield_metric_specs(),
        material_card_refs: thermo_material_card_refs(),
        winner_candidate_id: rows.first().map(|row| row.candidate_id.clone()),
        candidates: rows,
        warnings,
    })
}

fn thermo_material_card_refs() -> Vec<MaterialCardReference> {
    thermo_shield_screening_candidates()
        .iter()
        .map(|candidate| {
            built_in_material_card_ref(
                candidate.id,
                thermo_material_card_confidence(candidate),
                "room-temperature scalar thermo-mechanical screening values",
                "kyuubiki built-in thermo-shield screening fixture",
            )
        })
        .collect()
}

fn thermo_candidate_report(
    candidate: &ThermoMaterialCandidate,
    payload: &Value,
) -> ThermoMaterialCandidateReport {
    let result = descend_result_payload(payload);
    let max_stress_pa = read_f64(result, "max_stress");
    let max_displacement_m = read_f64(result, "max_displacement");
    let max_temperature_delta_k = read_f64(result, "max_temperature_delta");
    let mut missing_metrics = Vec::new();
    if max_stress_pa.is_none() {
        missing_metrics.push("max_stress_pa".to_string());
    }
    if max_displacement_m.is_none() {
        missing_metrics.push("max_displacement_m".to_string());
    }
    if max_temperature_delta_k.is_none() {
        missing_metrics.push("max_temperature_delta_k".to_string());
    }

    ThermoMaterialCandidateReport {
        candidate_id: candidate.id.to_string(),
        candidate_label: candidate.label.to_string(),
        rank: 0,
        score: 0.0,
        max_stress_pa,
        max_displacement_m,
        max_temperature_delta_k,
        areal_mass_kg_m2: candidate.density_kg_m3 * thermo_shield_thickness_m(),
        optimization_terms: Vec::new(),
        missing_metrics,
    }
}

fn apply_scores(
    rows: &mut [ThermoMaterialCandidateReport],
    candidates: &[ThermoMaterialCandidate],
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
    let expansion_values = candidates
        .iter()
        .map(|candidate| candidate.thermal_expansion_1_k)
        .collect::<Vec<_>>();

    for (row, candidate) in rows.iter_mut().zip(candidates.iter()) {
        let stress_weight = profile_weight(optimization, "max_stress_pa", 0.45);
        let displacement_weight = profile_weight(optimization, "max_displacement_m", 0.25);
        let mass_weight = profile_weight(optimization, "areal_mass_kg_m2", 0.25);
        let expansion_weight = profile_weight(optimization, "thermal_expansion_1_k", 0.05);
        let stress_score = row
            .max_stress_pa
            .map(|value| normalize_minimize(value, &stress_values))
            .unwrap_or(0.0);
        let displacement_score = row
            .max_displacement_m
            .map(|value| normalize_minimize(value, &displacement_values))
            .unwrap_or(0.0);
        let mass_score = normalize_minimize(row.areal_mass_kg_m2, &mass_values);
        let expansion_score =
            normalize_minimize(candidate.thermal_expansion_1_k, &expansion_values);
        row.optimization_terms = vec![
            material_optimization_term(
                "max_stress_pa",
                "minimize",
                row.max_stress_pa,
                stress_score,
                stress_weight,
                &less_equal_status(row.max_stress_pa, 120.0e6),
            ),
            material_optimization_term(
                "max_displacement_m",
                "minimize",
                row.max_displacement_m,
                displacement_score,
                displacement_weight,
                &less_equal_status(row.max_displacement_m, 0.00025),
            ),
            material_optimization_term(
                "areal_mass_kg_m2",
                "minimize",
                Some(row.areal_mass_kg_m2),
                mass_score,
                mass_weight,
                &less_equal_status(Some(row.areal_mass_kg_m2), 10.0),
            ),
            material_optimization_term(
                "thermal_expansion_1_k",
                "minimize",
                Some(candidate.thermal_expansion_1_k),
                expansion_score,
                expansion_weight,
                "observe",
            ),
        ];
        row.score = round_score(
            stress_score * stress_weight
                + displacement_score * displacement_weight
                + mass_score * mass_weight
                + expansion_score * expansion_weight,
        );
    }
}

fn thermo_shield_optimization_profile() -> MaterialOptimizationProfile {
    material_optimization_profile(
        "material.thermo_shield_screening.optimization.v1",
        "Minimize thermal stress, displacement, and areal mass under constrained heating.",
        "0.45*max_stress_pa:min + 0.25*max_displacement_m:min + 0.25*areal_mass_kg_m2:min + 0.05*thermal_expansion_1_k:min",
        vec![
            material_optimization_weight("max_stress_pa", "minimize", 0.45),
            material_optimization_weight("max_displacement_m", "minimize", 0.25),
            material_optimization_weight("areal_mass_kg_m2", "minimize", 0.25),
            material_optimization_weight("thermal_expansion_1_k", "minimize", 0.05),
        ],
        vec![
            material_optimization_constraint("max_stress_pa", "<=", 120.0e6, "warning"),
            material_optimization_constraint("max_displacement_m", "<=", 0.00025, "warning"),
            material_optimization_constraint("areal_mass_kg_m2", "<=", 10.0, "warning"),
        ],
    )
}

fn thermo_research_metadata(candidate: &ThermoMaterialCandidate) -> Value {
    json!({
        "study": "material.thermo_shield_screening.v1",
        "candidate_id": candidate.id,
        "candidate_label": candidate.label,
        "family": candidate.family,
        "density_kg_m3": candidate.density_kg_m3,
        "youngs_modulus_pa": candidate.youngs_modulus_pa,
        "thermal_expansion_1_k": candidate.thermal_expansion_1_k,
        "objective": "minimize constrained thermal stress, displacement, and areal mass",
        "note": candidate.note,
    })
}

fn thermo_quad_model(candidate: &ThermoMaterialCandidate) -> Value {
    json!({
        "nodes": [
            { "id": "fixed_left_bottom", "x": 0.0, "y": 0.0, "fix_x": true, "fix_y": true, "load_x": 0.0, "load_y": 0.0, "temperature_delta": 70.0 },
            { "id": "fixed_right_bottom", "x": 0.08, "y": 0.0, "fix_x": true, "fix_y": true, "load_x": 0.0, "load_y": 0.0, "temperature_delta": 70.0 },
            { "id": "free_right_top", "x": 0.08, "y": 0.04, "fix_x": false, "fix_y": false, "load_x": 0.0, "load_y": 0.0, "temperature_delta": 110.0 },
            { "id": "free_left_top", "x": 0.0, "y": 0.04, "fix_x": false, "fix_y": false, "load_x": 0.0, "load_y": 0.0, "temperature_delta": 110.0 }
        ],
        "elements": [{
            "id": format!("thermo_{}", candidate.id),
            "node_i": 0,
            "node_j": 1,
            "node_k": 2,
            "node_l": 3,
            "thickness": thermo_shield_thickness_m(),
            "youngs_modulus": candidate.youngs_modulus_pa,
            "poisson_ratio": candidate.poisson_ratio,
            "thermal_expansion": candidate.thermal_expansion_1_k
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

fn thermo_shield_thickness_m() -> f64 {
    0.002
}

fn thermo_material_card_confidence(candidate: &ThermoMaterialCandidate) -> &'static str {
    match candidate.id {
        "aluminum_6061_t6" | "titanium_grade_5" => "medium",
        _ => "low",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thermo_report_ranks_multiphysics_candidates() {
        let report = build_thermo_shield_screening_report(&[
            json!({ "result": { "max_stress": 180.0e6, "max_displacement": 0.00032, "max_temperature_delta": 110.0 } }),
            json!({ "result": { "max_stress": 90.0e6, "max_displacement": 0.00022, "max_temperature_delta": 110.0 } }),
            json!({ "max_stress": 35.0e6, "max_displacement": 0.00018, "max_temperature_delta": 110.0 }),
        ])
        .expect("report should build");

        assert_eq!(report.schema_version, "kyuubiki.thermo-material-report/v1");
        assert_eq!(report.candidates.len(), 3);
        assert_eq!(report.material_card_refs.len(), 3);
        assert!(
            report
                .material_card_refs
                .iter()
                .any(|reference| reference.material_card_id
                    == "kyuubiki.material_card.invar_36.v1"
                    && reference.unit_system == "si")
        );
        assert_eq!(report.winner_candidate_id.as_deref(), Some("invar_36"));
        assert_eq!(
            report.optimization.id,
            "material.thermo_shield_screening.optimization.v1"
        );
        assert!(
            report
                .optimization
                .score_formula
                .contains("max_stress_pa:min")
        );
        assert_eq!(report.candidates[0].rank, 1);
        assert_eq!(report.candidates[0].optimization_terms.len(), 4);
        assert!(
            report.candidates[0]
                .optimization_terms
                .iter()
                .any(|term| term.metric_id == "max_stress_pa" && term.constraint_status == "pass")
        );
        assert!(report.candidates[0].score > report.candidates[2].score);
    }

    #[test]
    fn thermo_report_keeps_missing_metric_warnings_visible() {
        let report = build_thermo_shield_screening_report(&[
            json!({ "result": { "kind": "simulated_result" } }),
            json!({ "result": { "max_stress": 90.0e6 } }),
            json!({ "result": { "max_stress": 35.0e6, "max_displacement": 0.00018, "max_temperature_delta": 110.0 } }),
        ])
        .expect("report should tolerate incomplete early results");

        assert!(report.warnings.len() >= 4);
        assert!(
            report
                .warnings
                .iter()
                .any(|warning| warning.contains("aluminum_6061_t6 is missing max_stress_pa"))
        );
    }
}
