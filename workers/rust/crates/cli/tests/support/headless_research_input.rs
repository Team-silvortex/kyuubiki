use super::agent_support::{LiveAgent, wait_for_lifecycle};
use super::orchestra_completion::http_json;
use super::write_ack_loss::count;
use super::{Scratch, start_live_server_with_state};
use kyuubiki_headless_sdk::{
    HeadlessExecutionBatch, HeadlessParameterPatch, HeadlessResearchRoundEvidence,
    HeadlessResearchRoundSpec, HeadlessRunReport, HeadlessWorkflowDocument,
    ServiceHeadlessExecutor, apply_parameter_patch, build_headless_research_round_evidence,
    execute_batch_with_executor, normalize_workflow_document, validate_batch,
    verify_headless_research_round_evidence,
};
use serde_json::{Value, json};
use std::{error::Error, fs};

fn spec(iteration: u64) -> Result<HeadlessResearchRoundSpec, serde_json::Error> {
    serde_json::from_value(json!({
        "schema_version":"kyuubiki.headless-research-round-spec/v1",
        "round_id":format!("input-bound-round-{iteration}"),"workflow_id":"input-bound-bar",
        "iteration":iteration,"primary_metric_ids":["tip_displacement"],
        "metrics":[{"metric_id":"tip_displacement",
            "pointer":"/steps/2/result_preview/result/result/tip_displacement",
            "unit":"m","objective":"observe"}]
    }))
}

#[test]
fn changed_real_solver_inputs_reject_old_reports_without_replaying_and_accept_new_results()
-> Result<(), Box<dyn Error>> {
    let owner = LiveAgent::start()?;
    let peer = LiveAgent::start()?;
    let data = Scratch::new()?;
    let server =
        start_live_server_with_state(Some(owner.port()), Some(peer.port()), Some(&data.0))?;
    let mut executor = ServiceHeadlessExecutor::new(&format!("http://127.0.0.1:{}", server.port));
    let document: HeadlessWorkflowDocument = serde_json::from_value(json!({
        "schema_version":"kyuubiki.headless-workflow/v1","language":"en",
        "exported_at":"2026-10-08T00:00:00Z","workflow":{"id":"input-bound-bar","steps":[
            {"action":"project_create","payload":{"name":"input-bound-parent"}},
            {"action":"model_create","payload":{
                "project_id":"{{steps.1.result.project_id}}","name":"input-bound-bar","kind":"axial_bar_1d",
                "payload":{"model_schema_version":"kyuubiki.model/v1","kind":"axial_bar_1d",
                    "name":"input-bound-bar","material":"steel","length":1.0,"area":0.01,
                    "youngs_modulus_gpa":210.0,"elements":512,"tip_force":1000.0}
            }},
            {"action":"solve_and_wait_from_model_version","payload":{
                "project_id":"{{steps.1.result.project_id}}",
                "model_version_id":"{{steps.2.result.latest_version_id}}",
                "endpoints":[format!("127.0.0.1:{}",owner.port())],"timeout_ms":5000,"interval_ms":10
            }}
        ]}
    }))?;
    let first_batch = normalize_workflow_document(&document)?;
    let first_report = execute_batch_with_executor(&first_batch, &mut executor, false, false);
    assert_eq!(
        first_report.status,
        "ok",
        "{first_report:?}\n{}",
        server.logs()
    );
    assert_eq!(first_report.executed_step_count, 3);
    assert_ne!(
        first_report.steps[1].payload, first_batch.steps[1].payload,
        "binding was resolved"
    );
    let first =
        build_headless_research_round_evidence(&first_batch, &first_report, &spec(1)?, None, None)?;
    let first_tip = first.metrics[0].value;
    assert!((first_tip / (1000.0 / (210e9 * 0.01)) - 1.0).abs() < 1e-12);
    let first_job_id = &first_report.execution_summary.job_ids[0];
    let original_job = http_json(server.port, &format!("/api/v1/jobs/{first_job_id}"), None)?.1;
    let original_result = http_json(
        server.port,
        &format!("/api/v1/results/{first_job_id}"),
        None,
    )?
    .1;
    assert_eq!(
        original_result["result"]["nodes"].as_array().unwrap().len(),
        513
    );
    let original_projects = http_json(server.port, "/api/v1/projects", None)?.1;
    let original_jobs = http_json(server.port, "/api/v1/jobs", None)?.1;
    wait_for_lifecycle(&owner, "accepting", 0)?;
    wait_for_lifecycle(&peer, "accepting", 0)?;
    assert_eq!(count(&owner)? + count(&peer)?, 1);

    let mut second_batch = first_batch.clone();
    let patch: HeadlessParameterPatch = serde_json::from_value(json!({
        "schema_version":"kyuubiki.headless-parameter-patch/v1","patch_id":"double-bar-tip-force",
        "workflow_id":"input-bound-bar","changes":[{
            "path":"/steps/1/payload/payload/tip_force","expected":1000.0,"value":2000.0
        }]
    }))?;
    let receipt = apply_parameter_patch(&mut second_batch, &patch)?;
    // Updating generic validation metadata must not turn old calculation results into new evidence.
    let mut stale = first_report.clone();
    stale.validation = validate_batch(&second_batch);
    stale.warning_count = second_batch.warnings.len();
    let error = build_headless_research_round_evidence(
        &second_batch,
        &stale,
        &spec(2)?,
        Some(&receipt),
        Some(&first),
    )
    .expect_err("old result must not qualify the changed load");
    assert!(error.contains("execution input fingerprint"), "{error}");
    assert!(
        verify_headless_research_round_evidence(&second_batch, &first_report, &first, None)
            .unwrap_err()
            .contains("execution input fingerprint")
    );
    assert_eq!(
        http_json(server.port, "/api/v1/projects", None)?.1,
        original_projects
    );
    assert_eq!(
        http_json(server.port, "/api/v1/jobs", None)?.1,
        original_jobs
    );
    assert_eq!(
        count(&owner)? + count(&peer)?,
        1,
        "refusal replayed a solve"
    );

    let second_report = execute_batch_with_executor(&second_batch, &mut executor, false, false);
    assert_eq!(
        second_report.status,
        "ok",
        "{second_report:?}\n{}",
        server.logs()
    );
    assert_ne!(first_report.execution_input, second_report.execution_input);
    let second = build_headless_research_round_evidence(
        &second_batch,
        &second_report,
        &spec(2)?,
        Some(&receipt),
        Some(&first),
    )?;
    assert!((second.metrics[0].value / first_tip - 2.0).abs() < 1e-12);
    assert_ne!(
        first_report.execution_summary.job_ids,
        second_report.execution_summary.job_ids
    );

    let retained = json!({"batch":second_batch,"report":second_report,"evidence":second});
    let retained_path = data.0.join("round-2-retained.json");
    fs::write(&retained_path, serde_json::to_vec_pretty(&retained)?)?;
    let reloaded: Value = serde_json::from_slice(&fs::read(&retained_path)?)?;
    let reloaded_batch: HeadlessExecutionBatch = serde_json::from_value(reloaded["batch"].clone())?;
    let reloaded_report: HeadlessRunReport = serde_json::from_value(reloaded["report"].clone())?;
    let reloaded_evidence: HeadlessResearchRoundEvidence =
        serde_json::from_value(reloaded["evidence"].clone())?;
    verify_headless_research_round_evidence(
        &reloaded_batch,
        &reloaded_report,
        &reloaded_evidence,
        Some(&first),
    )?;
    assert_eq!(
        http_json(server.port, &format!("/api/v1/jobs/{first_job_id}"), None)?.1,
        original_job
    );
    assert_eq!(
        http_json(
            server.port,
            &format!("/api/v1/results/{first_job_id}"),
            None
        )?
        .1,
        original_result
    );
    assert_eq!(
        http_json(server.port, "/api/v1/jobs", None)?.1["jobs"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    wait_for_lifecycle(&owner, "accepting", 0)?;
    wait_for_lifecycle(&peer, "accepting", 0)?;
    assert_eq!(
        count(&owner)? + count(&peer)?,
        2,
        "verification replayed a solve"
    );
    Ok(())
}
