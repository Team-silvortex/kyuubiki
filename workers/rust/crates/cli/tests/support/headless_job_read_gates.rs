use super::ack_loss_proxy::AckLossProxy;
use super::agent_support::{LiveAgent, wait_for_lifecycle};
use super::orchestra_completion::http_json;
use super::write_ack_loss::{count, document};
use super::{
    Scratch, parse_json_output, run_headless_command, start_live_server_with_state, write_temp_json,
};
use kyuubiki_headless_sdk::{
    HeadlessExecutor, ServiceHeadlessExecutor, execute_batch_with_executor,
};
use serde_json::{Value, json};
use std::{error::Error, fs};

type Result<T> = std::result::Result<T, Box<dyn Error>>;

fn assert_failure(report: &Value) {
    assert_eq!(report["status"], "failed");
    assert_eq!(report["executed_step_count"], 0);
    assert_eq!(report["steps"].as_array().unwrap().len(), 1);
    let failure = &report["execution_summary"]["failure"];
    assert_eq!(failure["category"], "contract_failure");
    assert_eq!(failure["stage"], "validation");
    assert_eq!(failure["retryable"], false);
    assert_eq!(failure["retry_strategy"], "none");
    assert!(
        failure["message"]
            .as_str()
            .unwrap()
            .starts_with("job_read_invalid:")
    );
    assert!(report["steps"][0]["result_preview"].get("result").is_none());
    assert!(report["steps"][0]["result_preview"].get("job_id").is_none());
}

#[test]
fn real_saved_version_reads_reject_missing_association_without_reexecution() -> Result<()> {
    let owner = LiveAgent::start()?;
    let peer = LiveAgent::start()?;
    let data = Scratch::new()?;
    let server =
        start_live_server_with_state(Some(owner.port()), Some(peer.port()), Some(&data.0))?;
    let url = format!("http://127.0.0.1:{}", server.port);
    let mut direct = ServiceHeadlessExecutor::new(&url);
    let project = direct
        .execute_step("project_create", 1, &json!({"name":"read-context-parent"}))
        .map_err(|error| error.message)?
        .result;
    let model = direct
        .execute_step(
            "model_create",
            1,
            &json!({"project_id":project["project_id"],
        "name":"read-context-bar","kind":"axial_bar_1d","payload":{
            "model_schema_version":"kyuubiki.model/v1","kind":"axial_bar_1d","name":"bar",
            "material":"steel","length":1.0,"area":0.01,"youngs_modulus_gpa":210.0,
            "elements":4,"tip_force":1000.0}}),
        )
        .map_err(|error| error.message)?
        .result;
    let version_id = &model["latest_version_id"];
    let completed = direct.execute_step("solve_and_wait_from_model_version", 1,
        &json!({"model_version_id":version_id,"projectId":project["project_id"],"endpoints":[format!("127.0.0.1:{}",owner.port())],
            "timeout_ms":5000,"interval_ms":10}))
        .map_err(|error| error.message)?.result;
    let job_id = completed["job_id"].as_str().unwrap();
    assert_eq!(completed["result"]["job"]["model_version_id"], *version_id);
    let job_path = format!("/api/v1/jobs/{job_id}");
    let status_path = format!("{job_path}/status");
    let result_path = format!("/api/v1/results/{job_id}");
    let original = http_json(server.port, &job_path, None)?.1;
    let original_result = http_json(server.port, &result_path, None)?.1;
    for (action, prefer, path) in [
        ("job_wait", true, &status_path),
        ("result_fetch", true, &job_path),
        ("result_fetch", false, &status_path),
    ] {
        let proxy = AckLossProxy::http_without_record_identity(
            server.port,
            "GET",
            path,
            "job",
            "model_version_id",
        )?;
        let payload = json!({"job_id":job_id,"project_id":project["project_id"],
            "model_version_id":version_id,"prefer_job_result":prefer,"timeout_ms":5000,"interval_ms":10});
        let batch = document(action, payload);
        let report = execute_batch_with_executor(
            &batch,
            &mut ServiceHeadlessExecutor::new(&format!("http://127.0.0.1:{}", proxy.port)),
            false,
            false,
        );
        assert_failure(&serde_json::to_value(report)?);
        let cli = action == "result_fetch" && !prefer;
        if cli {
            let source = write_temp_json("real-job-read-context", &serde_json::to_value(&batch)?);
            let report_path = source.parent().unwrap().join("report.json");
            let url = format!("http://127.0.0.1:{}", proxy.port);
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
            assert!(!output.status.success());
            let report = parse_json_output(&output);
            assert_failure(&report);
            assert_eq!(
                report,
                serde_json::from_slice::<Value>(&fs::read(report_path)?)?
            );
            let error: Value = serde_json::from_slice(&output.stderr)?;
            assert_eq!(error["error"]["code"], "headless_execution_failed");
            assert_eq!(error["error"]["retryable"], false);
        }
        let captured = proxy.capture();
        assert!(captured.errors.is_empty(), "{captured:?}");
        let expected = if cli { 2 } else { 1 };
        assert_eq!(captured.requests.len(), expected);
        assert_eq!(captured.discarded.len(), expected);
        assert_eq!(captured.replies.len(), expected);
        for (original, corrupted) in captured.discarded.iter().zip(&captured.replies) {
            assert_eq!(original["job"]["model_version_id"], *version_id);
            let mut expected = original.clone();
            expected["job"]
                .as_object_mut()
                .unwrap()
                .remove("model_version_id");
            assert_eq!(*corrupted, expected);
        }
    }
    assert_eq!(http_json(server.port, &job_path, None)?.1, original);
    assert_eq!(
        http_json(server.port, &result_path, None)?.1,
        original_result
    );
    for prefer in [true, false] {
        let fetched = direct.execute_step("result_fetch",1,&json!({"job_id":job_id,"jobId":job_id,
            "projectId":project["project_id"],"modelVersionId":version_id,"preferJobResult":prefer}))
            .map_err(|error|error.message)?.result;
        assert_eq!(fetched["result"], completed["result"]["result"]);
        let displacement = fetched["result"]["tip_displacement"].as_f64().unwrap();
        assert!((displacement / (1000.0 / (210e9 * 0.01)) - 1.0).abs() < 1e-12);
    }
    wait_for_lifecycle(&owner, "accepting", 0)?;
    wait_for_lifecycle(&peer, "accepting", 0)?;
    assert_eq!(
        count(&owner)? + count(&peer)?,
        1,
        "reading reran the solver"
    );
    let projects = http_json(server.port, "/api/v1/projects", None)?.1;
    assert_eq!(projects["projects"].as_array().unwrap().len(), 1);
    assert!(!projects.to_string().contains("must-not-follow-lost-ack"));
    assert_eq!(
        http_json(server.port, "/api/v1/jobs", None)?.1["jobs"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    Ok(())
}
