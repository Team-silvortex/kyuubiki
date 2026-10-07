use super::agent_support::{LiveAgent, wait_for_lifecycle};
use super::orchestra_completion::{assert_bar, bar_task, batch, http_json};
use super::{
    Scratch, parse_json_output, run_headless_command, start_live_server_with_state, write_temp_json,
};
use kyuubiki_headless_sdk::{
    HeadlessExecutionBatchStep, HeadlessExecutor, HeadlessExecutorError, HeadlessExecutorOutcome,
    HeadlessRisk, HeadlessRunReport, ServiceHeadlessExecutor, execute_batch_with_executor,
    validate_batch,
};
use kyuubiki_protocol::compute_operator_task_digest;
use serde_json::{Value, json};
use std::error::Error;
use std::fs;

type Result<T> = std::result::Result<T, Box<dyn Error>>;

struct RecordingService {
    service: ServiceHeadlessExecutor,
    calls: Vec<String>,
}

impl RecordingService {
    fn new(port: u16) -> Self {
        Self {
            service: ServiceHeadlessExecutor::new(&format!("http://127.0.0.1:{port}")),
            calls: vec![],
        }
    }
}

// Observe public SDK calls without altering any request, receipt or completion status.
impl HeadlessExecutor for RecordingService {
    fn name(&self) -> &'static str {
        "service"
    }

    fn execute_step(
        &mut self,
        action: &str,
        index: usize,
        payload: &Value,
    ) -> std::result::Result<HeadlessExecutorOutcome, HeadlessExecutorError> {
        self.calls.push(action.into());
        self.service.execute_step(action, index, payload)
    }
}

fn count(agent: &LiveAgent) -> Result<u64> {
    Ok(agent.request("binding-execution-count", "describe_agent", json!({}))?
        ["result"]["watchdog"]["total_started_execution_count"].as_u64().unwrap())
}

fn assert_binding_failure(report: &HeadlessRunReport, executor: &RecordingService) {
    assert_eq!(report.status, "failed", "{report:?}");
    assert_eq!(report.executed_step_count, 1);
    assert_eq!(report.steps.len(), 2);
    assert_eq!(executor.calls.len(), 1, "dependent HTTP call was sent");
    assert_eq!(report.steps[0].status, "executed");
    assert_eq!(report.steps[1].status, "failed");
    let failure = report.execution_summary.failure.as_ref().unwrap();
    assert_eq!(failure.error_code, "kyuubiki.headless.binding_resolution");
    assert_eq!(failure.stage, "payload_resolution");
    assert!(!failure.retryable);
    assert_eq!(failure.retry_strategy, "none");
}

#[test]
fn real_collection_reply_missing_declared_binding_stops_before_writes() -> Result<()> {
    let owner = LiveAgent::start()?;
    let peer = LiveAgent::start()?;
    let data = Scratch::new()?;
    let server =
        start_live_server_with_state(Some(owner.port()), Some(peer.port()), Some(&data.0))?;
    let mut workflow = batch(bar_task("unused-binding-source", 0.01), Some("unused"));
    workflow.steps[0].action = "service_health".into();
    // A configurable read route returns an actual collection, not a health envelope.
    // Its successful HTTP delivery cannot fabricate the declared `service` output.
    workflow.steps[0].payload = json!({"path":"/api/v1/projects"});
    workflow.steps[1].payload = json!({"name":"{{steps.1.result.service}}"});
    workflow.steps.push(HeadlessExecutionBatchStep {
        index: 3,
        action: "project_create".into(),
        risk: HeadlessRisk::Normal,
        payload: json!({"name":"later-project-must-not-exist"}),
    });
    assert!(validate_batch(&workflow).ok);
    let projects = http_json(server.port, "/api/v1/projects", None)?.1;
    let mut executor = RecordingService::new(server.port);
    let report = execute_batch_with_executor(&workflow, &mut executor, false, false);
    assert_binding_failure(&report, &executor);
    assert!(report.steps[0].result_preview["projects"].is_array());
    assert!(report.steps[0].result_preview.get("service").is_none());
    let document = write_temp_json("real-missing-output", &serde_json::to_value(&workflow)?);
    let report_path = document.parent().unwrap().join("run-report.json");
    let base_url = format!("http://127.0.0.1:{}", server.port);
    let output = run_headless_command(&[
        "run",
        document.to_str().unwrap(),
        "--execute",
        "--executor",
        "service",
        "--api-base-url",
        &base_url,
        "--json",
        "--report-out",
        report_path.to_str().unwrap(),
    ]);
    assert!(
        !output.status.success(),
        "CLI accepted missing binding: {}",
        server.logs()
    );
    let cli_report = parse_json_output(&output);
    assert_eq!(cli_report["status"], "failed");
    assert_eq!(cli_report["executed_step_count"], 1);
    assert_eq!(cli_report["steps"].as_array().unwrap().len(), 2);
    assert_eq!(
        cli_report["execution_summary"]["failure"]["error_code"],
        "kyuubiki.headless.binding_resolution"
    );
    assert_eq!(
        cli_report,
        serde_json::from_slice::<Value>(&fs::read(report_path)?)?
    );
    let error: Value = serde_json::from_slice(&output.stderr)?;
    assert_eq!(error["error"]["code"], "headless_execution_failed");
    assert_eq!(error["error"]["retryable"], false);
    assert_eq!(
        http_json(server.port, "/api/v1/projects", None)?.1,
        projects
    );
    assert_eq!(count(&owner)?, 0);
    assert_eq!(count(&peer)?, 0);
    Ok(())
}

#[test]
fn real_completed_task_null_binding_continues_only_after_explicit_downstream_repair() -> Result<()>
{
    let owner = LiveAgent::start()?;
    let peer = LiveAgent::start()?;
    let data = Scratch::new()?;
    let server =
        start_live_server_with_state(Some(owner.port()), Some(peer.port()), Some(&data.0))?;
    let mut task = bar_task("binding-source-bar", 0.01);
    task["runtime_hints"]["placement_tags"] = json!(["owned-live-owner"]);
    task["integrity"]["task_digest"] = json!(compute_operator_task_digest(&task)?);
    let mut workflow = batch(task.clone(), Some("unused"));
    // Built-in solver tasks have a valid nullable package_ref, not a project name.
    workflow.steps[1].payload = json!({"name":"{{steps.1.result.package_ref}}"});
    workflow.steps.push(HeadlessExecutionBatchStep {
        index: 3,
        action: "project_create".into(),
        risk: HeadlessRisk::Normal,
        payload: json!({"name":"later-project-must-not-exist"}),
    });
    assert!(validate_batch(&workflow).ok);
    let before = http_json(server.port, "/api/v1/projects", None)?.1;
    let mut executor = RecordingService::new(server.port);
    let report = execute_batch_with_executor(&workflow, &mut executor, false, false);
    assert_binding_failure(&report, &executor);
    assert_bar(&report.steps[0].result_preview);
    assert!(report.steps[0].result_preview["package_ref"].is_null());
    wait_for_lifecycle(&owner, "accepting", 0)?;
    assert_eq!(count(&owner)?, 1);
    assert_eq!(count(&peer)?, 0);
    assert_eq!(http_json(server.port, "/api/v1/projects", None)?.1, before);

    let inspect = || {
        executor
            .service
            .inspect_operator_task_dispatch(
                task["task_id"].as_str().unwrap(),
                task["integrity"]["task_digest"].as_str().unwrap(),
            )
            .map_err(|e| e.message)
    };
    let dispatch = inspect()?;
    assert_eq!(dispatch["retained_attempt_count"], 1);
    let attempt = dispatch["attempts"][0]["attempt_id"].as_str().unwrap();
    let journal_path = data
        .0
        .join("operator-task-dispatches")
        .join(format!("{attempt}.json"));
    let journal = fs::read(&journal_path)?;
    let recovered = executor
        .service
        .fetch_operator_task_result(&task, attempt)
        .map_err(|e| e.message)?;
    assert_eq!(recovered["status"], "receipt_recovered");
    assert_eq!(recovered["outcome"], "executed");
    assert_eq!(recovered["automatic_replay_authorized"], false);
    assert_eq!(recovered["publication_performed"], false);
    assert_bar(&recovered["completion"]);
    for field in [
        "task_id",
        "task_digest",
        "operator_id",
        "program_id",
        "result",
    ] {
        assert_eq!(
            recovered["completion"][field],
            report.steps[0].result_preview[field]
        );
    }
    assert_eq!(
        fs::read(&journal_path)?,
        journal,
        "result read mutated journal"
    );

    // Caller-authored continuation uses a separately validated original result.
    // It contains only the repaired downstream step, never the completed source.
    let mut continuation = workflow.clone();
    continuation.workflow_id = "explicit-repaired-continuation".into();
    continuation.steps = vec![HeadlessExecutionBatchStep {
        index: 1,
        action: "project_create".into(),
        risk: HeadlessRisk::Normal,
        payload: json!({"name":recovered["completion"]["task_id"]}),
    }];
    let continued = execute_batch_with_executor(&continuation, &mut executor, false, false);
    assert_eq!(continued.status, "ok", "{continued:?}");
    assert_eq!(continued.executed_step_count, 1);
    assert_eq!(executor.calls, ["operator_task_execute", "project_create"]);
    let after = http_json(server.port, "/api/v1/projects", None)?.1;
    let projects = after["projects"].as_array().unwrap();
    assert_eq!(
        projects.len(),
        before["projects"].as_array().unwrap().len() + 1
    );
    assert!(projects.iter().any(|p| p["name"] == task["task_id"]));
    assert!(
        !projects
            .iter()
            .any(|p| p["name"] == "later-project-must-not-exist")
    );
    assert_eq!(
        count(&owner)?,
        1,
        "continuation replayed the source computation"
    );
    assert_eq!(count(&peer)?, 0, "continuation executed on a peer");
    assert_eq!(
        fs::read(&journal_path)?,
        journal,
        "continuation mutated source journal"
    );
    let dispatch = executor
        .service
        .inspect_operator_task_dispatch(
            task["task_id"].as_str().unwrap(),
            task["integrity"]["task_digest"].as_str().unwrap(),
        )
        .map_err(|e| e.message)?;
    assert_eq!(
        dispatch["retained_attempt_count"], 1,
        "continuation created another source attempt"
    );
    Ok(())
}
