//! A bounded, real-service study. A completed job is not a numerical pass.
#[path = "layered_thermal_research/checks.rs"]
mod checks;
#[path = "layered_thermal_research/model.rs"]
mod model;

use kyuubiki_headless_sdk::{
    ControlPlaneClient, KyuubikiAgentClient, KyuubikiAuth, KyuubikiSession,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{env, fs, path::Path, time::Duration};

fn retain(root: &Path, name: &str, value: &Value) -> Result<String, Box<dyn std::error::Error>> {
    let bytes = serde_json::to_vec_pretty(value)?;
    fs::write(root.join(name), &bytes)?;
    Ok(format!("{:x}", Sha256::digest(&bytes)))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out = env::args()
        .nth(1)
        .ok_or("usage: layered_thermal_research <new-output-directory>")?;
    let root = Path::new(&out);
    // Do not overwrite a previous research round, including a failed one.
    fs::create_dir(root)?;
    let base_url = env::var("KYUUBIKI_BASE_URL").unwrap_or_else(|_| "http://127.0.0.1:4000".into());
    let auth = env::var("KYUUBIKI_ACCESS_TOKEN")
        .ok()
        .map(KyuubikiAuth::access_token);
    let client = ControlPlaneClient::new_with_auth(&base_url, auth.clone())?;
    let session = KyuubikiSession::from_control_plane_with_auth(&base_url, auth)?;
    let agent = KyuubikiAgentClient::new(session);
    let health = client.health()?;
    let definition_digest = format!(
        "{:x}",
        Sha256::digest(
            concat!(
                include_str!("layered_thermal_research.rs"),
                include_str!("layered_thermal_research/model.rs"),
                include_str!("layered_thermal_research/checks.rs"),
            )
            .as_bytes()
        )
    );
    let mut rows = vec![];
    for case in model::cases() {
        let (graph, inputs) = case.workflow();
        let request_name = format!("{}-request.json", case.id());
        let request_digest = retain(
            root,
            &request_name,
            &json!({"graph": graph, "input_artifacts": inputs}),
        )?;
        let outcome = agent.run_workflow_graph(
            &graph,
            &inputs,
            Duration::from_millis(200),
            Duration::from_secs(120),
            true,
        );
        let row = match outcome {
            Ok(outcome) => {
                let terminal = outcome.terminal;
                let result = outcome.result.unwrap_or(Value::Null);
                let result_name = format!("{}-result.json", case.id());
                let digest = retain(root, &result_name, &result)?;
                let mut row = checks::validate(case, &result).unwrap_or_else(
                    |error| json!({"case": case.id(), "passed": false, "error": error}),
                );
                let job = &terminal["job"];
                row["terminal"] = json!({"job_id": job["job_id"], "status": job["status"]});
                let readback = job["job_id"]
                    .as_str()
                    .ok_or("terminal job identity missing")
                    .map_err(|error| error.to_string())
                    .and_then(|id| client.fetch_result(id).map_err(|error| error.to_string()));
                let readback_matches = readback.as_ref().is_ok_and(|value| value == &result);
                row["result_readback_matches"] = json!(readback_matches);
                if let Err(error) = readback {
                    row["readback_error"] = json!(error);
                }
                if job["status"] != "completed"
                    || !readback_matches
                    || outcome.validated_outputs.is_none()
                {
                    row["passed"] = json!(false);
                }
                row["result_file"] = json!(result_name);
                row["result_sha256"] = json!(digest);
                row["output_manifest_validated"] = json!(outcome.validated_outputs.is_some());
                row
            }
            Err(error) => json!({"case": case.id(), "passed": false, "error": error.to_string()}),
        };
        eprintln!(
            "{}: {}",
            case.id(),
            if row["passed"] == true {
                "pass"
            } else {
                "FAIL"
            }
        );
        let mut row = row;
        row["request_file"] = json!(request_name);
        row["request_sha256"] = json!(request_digest);
        rows.push(row);
        retain(
            root,
            "report.json",
            &json!({"schema_version": "kyuubiki.layered-thermal-research/v1",
            "execution": "official_rust_headless_sdk_service", "complete": false,
            "study_definition_sha256": definition_digest,
            "qualification": "synthetic_reference_not_material_certification", "cases": rows}),
        )?;
    }
    let passed = rows.iter().filter(|row| row["passed"] == true).count();
    let report = json!({"schema_version": "kyuubiki.layered-thermal-research/v1",
        "execution": "official_rust_headless_sdk_service", "complete": true,
        "qualification": "synthetic_reference_not_material_certification",
        "service": health["service"], "deployment_mode": health["deployment"]["mode"],
        "case_count": rows.len(), "passed_count": passed, "failed_count": rows.len() - passed,
        "study_definition_sha256": definition_digest, "sdk_version": env!("CARGO_PKG_VERSION"),
        "source_revision": env::var("KYUUBIKI_SOURCE_REVISION").unwrap_or_else(|_| "unrecorded".into()),
        "cases": rows});
    retain(root, "report.json", &report)?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    if passed != model::cases().len() {
        return Err("research validation failed; evidence retained".into());
    }
    Ok(())
}
