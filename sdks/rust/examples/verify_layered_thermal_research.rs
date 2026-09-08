//! Read-only acceptance of retained jobs, including after an externally controlled restart.
#[path = "layered_thermal_research/checks.rs"]
mod checks;
#[path = "layered_thermal_research/model.rs"]
mod model;
#[path = "layered_thermal_research/patch.rs"]
mod patch;
#[path = "layered_thermal_research/retained.rs"]
mod retained;
#[path = "layered_thermal_research/suite.rs"]
mod suite;

use kyuubiki_headless_sdk::{
    ControlPlaneClient, KyuubikiAuth, validate_workflow_result_against_graph,
};
use serde_json::{Value, json};
use std::{env, fs, path::Path};

fn verify(client: &ControlPlaneClient, item: &retained::RetainedCase) -> Result<Value, String> {
    let job = client.fetch_job(&item.job_id).map_err(|e| e.to_string())?;
    if job["job"]["job_id"] != item.job_id || job["job"]["status"] != "completed" {
        return Err("retained job identity or completed status changed".into());
    }
    let result = client
        .fetch_result(&item.job_id)
        .map_err(|e| e.to_string())?;
    if result != item.result {
        return Err("retrieved result differs from retained evidence".into());
    }
    let graph = serde_json::from_value(item.case.workflow().0).map_err(|e| e.to_string())?;
    validate_workflow_result_against_graph(&graph, &result).map_err(|e| e.to_string())?;
    let mut row = item.case.validate(&result)?;
    row["job_id"] = json!(item.job_id);
    row["result_readback_matches"] = json!(true);
    row["output_manifest_validated"] = json!(true);
    Ok(row)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = env::args().skip(1);
    let baseline = args.next().ok_or(
        "usage: verify_layered_thermal_research <baseline-directory> <new-output-directory>",
    )?;
    let output = args.next().ok_or("missing new output directory")?;
    if args.next().is_some() {
        return Err("unexpected extra argument".into());
    }
    // Validate local provenance before contacting any service or creating output.
    let baseline = retained::load(Path::new(&baseline))?;
    let root = Path::new(&output);
    fs::create_dir(root)?;
    let auth = env::var("KYUUBIKI_ACCESS_TOKEN")
        .ok()
        .map(KyuubikiAuth::access_token);
    let client = ControlPlaneClient::new_with_auth(
        &env::var("KYUUBIKI_BASE_URL").unwrap_or_else(|_| "http://127.0.0.1:4000".into()),
        auth,
    )?;
    let mut report = json!({
        "schema_version":"kyuubiki.thermal-research-readback/v1",
        "execution":"official_rust_headless_sdk_read_only", "complete":false,
        "qualification":"synthetic_reference_not_material_certification",
        "restart_proof":"requires_separate_process_lifecycle_evidence",
        "baseline_report_sha256":baseline.report_sha256,
        "expected_case_count":baseline.cases.len(), "cases":[]
    });
    fs::write(
        root.join("report.json"),
        serde_json::to_vec_pretty(&report)?,
    )?;
    for item in baseline.cases {
        let row = verify(&client, &item).unwrap_or_else(|error| {
            json!({
                "case":item.case.id(), "job_id":item.job_id, "passed":false, "error":error
            })
        });
        eprintln!(
            "{}: {}",
            item.case.id(),
            if row["passed"] == true {
                "pass"
            } else {
                "FAIL"
            }
        );
        report["cases"].as_array_mut().unwrap().push(row);
        fs::write(
            root.join("report.json"),
            serde_json::to_vec_pretty(&report)?,
        )?;
    }
    let rows = report["cases"].as_array().unwrap();
    let failed = rows.iter().filter(|row| row["passed"] != true).count();
    report["complete"] = json!(true);
    report["failed_count"] = json!(failed);
    fs::write(
        root.join("report.json"),
        serde_json::to_vec_pretty(&report)?,
    )?;
    if failed > 0 {
        return Err("retained research readback failed; evidence retained".into());
    }
    println!("all retained cases passed without submitting any new jobs");
    Ok(())
}
