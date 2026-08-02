use crate::material_card_refs::built_in_material_card_ref;
use crate::material_composite_candidates::{CompositePanelCandidate, composite_panel_candidates};
use crate::material_composite_evidence::{composite_evidence_refs, composite_model_assumptions};
use crate::material_composite_interfaces::{
    CompositePanelInterfaceAssessment, CompositePanelMaterialRegion, assess_composite_interfaces,
    composite_material_regions,
};
use crate::material_composite_models::{
    composite_research_metadata, electrostatic_model, electrothermal_loss_model, heat_model,
    thermal_model,
};
use crate::material_composite_quality::composite_structural_quality_gates;
use crate::{
    CompositeElectrostaticCrossValidation, CompositeElectrostaticMeshConvergence,
    CompositeElectrothermalLossProjection, CompositeHeatCrossValidation,
    CompositeHeatMeshConvergence, CompositeHeatToThermalProjection,
    CompositeThermalConstraintSensitivity, CompositeThermalInterfaceGradingAssessment,
    CompositeThermalMeshConvergence, CompositeThermalStressRecovery, HeadlessWorkflowStep,
    MaterialCardReference, MaterialOptimizationProfile, MaterialOptimizationTerm,
    MaterialQualityGate, MaterialReliabilityEnvelope, MaterialResearchMetricSpec,
    composite_electrostatic_cross_validation, composite_electrostatic_mesh_convergence,
    composite_heat_cross_validation, composite_heat_cross_validation_for_distributed_load,
    composite_heat_mesh_convergence, composite_heat_mesh_convergence_for_distributed_load,
    composite_thermal_constraint_sensitivity, composite_thermal_interface_grading_assessment,
    composite_thermal_mesh_convergence, composite_thermal_stress_recovery,
    material_optimization_constraint, material_optimization_profile, material_optimization_term,
    material_optimization_weight, material_quality_gate, material_reliability_summary,
    profile_weight,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompositePanelCandidateReport {
    pub candidate_id: String,
    pub candidate_label: String,
    pub rank: usize,
    pub score: f64,
    pub max_electric_field_v_m: Option<f64>,
    pub electrostatic_cross_validation: CompositeElectrostaticCrossValidation,
    pub electrostatic_mesh_convergence: CompositeElectrostaticMeshConvergence,
    pub electrothermal_loss_projection: Option<CompositeElectrothermalLossProjection>,
    pub max_temperature_c: Option<f64>,
    pub heat_cross_validation: CompositeHeatCrossValidation,
    pub heat_mesh_convergence: CompositeHeatMeshConvergence,
    pub heat_to_thermal_projection: Option<CompositeHeatToThermalProjection>,
    pub max_thermal_stress_pa: Option<f64>,
    pub thermal_mesh_convergence: CompositeThermalMeshConvergence,
    pub thermal_constraint_regularized_mesh_convergence: CompositeThermalMeshConvergence,
    pub thermal_constraint_sensitivity: CompositeThermalConstraintSensitivity,
    pub thermal_stress_recovery: CompositeThermalStressRecovery,
    pub thermal_interface_grading_assessment: CompositeThermalInterfaceGradingAssessment,
    pub breakdown_safety_factor: Option<f64>,
    pub interface_risk_score: Option<f64>,
    pub weakest_interface: Option<CompositePanelInterfaceAssessment>,
    pub areal_mass_kg_m2: f64,
    pub optimization_terms: Vec<MaterialOptimizationTerm>,
    pub missing_metrics: Vec<String>,
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
    pub material_card_refs: Vec<MaterialCardReference>,
    pub candidates: Vec<CompositePanelCandidateReport>,
    pub winner_candidate_id: Option<String>,
    pub warnings: Vec<String>,
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
            "dielectric_loss_power_w",
            "Dielectric loss power",
            "W",
            "minimize",
            0.0,
            "electrothermal_loss_projection.total_loss_w",
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
            0.08,
            "candidate.stack_areal_mass",
        ),
        metric(
            "interface_risk_score",
            "Interface risk score",
            "0..1",
            "minimize",
            0.12,
            "candidate.interface_compatibility",
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
                    "electrothermal_loss": electrothermal_loss_model(&candidate),
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
    let mut warnings = rows
        .iter()
        .flat_map(|row| {
            row.missing_metrics
                .iter()
                .map(|metric| format!("{} is missing {}", row.candidate_id, metric))
        })
        .collect::<Vec<_>>();
    warnings.extend(rows.iter().filter_map(|row| {
        let risk = row.interface_risk_score?;
        (risk > 0.70).then(|| {
            format!(
                "{} exceeds prototype interface risk threshold: {:.3}",
                row.candidate_id, risk
            )
        })
    }));
    Ok(CompositePanelReport {
        schema_version: "kyuubiki.composite-panel-report/v1".to_string(),
        study: "material.composite_thermo_electric_panel.v1".to_string(),
        objective: "rank mixed-material electro-thermal-structural panel stacks".to_string(),
        coupling: "sequential_electrostatic_dielectric_loss_to_heat_to_thermal_stress".to_string(),
        material_regions: composite_material_regions(),
        optimization,
        reliability: composite_reliability_envelope(&rows),
        metric_specs: composite_panel_metric_specs(),
        material_card_refs: composite_material_card_refs(),
        winner_candidate_id: rows.first().map(|row| row.candidate_id.clone()),
        candidates: rows,
        warnings,
    })
}

fn composite_material_card_refs() -> Vec<MaterialCardReference> {
    composite_panel_candidates()
        .iter()
        .map(|candidate| {
            built_in_material_card_ref(
                candidate.id,
                composite_material_card_confidence(candidate),
                "room-temperature scalar composite stack screening values",
                "kyuubiki built-in composite thermo-electric panel fixture",
            )
        })
        .collect()
}

fn composite_candidate_report(
    candidate: &CompositePanelCandidate,
    payload: &Value,
) -> CompositePanelCandidateReport {
    let result = descend_result_payload(payload);
    let max_electric_field_v_m = read_path_f64(result, &["electrostatic", "max_electric_field"]);
    let electrothermal_loss_projection: Option<CompositeElectrothermalLossProjection> = result
        .get("electrothermal_loss_projection")
        .cloned()
        .and_then(|value| serde_json::from_value(value).ok());
    let max_temperature_c = read_path_f64(result, &["heat", "max_temperature"]);
    let max_thermal_stress_pa = read_path_f64(result, &["thermal", "max_stress"]);
    let thermal_mesh_convergence = result
        .get("thermal_mesh_convergence")
        .cloned()
        .and_then(|value| serde_json::from_value(value).ok())
        .unwrap_or_else(|| composite_thermal_mesh_convergence(&[]));
    let thermal_constraint_regularized_mesh_convergence = result
        .get("thermal_constraint_regularized_mesh_convergence")
        .cloned()
        .and_then(|value| serde_json::from_value(value).ok())
        .unwrap_or_else(|| crate::composite_thermal_regularized_mesh_convergence(&[]));
    let thermal_constraint_sensitivity = result
        .get("thermal_constraint_sensitivity")
        .cloned()
        .and_then(|value| serde_json::from_value(value).ok())
        .unwrap_or_else(|| {
            composite_thermal_constraint_sensitivity(
                &thermal_mesh_convergence,
                &thermal_constraint_regularized_mesh_convergence,
            )
        });
    let thermal_stress_recovery = result
        .get("thermal_stress_recovery")
        .cloned()
        .and_then(|value| serde_json::from_value(value).ok())
        .unwrap_or_else(|| composite_thermal_stress_recovery(&[]));
    let thermal_interface_grading_assessment = result
        .get("thermal_interface_grading_assessment")
        .cloned()
        .and_then(|value| serde_json::from_value(value).ok())
        .unwrap_or_else(|| {
            composite_thermal_interface_grading_assessment(
                &thermal_mesh_convergence,
                &thermal_stress_recovery,
                crate::composite_thermal_interface_graded_mesh_convergence(&[]),
                crate::composite_thermal_interface_graded_stress_recovery(&[]),
            )
        });
    let breakdown_safety_factor = max_electric_field_v_m
        .filter(|field| *field > 0.0)
        .map(|field| candidate.dielectric_breakdown_field_v_m / field);
    let electrostatic_cross_validation =
        composite_electrostatic_cross_validation(candidate, max_electric_field_v_m);
    let electrostatic_mesh_convergence = result
        .get("electrostatic_mesh_convergence")
        .cloned()
        .and_then(|value| serde_json::from_value(value).ok())
        .unwrap_or_else(|| composite_electrostatic_mesh_convergence(candidate, &[]));
    let heat_conductivities = [candidate.conductor_conductivity_w_mk, 0.25, 160.0];
    let heat_cross_validation = result
        .get("heat_cross_validation")
        .cloned()
        .and_then(|value| serde_json::from_value(value).ok())
        .unwrap_or_else(|| {
            electrothermal_loss_projection.as_ref().map_or_else(
                || composite_heat_cross_validation(heat_conductivities, max_temperature_c),
                |projection| {
                    composite_heat_cross_validation_for_distributed_load(
                        heat_conductivities,
                        projection.total_loss_w,
                        max_temperature_c,
                    )
                },
            )
        });
    let heat_mesh_convergence = result
        .get("heat_mesh_convergence")
        .cloned()
        .and_then(|value| serde_json::from_value(value).ok())
        .unwrap_or_else(|| {
            electrothermal_loss_projection.as_ref().map_or_else(
                || composite_heat_mesh_convergence(heat_conductivities, &[]),
                |projection| {
                    composite_heat_mesh_convergence_for_distributed_load(
                        heat_conductivities,
                        projection.total_loss_w,
                        &[],
                    )
                },
            )
        });
    let heat_to_thermal_projection = result
        .get("heat_to_thermal_projection")
        .cloned()
        .and_then(|value| serde_json::from_value(value).ok());
    let interfaces = assess_composite_interfaces(candidate);
    let weakest_interface = interfaces
        .iter()
        .max_by(|left, right| {
            left.risk_score
                .partial_cmp(&right.risk_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .cloned();
    let interface_risk_score = weakest_interface
        .as_ref()
        .map(|interface| interface.risk_score);
    let mut missing_metrics = Vec::new();
    for (metric, value) in [
        ("max_electric_field_v_m", max_electric_field_v_m),
        ("max_temperature_c", max_temperature_c),
        ("max_thermal_stress_pa", max_thermal_stress_pa),
        ("breakdown_safety_factor", breakdown_safety_factor),
        ("interface_risk_score", interface_risk_score),
    ] {
        if value.is_none() {
            missing_metrics.push(metric.to_string());
        }
    }
    if electrothermal_loss_projection.is_none() {
        missing_metrics.push("electrothermal_loss_projection".to_string());
    }
    if heat_to_thermal_projection.is_none() {
        missing_metrics.push("heat_to_thermal_projection".to_string());
    }
    CompositePanelCandidateReport {
        candidate_id: candidate.id.to_string(),
        candidate_label: candidate.label.to_string(),
        rank: 0,
        score: 0.0,
        max_electric_field_v_m,
        electrostatic_cross_validation,
        electrostatic_mesh_convergence,
        electrothermal_loss_projection,
        max_temperature_c,
        heat_cross_validation,
        heat_mesh_convergence,
        heat_to_thermal_projection,
        max_thermal_stress_pa,
        thermal_mesh_convergence,
        thermal_constraint_regularized_mesh_convergence,
        thermal_constraint_sensitivity,
        thermal_stress_recovery,
        thermal_interface_grading_assessment,
        breakdown_safety_factor,
        interface_risk_score,
        weakest_interface,
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
    let interface_risks = rows
        .iter()
        .filter_map(|r| r.interface_risk_score)
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
            term_min(
                row.interface_risk_score,
                &interface_risks,
                profile,
                "interface_risk_score",
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
        "Balance electric margin, peak temperature, thermal stress, interface risk, and mass.",
        "0.23*E:min + 0.23*T:min + 0.22*stress:min + 0.15*margin:max + 0.12*interface:min + 0.05*mass:min",
        vec![
            material_optimization_weight("max_electric_field_v_m", "minimize", 0.23),
            material_optimization_weight("max_temperature_c", "minimize", 0.23),
            material_optimization_weight("max_thermal_stress_pa", "minimize", 0.22),
            material_optimization_weight("breakdown_safety_factor", "maximize", 0.15),
            material_optimization_weight("interface_risk_score", "minimize", 0.12),
            material_optimization_weight("areal_mass_kg_m2", "minimize", 0.05),
        ],
        vec![
            material_optimization_constraint("breakdown_safety_factor", ">=", 1.5, "warning"),
            material_optimization_constraint("max_temperature_c", "<=", 140.0, "warning"),
            material_optimization_constraint("interface_risk_score", "<=", 0.70, "warning"),
        ],
    )
}

fn composite_reliability_envelope(
    rows: &[CompositePanelCandidateReport],
) -> MaterialReliabilityEnvelope {
    let quality_gates = composite_quality_gates(rows);
    MaterialReliabilityEnvelope {
        schema_version: "kyuubiki.material-reliability-envelope/v1".to_string(),
        posture: "prototype_screening_only".to_string(),
        material_card_version: "kyuubiki.material-cards.composite-panel.v1".to_string(),
        unit_system: "SI".to_string(),
        evidence_refs: composite_evidence_refs(),
        model_assumptions: composite_model_assumptions(),
        summary: material_reliability_summary(&quality_gates),
        quality_gates,
        limitations: vec![
            "Sequential coupling is one-way and weakly coupled; temperature-dependent electrical feedback and a monolithic coupled Jacobian are not modeled yet.".to_string(),
            "Material regions are scalar and isotropic; anisotropy, temperature-dependent curves, and delamination propagation are not modeled yet.".to_string(),
            "Electrical heating projects harmonic dielectric loss from solved RMS fields; conductor current flow, contact resistance, and broadband dielectric curves remain outside this screening model.".to_string(),
            "Interface risk is a screening heuristic over CTE mismatch and stiffness contrast, not an adhesive fracture mechanics model.".to_string(),
            "The regularized restraint solve is diagnostic only; persistent strain-energy nonconvergence remains a qualification blocker.".to_string(),
            "Use this prototype for architecture validation and candidate ordering only, not qualification claims.".to_string(),
        ],
    }
}

fn composite_quality_gates(rows: &[CompositePanelCandidateReport]) -> Vec<MaterialQualityGate> {
    let mut gates = vec![
        material_quality_gate(
            "gate.electrostatic_closed_form.relative_error",
            "Layered dielectric closed-form cross-validation",
            "electrostatic_closed_form_relative_error",
            "<=",
            1.0e-9,
            max_optional(
                rows.iter()
                    .filter_map(|row| row.electrostatic_cross_validation.relative_error),
            ),
            "FEM maximum electric field must match the independent layered-dielectric closed form.",
        ),
        material_quality_gate(
            "gate.electrostatic_mesh_convergence.finest_pair",
            "Electrostatic mesh convergence",
            "electrostatic_mesh_finest_pair_relative_change",
            "<=",
            1.0e-8,
            max_optional(rows.iter().filter_map(|row| {
                row.electrostatic_mesh_convergence
                    .finest_pair_relative_change
            })),
            "The maximum electric field must remain stable between the two finest retained meshes.",
        ),
        material_quality_gate(
            "gate.electrostatic_mesh_convergence.analytic_error",
            "Electrostatic refined-mesh analytic error",
            "electrostatic_mesh_max_analytic_relative_error",
            "<=",
            1.0e-8,
            max_optional(rows.iter().filter_map(|row| {
                row.electrostatic_mesh_convergence
                    .max_analytic_relative_error
            })),
            "Every retained mesh level must remain consistent with the layered-dielectric closed form.",
        ),
        material_quality_gate(
            "gate.electrothermal_loss.energy_balance",
            "Electrothermal loss projection energy balance",
            "electrothermal_loss_energy_balance_relative_error",
            "<=",
            1.0e-12,
            max_optional(rows.iter().filter_map(|row| {
                row.electrothermal_loss_projection
                    .as_ref()
                    .map(|projection| projection.energy_balance_relative_error)
            })),
            "Projected dielectric loss must equal the total heat load distributed to heat nodes.",
        ),
        material_quality_gate(
            "gate.heat_closed_form.relative_error",
            "Layered thermal-resistance cross-validation",
            "heat_closed_form_relative_error",
            "<=",
            1.0e-9,
            max_optional(
                rows.iter()
                    .filter_map(|row| row.heat_cross_validation.relative_error),
            ),
            "FEM maximum temperature must match the independent layered thermal-resistance solution.",
        ),
        material_quality_gate(
            "gate.heat_mesh_convergence.finest_pair",
            "Heat mesh convergence",
            "heat_mesh_finest_pair_relative_change",
            "<=",
            1.0e-8,
            max_optional(
                rows.iter()
                    .filter_map(|row| row.heat_mesh_convergence.finest_pair_relative_change),
            ),
            "The maximum temperature must remain stable between the two finest retained meshes.",
        ),
        material_quality_gate(
            "gate.heat_mesh_convergence.analytic_error",
            "Heat refined-mesh analytic error",
            "heat_mesh_max_analytic_relative_error",
            "<=",
            1.0e-8,
            max_optional(
                rows.iter()
                    .filter_map(|row| row.heat_mesh_convergence.max_analytic_relative_error),
            ),
            "Every retained heat mesh must remain consistent with the layered thermal-resistance solution.",
        ),
        material_quality_gate(
            "gate.heat_to_thermal.coordinate_alignment",
            "Heat-to-thermal node coordinate alignment",
            "heat_to_thermal_maximum_coordinate_error_m",
            "<=",
            1.0e-12,
            max_optional(rows.iter().filter_map(|row| {
                row.heat_to_thermal_projection
                    .as_ref()
                    .map(|projection| projection.maximum_coordinate_error_m)
            })),
            "Every projected temperature must target a structurally identical node coordinate.",
        ),
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
            "gate.thermal_mesh_convergence.displacement",
            "Thermal-structural displacement mesh convergence",
            "thermal_mesh_finest_pair_displacement_relative_change",
            "<=",
            2.0e-2,
            max_optional(rows.iter().filter_map(|row| {
                row.thermal_mesh_convergence
                    .finest_pair_displacement_relative_change
            })),
            "Maximum displacement should stabilize between the two finest two-dimensional meshes.",
        ),
        material_quality_gate(
            "gate.thermal_mesh_convergence.strain_energy",
            "Thermal-structural energy mesh convergence",
            "thermal_mesh_finest_pair_strain_energy_relative_change",
            "<=",
            2.0e-2,
            max_optional(rows.iter().filter_map(|row| {
                row.thermal_mesh_convergence
                    .finest_pair_strain_energy_relative_change
            })),
            "Total strain energy should stabilize between the two finest two-dimensional meshes.",
        ),
        material_quality_gate(
            "gate.interface_risk.prototype",
            "Interface compatibility prototype gate",
            "interface_risk_score",
            "<=",
            0.70,
            max_optional(rows.iter().filter_map(|row| row.interface_risk_score)),
            "Mixed-material interfaces should remain below the prototype mismatch risk threshold.",
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
    ];
    gates.extend(composite_structural_quality_gates(rows));
    gates
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

fn composite_material_card_confidence(candidate: &CompositePanelCandidate) -> &'static str {
    match candidate.id {
        "copper_polyimide_aluminum" | "aluminum_alumina_aluminum" => "medium",
        _ => "low",
    }
}
