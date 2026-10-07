use super::agent_support::{LiveAgent, wait_for_lifecycle};
use super::orchestra_completion::http_json;
use super::write_ack_loss::count;
use super::{
    Scratch, parse_json_output, run_headless_command, start_live_server_with_state, write_temp_json,
};
use kyuubiki_headless_sdk::{
    HeadlessExecutionBatch, HeadlessExecutor, HeadlessResearchRoundSpec, HeadlessRunReport,
    ServiceHeadlessExecutor, build_headless_research_round_evidence,
    verify_headless_research_round_evidence,
};
use serde_json::{Value, json};
use std::{error::Error, fs};

#[test]
fn completed_real_solve_survives_evidence_generation_failure_and_regenerates_without_execution()
-> Result<(), Box<dyn Error>> {
    let owner = LiveAgent::start()?;
    let peer = LiveAgent::start()?;
    let data = Scratch::new()?;
    let server =
        start_live_server_with_state(Some(owner.port()), Some(peer.port()), Some(&data.0))?;
    let url = format!("http://127.0.0.1:{}", server.port);
    let mut executor = ServiceHeadlessExecutor::new(&url);
    let project = executor
        .execute_step("project_create", 1, &json!({"name":"generation-parent"}))
        .map_err(|error| error.message)?
        .result;
    let model = executor
        .execute_step(
            "model_create",
            1,
            &json!({
                "project_id":project["project_id"],"name":"generation-bar","kind":"axial_bar_1d",
                "payload":{"model_schema_version":"kyuubiki.model/v1","kind":"axial_bar_1d",
                    "name":"generation-bar","material":"steel","length":1.0,"area":0.01,
                    "youngs_modulus_gpa":210.0,"elements":512,"tip_force":1000.0}
            }),
        )
        .map_err(|error| error.message)?
        .result;
    let source = write_temp_json(
        "generation-solve",
        &json!({
            "schema_version":"kyuubiki.headless-workflow/v1","language":"en","exported_at":"2026-10-07T00:00:00Z",
            "workflow":{"id":"generation-solve","steps":[{"action":"solve_and_wait_from_model_version","payload":{
                "project_id":project["project_id"],"model_version_id":model["latest_version_id"],
                "endpoints":[format!("127.0.0.1:{}",owner.port())],"timeout_ms":5000,"interval_ms":10
            }}]}
        }),
    );
    let original_source = fs::read(&*source)?;
    let spec_path = source.parent().unwrap().join("round.json");
    let mut spec = json!({
        "schema_version":"kyuubiki.headless-research-round-spec/v1","round_id":"generation-round-1",
        "workflow_id":"generation-solve","iteration":1,"primary_metric_ids":["tip_displacement"],
        "metrics":[{"metric_id":"tip_displacement","pointer":"/steps/0/result_preview/result/result/missing_displacement",
            "unit":"m","objective":"observe"}]
    });
    fs::write(&spec_path, serde_json::to_vec_pretty(&spec)?)?;
    let evidence_path = source.parent().unwrap().join("evidence.json");
    let retained_evidence = b"{\"retained_evidence\":true}";
    fs::write(&evidence_path, retained_evidence)?;
    let report_path = source.parent().unwrap().join("completed-run.json");
    let output = run_headless_command(&[
        "run",
        source.to_str().unwrap(),
        "--execute",
        "--executor",
        "service",
        "--execution-posture",
        "research",
        "--api-base-url",
        &url,
        "--json",
        "--report-out",
        report_path.to_str().unwrap(),
        "--research-round-spec",
        spec_path.to_str().unwrap(),
        "--research-round-out",
        evidence_path.to_str().unwrap(),
    ]);
    assert!(!output.status.success(), "{}", server.logs());
    let report = parse_json_output(&output);
    assert_eq!(report["status"], "ok", "{report}");
    assert_eq!(report["executed_step_count"], 1);
    assert_eq!(report["execution_summary"]["job_count"], 1);
    assert!(report["execution_summary"]["failure"].is_null());
    assert_eq!(
        report,
        serde_json::from_slice::<Value>(&fs::read(&report_path)?)?
    );
    let diagnostic: Value = serde_json::from_slice(&output.stderr)?;
    assert_eq!(diagnostic["error"]["code"], "report_generation_failure");
    assert_eq!(diagnostic["error"]["stage"], "artifact_output");
    assert_eq!(diagnostic["error"]["retryable"], false);
    assert!(
        diagnostic["error"]["recommended_action"]
            .as_str()
            .unwrap()
            .contains("do not replay")
    );
    assert_eq!(fs::read(&*source)?, original_source);
    assert_eq!(fs::read(&evidence_path)?, retained_evidence);
    assert_eq!(fs::read_dir(source.parent().unwrap())?.count(), 4);
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
    assert_eq!(
        report
            .pointer("/steps/0/result_preview/result/result/tip_displacement")
            .and_then(Value::as_f64),
        Some(tip)
    );

    // Repair only the derived metric mapping through the public SDK, not the solve workflow.
    spec["metrics"][0]["pointer"] = json!("/steps/0/result_preview/result/result/tip_displacement");
    let repaired_spec: HeadlessResearchRoundSpec = serde_json::from_value(spec)?;
    let rendered = run_headless_command(&["render", source.to_str().unwrap(), "--json"]);
    assert!(
        rendered.status.success(),
        "{}",
        String::from_utf8_lossy(&rendered.stderr)
    );
    let batch: HeadlessExecutionBatch = serde_json::from_value(parse_json_output(&rendered))?;
    let typed_report: HeadlessRunReport = serde_json::from_value(report)?;
    let recovered =
        build_headless_research_round_evidence(&batch, &typed_report, &repaired_spec, None, None)?;
    verify_headless_research_round_evidence(&batch, &typed_report, &recovered, None)?;
    assert!(recovered.qualified);
    assert_eq!(recovered.metrics[0].value, tip);
    let recovered_path = source.parent().unwrap().join("recovered-evidence.json");
    fs::write(&recovered_path, serde_json::to_vec_pretty(&recovered)?)?;
    assert_eq!(
        serde_json::from_slice::<Value>(&fs::read(recovered_path)?)?,
        serde_json::to_value(recovered)?
    );
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
        "evidence generation/recovery repeated calculation"
    );
    assert_eq!(fs::read(&evidence_path)?, retained_evidence);
    assert_eq!(fs::read(&*source)?, original_source);
    Ok(())
}
