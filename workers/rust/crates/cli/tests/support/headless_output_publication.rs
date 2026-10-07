use super::agent_support::{LiveAgent, wait_for_lifecycle};
use super::orchestra_completion::http_json;
use super::write_ack_loss::count;
use super::{
    Scratch, parse_json_output, run_headless_command, start_live_server_with_state, write_temp_json,
};
use kyuubiki_headless_sdk::{HeadlessExecutor, ServiceHeadlessExecutor};
use serde_json::{Value, json};
use std::{error::Error, fs};

#[test]
fn completed_real_solve_survives_report_publication_failure_and_recovers_by_reading_only()
-> Result<(), Box<dyn Error>> {
    let owner = LiveAgent::start()?;
    let peer = LiveAgent::start()?;
    let data = Scratch::new()?;
    let server =
        start_live_server_with_state(Some(owner.port()), Some(peer.port()), Some(&data.0))?;
    let url = format!("http://127.0.0.1:{}", server.port);
    let mut executor = ServiceHeadlessExecutor::new(&url);
    let project = executor
        .execute_step("project_create", 1, &json!({"name":"publication-parent"}))
        .map_err(|error| error.message)?
        .result;
    let model = executor
        .execute_step(
            "model_create",
            1,
            &json!({
                "project_id":project["project_id"], "name":"publication-bar", "kind":"axial_bar_1d",
                "payload":{"model_schema_version":"kyuubiki.model/v1", "kind":"axial_bar_1d",
                    "name":"publication-bar", "material":"steel", "length":1.0, "area":0.01,
                    "youngs_modulus_gpa":210.0, "elements":512, "tip_force":1000.0}
            }),
        )
        .map_err(|error| error.message)?
        .result;
    let workflow = |id, steps: Value| {
        json!({
            "schema_version":"kyuubiki.headless-workflow/v1", "language":"en",
            "exported_at":"2026-10-07T00:00:00Z", "workflow":{"id":id,"steps":steps}
        })
    };
    let source = write_temp_json(
        "publication-solve",
        &workflow(
            "publication-solve",
            json!([{
                "action":"solve_and_wait_from_model_version", "payload":{
                    "project_id":project["project_id"], "model_version_id":model["latest_version_id"],
                    "endpoints":[format!("127.0.0.1:{}",owner.port())], "timeout_ms":5000,"interval_ms":10
                }
            }]),
        ),
    );
    let report_path = source.parent().unwrap().join("blocked-report.json");
    fs::create_dir(&report_path)?;
    let output = run_headless_command(&[
        "run",
        source.to_str().unwrap(),
        "--execute",
        "--executor",
        "service",
        "--api-base-url",
        &url,
        "--json",
        "--report-out",
        report_path.to_str().unwrap(),
    ]);
    assert!(!output.status.success(), "{}", server.logs());
    let report = parse_json_output(&output);
    assert_eq!(report["status"], "ok", "{report}");
    assert_eq!(report["executed_step_count"], 1);
    assert_eq!(report["execution_summary"]["job_count"], 1);
    assert!(report["execution_summary"]["failure"].is_null());
    let diagnostic: Value = serde_json::from_slice(&output.stderr)?;
    assert_eq!(diagnostic["error"]["code"], "report_output_failure");
    assert_eq!(diagnostic["error"]["stage"], "artifact_output");
    assert_eq!(diagnostic["error"]["retryable"], false);
    assert!(
        diagnostic["error"]["recommended_action"]
            .as_str()
            .unwrap()
            .contains("do not replay")
    );
    assert!(report_path.is_dir());
    assert_eq!(fs::read_dir(source.parent().unwrap())?.count(), 2);
    let id = report["steps"][0]["result_preview"]["job_id"]
        .as_str()
        .unwrap();
    let job_path = format!("/api/v1/jobs/{id}");
    let result_path = format!("/api/v1/results/{id}");
    let original_job = http_json(server.port, &job_path, None)?.1;
    let original_result = http_json(server.port, &result_path, None)?.1;
    assert_eq!(
        original_result["result"]["nodes"].as_array().unwrap().len(),
        513
    );
    let tip = original_result["result"]["tip_displacement"]
        .as_f64()
        .unwrap();
    assert!((tip / (1000.0 / (210e9 * 0.01)) - 1.0).abs() < 1e-12);

    // Recovery is an explicitly requested read-only workflow, not a rerun of the solve.
    let context = json!({"job_id":id,"project_id":project["project_id"],
        "model_version_id":model["latest_version_id"]});
    let recovery = write_temp_json(
        "publication-read-recovery",
        &workflow(
            "publication-read-recovery",
            json!([
                {"action":"job_fetch","payload":context}, {"action":"result_fetch","payload":context}
            ]),
        ),
    );
    let repaired_path = recovery.parent().unwrap().join("recovered-report.json");
    let recovered = run_headless_command(&[
        "run",
        recovery.to_str().unwrap(),
        "--execute",
        "--executor",
        "service",
        "--api-base-url",
        &url,
        "--json",
        "--report-out",
        repaired_path.to_str().unwrap(),
    ]);
    assert!(
        recovered.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&recovered.stderr),
        server.logs()
    );
    assert!(recovered.stderr.is_empty());
    let recovered_report = parse_json_output(&recovered);
    assert_eq!(recovered_report["status"], "ok");
    assert_eq!(recovered_report["executed_step_count"], 2);
    assert_eq!(
        recovered_report["execution_summary"]["job_ids"],
        json!([id])
    );
    assert_eq!(
        recovered_report,
        serde_json::from_slice::<Value>(&fs::read(&repaired_path)?)?
    );
    for step in recovered_report["steps"].as_array().unwrap() {
        assert_eq!(step["result_preview"]["job_id"], id);
    }
    assert_eq!(http_json(server.port, &job_path, None)?.1, original_job);
    assert_eq!(
        http_json(server.port, &result_path, None)?.1,
        original_result
    );
    assert_eq!(
        http_json(server.port, "/api/v1/jobs", None)?.1["jobs"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    wait_for_lifecycle(&owner, "accepting", 0)?;
    wait_for_lifecycle(&peer, "accepting", 0)?;
    assert_eq!(
        count(&owner)? + count(&peer)?,
        1,
        "publication failure replayed computation"
    );
    Ok(())
}
