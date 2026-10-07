use super::{MaterialStudyDescriptor, find_material_study};
use crate::workflow_bindings::parse_binding;
use crate::{HeadlessExecutionBatch, HeadlessExecutionBatchStep, validate_batch};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

#[path = "material_report_inputs.rs"]
mod inputs;

fn mismatch(message: impl std::fmt::Display) -> String {
    format!("material-report input contract mismatch: {message}")
}

pub(super) fn validate_input_contract(
    study: &MaterialStudyDescriptor,
    batch: &HeadlessExecutionBatch,
) -> Result<(), String> {
    let (action, candidates) = expected_candidates(study.id)?;
    let fetch_count = batch
        .steps
        .iter()
        .filter(|step| step.action == "result_fetch")
        .count();
    if fetch_count != candidates.len() {
        return Err(mismatch(format!(
            "study {} requires {} ordered result_fetch steps, received {fetch_count}",
            study.id,
            candidates.len()
        )));
    }
    let validation = validate_batch(batch);
    if !validation.ok {
        return Err(mismatch(format!(
            "invalid material execution batch: {}",
            validation.issues.join("; ")
        )));
    }
    let solves = batch
        .steps
        .iter()
        .filter(|step| step.action == action)
        .collect::<Vec<_>>();
    if solves.len() != candidates.len() {
        return Err(mismatch(format!(
            "study {} requires {} candidate {action} steps, received {}",
            study.id,
            candidates.len(),
            solves.len()
        )));
    }
    let mut sources = BTreeMap::new();
    let mut identities = BTreeSet::new();
    for step in solves {
        let candidate = step
            .payload
            .pointer("/research/candidate_id")
            .and_then(Value::as_str)
            .filter(|id| candidates.contains(id))
            .ok_or_else(|| {
                mismatch(format!(
                    "step {} has a missing or unknown candidate identity",
                    step.index
                ))
            })?;
        let declared_study = step
            .payload
            .pointer("/research/study")
            .and_then(Value::as_str)
            .and_then(find_material_study);
        if declared_study.is_none_or(|declared| declared.id != study.id) {
            return Err(mismatch(format!(
                "step {} candidate {candidate} has a missing or mismatched research study",
                step.index
            )));
        }
        if !identities.insert(candidate) {
            return Err(mismatch(format!("duplicate candidate {candidate}")));
        }
        sources.insert(step.index, candidate);
    }
    if study.id == "material_dielectric_screening" {
        validate_dielectric_permittivity(batch)?;
    }
    for step in batch.steps.iter().filter(|step| step.action == action) {
        inputs::validate_fixed_inputs(study.id, step)?;
    }
    // Track only earlier owned sources; waits can forward the same job identity.
    let mut roots = BTreeMap::new();
    let mut waited = BTreeSet::new();
    let mut result_position = 0;
    for step in &batch.steps {
        if sources.contains_key(&step.index) {
            roots.insert(step.index, step.index);
        } else if step.action == "job_wait" {
            if let Some(root) = owned_job_source(step, &roots)? {
                waited.insert(root);
                roots.insert(step.index, root);
            }
        } else if step.action == "result_fetch" {
            let root = owned_job_source(step, &roots)?
                .ok_or_else(|| mismatch(format!("result_fetch step {} requires a tracked candidate source, not a literal or unrelated job", step.index)))?;
            let candidate = sources[&root];
            if candidate != candidates[result_position] {
                return Err(mismatch(format!(
                    "result_fetch step {} violates candidate order: expected {}, received {candidate}",
                    step.index, candidates[result_position]
                )));
            }
            if !waited.contains(&root) {
                return Err(mismatch(format!(
                    "result_fetch step {} candidate {candidate} requires a preceding job_wait for its source",
                    step.index
                )));
            }
            result_position += 1;
        }
    }
    Ok(())
}

fn owned_job_source(
    step: &HeadlessExecutionBatchStep,
    roots: &BTreeMap<usize, usize>,
) -> Result<Option<usize>, String> {
    let job = match (step.payload.get("job_id"), step.payload.get("jobId")) {
        (Some(primary), Some(alias)) if primary != alias => {
            return Err(mismatch(format!(
                "step {} has contradictory job_id aliases",
                step.index
            )));
        }
        (Some(value), _) | (_, Some(value)) => value,
        _ => return Ok(None),
    };
    Ok(job
        .as_str()
        .and_then(parse_binding)
        .filter(|(_, field)| *field == "job_id")
        .and_then(|(source, _)| roots.get(&source).copied()))
}

fn expected_candidates(study: &str) -> Result<(&'static str, Vec<&'static str>), String> {
    Ok(match study {
        "material_heat_spreader_screening" => (
            "solve_heat_plane_quad_2d",
            crate::heat_spreader_screening_candidates()
                .into_iter()
                .map(|candidate| candidate.id)
                .collect(),
        ),
        "material_dielectric_screening" => (
            "solve_electrostatic_plane_quad_2d",
            crate::dielectric_screening_candidates()
                .into_iter()
                .map(|candidate| candidate.id)
                .collect(),
        ),
        "material_thermo_shield_screening" => (
            "solve_thermal_plane_quad_2d",
            crate::thermo_shield_screening_candidates()
                .into_iter()
                .map(|candidate| candidate.id)
                .collect(),
        ),
        "material_structural_panel_screening" => (
            "solve_plane_quad_2d",
            crate::structural_panel_screening_candidates()
                .into_iter()
                .map(|candidate| candidate.id)
                .collect(),
        ),
        "material_composite_thermo_electric_panel" => (
            "solve_composite_thermo_electric_panel",
            crate::composite_panel_candidates()
                .into_iter()
                .map(|candidate| candidate.id)
                .collect(),
        ),
        _ => return Err(mismatch("study has no candidate source contract")),
    })
}

fn validate_dielectric_permittivity(batch: &HeadlessExecutionBatch) -> Result<(), String> {
    const VACUUM_PERMITTIVITY_F_M: f64 = 8.854_187_812_8e-12;
    for step in batch
        .steps
        .iter()
        .filter(|step| step.action == "solve_electrostatic_plane_quad_2d")
    {
        let candidate = step
            .payload
            .pointer("/research/candidate_id")
            .and_then(Value::as_str)
            .unwrap_or("unknown");
        let relative = step.payload.pointer("/research/relative_permittivity").and_then(Value::as_f64)
            .filter(|value| value.is_finite() && *value > 0.0)
            .ok_or_else(|| mismatch(format!("dielectric candidate {candidate} requires a finite positive research.relative_permittivity")))?;
        let elements = step
            .payload
            .pointer("/model/elements")
            .and_then(Value::as_array)
            .filter(|elements| !elements.is_empty())
            .ok_or_else(|| {
                mismatch(format!(
                    "dielectric candidate {candidate} requires non-empty model.elements"
                ))
            })?;
        let expected = VACUUM_PERMITTIVITY_F_M * relative;
        if !expected.is_finite() || expected <= 0.0 {
            return Err(mismatch(format!(
                "dielectric candidate {candidate} relative permittivity is outside the representable SI range"
            )));
        }
        for (index, element) in elements.iter().enumerate() {
            let actual = element.get("permittivity").and_then(Value::as_f64)
                .filter(|value| value.is_finite() && *value > 0.0)
                .ok_or_else(|| mismatch(format!("dielectric candidate {candidate} element {index} requires a finite positive SI permittivity")))?;
            if (actual - expected).abs() / expected > 1.0e-9 {
                return Err(mismatch(format!(
                    "dielectric candidate {candidate} element {index} has permittivity={actual:e} F/m, expected {expected:e} F/m from research.relative_permittivity={relative}; update both fields together"
                )));
            }
        }
    }
    Ok(())
}
