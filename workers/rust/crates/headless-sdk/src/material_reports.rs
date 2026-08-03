use crate::{
    HeadlessExecutionBatch, HeadlessRunReport, MaterialOptimizationProfile,
    MaterialResearchMetricSpec, build_composite_panel_report, build_dielectric_screening_report,
    build_dielectric_screening_report_with_optimization, build_heat_spreader_screening_report,
    build_heat_spreader_screening_report_with_optimization,
    build_structural_panel_screening_report,
    build_structural_panel_screening_report_with_optimization,
    build_thermo_shield_screening_report, build_thermo_shield_screening_report_with_optimization,
};
use serde::Serialize;
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MaterialStudyDescriptor {
    pub id: &'static str,
    pub title: &'static str,
    pub domain: &'static str,
    pub objective: &'static str,
    pub aliases: &'static [&'static str],
    pub schema_version: &'static str,
    pub template_id: &'static str,
    pub material_card_contract_required: bool,
    pub material_card_schema_version: &'static str,
    pub material_card_ref_count: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct MaterialStudyCatalogEntry {
    pub id: String,
    pub title: String,
    pub domain: String,
    pub objective: String,
    pub aliases: Vec<String>,
    pub schema_version: String,
    pub template_id: String,
    pub material_card_contract_required: bool,
    pub material_card_schema_version: String,
    pub material_card_ref_count: usize,
    pub metric_specs: Vec<MaterialResearchMetricSpec>,
}

const MATERIAL_STUDIES: &[MaterialStudyDescriptor] = &[
    MaterialStudyDescriptor {
        id: "material_heat_spreader_screening",
        title: "Heat Spreader Material Screening",
        domain: "thermal",
        objective: "rank heat spreader candidates by peak temperature, mass, and conductivity efficiency",
        aliases: &[
            "heat-spreader",
            "heat_spreader",
            "material.heat_spreader_screening.v1",
        ],
        schema_version: "kyuubiki.material-research-report/v1",
        template_id: "material_heat_spreader_screening",
        material_card_contract_required: true,
        material_card_schema_version: "kyuubiki.material-card/v1",
        material_card_ref_count: 3,
    },
    MaterialStudyDescriptor {
        id: "material_dielectric_screening",
        title: "Dielectric Material Screening",
        domain: "electromagnetic",
        objective: "rank dielectric candidates by electric-field margin, low loss, and low mass",
        aliases: &[
            "dielectric-screening",
            "dielectric_screening",
            "material.dielectric_screening.v1",
        ],
        schema_version: "kyuubiki.dielectric-material-report/v1",
        template_id: "material_dielectric_screening",
        material_card_contract_required: true,
        material_card_schema_version: "kyuubiki.material-card/v1",
        material_card_ref_count: 3,
    },
    MaterialStudyDescriptor {
        id: "material_thermo_shield_screening",
        title: "Thermo-Mechanical Shield Screening",
        domain: "thermo_mechanical",
        objective: "rank thermo-mechanical shield candidates by stress, displacement, thermal expansion, and mass",
        aliases: &[
            "thermo-shield",
            "thermo_shield",
            "material.thermo_shield_screening.v1",
        ],
        schema_version: "kyuubiki.thermo-material-report/v1",
        template_id: "material_thermo_shield_screening",
        material_card_contract_required: true,
        material_card_schema_version: "kyuubiki.material-card/v1",
        material_card_ref_count: 3,
    },
    MaterialStudyDescriptor {
        id: "material_structural_panel_screening",
        title: "Structural Panel Material Screening",
        domain: "structural",
        objective: "rank structural panel candidates by stress, deflection, mass, stiffness, and yield margin",
        aliases: &[
            "structural-panel",
            "structural_panel",
            "material.structural_panel_screening.v1",
        ],
        schema_version: "kyuubiki.structural-material-report/v1",
        template_id: "material_structural_panel_screening",
        material_card_contract_required: true,
        material_card_schema_version: "kyuubiki.material-card/v1",
        material_card_ref_count: 3,
    },
    MaterialStudyDescriptor {
        id: "material_composite_thermo_electric_panel",
        title: "Composite Thermo-Electric Panel",
        domain: "multiphysics_materials",
        objective: "rank mixed-material panel stacks across electric field, heat, thermal stress, and mass",
        aliases: &[
            "composite-thermo-electric-panel",
            "composite_thermo_electric_panel",
            "material.composite_thermo_electric_panel.v1",
        ],
        schema_version: "kyuubiki.composite-panel-report/v1",
        template_id: "material_composite_thermo_electric_panel_screening",
        material_card_contract_required: true,
        material_card_schema_version: "kyuubiki.material-card/v1",
        material_card_ref_count: 3,
    },
];

pub fn material_study_descriptors() -> &'static [MaterialStudyDescriptor] {
    MATERIAL_STUDIES
}

pub fn find_material_study(study: &str) -> Option<&'static MaterialStudyDescriptor> {
    let normalized = normalize_study_key(study);
    MATERIAL_STUDIES.iter().find(|descriptor| {
        normalize_study_key(descriptor.id) == normalized
            || descriptor
                .aliases
                .iter()
                .any(|alias| normalize_study_key(alias) == normalized)
    })
}

pub fn material_study_catalog() -> Vec<MaterialStudyCatalogEntry> {
    MATERIAL_STUDIES
        .iter()
        .map(|descriptor| MaterialStudyCatalogEntry::from_descriptor(descriptor))
        .collect()
}

pub fn describe_material_study(study: &str) -> Option<MaterialStudyCatalogEntry> {
    find_material_study(study).map(MaterialStudyCatalogEntry::from_descriptor)
}

pub fn supported_material_report_study_ids(template_id: &str) -> Vec<&'static str> {
    MATERIAL_STUDIES
        .iter()
        .filter(|study| study.template_id == template_id)
        .map(|study| study.id)
        .collect()
}

pub fn validate_material_report_compatibility(
    study: &str,
    batch: &HeadlessExecutionBatch,
) -> Result<(), String> {
    let descriptor = find_material_study(study)
        .ok_or_else(|| format!("unsupported material report study: {study}"))?;
    let template_id = batch
        .template_id
        .as_deref()
        .or_else(|| batch.workflow_id.strip_prefix("template."))
        .ok_or_else(|| {
            format!(
                "material-report study {} requires template provenance; workflow {} does not declare template_id",
                descriptor.id, batch.workflow_id
            )
        })?;
    if template_id == descriptor.template_id {
        return Ok(());
    }
    let supported = MATERIAL_STUDIES
        .iter()
        .find(|candidate| candidate.template_id == template_id)
        .map(|candidate| candidate.id)
        .unwrap_or("none");
    Err(format!(
        "material-report study {} is not supported by template {}; supported material-report study: {}",
        descriptor.id, template_id, supported
    ))
}

pub fn build_material_report(study: &str, result_payloads: &[Value]) -> Result<Value, String> {
    build_material_report_with_optimization(study, result_payloads, None)
}

pub fn build_material_report_with_optimization(
    study: &str,
    result_payloads: &[Value],
    optimization: Option<MaterialOptimizationProfile>,
) -> Result<Value, String> {
    let descriptor = find_material_study(study)
        .ok_or_else(|| format!("unsupported material report study: {study}"))?;
    match descriptor.id {
        "material_heat_spreader_screening" => to_value(match optimization {
            Some(profile) => {
                build_heat_spreader_screening_report_with_optimization(result_payloads, profile)
            }
            None => build_heat_spreader_screening_report(result_payloads),
        }),
        "material_dielectric_screening" => to_value(match optimization {
            Some(profile) => {
                build_dielectric_screening_report_with_optimization(result_payloads, profile)
            }
            None => build_dielectric_screening_report(result_payloads),
        }),
        "material_thermo_shield_screening" => to_value(match optimization {
            Some(profile) => {
                build_thermo_shield_screening_report_with_optimization(result_payloads, profile)
            }
            None => build_thermo_shield_screening_report(result_payloads),
        }),
        "material_structural_panel_screening" => to_value(match optimization {
            Some(profile) => {
                build_structural_panel_screening_report_with_optimization(result_payloads, profile)
            }
            None => build_structural_panel_screening_report(result_payloads),
        }),
        "material_composite_thermo_electric_panel" => {
            if optimization.is_some() {
                return Err(
                    "composite thermo-electric panel does not yet accept custom optimization profiles"
                        .to_string(),
                );
            }
            to_value(build_composite_panel_report(result_payloads))
        }
        other => Err(format!("unsupported material report study: {other}")),
    }
}

pub fn build_material_report_from_run(
    study: &str,
    report: &HeadlessRunReport,
) -> Result<Value, String> {
    let payloads = extract_result_payloads_from_run(report)?;
    build_material_report(study, &payloads)
}

pub fn extract_material_result_payloads(payload: &Value) -> Result<Vec<Value>, String> {
    if let Some(array) = payload.as_array() {
        return Ok(array.clone());
    }
    if payload.get("schema_version").and_then(Value::as_str)
        == Some("kyuubiki.headless-execution-run/v1")
    {
        return extract_result_payloads_from_run_value(payload);
    }
    for key in ["results", "result_payloads"] {
        if let Some(array) = payload.get(key).and_then(Value::as_array) {
            return Ok(array.clone());
        }
    }
    Err("material report input must be an array, include a results array, or be a headless execution run report".to_string())
}

pub fn extract_result_payloads_from_run(report: &HeadlessRunReport) -> Result<Vec<Value>, String> {
    let results = report
        .steps
        .iter()
        .filter(|step| {
            step.action == "result_fetch" && step.status != "blocked" && step.status != "failed"
        })
        .filter_map(|step| {
            step.result_preview
                .get("result")
                .cloned()
                .or_else(|| Some(step.result_preview.clone()))
        })
        .collect::<Vec<_>>();
    require_non_empty_results(results, typed_run_failure_context(report))
}

fn extract_result_payloads_from_run_value(payload: &Value) -> Result<Vec<Value>, String> {
    let results = payload
        .get("steps")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|step| {
            step.get("action").and_then(Value::as_str) == Some("result_fetch")
                && step.get("status").and_then(Value::as_str) != Some("blocked")
                && step.get("status").and_then(Value::as_str) != Some("failed")
        })
        .filter_map(|step| {
            let preview = step.get("result_preview")?;
            preview
                .get("result")
                .cloned()
                .or_else(|| Some(preview.clone()))
        })
        .collect::<Vec<_>>();
    require_non_empty_results(results, value_run_failure_context(payload))
}

fn require_non_empty_results(
    results: Vec<Value>,
    failure_context: Option<String>,
) -> Result<Vec<Value>, String> {
    if results.is_empty() {
        let message =
            "headless execution run report does not contain successful result_fetch payloads";
        return Err(match failure_context {
            Some(context) => format!("{message}; {context}"),
            None => message.to_string(),
        });
    }
    Ok(results)
}

fn typed_run_failure_context(report: &HeadlessRunReport) -> Option<String> {
    if report.status != "failed" {
        return None;
    }
    report
        .steps
        .iter()
        .find(|step| step.status == "failed")
        .map(|step| format_failed_step(step.index, &step.action, &step.result_preview))
        .or_else(|| Some("run status=failed without a failed step report".to_string()))
}

fn value_run_failure_context(report: &Value) -> Option<String> {
    if report.get("status").and_then(Value::as_str) != Some("failed") {
        return None;
    }
    report
        .get("steps")
        .and_then(Value::as_array)
        .and_then(|steps| {
            steps
                .iter()
                .find(|step| step.get("status").and_then(Value::as_str) == Some("failed"))
        })
        .map(|step| {
            format_failed_step(
                step.get("index")
                    .and_then(Value::as_u64)
                    .unwrap_or_default() as usize,
                step.get("action")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown"),
                step.get("result_preview").unwrap_or(&Value::Null),
            )
        })
        .or_else(|| Some("run status=failed without a failed step report".to_string()))
}

fn format_failed_step(index: usize, action: &str, result_preview: &Value) -> String {
    match result_preview.get("error").and_then(Value::as_str) {
        Some(error) => format!("run failed at step {index} ({action}): {error}"),
        None => format!("run failed at step {index} ({action})"),
    }
}

fn to_value<T: Serialize>(report: Result<T, String>) -> Result<Value, String> {
    serde_json::to_value(report?).map_err(|error| error.to_string())
}

fn normalize_study_key(study: &str) -> String {
    study.trim().replace(['-', '.'], "_").to_lowercase()
}

impl MaterialStudyCatalogEntry {
    fn from_descriptor(descriptor: &MaterialStudyDescriptor) -> Self {
        Self {
            id: descriptor.id.to_string(),
            title: descriptor.title.to_string(),
            domain: descriptor.domain.to_string(),
            objective: descriptor.objective.to_string(),
            aliases: descriptor
                .aliases
                .iter()
                .map(|alias| (*alias).to_string())
                .collect(),
            schema_version: descriptor.schema_version.to_string(),
            template_id: descriptor.template_id.to_string(),
            material_card_contract_required: descriptor.material_card_contract_required,
            material_card_schema_version: descriptor.material_card_schema_version.to_string(),
            material_card_ref_count: descriptor.material_card_ref_count,
            metric_specs: material_study_metric_specs(descriptor.id),
        }
    }
}

fn material_study_metric_specs(study_id: &str) -> Vec<MaterialResearchMetricSpec> {
    match study_id {
        "material_heat_spreader_screening" => {
            crate::material_research::heat_spreader_screening_metric_specs()
        }
        "material_dielectric_screening" => {
            crate::material_dielectric::dielectric_screening_metric_specs()
        }
        "material_thermo_shield_screening" => crate::material_thermo::thermo_shield_metric_specs(),
        "material_structural_panel_screening" => {
            crate::material_structural::structural_panel_metric_specs()
        }
        "material_composite_thermo_electric_panel" => {
            crate::material_composite::composite_panel_metric_specs()
        }
        _ => vec![],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{HeadlessExecutionStepReport, HeadlessRisk};
    use serde_json::json;

    #[test]
    fn material_report_dispatches_dielectric_alias() {
        let report = build_material_report(
            "dielectric-screening",
            &[
                json!({ "result": { "max_electric_field": 42.0e6, "max_flux_density": 1.2e-3 } }),
                json!({ "result": { "max_electric_field": 38.0e6, "max_flux_density": 3.3e-3 } }),
                json!({ "max_electric_field": 48.0e6, "max_flux_density": 0.9e-3 }),
            ],
        )
        .expect("dielectric report should build");

        assert_eq!(
            report["schema_version"].as_str(),
            Some("kyuubiki.dielectric-material-report/v1")
        );
        assert_eq!(
            report["winner_candidate_id"].as_str(),
            Some("polyimide_film")
        );
    }

    #[test]
    fn material_study_catalog_exposes_machine_readable_contracts() {
        let catalog = material_study_catalog();

        assert_eq!(catalog.len(), 5);
        assert!(catalog.iter().all(|study| !study.metric_specs.is_empty()));
        assert!(catalog.iter().all(|study| {
            study.material_card_contract_required
                && study.material_card_schema_version == "kyuubiki.material-card/v1"
                && study.material_card_ref_count == 3
        }));
        assert!(catalog.iter().any(|study| {
            study.id == "material_dielectric_screening"
                && study.domain == "electromagnetic"
                && study.template_id == "material_dielectric_screening"
                && study
                    .aliases
                    .iter()
                    .any(|alias| alias == "dielectric-screening")
        }));
    }

    #[test]
    fn describe_material_study_resolves_aliases() {
        let description = describe_material_study("structural-panel").expect("study description");

        assert_eq!(description.id, "material_structural_panel_screening");
        assert_eq!(description.domain, "structural");
        assert!(
            description
                .metric_specs
                .iter()
                .any(|metric| metric.id == "yield_safety_factor")
        );
    }

    #[test]
    fn material_report_compatibility_uses_template_provenance() {
        let dielectric = crate::build_template_document("material_dielectric_screening", None)
            .and_then(|document| crate::normalize_workflow_document(&document).ok())
            .expect("dielectric batch");
        let direct = crate::build_template_document("direct_plane_quad", None)
            .and_then(|document| crate::normalize_workflow_document(&document).ok())
            .expect("direct batch");

        validate_material_report_compatibility("dielectric-screening", &dielectric)
            .expect("matching template should support the report");
        let error = validate_material_report_compatibility("dielectric-screening", &direct)
            .expect_err("direct template must reject dielectric report");
        assert!(error.contains("not supported by template direct_plane_quad"));
        assert!(error.contains("supported material-report study: none"));
    }

    #[test]
    fn material_report_compatibility_accepts_legacy_template_workflow_id() {
        let mut batch = crate::build_template_document("material_dielectric_screening", None)
            .and_then(|document| crate::normalize_workflow_document(&document).ok())
            .expect("dielectric batch");
        batch.template_id = None;

        validate_material_report_compatibility("dielectric-screening", &batch)
            .expect("legacy template workflow id should remain compatible");
    }

    #[test]
    fn material_report_compatibility_covers_every_template_and_study() {
        for template in crate::list_templates() {
            let batch = crate::build_template_document(template.id, None)
                .and_then(|document| crate::normalize_workflow_document(&document).ok())
                .unwrap_or_else(|| panic!("template {} should normalize", template.id));
            for study in MATERIAL_STUDIES {
                let result = validate_material_report_compatibility(study.id, &batch);
                assert_eq!(
                    result.is_ok(),
                    template.id == study.template_id,
                    "template {} and study {} returned {:?}",
                    template.id,
                    study.id,
                    result
                );
            }
        }
    }

    #[test]
    fn material_report_capability_lookup_is_explicit() {
        assert_eq!(
            supported_material_report_study_ids("material_dielectric_screening"),
            vec!["material_dielectric_screening"]
        );
        assert!(supported_material_report_study_ids("direct_plane_quad").is_empty());
    }

    #[test]
    fn material_report_extracts_successful_result_fetch_steps() {
        let run = HeadlessRunReport {
            schema_version: "kyuubiki.headless-execution-run/v1".to_string(),
            workflow_id: "template.material_dielectric_screening".to_string(),
            mode: "execute:mock".to_string(),
            status: "ok".to_string(),
            executed_step_count: 3,
            warning_count: 0,
            blocked_by_confirmation: None,
            validation: crate::HeadlessValidationReport {
                ok: true,
                issue_count: 0,
                issues: vec![],
                warning_count: 0,
                warnings: vec![],
                summary: None,
                policy: None,
            },
            steps: vec![HeadlessExecutionStepReport {
                index: 3,
                action: "result_fetch".to_string(),
                risk: HeadlessRisk::Normal,
                status: "executed".to_string(),
                payload: json!({ "job_id": "job-1" }),
                result_preview: json!({ "result": { "max_electric_field": 42.0e6 } }),
                requires_confirmation: false,
            }],
        };

        let payloads = extract_result_payloads_from_run(&run).expect("payloads");
        assert_eq!(payloads.len(), 1);
        assert_eq!(payloads[0]["max_electric_field"].as_f64(), Some(42.0e6));
    }

    #[test]
    fn material_report_preserves_failed_execution_context() {
        let run = HeadlessRunReport {
            schema_version: "kyuubiki.headless-execution-run/v1".to_string(),
            workflow_id: "workflow.failed-service".to_string(),
            mode: "execute:service".to_string(),
            status: "failed".to_string(),
            executed_step_count: 0,
            warning_count: 0,
            blocked_by_confirmation: None,
            validation: crate::HeadlessValidationReport {
                ok: true,
                issue_count: 0,
                issues: vec![],
                warning_count: 0,
                warnings: vec![],
                summary: None,
                policy: None,
            },
            steps: vec![HeadlessExecutionStepReport {
                index: 1,
                action: "service_health".to_string(),
                risk: HeadlessRisk::Normal,
                status: "failed".to_string(),
                payload: json!({}),
                result_preview: json!({
                    "error": "failed to connect to 127.0.0.1:19999: connection refused"
                }),
                requires_confirmation: false,
            }],
        };

        let error = extract_result_payloads_from_run(&run)
            .expect_err("failed execution should not produce material payloads");
        assert!(error.contains("step 1 (service_health)"));
        assert!(error.contains("failed to connect"));
    }
}
