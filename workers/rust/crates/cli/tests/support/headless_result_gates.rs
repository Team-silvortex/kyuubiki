use super::agent_support::{LiveAgent, wait_for_lifecycle};
use super::orchestra_completion::{bar_task, batch, http_json};
use super::{
    Scratch, parse_json_output, run_headless_command, start_live_server_with_result_fixtures,
    start_live_server_with_state, write_temp_json,
};
use kyuubiki_headless_sdk::{
    HeadlessExecutionBatchStep, HeadlessExecutor, HeadlessExecutorError, HeadlessExecutorOutcome,
    HeadlessRisk, ServiceHeadlessExecutor, execute_batch_with_executor,
};
use serde_json::{Value, json};
use std::{error::Error, fs};

struct RecordingService {
    service: ServiceHeadlessExecutor,
    calls: Vec<String>,
    payloads: Vec<Value>,
}

impl RecordingService {
    fn new(port: u16) -> Self {
        Self {
            service: ServiceHeadlessExecutor::new(&format!("http://127.0.0.1:{port}")),
            calls: vec![],
            payloads: vec![],
        }
    }
}

impl HeadlessExecutor for RecordingService {
    fn name(&self) -> &'static str {
        "service"
    }
    fn execute_step(
        &mut self,
        action: &str,
        index: usize,
        payload: &Value,
    ) -> Result<HeadlessExecutorOutcome, HeadlessExecutorError> {
        self.calls.push(action.into());
        self.payloads.push(payload.clone());
        self.service.execute_step(action, index, payload)
    }
}

fn count(agent: &LiveAgent) -> Result<u64, Box<dyn Error>> {
    Ok(agent.request("result-gate-count", "describe_agent", json!({}))?
        ["result"]["watchdog"]["total_started_execution_count"].as_u64().unwrap())
}

#[test]
fn real_retained_incomplete_results_fail_sdk_and_cli_before_downstream_writes()
-> Result<(), Box<dyn Error>> {
    let owner = LiveAgent::start()?;
    let peer = LiveAgent::start()?;
    let data = Scratch::new()?;
    let server = start_live_server_with_result_fixtures(
        Some(owner.port()),
        Some(peer.port()),
        Some(&data.0),
        true,
    )?;
    let base_url = format!("http://127.0.0.1:{}", server.port);
    let projects = http_json(server.port, "/api/v1/projects", None)?.1;
    let jobs = http_json(server.port, "/api/v1/jobs", None)?.1;
    let results = http_json(server.port, "/api/v1/results", None)?.1;
    for status in ["queued", "solving", "failed", "cancelled"] {
        let id = format!("owned-result-gate-{status}");
        let path = format!("/api/v1/jobs/{id}");
        let before = http_json(server.port, &path, None)?.1;
        assert_eq!(before["job"]["status"], status);
        assert_eq!(before["job"]["has_result"], true);
        assert!(
            before["result"]["artifacts"]["partial"]["fixture"]
                .as_bool()
                .unwrap()
        );
        assert!(
            http_json(server.port, &format!("/api/v1/results/{id}"), None)?.1["result"].is_object()
        );
        for prefer in [true, false] {
            let mut document = batch(
                bar_task("unused-result-gate-task", 0.01),
                Some("must-not-follow-incomplete-result"),
            );
            document.steps[0].action = "result_fetch".into();
            document.steps[0].payload = json!({"job_id":id,"prefer_job_result":prefer});
            let mut executor = RecordingService::new(server.port);
            let report = execute_batch_with_executor(&document, &mut executor, false, false);
            assert_eq!(report.status, "failed", "{report:?}");
            assert_eq!(report.executed_step_count, 0);
            assert_eq!(report.steps.len(), 1);
            assert_eq!(executor.calls, ["result_fetch"]);
            let failure = report.execution_summary.failure.unwrap();
            assert_eq!(
                failure.error_code,
                "kyuubiki.headless.job_result_unavailable"
            );
            assert!(!failure.retryable);
            let source = write_temp_json(
                "retained-incomplete-result",
                &serde_json::to_value(&document)?,
            );
            let report_path = source.parent().unwrap().join("run-report.json");
            let output = run_headless_command(&[
                "run",
                source.to_str().unwrap(),
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
                "incomplete result accepted: {}",
                server.logs()
            );
            let report = parse_json_output(&output);
            assert_eq!(report["status"], "failed");
            assert_eq!(report["executed_step_count"], 0);
            assert_eq!(report["steps"].as_array().unwrap().len(), 1);
            assert_eq!(
                report["execution_summary"]["failure"]["error_code"],
                "kyuubiki.headless.job_result_unavailable"
            );
            assert_eq!(
                report,
                serde_json::from_slice::<Value>(&fs::read(report_path)?)?
            );
            let error: Value = serde_json::from_slice(&output.stderr)?;
            assert_eq!(error["error"]["code"], "headless_execution_failed");
            assert_eq!(error["error"]["retryable"], false);
        }
        assert_eq!(http_json(server.port, &path, None)?.1, before);
    }
    assert_eq!(
        http_json(server.port, "/api/v1/projects", None)?.1,
        projects
    );
    assert_eq!(http_json(server.port, "/api/v1/jobs", None)?.1, jobs);
    assert_eq!(http_json(server.port, "/api/v1/results", None)?.1, results);
    assert_eq!(count(&owner)?, 0);
    assert_eq!(count(&peer)?, 0);
    assert!(!data.0.join("operator-task-dispatches").exists());
    Ok(())
}

#[test]
fn real_completed_bar_result_can_be_fetched_and_continued_without_resubmission()
-> Result<(), Box<dyn Error>> {
    let owner = LiveAgent::start()?;
    let peer = LiveAgent::start()?;
    let data = Scratch::new()?;
    let server =
        start_live_server_with_state(Some(owner.port()), Some(peer.port()), Some(&data.0))?;
    let mut executor = RecordingService::new(server.port);
    let mut document = batch(bar_task("unused-job-bar-task", 0.01), None);
    document.steps = vec![
        HeadlessExecutionBatchStep {
            index: 1,
            action: "solve_bar_1d".into(),
            risk: HeadlessRisk::Normal,
            payload: json!({"model":{"length":1.0,"area":0.01,"youngs_modulus_gpa":210,"elements":4,"tip_force":1000}}),
        },
        HeadlessExecutionBatchStep {
            index: 2,
            action: "job_wait".into(),
            risk: HeadlessRisk::Normal,
            payload: json!({"job_id":"{{steps.1.result.job_id}}","interval_ms":10,"timeout_ms":5000}),
        },
        HeadlessExecutionBatchStep {
            index: 3,
            action: "result_fetch".into(),
            risk: HeadlessRisk::Normal,
            payload: json!({"job_id":"{{steps.1.result.job_id}}","prefer_job_result":false}),
        },
        HeadlessExecutionBatchStep {
            index: 4,
            action: "project_create".into(),
            risk: HeadlessRisk::Normal,
            payload: json!({"name":"{{steps.3.result.job_id}}","metadata":{"forwarded":"{{steps.3.result.result}}"}}),
        },
    ];
    let report = execute_batch_with_executor(&document, &mut executor, false, false);
    assert_eq!(report.status, "ok", "{report:?}");
    assert_eq!(report.executed_step_count, 4);
    assert_eq!(
        executor.calls,
        ["solve_bar_1d", "job_wait", "result_fetch", "project_create"]
    );
    let id = report.steps[0].result_preview["job_id"].as_str().unwrap();
    assert_eq!(report.steps[1].result_preview["job_id"], id);
    assert_eq!(report.steps[1].result_preview["status"], "completed");
    let result = &report.steps[2].result_preview["result"];
    let displacement = result["tip_displacement"]
        .as_f64()
        .ok_or("missing real bar displacement")?;
    assert!((displacement / (1000.0 / (210e9 * 0.01)) - 1.0).abs() < 1e-12);
    assert_eq!(executor.payloads[3]["metadata"]["forwarded"], *result);
    assert!(report.steps[2].result_preview.get("raw").is_none());
    let projects = http_json(server.port, "/api/v1/projects", None)?.1;
    assert_eq!(projects["projects"].as_array().unwrap().len(), 1);
    assert_eq!(projects["projects"][0]["name"], id);
    wait_for_lifecycle(&owner, "accepting", 0)?;
    wait_for_lifecycle(&peer, "accepting", 0)?;
    assert_eq!(
        count(&owner)? + count(&peer)?,
        1,
        "fetch or continuation reran the solver"
    );
    for prefer in [true, false] {
        let reread = executor
            .service
            .execute_step(
                "result_fetch",
                1,
                &json!({"job_id":id,"prefer_job_result":prefer}),
            )
            .map_err(|error| error.message)?;
        assert_eq!(reread.result["result"], *result);
        assert!(reread.result.get("raw").is_none());
    }
    assert_eq!(
        count(&owner)? + count(&peer)?,
        1,
        "result reread resubmitted work"
    );
    Ok(())
}
