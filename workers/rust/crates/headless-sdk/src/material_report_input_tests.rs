use super::*;
use crate::{build_template_document, normalize_workflow_document};
use serde_json::json;

fn batch(study: &MaterialStudyDescriptor) -> HeadlessExecutionBatch {
    normalize_workflow_document(&build_template_document(study.template_id, None).unwrap()).unwrap()
}

fn model_key(study: &MaterialStudyDescriptor) -> &str {
    if study.id == "material_composite_thermo_electric_panel" {
        "heat_model"
    } else {
        "model"
    }
}

#[test]
fn fixed_material_reports_reject_transplanted_models_under_unchanged_candidate_labels() {
    for study in MATERIAL_STUDIES {
        let mut edited = batch(study);
        let key = model_key(study);
        edited.steps[0].payload[key] = edited.steps[3].payload[key].clone();
        let error = validate_material_report_compatibility(study.id, &edited)
            .expect_err("a different candidate model cannot keep the original report constants");
        assert!(error.starts_with("material-report input contract mismatch:"));
    }
}

#[test]
fn fixed_report_models_reject_geometry_boundary_thickness_and_extra_physical_inputs() {
    for study in MATERIAL_STUDIES {
        let original = batch(study);
        let key = model_key(study);
        for edit in [
            "geometry",
            "thickness",
            "boundary",
            "extra",
            "missing",
            "null",
        ] {
            let mut edited = original.clone();
            let model = &mut edited.steps[0].payload[key];
            match edit {
                "geometry" => model["nodes"][1]["x"] = json!(0.17),
                "thickness" => model["elements"][0]["thickness"] = json!(0.007),
                "boundary" => {
                    let node = model["nodes"][0].as_object_mut().unwrap();
                    let fix = node
                        .keys()
                        .find(|key| key.starts_with("fix_"))
                        .unwrap()
                        .clone();
                    node.insert(fix.clone(), json!(!node[&fix].as_bool().unwrap()));
                }
                "extra" => model["contact_interfaces"] = json!([{"conductance": 99.0}]),
                "missing" => {
                    model.as_object_mut().unwrap().remove("nodes");
                }
                _ => *model = Value::Null,
            }
            assert!(
                validate_material_report_compatibility(study.id, &edited).is_err(),
                "{} accepted {edit}",
                study.id
            );
        }
    }
}

#[test]
fn every_composite_model_and_coupling_parameter_is_part_of_the_fixed_report_profile() {
    let study = find_material_study("composite-thermo-electric-panel").unwrap();
    let original = batch(study);
    for path in [
        "/electrostatic_model/elements/1/permittivity",
        "/heat_model/elements/0/conductivity",
        "/thermal_model/elements/2/youngs_modulus",
        "/electric_conduction_model/elements/0/electrical_conductivity_s_m",
        "/electrothermal_loss/loss_tangent",
        "/electrothermal_feedback/relaxation_factor",
        "/electric_conduction_feedback/regions/0/reference_resistivity_ohm_m",
        "/thermal_expansion_feedback/regions/0/temperature_coefficient_1_k",
    ] {
        let mut edited = original.clone();
        let value = edited.steps[0].payload.pointer_mut(path).unwrap();
        *value = json!(value.as_f64().unwrap() * 0.5);
        let error = validate_material_report_compatibility(study.id, &edited).unwrap_err();
        assert!(error.contains(&format!("profile at {path}")), "{error}");
    }
    let mut injected = original.clone();
    injected.steps[0].payload["coupling_override"] = json!({"skip_heat":true});
    assert!(
        validate_material_report_compatibility(study.id, &injected)
            .unwrap_err()
            .contains("/coupling_override")
    );
}

#[test]
fn candidate_metadata_cannot_override_fixed_mass_strength_or_material_cards() {
    for study in MATERIAL_STUDIES {
        let original = batch(study);
        for field in [
            "density_kg_m3",
            "yield_strength_pa",
            "breakdown_field_v_m",
            "material_card_id",
            "materials",
        ] {
            let mut edited = original.clone();
            edited.steps[0].payload["research"][field] = json!("not-the-fixed-candidate");
            assert!(
                validate_material_report_compatibility(study.id, &edited).is_err(),
                "{} accepted {field}",
                study.id
            );
        }
    }
}

#[test]
fn consistent_si_edits_do_not_prove_the_original_fixed_dielectric_candidate() {
    let study = find_material_study("dielectric-screening").unwrap();
    let mut edited = batch(study);
    edited.steps[0].payload["research"]["relative_permittivity"] = json!(4.7);
    edited.steps[0].payload["model"]["elements"][0]["permittivity"] =
        json!(8.854_187_812_8e-12 * 4.7);
    assert!(crate::validate_batch(&edited).ok);
    assert!(
        validate_material_report_compatibility(study.id, &edited)
            .unwrap_err()
            .contains("fixed report input profile")
    );
}

#[test]
fn serialized_profiles_allow_study_aliases_annotations_and_explicit_runtime_context() {
    for study in MATERIAL_STUDIES {
        let original = batch(study);
        let mut edited: HeadlessExecutionBatch =
            serde_json::from_slice(&serde_json::to_vec(&original).unwrap()).unwrap();
        for step in edited
            .steps
            .iter_mut()
            .filter(|step| step.payload.get("research").is_some())
        {
            step.payload["project_id"] = json!("research-project");
            step.payload["model_version_id"] = json!("research-version");
            step.payload["simulation_case_id"] = json!("research-case");
            step.payload["notes"] = json!("custom display notes");
            step.payload["research"]["study"] = json!(study.aliases[0]);
            step.payload["research"]["candidate_label"] = json!("custom display label");
            step.payload["research"]["note"] = json!("timed out waiting for job is only text");
            step.payload["research"]["annotations"] = json!({"trace": "local-check"});
        }
        let before = serde_json::to_value(&edited).unwrap();
        validate_material_report_compatibility(study.aliases[0], &edited).unwrap();
        assert_eq!(serde_json::to_value(&edited).unwrap(), before);
    }
}

#[test]
fn physical_input_edits_remain_available_to_generic_sdk_workflows() {
    for study in MATERIAL_STUDIES {
        let mut edited = batch(study);
        edited.steps[0].payload[model_key(study)]["elements"][0]["thickness"] = json!(0.004);
        assert!(crate::validate_batch(&edited).ok);
        let report = crate::execute_batch_with_executor(
            &edited,
            &mut crate::MockHeadlessExecutor,
            false,
            false,
        );
        assert_eq!(report.status, "ok");
        assert_eq!(report.executed_step_count, 9);
        assert!(validate_material_report_compatibility(study.id, &edited).is_err());
    }
}

#[test]
fn constitutive_parameter_drift_is_rejected_without_relying_on_changed_model_ids() {
    for study in MATERIAL_STUDIES {
        let mut edited = batch(study);
        let key = model_key(study);
        let parameter = match study.id {
            "material_heat_spreader_screening" | "material_composite_thermo_electric_panel" => {
                "conductivity"
            }
            "material_dielectric_screening" => "permittivity",
            _ => "youngs_modulus",
        };
        let value = &mut edited.steps[0].payload[key]["elements"][0][parameter];
        *value = json!(value.as_f64().unwrap() * 1.2);
        let error = validate_material_report_compatibility(study.id, &edited).unwrap_err();
        assert!(error.contains("input contract mismatch"));
        if parameter != "permittivity" {
            assert!(
                error.contains(&format!("/{key}/elements/0/{parameter}")),
                "{error}"
            );
        }
    }
}
