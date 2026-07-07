use crate::material_composite_models::{
    composite_research_metadata, electrostatic_model, heat_model, thermal_model,
};
use crate::{
    HeadlessWorkflowStep, MaterialEvidenceRef, MaterialModelAssumption,
    MaterialOptimizationProfile, MaterialOptimizationTerm, MaterialQualityGate,
    MaterialReliabilityEnvelope, MaterialResearchMetricSpec, material_evidence_ref,
    material_model_assumption, material_optimization_constraint, material_optimization_profile,
    material_optimization_term, material_optimization_weight, material_quality_gate,
    profile_weight,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompositePanelCandidateReport {
    pub candidate_id: String,
    pub candidate_label: String,
    pub rank: usize,
    pub score: f64,
    pub max_electric_field_v_m: Option<f64>,
    pub max_temperature_c: Option<f64>,
    pub max_thermal_stress_pa: Option<f64>,
    pub breakdown_safety_factor: Option<f64>,
    pub areal_mass_kg_m2: f64,
    pub optimization_terms: Vec<MaterialOptimizationTerm>,
    pub missing_metrics: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompositePanelMaterialRegion {
    pub id: String,
    pub role: String,
    pub material_family: String,
    pub elements: Vec<String>,
    pub active_fields: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompositePanelReport {
    pub schema_version: String,
    pub study: String,
    pub objective: String,
    pub coupling: String,
    pub material_regions: Vec<CompositePanelMaterialRegion>,
    pub optimization: MaterialOptimizationProfile,
    pub reliability: MaterialReliabilityEnvelope,
    pub metric_specs: Vec<MaterialResearchMetricSpec>,
    pub candidates: Vec<CompositePanelCandidateReport>,
    pub winner_candidate_id: Option<String>,
    pub warnings: Vec<String>,
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

pub fn composite_panel_metric_specs() -> Vec<MaterialResearchMetricSpec> {
    vec![
        metric(
            "max_electric_field_v_m",
            "Max electric field",
            "V/m",
            "minimize",
            0.25,
            "electrostatic.max_electric_field",
        ),
        metric(
            "max_temperature_c",
            "Max temperature",
            "C",
            "minimize",
            0.25,
            "heat.max_temperature",
        ),
        metric(
            "max_thermal_stress_pa",
            "Max thermal stress",
            "Pa",
            "minimize",
            0.25,
            "thermal.max_stress",
        ),
        metric(
            "breakdown_safety_factor",
            "Breakdown safety factor",
            "ratio",
            "maximize",
            0.15,
            "candidate.breakdown_field / electrostatic.max_electric_field",
        ),
        metric(
            "areal_mass_kg_m2",
            "Areal mass",
            "kg/m^2",
            "minimize",
            0.10,
            "candidate.stack_areal_mass",
        ),
    ]
}

pub fn build_composite_panel_steps() -> Vec<HeadlessWorkflowStep> {
    composite_panel_candidates()
        .into_iter()
        .map(|candidate| {
            HeadlessWorkflowStep::new(
                "solve_composite_thermo_electric_panel",
                json!({
                    "research": composite_research_metadata(&candidate),
                    "electrostatic_model": electrostatic_model(&candidate),
                    "heat_model": heat_model(&candidate),
                    "thermal_model": thermal_model(&candidate),
                }),
            )
        })
        .collect()
}

pub fn build_composite_panel_report(
    result_payloads: &[Value],
) -> Result<CompositePanelReport, String> {
    let candidates = composite_panel_candidates();
    if result_payloads.len() != candidates.len() {
        return Err(format!(
            "composite panel expects {} result payloads, received {}",
            candidates.len(),
            result_payloads.len()
        ));
    }
    let optimization = composite_optimization_profile();
    let mut rows = candidates
        .iter()
        .zip(result_payloads.iter())
        .map(|(candidate, payload)| composite_candidate_report(candidate, payload))
        .collect::<Vec<_>>();
    apply_scores(&mut rows, &optimization);
    rows.sort_by(|left, right| right.score.partial_cmp(&left.score).unwrap());
    for (index, row) in rows.iter_mut().enumerate() {
        row.rank = index + 1;
    }
    let warnings = rows
        .iter()
        .flat_map(|row| {
            row.missing_metrics
                .iter()
                .map(|metric| format!("{} is missing {}", row.candidate_id, metric))
        })
        .collect::<Vec<_>>();
    Ok(CompositePanelReport {
        schema_version: "kyuubiki.composite-panel-report/v1".to_string(),
        study: "material.composite_thermo_electric_panel.v1".to_string(),
        objective: "rank mixed-material electro-thermal-structural panel stacks".to_string(),
        coupling: "sequential_electrostatic_to_heat_to_thermal_stress".to_string(),
        material_regions: composite_material_regions(),
        optimization,
        reliability: composite_reliability_envelope(&rows),
        metric_specs: composite_panel_metric_specs(),
        winner_candidate_id: rows.first().map(|row| row.candidate_id.clone()),
        candidates: rows,
        warnings,
    })
}

pub fn composite_material_regions() -> Vec<CompositePanelMaterialRegion> {
    vec![
        CompositePanelMaterialRegion {
            id: "conductor_left".to_string(),
            role: "conductor_heat_spreader".to_string(),
            material_family: "metal".to_string(),
            elements: vec!["conductor_left".to_string()],
            active_fields: vec![
                "electrostatic".to_string(),
                "heat".to_string(),
                "thermal_stress".to_string(),
            ],
        },
        CompositePanelMaterialRegion {
            id: "dielectric_core".to_string(),
            role: "electrical_isolation".to_string(),
            material_family: "dielectric".to_string(),
            elements: vec!["dielectric_core".to_string()],
            active_fields: vec![
                "electrostatic".to_string(),
                "heat".to_string(),
                "thermal_stress".to_string(),
            ],
        },
        CompositePanelMaterialRegion {
            id: "substrate_right".to_string(),
            role: "mechanical_support".to_string(),
            material_family: "substrate".to_string(),
            elements: vec!["substrate_right".to_string()],
            active_fields: vec![
                "electrostatic".to_string(),
                "heat".to_string(),
                "thermal_stress".to_string(),
            ],
        },
    ]
}

fn composite_candidate_report(
    candidate: &CompositePanelCandidate,
    payload: &Value,
) -> CompositePanelCandidateReport {
    let result = descend_result_payload(payload);
    let max_electric_field_v_m = read_path_f64(result, &["electrostatic", "max_electric_field"]);
    let max_temperature_c = read_path_f64(result, &["heat", "max_temperature"]);
    let max_thermal_stress_pa = read_path_f64(result, &["thermal", "max_stress"]);
    let breakdown_safety_factor = max_electric_field_v_m
        .filter(|field| *field > 0.0)
        .map(|field| candidate.dielectric_breakdown_field_v_m / field);
    let mut missing_metrics = Vec::new();
    for (metric, value) in [
        ("max_electric_field_v_m", max_electric_field_v_m),
        ("max_temperature_c", max_temperature_c),
        ("max_thermal_stress_pa", max_thermal_stress_pa),
        ("breakdown_safety_factor", breakdown_safety_factor),
    ] {
        if value.is_none() {
            missing_metrics.push(metric.to_string());
        }
    }
    CompositePanelCandidateReport {
        candidate_id: candidate.id.to_string(),
        candidate_label: candidate.label.to_string(),
        rank: 0,
        score: 0.0,
        max_electric_field_v_m,
        max_temperature_c,
        max_thermal_stress_pa,
        breakdown_safety_factor,
        areal_mass_kg_m2: candidate.areal_mass_kg_m2,
        optimization_terms: Vec::new(),
        missing_metrics,
    }
}

fn apply_scores(rows: &mut [CompositePanelCandidateReport], profile: &MaterialOptimizationProfile) {
    let fields = rows
        .iter()
        .filter_map(|r| r.max_electric_field_v_m)
        .collect::<Vec<_>>();
    let temps = rows
        .iter()
        .filter_map(|r| r.max_temperature_c)
        .collect::<Vec<_>>();
    let stresses = rows
        .iter()
        .filter_map(|r| r.max_thermal_stress_pa)
        .collect::<Vec<_>>();
    let margins = rows
        .iter()
        .filter_map(|r| r.breakdown_safety_factor)
        .collect::<Vec<_>>();
    let masses = rows.iter().map(|r| r.areal_mass_kg_m2).collect::<Vec<_>>();
    for row in rows {
        row.optimization_terms.clear();
        let terms = [
            term_min(
                row.max_electric_field_v_m,
                &fields,
                profile,
                "max_electric_field_v_m",
            ),
            term_min(row.max_temperature_c, &temps, profile, "max_temperature_c"),
            term_min(
                row.max_thermal_stress_pa,
                &stresses,
                profile,
                "max_thermal_stress_pa",
            ),
            term_max(
                row.breakdown_safety_factor,
                &margins,
                profile,
                "breakdown_safety_factor",
            ),
            term_min(
                Some(row.areal_mass_kg_m2),
                &masses,
                profile,
                "areal_mass_kg_m2",
            ),
        ];
        row.score = terms.iter().map(|term| term.weighted_score).sum();
        row.optimization_terms.extend(terms);
    }
}

fn term_min(
    value: Option<f64>,
    values: &[f64],
    profile: &MaterialOptimizationProfile,
    id: &str,
) -> MaterialOptimizationTerm {
    let weight = profile_weight(profile, id, 0.0);
    let score = value.map(|v| normalize_minimize(v, values)).unwrap_or(0.0);
    material_optimization_term(id, "minimize", value, score, weight, "")
}

fn term_max(
    value: Option<f64>,
    values: &[f64],
    profile: &MaterialOptimizationProfile,
    id: &str,
) -> MaterialOptimizationTerm {
    let weight = profile_weight(profile, id, 0.0);
    let score = value.map(|v| normalize_maximize(v, values)).unwrap_or(0.0);
    material_optimization_term(id, "maximize", value, score, weight, "")
}

fn composite_optimization_profile() -> MaterialOptimizationProfile {
    material_optimization_profile(
        "material.composite_thermo_electric_panel.optimization.v1",
        "Balance electric margin, peak temperature, thermal stress, and mass.",
        "0.25*E:min + 0.25*T:min + 0.25*stress:min + 0.15*margin:max + 0.10*mass:min",
        vec![
            material_optimization_weight("max_electric_field_v_m", "minimize", 0.25),
            material_optimization_weight("max_temperature_c", "minimize", 0.25),
            material_optimization_weight("max_thermal_stress_pa", "minimize", 0.25),
            material_optimization_weight("breakdown_safety_factor", "maximize", 0.15),
            material_optimization_weight("areal_mass_kg_m2", "minimize", 0.10),
        ],
        vec![
            material_optimization_constraint("breakdown_safety_factor", ">=", 1.5, "warning"),
            material_optimization_constraint("max_temperature_c", "<=", 140.0, "warning"),
        ],
    )
}

fn composite_reliability_envelope(
    rows: &[CompositePanelCandidateReport],
) -> MaterialReliabilityEnvelope {
    MaterialReliabilityEnvelope {
        schema_version: "kyuubiki.material-reliability-envelope/v1".to_string(),
        posture: "prototype_screening_only".to_string(),
        material_card_version: "kyuubiki.material-cards.composite-panel.v1".to_string(),
        unit_system: "SI".to_string(),
        evidence_refs: composite_evidence_refs(),
        model_assumptions: composite_model_assumptions(),
        quality_gates: composite_quality_gates(rows),
        limitations: vec![
            "Sequential coupling maps electrostatic, heat, and thermal stress through fixed synthetic fixtures rather than a monolithic coupled matrix.".to_string(),
            "Material regions are scalar and isotropic; anisotropy, temperature-dependent curves, interfaces, and delamination are not modeled yet.".to_string(),
            "Electrical heating is represented by a screening heat fixture, not a Joule-loss field projection from electrostatic energy density.".to_string(),
            "Use this prototype for architecture validation and candidate ordering only, not qualification claims.".to_string(),
        ],
    }
}

fn composite_quality_gates(rows: &[CompositePanelCandidateReport]) -> Vec<MaterialQualityGate> {
    vec![
        material_quality_gate(
            "gate.breakdown_margin.prototype",
            "Breakdown safety prototype gate",
            "breakdown_safety_factor",
            ">=",
            1.5,
            min_optional(rows.iter().filter_map(|row| row.breakdown_safety_factor)),
            "The weakest candidate margin should remain above the prototype warning threshold.",
        ),
        material_quality_gate(
            "gate.max_temperature.prototype",
            "Peak temperature prototype gate",
            "max_temperature_c",
            "<=",
            140.0,
            max_optional(rows.iter().filter_map(|row| row.max_temperature_c)),
            "Screening fixtures should keep peak panel temperature below the warning limit.",
        ),
        material_quality_gate(
            "gate.max_thermal_stress.prototype",
            "Thermal stress prototype gate",
            "max_thermal_stress_pa",
            "<=",
            250.0e6,
            max_optional(rows.iter().filter_map(|row| row.max_thermal_stress_pa)),
            "Thermal stress should stay within a conservative prototype warning bound.",
        ),
        material_quality_gate(
            "gate.result_completeness",
            "Composite result completeness",
            "complete_candidate_count",
            ">=",
            rows.len() as f64,
            Some(
                rows.iter()
                    .filter(|row| row.missing_metrics.is_empty())
                    .count() as f64,
            ),
            "Every candidate should expose electric, heat, thermal, and margin metrics.",
        ),
    ]
}

fn composite_evidence_refs() -> Vec<MaterialEvidenceRef> {
    vec![
        material_evidence_ref(
            "evidence.prototype_material_cards",
            "Prototype material cards",
            "internal_screening_card",
            "kyuubiki.material-cards.composite-panel.v1",
            "screening",
            "Scalar room-temperature values for conductor, dielectric, and substrate families.",
        ),
        material_evidence_ref(
            "evidence.synthetic_multiphysics_fixture",
            "Synthetic mixed-material panel fixture",
            "solver_fixture",
            "kyuubiki.composite_thermo_electric_panel.fixture.v1",
            "prototype",
            "Three-region quad panel used to validate mixed-material sequential coupling.",
        ),
    ]
}

fn composite_model_assumptions() -> Vec<MaterialModelAssumption> {
    vec![
        material_model_assumption(
            "assumption.sequential_coupling",
            "Sequential coupling",
            "electrostatic -> heat -> thermal stress",
            "Good for workflow validation, but misses strong bidirectional coupling.",
        ),
        material_model_assumption(
            "assumption.scalar_regions",
            "Scalar isotropic regions",
            "one material parameter set per region and field",
            "Fast to evaluate, but does not represent anisotropy or interfaces.",
        ),
        material_model_assumption(
            "assumption.prototype_geometry",
            "Prototype panel geometry",
            "three quad regions sharing boundary nodes",
            "Captures multi-region topology without CAD-level geometric fidelity.",
        ),
    ]
}

fn min_optional(values: impl Iterator<Item = f64>) -> Option<f64> {
    values.fold(None, |current: Option<f64>, value| {
        Some(current.map_or(value, |min| min.min(value)))
    })
}

fn max_optional(values: impl Iterator<Item = f64>) -> Option<f64> {
    values.fold(None, |current: Option<f64>, value| {
        Some(current.map_or(value, |max| max.max(value)))
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

fn read_path_f64(payload: &Value, path: &[&str]) -> Option<f64> {
    path.iter()
        .try_fold(payload, |current, key| current.get(*key))?
        .as_f64()
}

fn normalize_minimize(value: f64, values: &[f64]) -> f64 {
    let (min, max) = value_range(values);
    if (max - min).abs() < f64::EPSILON {
        1.0
    } else {
        (max - value) / (max - min)
    }
}

fn normalize_maximize(value: f64, values: &[f64]) -> f64 {
    let (min, max) = value_range(values);
    if (max - min).abs() < f64::EPSILON {
        1.0
    } else {
        (value - min) / (max - min)
    }
}

fn value_range(values: &[f64]) -> (f64, f64) {
    values
        .iter()
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(min, max), value| {
            (min.min(*value), max.max(*value))
        })
}
