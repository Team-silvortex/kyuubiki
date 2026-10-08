use kyuubiki_headless_sdk::{
    MockHeadlessExecutor, build_template_document, execute_batch_with_executor,
    material_study_descriptors, normalize_workflow_document,
};
use serde_json::{Value, json};
use std::{
    fs,
    path::PathBuf,
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

struct Scratch(PathBuf);
impl Scratch {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "kyuubiki-material-results-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn retained_run_rejections_preserve_source_and_existing_material_output() {
    let scratch = Scratch::new();
    let source = scratch.0.join("run.json");
    let out = scratch.0.join("material.json");
    for study in material_study_descriptors() {
        let batch =
            normalize_workflow_document(&build_template_document(study.template_id, None).unwrap())
                .unwrap();
        let baseline = execute_batch_with_executor(&batch, &mut MockHeadlessExecutor, false, false);
        for edit in 0..9 {
            let mut run = serde_json::to_value(&baseline).unwrap();
            match edit {
                0 => run["status"] = json!("failed"),
                1 => run["steps"][2]["status"] = json!("future-status"),
                2 => run["steps"][2]["result_preview"]["job_id"] = json!("wrong-job"),
                3 => run["steps"][0]["payload"]["research"]["candidate_id"] = json!("ghost"),
                4 => run["steps"][1]["result_preview"]["status"] = json!("solving"),
                5 => run["mode"] = json!("dry_run"),
                6 => run["executed_step_count"] = json!(8),
                7 => run["steps"][2]["result_preview"]["result"] = Value::Null,
                _ => {
                    run["schema_version"] = json!("kyuubiki.headless-execution-run/v99");
                    run["results"] = json!([{}, {}, {}]);
                }
            }
            let bytes = serde_json::to_vec_pretty(&run).unwrap();
            fs::write(&source, &bytes).unwrap();
            let retained = b"{\"retained_material\":true}";
            fs::write(&out, retained).unwrap();
            let output = Command::new(env!("CARGO_BIN_EXE_kyuubiki-material-report"))
                .args([
                    study.id,
                    "--results",
                    source.to_str().unwrap(),
                    "--out",
                    out.to_str().unwrap(),
                    "--json",
                ])
                .output()
                .unwrap();
            assert!(!output.status.success(), "study {} edit {edit}", study.id);
            assert!(output.stdout.is_empty());
            assert!(
                String::from_utf8_lossy(&output.stderr)
                    .contains("material-report run contract mismatch")
            );
            assert_eq!(fs::read(&source).unwrap(), bytes);
            assert_eq!(fs::read(&out).unwrap(), retained);
            assert_eq!(fs::read_dir(&scratch.0).unwrap().count(), 2);
        }
    }
}

#[test]
fn wrong_study_cannot_reinterpret_a_retained_execution_run() {
    let scratch = Scratch::new();
    let source = scratch.0.join("run.json");
    let batch = normalize_workflow_document(
        &build_template_document("material_heat_spreader_screening", None).unwrap(),
    )
    .unwrap();
    let run = execute_batch_with_executor(&batch, &mut MockHeadlessExecutor, false, false);
    fs::write(&source, serde_json::to_vec(&run).unwrap()).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_kyuubiki-material-report"))
        .args([
            "dielectric-screening",
            "--results",
            source.to_str().unwrap(),
            "--json",
        ])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("reads an unowned candidate job"));
}
