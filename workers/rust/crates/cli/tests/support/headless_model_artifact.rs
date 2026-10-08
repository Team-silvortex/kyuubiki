use super::ack_loss_proxy::AckLossProxy;
use super::agent_support::{LiveAgent, wait_for_lifecycle};
use super::model_reference_gates::model;
use super::orchestra_completion::http_json;
use super::write_ack_loss::{count, document};
use super::{
    Scratch, parse_json_output, run_headless_command, start_live_server_with_state, write_temp_json,
};
use kyuubiki_headless_sdk::{
    HeadlessExecutionBatch, HeadlessExecutionBatchStep, HeadlessResearchRoundSpec,
    HeadlessRunReport, ServiceHeadlessExecutor, build_headless_research_round_evidence,
    execute_batch_with_executor, find_action_contract,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{error::Error, fs};

type Result<T> = std::result::Result<T, Box<dyn Error>>;

fn large_model(force: f64) -> Value {
    let mut value = model(force);
    // Transport-sized fixture, deliberately not a million-node physics benchmark.
    value["transport_padding"] = json!("x".repeat(8_000_000));
    value
}

fn step(action: &str, index: usize, payload: Value) -> HeadlessExecutionBatchStep {
    HeadlessExecutionBatchStep {
        action: action.into(),
        index,
        payload,
        risk: find_action_contract(action).unwrap().risk,
    }
}

pub(super) fn solve_batch(model: Value) -> HeadlessExecutionBatch {
    let mut batch = document("solve_bar_1d", json!({"model":model}));
    batch.steps.truncate(1);
    batch.steps.push(step(
        "job_wait",
        2,
        json!({
            "job_id":"{{steps.1.result.job_id}}","timeout_ms":5000,"interval_ms":10
        }),
    ));
    batch.steps.push(step(
        "result_fetch",
        3,
        json!({"job_id":"{{steps.2.result.job_id}}"}),
    ));
    batch.steps.push(step(
        "service_health",
        4,
        json!({
            "retained_upload":"{{steps.1.result.model_artifact_upload}}"
        }),
    ));
    batch
}

fn assert_rejected(report: &Value) {
    assert_eq!(report["status"], "failed");
    assert_eq!(report["executed_step_count"], 0);
    assert_eq!(report["steps"].as_array().unwrap().len(), 1);
    let failure = &report["execution_summary"]["failure"];
    assert_eq!(failure["category"], "contract_failure");
    assert_eq!(failure["stage"], "artifact_upload");
    assert_eq!(failure["retryable"], false);
    assert_eq!(failure["retry_strategy"], "none");
    assert!(
        failure["message"]
            .as_str()
            .unwrap()
            .starts_with("model_artifact_receipt_invalid:")
    );
    assert!(report["steps"][0]["result_preview"].get("job_id").is_none());
}

pub(super) fn physical_result(report: &HeadlessRunReport) -> Value {
    let result = &report.steps[2].result_preview["result"];
    assert!(
        result.get("result_artifact_ref").is_none(),
        "SDK must resolve the physical result itself"
    );
    result.clone()
}

fn tip(report: &HeadlessRunReport) -> Result<f64> {
    Ok(physical_result(report)["tip_displacement"]
        .as_f64()
        .ok_or("missing tip displacement")?)
}

#[test]
fn real_large_upload_identity_blocks_substituted_existing_content_before_calculation() -> Result<()>
{
    let mut owner = LiveAgent::start()?;
    let mut peer = LiveAgent::start()?;
    let data = Scratch::new()?;
    let server =
        start_live_server_with_state(Some(owner.port()), Some(peer.port()), Some(&data.0))?;
    let direct_url = format!("http://127.0.0.1:{}", server.port);
    owner.restart_with_orchestrator(&direct_url)?;
    peer.restart_with_orchestrator(&direct_url)?;
    let mut direct = ServiceHeadlessExecutor::new(&direct_url);
    let baseline_batch = solve_batch(large_model(2000.0));
    let baseline = execute_batch_with_executor(&baseline_batch, &mut direct, false, false);
    assert_eq!(baseline.status, "ok", "{baseline:?}\n{}", server.logs());
    let alternative = baseline.steps[0].result_preview["model_artifact_upload"].clone();
    assert_eq!(alternative["immutable"], true);
    let baseline_tip = tip(&baseline)?;
    assert!((baseline_tip / (2000.0 / (210e9 * 0.01)) - 1.0).abs() < 1e-12);
    let jobs = http_json(server.port, "/api/v1/jobs", None)?.1;
    let projects = http_json(server.port, "/api/v1/projects", None)?.1;
    let input = large_model(1000.0);
    let expected_hash = format!("{:x}", Sha256::digest(serde_json::to_vec(&input)?));
    let guarded = document("solve_bar_1d", json!({"model":input}));
    let proxy = AckLossProxy::http_with_artifact_reference(server.port, alternative.clone())?;
    let proxy_url = format!("http://127.0.0.1:{}", proxy.port);
    let failed = execute_batch_with_executor(
        &guarded,
        &mut ServiceHeadlessExecutor::new(&proxy_url),
        false,
        false,
    );
    assert_rejected(&serde_json::to_value(failed)?);

    let source = write_temp_json("mismatched-real-upload", &serde_json::to_value(&guarded)?);
    let report_path = source.parent().unwrap().join("report.json");
    let output = run_headless_command(&[
        "run",
        source.to_str().unwrap(),
        "--execute",
        "--executor",
        "service",
        "--api-base-url",
        &proxy_url,
        "--json",
        "--report-out",
        report_path.to_str().unwrap(),
    ]);
    assert!(!output.status.success());
    let cli_report = parse_json_output(&output);
    assert_rejected(&cli_report);
    assert_eq!(
        cli_report,
        serde_json::from_slice::<Value>(&fs::read(report_path)?)?
    );
    let error: Value = serde_json::from_slice(&output.stderr)?;
    assert_eq!(error["error"]["code"], "headless_execution_failed");
    assert_eq!(error["error"]["retryable"], false);
    let captured = proxy.capture();
    assert!(captured.errors.is_empty(), "{:?}", captured.errors);
    assert_eq!(captured.requests.len(), 2);
    assert_eq!(captured.discarded.len(), 2);
    assert_eq!(captured.replies.len(), 2);
    for (actual, substituted) in captured.discarded.iter().zip(&captured.replies) {
        assert_eq!(actual["artifact"]["sha256"], expected_hash);
        assert_ne!(
            actual["artifact"]["artifact_id"],
            alternative["artifact_id"]
        );
        assert_eq!(actual["artifact"]["size_bytes"], alternative["size_bytes"]);
        assert_eq!(substituted["artifact"], alternative);
    }
    let stored = http_json(
        server.port,
        &format!("/api/v1/model-artifacts/{expected_hash}"),
        None,
    )?
    .1;
    assert_eq!(stored["artifact"], captured.discarded[0]["artifact"]);
    assert_eq!(http_json(server.port, "/api/v1/jobs", None)?.1, jobs);
    assert_eq!(
        http_json(server.port, "/api/v1/projects", None)?.1,
        projects
    );
    wait_for_lifecycle(&owner, "accepting", 0)?;
    wait_for_lifecycle(&peer, "accepting", 0)?;
    assert_eq!(count(&owner)? + count(&peer)?, 1);
    drop(captured);
    drop(proxy);

    // Explicit caller-authorized healthy execution, not automatic fault replay.
    let healthy_batch = solve_batch(large_model(1000.0));
    let healthy = execute_batch_with_executor(&healthy_batch, &mut direct, false, false);
    assert_eq!(healthy.status, "ok", "{healthy:?}");
    assert_eq!(
        healthy.steps[0].result_preview["model_artifact_upload"],
        stored["artifact"]
    );
    assert_eq!(
        healthy.steps[3].payload["retained_upload"],
        stored["artifact"]
    );
    assert!((tip(&healthy)? / baseline_tip - 0.5).abs() < 1e-12);
    assert_ne!(
        healthy.execution_summary.job_ids,
        baseline.execution_summary.job_ids
    );
    let spec: HeadlessResearchRoundSpec = serde_json::from_value(json!({
        "schema_version":"kyuubiki.headless-research-round-spec/v1","round_id":"uploaded-bar",
        "workflow_id":healthy_batch.workflow_id,"iteration":1,"primary_metric_ids":["tip_displacement"],
        "metrics":[{"metric_id":"tip_displacement","pointer":"/steps/2/result_preview/result/tip_displacement",
            "unit":"m","objective":"observe"}]
    }))?;
    let evidence =
        build_headless_research_round_evidence(&healthy_batch, &healthy, &spec, None, None)?;
    let retained_path = data.0.join("uploaded-research.json");
    fs::write(
        &retained_path,
        serde_json::to_vec(&json!({"batch":healthy_batch,"report":healthy}))?,
    )?;
    let retained: Value = serde_json::from_slice(&fs::read(retained_path)?)?;
    let retained_batch = serde_json::from_value(retained["batch"].clone())?;
    let retained_report = serde_json::from_value(retained["report"].clone())?;
    assert_eq!(
        build_headless_research_round_evidence(
            &retained_batch,
            &retained_report,
            &spec,
            None,
            None
        )?,
        evidence
    );
    wait_for_lifecycle(&owner, "accepting", 0)?;
    wait_for_lifecycle(&peer, "accepting", 0)?;
    assert_eq!(count(&owner)? + count(&peer)?, 2);
    assert_eq!(
        http_json(server.port, "/api/v1/jobs", None)?.1["jobs"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    Ok(())
}
