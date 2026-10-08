use super::agent_support::{LiveAgent, wait_for_lifecycle};
use super::model_artifact::solve_batch;
use super::model_reference_gates::model;
use super::orchestra_completion::http_json;
use super::result_content_proxy::ResultContentProxy;
use super::write_ack_loss::{count, document};
use super::{
    Scratch, parse_json_output, run_headless_command, start_live_server_with_state, write_temp_json,
};
use kyuubiki_headless_sdk::{
    HeadlessResearchRoundSpec, HeadlessRunReport, ServiceHeadlessExecutor,
    build_headless_research_round_evidence, execute_batch_with_executor,
};
use serde_json::{Value, json};
use std::{error::Error, fs};

type Result<T> = std::result::Result<T, Box<dyn Error>>;

fn assert_rejected(report: &HeadlessRunReport) {
    assert_eq!(report.status, "failed");
    assert_eq!(report.executed_step_count, 0);
    assert_eq!(report.steps.len(), 1);
    let failure = report.execution_summary.failure.as_ref().unwrap();
    assert_eq!(failure.stage, "result_fetch");
    assert_eq!(failure.category, "result_artifact_readback_failed");
    assert!(!failure.retryable);
    assert_eq!(failure.retry_strategy, "none");
    assert!(failure.message.contains("SHA-256"));
}

#[test]
fn real_sdk_cli_result_readback_extracts_physics_rejects_corruption_and_recovers_same_job()
-> Result<()> {
    let mut owner = LiveAgent::start()?;
    let mut peer = LiveAgent::start()?;
    let data = Scratch::new()?;
    let server =
        start_live_server_with_state(Some(owner.port()), Some(peer.port()), Some(&data.0))?;
    let url = format!("http://127.0.0.1:{}", server.port);
    owner.restart_with_orchestrator(&url)?;
    peer.restart_with_orchestrator(&url)?;
    let mut input = model(1000.0);
    input["transport_padding"] = json!("x".repeat(8_000_000));
    let solved = execute_batch_with_executor(
        &solve_batch(input),
        &mut ServiceHeadlessExecutor::new(&url),
        false,
        false,
    );
    assert_eq!(solved.status, "ok", "{solved:?}\n{}", server.logs());
    let original = &solved.steps[2].result_preview;
    let tip = original["result"]["tip_displacement"]
        .as_f64()
        .ok_or("SDK did not return physical result")?;
    assert!((tip / (1000.0 / (210e9 * 0.01)) - 1.0).abs() < 1e-12);
    assert_eq!(original["result_artifact_readback"]["verified"], true);
    let id = original["result_artifact_readback"]["artifact"]["artifact_id"]
        .as_str()
        .ok_or("missing verified descriptor")?;
    let job = solved.steps[0].result_preview["job_id"].as_str().unwrap();
    let jobs = http_json(server.port, "/api/v1/jobs", None)?.1;
    let projects = http_json(server.port, "/api/v1/projects", None)?.1;
    let proxy = ResultContentProxy::start(
        server.port,
        format!("/api/v1/result-artifacts/{id}/content"),
    )?;
    let faulty_url = format!("http://127.0.0.1:{}", proxy.port);
    let guarded = document("result_fetch", json!({"job_id":job}));
    let rejected = execute_batch_with_executor(
        &guarded,
        &mut ServiceHeadlessExecutor::new(&faulty_url),
        false,
        false,
    );
    assert_rejected(&rejected);
    let source = write_temp_json("real-result-readback", &serde_json::to_value(&guarded)?);
    let retained = source.parent().unwrap().join("report.json");
    let output = run_headless_command(&[
        "run",
        source.to_str().unwrap(),
        "--execute",
        "--executor",
        "service",
        "--api-base-url",
        &faulty_url,
        "--json",
        "--report-out",
        retained.to_str().unwrap(),
    ]);
    assert!(!output.status.success());
    let report = parse_json_output(&output);
    assert_eq!(
        report,
        serde_json::from_slice::<Value>(&fs::read(&retained)?)?
    );
    assert_rejected(&serde_json::from_value(report)?);
    let stderr: Value = serde_json::from_slice(&output.stderr)?;
    assert_eq!(stderr["error"]["retryable"], false);
    let errors = proxy.errors.lock().unwrap().clone();
    assert!(errors.is_empty(), "{errors:?}");
    assert_eq!(proxy.paths.lock().unwrap().len(), 4);
    drop(proxy);
    assert_eq!(http_json(server.port, "/api/v1/jobs", None)?.1, jobs);
    assert_eq!(
        http_json(server.port, "/api/v1/projects", None)?.1,
        projects
    );

    // Caller explicitly retries only the read of the completed job, never its solve.
    let mut recovery = guarded;
    recovery.steps.truncate(1);
    let recovered = execute_batch_with_executor(
        &recovery,
        &mut ServiceHeadlessExecutor::new(&url),
        false,
        false,
    );
    assert_eq!(recovered.status, "ok", "{recovered:?}");
    assert_eq!(
        recovered.steps[0].result_preview["result"],
        original["result"]
    );
    assert_eq!(
        recovered.steps[0].result_preview["result_artifact_readback"],
        original["result_artifact_readback"]
    );
    let mut separate = recovery.clone();
    separate.steps[0].payload["prefer_job_result"] = json!(false);
    let separate = execute_batch_with_executor(
        &separate,
        &mut ServiceHeadlessExecutor::new(&url),
        false,
        false,
    );
    assert_eq!(separate.status, "ok", "{separate:?}");
    assert_eq!(
        separate.steps[0].result_preview["result"],
        original["result"]
    );
    let mut reference_only = recovery.clone();
    reference_only.steps[0].payload["resolve_result_artifact"] = json!(false);
    let reference_only = execute_batch_with_executor(
        &reference_only,
        &mut ServiceHeadlessExecutor::new(&url),
        false,
        false,
    );
    assert_eq!(reference_only.status, "ok", "{reference_only:?}");
    assert_eq!(
        reference_only.steps[0].result_preview["result"]["result_artifact_ref"],
        original["result_artifact_readback"]["artifact"]
    );
    assert!(
        reference_only.steps[0]
            .result_preview
            .get("result_artifact_readback")
            .is_none()
    );
    let spec: HeadlessResearchRoundSpec = serde_json::from_value(json!({
        "schema_version":"kyuubiki.headless-research-round-spec/v1","round_id":"verified-bar-readback",
        "workflow_id":recovery.workflow_id,"iteration":1,"primary_metric_ids":["tip_displacement"],
        "metrics":[{"metric_id":"tip_displacement","pointer":"/steps/0/result_preview/result/tip_displacement","unit":"m","objective":"observe"}]
    }))?;
    let evidence =
        build_headless_research_round_evidence(&recovery, &recovered, &spec, None, None)?;
    let source = write_temp_json(
        "real-result-read-recovery",
        &serde_json::to_value(&recovery)?,
    );
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
        retained.to_str().unwrap(),
    ]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let cli = parse_json_output(&output);
    assert_eq!(cli, serde_json::from_slice::<Value>(&fs::read(&retained)?)?);
    assert_eq!(
        cli["steps"][0]["result_preview"]["result"],
        original["result"]
    );
    let reloaded: HeadlessRunReport = serde_json::from_slice(&fs::read(&retained)?)?;
    assert_eq!(
        build_headless_research_round_evidence(&recovery, &reloaded, &spec, None, None)?,
        evidence
    );
    wait_for_lifecycle(&owner, "accepting", 0)?;
    wait_for_lifecycle(&peer, "accepting", 0)?;
    assert_eq!(count(&owner)? + count(&peer)?, 1);
    assert_eq!(http_json(server.port, "/api/v1/jobs", None)?.1, jobs);
    assert_eq!(
        http_json(server.port, "/api/v1/projects", None)?.1,
        projects
    );
    Ok(())
}
