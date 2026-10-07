use super::agent_support::{LiveAgent, wait_for_lifecycle};
use super::orchestra_completion::http_json;
use super::write_ack_loss::count;
use super::{
    Scratch, parse_json_output, run_headless_command, start_live_server_with_state, write_temp_json,
};
use kyuubiki_headless_sdk::{
    HeadlessExecutionBatchStep, HeadlessRisk, build_template_document, material_study_descriptors,
    normalize_workflow_document,
};
use serde_json::{Value, json};
use std::{error::Error, fs};

#[test]
fn invalid_material_candidate_plans_cannot_create_projects_or_dispatch_real_agents()
-> Result<(), Box<dyn Error>> {
    exercise_invalid_plans(false)
}

#[test]
fn changed_material_physics_cannot_create_projects_or_dispatch_real_agents()
-> Result<(), Box<dyn Error>> {
    exercise_invalid_plans(true)
}

fn exercise_invalid_plans(physical: bool) -> Result<(), Box<dyn Error>> {
    let owner = LiveAgent::start()?;
    let peer = LiveAgent::start()?;
    let data = Scratch::new()?;
    let server =
        start_live_server_with_state(Some(owner.port()), Some(peer.port()), Some(&data.0))?;
    let url = format!("http://127.0.0.1:{}", server.port);
    let projects_before = http_json(server.port, "/api/v1/projects", None)?.1;
    let jobs_before = http_json(server.port, "/api/v1/jobs", None)?.1;
    assert_eq!(jobs_before["jobs"].as_array().unwrap().len(), 0);
    for study in material_study_descriptors()
        .iter()
        .filter(|study| physical || study.id == "material_heat_spreader_screening")
    {
        let document = build_template_document(study.template_id, None)
            .ok_or("missing built-in material template")?;
        let mut original = normalize_workflow_document(&document)?;
        original.steps.insert(
            0,
            HeadlessExecutionBatchStep {
                index: 1,
                action: "project_create".into(),
                risk: HeadlessRisk::Normal,
                payload: json!({"name":"must-not-be-created-by-invalid-material-plan"}),
            },
        );
        for (position, step) in original.steps.iter_mut().enumerate() {
            step.index = position + 1;
            if matches!(step.action.as_str(), "job_wait" | "result_fetch") {
                let source = (position - 1) / 3 * 3 + 2;
                step.payload["job_id"] = json!(format!("{{{{steps.{source}.result.job_id}}}}"));
            }
        }
        let variants = if physical {
            ["model", "metadata", "geometry"]
        } else {
            ["missing", "duplicate", "identity"]
        };
        for variant in variants {
            let mut batch = original.clone();
            let key = if study.id == "material_composite_thermo_electric_panel" {
                "heat_model"
            } else {
                "model"
            };
            match variant {
                "missing" => {
                    batch.steps.pop();
                }
                "duplicate" => {
                    batch.steps[9].payload["job_id"] = json!("{{steps.2.result.job_id}}")
                }
                "model" => batch.steps[1].payload[key] = batch.steps[4].payload[key].clone(),
                "metadata" => batch.steps[1].payload["research"]["density_kg_m3"] = json!(9999.0),
                "geometry" => {
                    let x = &mut batch.steps[1].payload[key]["nodes"][1]["x"];
                    *x = json!(x.as_f64().unwrap() * 1.1);
                }
                _ => {
                    batch.steps[1].payload["research"]["candidate_id"] = json!("unknown-candidate")
                }
            }
            if variant == "model" && study.id == "material_dielectric_screening" {
                batch.steps[1].payload["research"]["relative_permittivity"] =
                    batch.steps[4].payload["research"]["relative_permittivity"].clone();
            }
            let source = write_temp_json("material-preflight", &serde_json::to_value(&batch)?);
            let source_bytes = fs::read(&*source)?;
            let root = source.parent().unwrap();
            let material_path = root.join("material.json");
            let report_path = root.join("run.json");
            let retained_material = b"{\"old_material\":true}";
            fs::write(&material_path, retained_material)?;
            let output = run_headless_command(&[
                "run",
                source.to_str().unwrap(),
                "--execute",
                "--executor",
                "service",
                "--api-base-url",
                &url,
                "--json",
                "--material-report",
                study.id,
                "--material-report-out",
                material_path.to_str().unwrap(),
                "--report-out",
                report_path.to_str().unwrap(),
            ]);
            assert!(!output.status.success(), "{}", server.logs());
            let report = parse_json_output(&output);
            assert_eq!(report["status"], "invalid", "{report}");
            assert_eq!(report["executed_step_count"], 0);
            assert_eq!(report["steps"], json!([]));
            assert_eq!(report["execution_summary"]["job_count"], 0);
            assert_eq!(
                report["execution_summary"]["failure"]["error_code"],
                "kyuubiki.headless.material_report_input_contract_mismatch"
            );
            let diagnostic: Value = serde_json::from_slice(&output.stderr)?;
            assert_eq!(
                diagnostic["error"]["code"],
                "material_report_input_contract_mismatch"
            );
            assert_eq!(diagnostic["error"]["stage"], "material_report_validation");
            assert_eq!(diagnostic["error"]["retryable"], false);
            if physical {
                assert!(
                    diagnostic["error"]["message"]
                        .as_str()
                        .unwrap()
                        .contains("fixed report input profile"),
                    "{diagnostic}"
                );
            }
            assert_eq!(
                serde_json::from_slice::<Value>(&fs::read(&report_path)?)?,
                report
            );
            assert_eq!(fs::read(&*source)?, source_bytes);
            assert_eq!(fs::read(&material_path)?, retained_material);
            assert_eq!(fs::read_dir(root)?.count(), 3);
            assert_eq!(
                http_json(server.port, "/api/v1/projects", None)?.1,
                projects_before
            );
            assert_eq!(http_json(server.port, "/api/v1/jobs", None)?.1, jobs_before);
            wait_for_lifecycle(&owner, "accepting", 0)?;
            wait_for_lifecycle(&peer, "accepting", 0)?;
            assert_eq!(
                count(&owner)? + count(&peer)?,
                0,
                "invalid material plan dispatched calculation"
            );
        }
    }
    Ok(())
}
