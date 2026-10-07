use super::agent_support::{LiveAgent, wait_for_lifecycle};
use super::orchestra_completion::http_json;
use super::write_ack_loss::count;
use super::{
    Scratch, parse_json_output, run_headless_command, start_live_server_with_state, write_temp_json,
};
use kyuubiki_headless_sdk::{
    HeadlessExecutionBatch, HeadlessExecutionBatchStep, HeadlessExecutor, HeadlessRisk,
    ServiceHeadlessExecutor, execute_batch_with_executor, validate_batch,
};
use serde_json::{Value, json};
use std::{error::Error, fs};

type Result<T> = std::result::Result<T, Box<dyn Error>>;

fn bar() -> Value {
    json!({"model_schema_version":"kyuubiki.model/v1","kind":"axial_bar_1d",
        "name":"binding-lifetime-bar","material":"steel","length":1.0,
        "area":0.01,"youngs_modulus_gpa":210.0,"elements":512,"tip_force":1000.0})
}

fn request_note() -> String {
    "\u{7814}\u{7a76}\u{8bb0}\u{5f55}\n\u{0628}\u{062d}\u{062b}\t\"\\{{literal}}".repeat(8192)
}

fn document(context: &Value, model_id: &Value) -> HeadlessExecutionBatch {
    let read = |job_id: Value| {
        let mut payload = context.clone();
        payload["job_id"] = job_id;
        payload
    };
    let version = |source: usize, repeat: bool| {
        let binding = format!("{{{{steps.{source}.result.result}}}}");
        let mut payload = bar();
        payload["research_context"] =
            json!({"baseline_result":binding,"request_note":request_note()});
        if repeat {
            payload["research_context"]["repeated_baseline"] = json!(binding);
        }
        json!({"model_id":model_id,"kind":"axial_bar_1d","payload":payload})
    };
    let mut wait = read(json!("{{steps.1.result.job_id}}"));
    wait["timeout_ms"] = json!(5000);
    wait["interval_ms"] = json!(10);
    let mut alternate = read(json!("{{steps.5.result.job_id}}"));
    alternate["prefer_job_result"] = json!(false);
    let steps = vec![
        ("job_fetch", context.clone()),
        ("job_wait", wait),
        ("result_fetch", read(json!("{{steps.1.result.job_id}}"))),
        ("model_version_create", version(3, true)),
        ("job_fetch", read(json!("{{steps.1.result.job_id}}"))),
        ("model_version_create", version(3, false)),
        ("result_fetch", alternate),
        ("model_version_create", version(7, false)),
        ("job_fetch", read(json!("{{steps.7.result.job_id}}"))),
    ];
    batch(steps)
}

fn batch(steps: Vec<(&str, Value)>) -> HeadlessExecutionBatch {
    HeadlessExecutionBatch {
        schema_version: "kyuubiki.headless-execution-batch/v1".into(),
        exported_at: "2026-10-07T00:00:00Z".into(),
        language: "en".into(),
        workflow_id: "real-binding-lifetimes".into(),
        template_id: None,
        warnings: vec![],
        steps: steps
            .into_iter()
            .enumerate()
            .map(|(offset, (action, payload))| HeadlessExecutionBatchStep {
                index: offset + 1,
                action: action.into(),
                risk: HeadlessRisk::Normal,
                payload,
            })
            .collect(),
    }
}

fn assert_full_versions(port: u16, report: &Value, original: &Value) -> Result<()> {
    assert_eq!(report["status"], "ok", "{report}");
    assert_eq!(report["executed_step_count"], 9);
    assert_eq!(report["steps"].as_array().unwrap().len(), 9);
    for index in [3, 5, 7] {
        let step = &report["steps"][index];
        let preview = &step["payload"]["payload"]["research_context"]["baseline_result"];
        assert_eq!(preview["nodes"]["$kyuubiki_report_summary"], "array");
        assert_eq!(preview["nodes"]["item_count"], 513);
        assert_eq!(preview["elements"]["item_count"], 512);
        let id = step["result_preview"]["model_version_id"].as_str().unwrap();
        let (status, stored) = http_json(port, &format!("/api/v1/model-versions/{id}"), None)?;
        assert_eq!(status, 200);
        let context = &stored["version"]["payload"]["research_context"];
        assert_eq!(context["baseline_result"], *original);
        assert_eq!(context["request_note"], request_note());
        assert_eq!(
            context["baseline_result"]["nodes"]
                .as_array()
                .unwrap()
                .len(),
            513
        );
        if index == 3 {
            assert_eq!(context["repeated_baseline"], *original);
        }
    }
    for index in [0, 1, 2, 4, 6, 8] {
        assert_eq!(
            report["steps"][index]["result_preview"]["job_id"],
            report["steps"][0]["result_preview"]["job_id"]
        );
    }
    Ok(())
}

#[test]
fn real_result_fanout_persists_complete_versions_through_sdk_and_cli_without_reexecution()
-> Result<()> {
    let owner = LiveAgent::start()?;
    let peer = LiveAgent::start()?;
    let data = Scratch::new()?;
    let server =
        start_live_server_with_state(Some(owner.port()), Some(peer.port()), Some(&data.0))?;
    let url = format!("http://127.0.0.1:{}", server.port);
    let mut executor = ServiceHeadlessExecutor::new(&url);
    let project = executor
        .execute_step(
            "project_create",
            1,
            &json!({"name":"binding-lifetime-parent"}),
        )
        .map_err(|error| error.message)?
        .result;
    let model = executor
        .execute_step(
            "model_create",
            1,
            &json!({"project_id":project["project_id"],"name":"binding-lifetime-bar",
            "kind":"axial_bar_1d","payload":bar()}),
        )
        .map_err(|error| error.message)?
        .result;
    let completed = executor.execute_step("solve_and_wait_from_model_version", 1,
        &json!({"model_version_id":model["latest_version_id"],"project_id":project["project_id"],
            "endpoints":[format!("127.0.0.1:{}",owner.port())],"timeout_ms":5000,"interval_ms":10}))
        .map_err(|error|error.message)?.result;
    let id = completed["job_id"].as_str().unwrap();
    let job_path = format!("/api/v1/jobs/{id}");
    let result_path = format!("/api/v1/results/{id}");
    let original_job = http_json(server.port, &job_path, None)?.1;
    let original_result = http_json(server.port, &result_path, None)?.1;
    let result = &original_result["result"];
    assert_eq!(completed["result"]["result"], *result);
    let mut observed = executor.execute_step("job_fetch", 1,
        &json!({"job_id":id,"project_id":project["project_id"],"model_version_id":model["latest_version_id"]}))
        .map_err(|error|error.message)?.result;
    assert_eq!(observed["raw"], original_job);
    assert_eq!(observed["job"], original_job["job"]);
    assert_eq!(observed["result"], *result);
    observed["result"]["nodes"][0]["displacement"] = json!(9999.0);
    assert_eq!(observed["raw"]["result"], *result);
    assert_eq!(result["nodes"].as_array().unwrap().len(), 513);
    assert_eq!(result["elements"].as_array().unwrap().len(), 512);
    let tip = result["tip_displacement"].as_f64().unwrap();
    assert!((tip / (1000.0 / (210e9 * 0.01)) - 1.0).abs() < 1e-12);
    let batch = document(
        &json!({"job_id":id,"project_id":project["project_id"],
        "model_version_id":model["latest_version_id"]}),
        &model["model_id"],
    );
    assert!(validate_batch(&batch).ok);

    // Both public paths bind original results, never compacted report previews.
    let report = execute_batch_with_executor(&batch, &mut executor, false, false);
    assert_full_versions(server.port, &serde_json::to_value(&report)?, result)?;
    let source = write_temp_json("real-binding-lifetimes", &serde_json::to_value(&batch)?);
    let report_path = source.parent().unwrap().join("report.json");
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
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stderr),
        server.logs()
    );
    assert!(output.stderr.is_empty());
    let report = parse_json_output(&output);
    assert_eq!(
        report,
        serde_json::from_slice::<Value>(&fs::read(report_path)?)?
    );
    assert_full_versions(server.port, &report, result)?;

    assert_eq!(http_json(server.port, &job_path, None)?.1, original_job);
    assert_eq!(
        http_json(server.port, &result_path, None)?.1,
        original_result
    );
    let (_, versions) = http_json(
        server.port,
        &format!(
            "/api/v1/models/{}/versions",
            model["model_id"].as_str().unwrap()
        ),
        None,
    )?;
    assert_eq!(versions["versions"].as_array().unwrap().len(), 7);
    assert!(serde_json::to_vec(&versions)?.len() > 2 * 1024 * 1024);
    let jobs = http_json(server.port, "/api/v1/jobs", None)?.1;
    assert_eq!(jobs["jobs"].as_array().unwrap().len(), 1);
    wait_for_lifecycle(&owner, "accepting", 0)?;
    wait_for_lifecycle(&peer, "accepting", 0)?;
    assert_eq!(
        count(&owner)? + count(&peer)?,
        1,
        "binding replayed computation"
    );
    Ok(())
}

fn combined_document(context: &Value, model_id: &Value, port: u16) -> HeadlessExecutionBatch {
    let mut payload = bar();
    payload["research_context"] = json!({"baseline_receipt":"{{steps.1.result.result}}",
        "request_note":request_note()});
    let mut read = context.clone();
    read.as_object_mut().unwrap().remove("job_id");
    read["job_id"] = json!("{{steps.1.result.job_id}}");
    read["prefer_job_result"] = json!(false);
    let mut document = batch(vec![
        (
            "solve_and_wait_from_model_version",
            json!({"project_id":context["project_id"],
            "model_version_id":context["model_version_id"],"endpoints":[format!("127.0.0.1:{port}")],
            "timeout_ms":5000,"interval_ms":10}),
        ),
        (
            "model_version_create",
            json!({"model_id":model_id,"kind":"axial_bar_1d","payload":payload}),
        ),
        ("result_fetch", read.clone()),
        ("job_fetch", read),
    ]);
    document.workflow_id = "real-combined-result-ownership".into();
    document
}

fn assert_combined_results(port: u16, report: &Value) -> Result<Value> {
    assert_eq!(report["status"], "ok", "{report}");
    assert_eq!(report["executed_step_count"], 4);
    assert_eq!(report["steps"].as_array().unwrap().len(), 4);
    let completed = &report["steps"][0]["result_preview"];
    let id = completed["job_id"].as_str().unwrap();
    assert_eq!(completed["status"], "completed");
    assert!(completed.get("endpoint").is_some());
    for path in ["/solve/job_id", "/wait/job_id", "/result/job_id"] {
        assert_eq!(completed.pointer(path).unwrap(), id);
    }
    for index in [2, 3] {
        assert_eq!(report["steps"][index]["result_preview"]["job_id"], id);
    }
    let original_job = http_json(port, &format!("/api/v1/jobs/{id}"), None)?.1;
    let original_result = http_json(port, &format!("/api/v1/results/{id}"), None)?.1;
    let original = &original_result["result"];
    let version = report["steps"][1]["result_preview"]["model_version_id"]
        .as_str()
        .unwrap();
    let saved = http_json(port, &format!("/api/v1/model-versions/{version}"), None)?.1;
    assert_eq!(
        saved["version"]["payload"]["research_context"]["request_note"],
        request_note()
    );
    let retained = &saved["version"]["payload"]["research_context"]["baseline_receipt"];
    assert_eq!(retained["job_id"], id);
    assert_eq!(retained["status"], "completed");
    assert_eq!(retained["job"], original_job["job"]);
    assert_eq!(retained["result"], *original);
    assert_eq!(original["nodes"].as_array().unwrap().len(), 513);
    assert_eq!(original["elements"].as_array().unwrap().len(), 512);
    let tip = original["tip_displacement"].as_f64().unwrap();
    assert!((tip / (1000.0 / (210e9 * 0.01)) - 1.0).abs() < 1e-12);
    assert_eq!(completed["result"]["result"]["nodes"]["item_count"], 513);
    assert_eq!(completed["result"]["result"]["elements"]["item_count"], 512);
    assert_eq!(
        report["steps"][3]["result_preview"]["raw"]["result"]["nodes"]["item_count"],
        513
    );
    Ok(original_result)
}

#[test]
fn real_combined_solve_preserves_full_result_shapes_through_sdk_and_cli() -> Result<()> {
    let owner = LiveAgent::start()?;
    let peer = LiveAgent::start()?;
    let data = Scratch::new()?;
    let server =
        start_live_server_with_state(Some(owner.port()), Some(peer.port()), Some(&data.0))?;
    let url = format!("http://127.0.0.1:{}", server.port);
    let mut executor = ServiceHeadlessExecutor::new(&url);
    let project = executor
        .execute_step(
            "project_create",
            1,
            &json!({"name":"combined-result-parent"}),
        )
        .map_err(|error| error.message)?
        .result;
    let model = executor
        .execute_step(
            "model_create",
            1,
            &json!({"project_id":project["project_id"],
        "name":"combined-result-bar","kind":"axial_bar_1d","payload":bar()}),
        )
        .map_err(|error| error.message)?
        .result;
    let context =
        json!({"project_id":project["project_id"],"model_version_id":model["latest_version_id"]});
    let document = combined_document(&context, &model["model_id"], owner.port());
    assert!(validate_batch(&document).ok);
    let report = execute_batch_with_executor(&document, &mut executor, false, false);
    let report = serde_json::to_value(report)?;
    let sdk_result = assert_combined_results(server.port, &report)?;
    let source = write_temp_json("real-combined-ownership", &serde_json::to_value(&document)?);
    let report_path = source.parent().unwrap().join("report.json");
    // A distinct authorized CLI experiment computes one new job, not a recovery replay.
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
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stderr),
        server.logs()
    );
    assert!(output.stderr.is_empty());
    let report = parse_json_output(&output);
    assert_eq!(
        report,
        serde_json::from_slice::<Value>(&fs::read(report_path)?)?
    );
    let cli_result = assert_combined_results(server.port, &report)?;
    assert_ne!(sdk_result["job_id"], cli_result["job_id"]);
    assert_eq!(sdk_result["result"], cli_result["result"]);
    for original in [&sdk_result, &cli_result] {
        let path = format!("/api/v1/results/{}", original["job_id"].as_str().unwrap());
        assert_eq!(http_json(server.port, &path, None)?.1, *original);
    }
    let (_, versions) = http_json(
        server.port,
        &format!(
            "/api/v1/models/{}/versions",
            model["model_id"].as_str().unwrap()
        ),
        None,
    )?;
    assert_eq!(versions["versions"].as_array().unwrap().len(), 3);
    let jobs = http_json(server.port, "/api/v1/jobs", None)?.1;
    assert_eq!(jobs["jobs"].as_array().unwrap().len(), 2);
    wait_for_lifecycle(&owner, "accepting", 0)?;
    wait_for_lifecycle(&peer, "accepting", 0)?;
    assert_eq!(
        count(&owner)? + count(&peer)?,
        2,
        "result normalization repeated computation"
    );
    Ok(())
}
