use crate::material_composite_interfaces::assess_composite_interfaces;
use crate::material_composite_models::{electrostatic_model, heat_model, thermal_model};
use crate::{CompositePanelCandidate, HeadlessWorkflowStep, composite_panel_candidates};
use serde_json::{Value, json};

pub fn build_composite_materialized_candidate_steps(
    plan: &Value,
) -> Result<Vec<HeadlessWorkflowStep>, String> {
    let candidates = plan
        .get("materialized_candidates")
        .and_then(Value::as_array)
        .ok_or_else(|| "materialization plan is missing materialized_candidates".to_string())?;
    candidates.iter().map(materialized_candidate_step).collect()
}

fn materialized_candidate_step(spec: &Value) -> Result<HeadlessWorkflowStep, String> {
    if spec.get("status").and_then(Value::as_str) != Some("requires_solver_rerun") {
        return Err("materialized candidate does not require a solver rerun".to_string());
    }
    if required_str(spec, "study")? != "material_composite_thermo_electric_panel" {
        return Err("materialized candidate is not a composite panel study".to_string());
    }
    let source_candidate_id = required_str(spec, "source_candidate_id")?;
    let source_candidate = source_composite_candidate(source_candidate_id)?;
    let strategy = required_str(spec, "strategy")?;
    let candidate = apply_materialization_strategy(source_candidate, strategy)?;
    Ok(HeadlessWorkflowStep::new(
        "solve_composite_thermo_electric_panel",
        json!({
            "research": materialized_research_metadata(spec, &candidate)?,
            "electrostatic_model": electrostatic_model(&candidate),
            "heat_model": heat_model(&candidate),
            "thermal_model": thermal_model(&candidate),
        }),
    ))
}

fn source_composite_candidate(
    source_candidate_id: &str,
) -> Result<CompositePanelCandidate, String> {
    composite_panel_candidates()
        .into_iter()
        .find(|candidate| candidate.id == source_candidate_id)
        .ok_or_else(|| {
            format!("materialized candidate references unknown source: {source_candidate_id}")
        })
}

fn apply_materialization_strategy(
    mut candidate: CompositePanelCandidate,
    strategy: &str,
) -> Result<CompositePanelCandidate, String> {
    match strategy {
        "add_compliant_interlayer" => {
            candidate.substrate_youngs_modulus_pa *= 0.88;
            candidate.substrate_thermal_expansion_1_k =
                (candidate.substrate_thermal_expansion_1_k + 17.0e-6) / 2.0;
            candidate.dielectric_breakdown_field_v_m *= 0.98;
            candidate.areal_mass_kg_m2 += 0.05;
        }
        "replace_high_cte_dielectric" => {
            candidate.dielectric = "polyimide";
            candidate.dielectric_relative_permittivity =
                candidate.dielectric_relative_permittivity.max(3.4);
            candidate.dielectric_breakdown_field_v_m =
                candidate.dielectric_breakdown_field_v_m.max(300.0e6);
            candidate.areal_mass_kg_m2 += 0.18;
        }
        "reduce_stiffness_contrast" => {
            candidate.substrate_youngs_modulus_pa *= 0.82;
            candidate.substrate_thermal_expansion_1_k =
                (candidate.substrate_thermal_expansion_1_k + 25.0e-6) / 2.0;
            candidate.areal_mass_kg_m2 += 0.08;
        }
        _ => {
            return Err(format!(
                "unsupported composite materialization strategy: {strategy}"
            ));
        }
    }
    Ok(candidate)
}

fn materialized_research_metadata(
    spec: &Value,
    candidate: &CompositePanelCandidate,
) -> Result<Value, String> {
    let candidate_id = required_str(spec, "candidate_id")?;
    let strategy = required_str(spec, "strategy")?;
    Ok(json!({
        "study": "material.composite_thermo_electric_panel.v1",
        "candidate_id": candidate_id,
        "candidate_label": format!("{} / {}", candidate.label, strategy.replace('_', " ")),
        "source_candidate_id": required_str(spec, "source_candidate_id")?,
        "source_draft_id": required_str(spec, "source_draft_id")?,
        "strategy": strategy,
        "materials": {
            "conductor": candidate.conductor,
            "dielectric": candidate.dielectric,
            "substrate": candidate.substrate
        },
        "coupling": "electrostatic_to_heat_to_thermal_stress",
        "screening_parameters": screening_parameters(candidate),
        "materialization_status": "prototype_materialized_requires_solver_rerun"
    }))
}

fn screening_parameters(candidate: &CompositePanelCandidate) -> Value {
    let interface_risk_score = assess_composite_interfaces(candidate)
        .into_iter()
        .map(|interface| interface.risk_score)
        .fold(0.0, f64::max);
    json!({
        "dielectric_breakdown_field_v_m": candidate.dielectric_breakdown_field_v_m,
        "areal_mass_kg_m2": candidate.areal_mass_kg_m2,
        "interface_risk_score": interface_risk_score
    })
}

fn required_str<'a>(value: &'a Value, key: &str) -> Result<&'a str, String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("materialized candidate is missing {key}"))
}
