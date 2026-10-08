use super::agent_support::{LiveAgent, wait_for_lifecycle};
use super::model_reference_gates::{library, model};
use super::orchestra_completion::http_json;
use super::write_ack_loss::{count, document};
use super::{Scratch, start_live_server_with_state};
use kyuubiki_headless_sdk::{
    HeadlessExecutionBatch, HeadlessExecutionBatchStep, HeadlessExecutor, HeadlessModelSourceKind,
    HeadlessResearchRoundSpec, HeadlessRunReport, ServiceHeadlessExecutor,
    build_headless_research_round_evidence, execute_batch_with_executor, find_action_contract,
    headless_saved_model_source,
};
use serde_json::{Value, json};
use std::{error::Error, fs};

type Result<T> = std::result::Result<T, Box<dyn Error>>;

fn step(action: &str, index: usize, payload: Value) -> HeadlessExecutionBatchStep {
    HeadlessExecutionBatchStep {
        action: action.into(),
        index,
        payload,
        risk: find_action_contract(action).unwrap().risk,
    }
}

fn batch(action: &str, reference: Value) -> HeadlessExecutionBatch {
    let mut batch = document(action, reference.clone());
    batch.steps = if action == "direct_mesh_solve" {
        vec![
            step(action, 1, reference),
            step(
                "job_wait",
                2,
                json!({"job_id":"{{steps.1.result.job_id}}","timeout_ms":5000,"interval_ms":10}),
            ),
            step(
                "result_fetch",
                3,
                json!({"job_id":"{{steps.2.result.job_id}}"}),
            ),
        ]
    } else {
        vec![step(action, 1, reference)]
    };
    batch
}

#[test]
fn real_saved_source_pins_reject_mutable_drift_and_preserve_historical_version_results()
-> Result<()> {
    let owner = LiveAgent::start()?;
    let peer = LiveAgent::start()?;
    let data = Scratch::new()?;
    let server =
        start_live_server_with_state(Some(owner.port()), Some(peer.port()), Some(&data.0))?;
    let mut executor = ServiceHeadlessExecutor::new(&format!("http://127.0.0.1:{}", server.port));
    let (_, created) = library(&mut executor)?;
    let model_id = created["model_id"].as_str().unwrap();
    let version_id = created["latest_version_id"].as_str().unwrap();
    let current_path = format!("/api/v1/models/{model_id}");
    let version_path = format!("/api/v1/model-versions/{version_id}");
    let original = http_json(server.port, &current_path, None)?.1;
    let original_version = http_json(server.port, &version_path, None)?.1;
    let original_source =
        headless_saved_model_source(&original["model"], HeadlessModelSourceKind::Model)?;
    let version_source = headless_saved_model_source(
        &original_version["version"],
        HeadlessModelSourceKind::ModelVersion,
    )?;
    let old_batch = batch(
        "direct_mesh_solve",
        json!({"model_id":model_id,"expected_model_source":original_source}),
    );
    let old_report = execute_batch_with_executor(&old_batch, &mut executor, false, false);
    assert_eq!(old_report.status, "ok", "{old_report:?}\n{}", server.logs());
    assert_eq!(
        old_report.steps[0].result_preview["model_source"],
        serde_json::to_value(&original_source)?
    );
    let old_tip = old_report.steps[2].result_preview["result"]["tip_displacement"]
        .as_f64()
        .unwrap();
    assert!((old_tip / (1000.0 / (210e9 * 0.01)) - 1.0).abs() < 1e-12);

    executor
        .execute_step(
            "model_version_create",
            1,
            &json!({"model_id":model_id,"payload":model(2000.0)}),
        )
        .map_err(|error| error.message)?;
    let updated = http_json(server.port, &current_path, None)?.1;
    let new_source =
        headless_saved_model_source(&updated["model"], HeadlessModelSourceKind::Model)?;
    assert_ne!(original_source, new_source);
    let jobs = http_json(server.port, "/api/v1/jobs", None)?.1;
    wait_for_lifecycle(&owner, "accepting", 0)?;
    wait_for_lifecycle(&peer, "accepting", 0)?;
    assert_eq!(count(&owner)? + count(&peer)?, 1);
    let mut guarded = old_batch.clone();
    guarded.steps.push(step(
        "project_create",
        4,
        json!({"name":"must-not-follow-stale-source"}),
    ));
    let failed = execute_batch_with_executor(&guarded, &mut executor, false, false);
    assert_eq!(failed.status, "failed");
    assert_eq!(failed.executed_step_count, 0);
    assert_eq!(failed.steps.len(), 1);
    let failure = failed.execution_summary.failure.unwrap();
    assert_eq!(failure.category, "contract_failure");
    assert_eq!(failure.stage, "validation");
    assert!(!failure.retryable);
    assert!(failure.message.contains("source content"));
    assert_eq!(http_json(server.port, "/api/v1/jobs", None)?.1, jobs);
    assert_eq!(count(&owner)? + count(&peer)?, 1);
    assert_eq!(
        http_json(server.port, "/api/v1/projects", None)?.1["projects"]
            .as_array()
            .unwrap()
            .len(),
        1
    );

    let new_batch = batch(
        "direct_mesh_solve",
        json!({"model_id":model_id,"expected_model_source":new_source}),
    );
    let new_report = execute_batch_with_executor(&new_batch, &mut executor, false, false);
    assert_eq!(new_report.status, "ok", "{new_report:?}");
    let new_tip = new_report.steps[2].result_preview["result"]["tip_displacement"]
        .as_f64()
        .unwrap();
    assert!((new_tip / old_tip - 2.0).abs() < 1e-12);
    assert_ne!(
        old_report.execution_summary.job_ids,
        new_report.execution_summary.job_ids
    );

    let version_batch = batch(
        "solve_and_wait_from_model_version",
        json!({"model_version_id":version_id,"endpoints":[format!("127.0.0.1:{}",owner.port())],
        "expected_model_source":version_source,"timeout_ms":5000,"interval_ms":10}),
    );
    let version_report = execute_batch_with_executor(&version_batch, &mut executor, false, false);
    assert_eq!(version_report.status, "ok", "{version_report:?}");
    assert_eq!(
        version_report.steps[0].result_preview["model_source"],
        serde_json::to_value(&version_source)?
    );
    assert_eq!(
        version_report.steps[0].result_preview["solve"]["model_source"],
        serde_json::to_value(&version_source)?
    );
    let historical_tip =
        version_report.steps[0].result_preview["result"]["result"]["tip_displacement"]
            .as_f64()
            .unwrap();
    assert!((historical_tip / old_tip - 1.0).abs() < 1e-12);
    assert_eq!(
        http_json(server.port, &version_path, None)?.1,
        original_version
    );

    let spec: HeadlessResearchRoundSpec = serde_json::from_value(json!({
        "schema_version":"kyuubiki.headless-research-round-spec/v1","round_id":"source-bound-bar",
        "workflow_id":version_batch.workflow_id,"iteration":1,"primary_metric_ids":["tip_displacement"],
        "metrics":[{"metric_id":"tip_displacement","pointer":"/steps/0/result_preview/result/result/tip_displacement",
            "unit":"m","objective":"observe"}]}))?;
    let evidence =
        build_headless_research_round_evidence(&version_batch, &version_report, &spec, None, None)?;
    let path = data.0.join("source-bound-run.json");
    fs::write(
        &path,
        serde_json::to_vec(&json!({"batch":version_batch,"report":version_report}))?,
    )?;
    let retained: Value = serde_json::from_slice(&fs::read(path)?)?;
    let retained_batch: HeadlessExecutionBatch = serde_json::from_value(retained["batch"].clone())?;
    let mut retained_report: HeadlessRunReport =
        serde_json::from_value(retained["report"].clone())?;
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
    retained_report.steps[0].result_preview["model_source"]["sha256"] = json!("0".repeat(64));
    assert!(
        build_headless_research_round_evidence(
            &retained_batch,
            &retained_report,
            &spec,
            None,
            None
        )
        .unwrap_err()
        .contains("saved model source")
    );
    wait_for_lifecycle(&owner, "accepting", 0)?;
    wait_for_lifecycle(&peer, "accepting", 0)?;
    assert_eq!(count(&owner)? + count(&peer)?, 3);
    assert_eq!(
        http_json(server.port, "/api/v1/jobs", None)?.1["jobs"]
            .as_array()
            .unwrap()
            .len(),
        3
    );
    Ok(())
}
