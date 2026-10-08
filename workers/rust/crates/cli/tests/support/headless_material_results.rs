use super::agent_support::{LiveAgent, wait_for_lifecycle};
use super::orchestra_completion::http_json;
use super::write_ack_loss::count;
use super::{Scratch, start_live_server_with_state};
use kyuubiki_headless_sdk::{
    ServiceHeadlessExecutor, build_material_report_from_run, build_template_document,
    execute_batch_with_executor, material_study_descriptors, normalize_workflow_document,
    validate_material_report_compatibility,
};
use serde_json::{Value, json};
use std::{error::Error, fs, process::Command};

#[test]
fn five_real_material_studies_regenerate_reports_without_replaying_agent_calculations()
-> Result<(), Box<dyn Error>> {
    let owner = LiveAgent::start()?;
    let peer = LiveAgent::start()?;
    let data = Scratch::new()?;
    let server =
        start_live_server_with_state(Some(owner.port()), Some(peer.port()), Some(&data.0))?;
    let url = format!("http://127.0.0.1:{}", server.port);
    let artifacts = Scratch::new()?;
    let source = artifacts.0.join("run.json");
    let out = artifacts.0.join("material.json");
    let mut retained = Vec::new();
    for study in material_study_descriptors() {
        let mut batch = normalize_workflow_document(
            &build_template_document(study.template_id, None).unwrap(),
        )?;
        for step in batch
            .steps
            .iter_mut()
            .filter(|step| step.action == "job_wait")
        {
            step.payload["interval_ms"] = json!(10);
            step.payload["timeout_ms"] = json!(10_000);
        }
        validate_material_report_compatibility(study.id, &batch)?;
        let mut executor = ServiceHeadlessExecutor::new(&url);
        let run = execute_batch_with_executor(&batch, &mut executor, false, false);
        assert_eq!(
            run.status,
            "ok",
            "study {}: {run:?}; {}",
            study.id,
            server.logs()
        );
        assert_eq!(run.mode, "execute:service");
        let report = build_material_report_from_run(study.id, &run)?;
        assert_eq!(report["candidates"].as_array().unwrap().len(), 3);
        retained.push((study.id, serde_json::to_value(run)?, report));
    }
    wait_for_lifecycle(&owner, "accepting", 0)?;
    wait_for_lifecycle(&peer, "accepting", 0)?;
    let calculations = count(&owner)? + count(&peer)?;
    assert!(
        calculations > 0,
        "fixture must include real Agent computation"
    );
    let jobs_before = http_json(server.port, "/api/v1/jobs", None)?.1;
    let projects_before = http_json(server.port, "/api/v1/projects", None)?.1;
    for (study, original, expected) in retained {
        fs::write(&source, serde_json::to_vec_pretty(&original)?)?;
        let bytes = fs::read(&source)?;
        let result = Command::new(env!("CARGO_BIN_EXE_kyuubiki-material-report"))
            .args([
                study,
                "--results",
                source.to_str().unwrap(),
                "--out",
                out.to_str().unwrap(),
                "--json",
            ])
            .output()?;
        assert!(
            result.status.success(),
            "study {study}: {}",
            String::from_utf8_lossy(&result.stderr)
        );
        assert_eq!(serde_json::from_slice::<Value>(&result.stdout)?, expected);
        assert_eq!(serde_json::from_slice::<Value>(&fs::read(&out)?)?, expected);
        assert_eq!(fs::read(&source)?, bytes);
        for edit in 0..5 {
            let mut corrupt = original.clone();
            match edit {
                0 => corrupt["status"] = json!("failed"),
                1 => {
                    corrupt["steps"][2]["result_preview"]["job_id"] =
                        json!("different-retained-job")
                }
                2 => corrupt["steps"][0]["payload"]["research"]["candidate_id"] = json!("ghost"),
                3 => corrupt["steps"][1]["result_preview"]["status"] = json!("solving"),
                _ => {
                    corrupt["steps"][2]["result_preview"]["result"]["research"] =
                        json!({"candidate_id":"ghost"})
                }
            }
            fs::write(&source, serde_json::to_vec_pretty(&corrupt)?)?;
            let source_bytes = fs::read(&source)?;
            let old_material = fs::read(&out)?;
            let rejected = Command::new(env!("CARGO_BIN_EXE_kyuubiki-material-report"))
                .args([
                    study,
                    "--results",
                    source.to_str().unwrap(),
                    "--out",
                    out.to_str().unwrap(),
                    "--json",
                ])
                .output()?;
            assert!(!rejected.status.success(), "study {study} edit {edit}");
            assert!(rejected.stdout.is_empty());
            assert!(
                String::from_utf8_lossy(&rejected.stderr)
                    .contains("material-report run contract mismatch")
            );
            assert_eq!(fs::read(&source)?, source_bytes);
            assert_eq!(fs::read(&out)?, old_material);
        }
    }
    assert_eq!(fs::read_dir(&artifacts.0)?.count(), 2);
    assert_eq!(http_json(server.port, "/api/v1/jobs", None)?.1, jobs_before);
    assert_eq!(
        http_json(server.port, "/api/v1/projects", None)?.1,
        projects_before
    );
    assert_eq!(
        count(&owner)? + count(&peer)?,
        calculations,
        "report reconstruction replayed computation"
    );
    Ok(())
}
