use super::agent_support::LiveAgent;
use super::orchestra_completion::{bar_task, batch, http_json};
use super::{
    Scratch, parse_json_output, run_headless_command, start_live_server_with_state, write_temp_json,
};
use kyuubiki_headless_sdk::{
    HeadlessExecutionBatchStep, HeadlessExecutor, HeadlessExecutorError, HeadlessExecutorOutcome,
    HeadlessRisk, ServiceHeadlessExecutor, execute_batch_with_executor, validate_batch,
};
use serde_json::{Value, json};
use std::{error::Error, fs};

struct RecordingService {
    service: ServiceHeadlessExecutor,
    calls: Vec<String>,
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
        self.service.execute_step(action, index, payload)
    }
}

#[test]
fn imported_delete_risk_cannot_bypass_confirmation_or_run_earlier_work()
-> Result<(), Box<dyn Error>> {
    let owner = LiveAgent::start()?;
    let peer = LiveAgent::start()?;
    let data = Scratch::new()?;
    let server =
        start_live_server_with_state(Some(owner.port()), Some(peer.port()), Some(&data.0))?;
    let base_url = format!("http://127.0.0.1:{}", server.port);
    let mut executor = RecordingService {
        service: ServiceHeadlessExecutor::new(&base_url),
        calls: vec![],
    };
    // Only this test-owned temporary project's deletion is ever authorized.
    let created = executor
        .service
        .execute_step("project_create", 1, &json!({"name":"owned-risk-target"}))
        .map_err(|error| error.message)?;
    let target = created.result["project_id"]
        .as_str()
        .ok_or("missing created project id")?;
    let before = http_json(server.port, "/api/v1/projects", None)?.1;
    assert!(
        before["projects"]
            .as_array()
            .unwrap()
            .iter()
            .any(|p| p["project_id"] == target)
    );

    let mut document = batch(
        bar_task("risk-gate-unused-source", 0.01),
        Some("earlier-write-must-not-exist"),
    );
    document.steps.push(HeadlessExecutionBatchStep {
        index: 3,
        action: "project_delete".into(),
        risk: HeadlessRisk::Destructive,
        payload: json!({"project_id":target}),
    });
    assert!(validate_batch(&document).ok);
    for risk in [HeadlessRisk::Normal, HeadlessRisk::Sensitive] {
        let mut imported = serde_json::to_value(&document)?;
        imported["steps"][2]["risk"] = serde_json::to_value(risk)?;
        let imported = serde_json::from_value(imported)?;
        for (sensitive, destructive) in [(false, false), (true, false), (false, true), (true, true)]
        {
            let report =
                execute_batch_with_executor(&imported, &mut executor, sensitive, destructive);
            assert_eq!(report.status, "invalid", "{report:?}");
            assert_eq!(report.executed_step_count, 0);
            assert!(report.steps.is_empty());
            assert!(
                executor.calls.is_empty(),
                "malformed batch reached service executor"
            );
        }
        let source = write_temp_json("imported-delete-risk", &serde_json::to_value(&imported)?);
        let report_path = source.parent().unwrap().join("run-report.json");
        let output = run_headless_command(&[
            "run",
            source.to_str().unwrap(),
            "--execute",
            "--executor",
            "service",
            "--api-base-url",
            &base_url,
            "--allow-sensitive",
            "--allow-destructive",
            "--json",
            "--report-out",
            report_path.to_str().unwrap(),
        ]);
        assert!(
            !output.status.success(),
            "CLI accepted forged risk: {}",
            server.logs()
        );
        let report = parse_json_output(&output);
        assert_eq!(report["status"], "invalid");
        assert_eq!(report["executed_step_count"], 0);
        assert!(report["steps"].as_array().unwrap().is_empty());
        assert!(
            report["validation"]["issues"]
                .as_array()
                .unwrap()
                .iter()
                .any(|issue| {
                    issue
                        .as_str()
                        .unwrap()
                        .contains("risk must match action contract")
                })
        );
        assert_eq!(
            report["execution_summary"]["failure"]["error_code"],
            "kyuubiki.headless.document_validation"
        );
        assert_eq!(
            report["execution_summary"]["failure"]["stage"],
            "batch_validation"
        );
        assert_eq!(
            report,
            serde_json::from_slice::<Value>(&fs::read(report_path)?)?
        );
        let error: Value = serde_json::from_slice(&output.stderr)?;
        assert_eq!(error["error"]["retryable"], false);
        assert_eq!(http_json(server.port, "/api/v1/projects", None)?.1, before);
    }
    for agent in [&owner, &peer] {
        assert_eq!(
            agent.request("risk-gate-count", "describe_agent", json!({}))?["result"]["watchdog"]["total_started_execution_count"],
            0
        );
    }
    assert!(
        !data.0.join("operator-task-dispatches").exists(),
        "invalid batch retained an execution attempt"
    );

    // Repairing metadata restores the specific gate; sensitive approval is not delete approval.
    document.steps = vec![HeadlessExecutionBatchStep {
        index: 1,
        action: "project_delete".into(),
        risk: HeadlessRisk::Destructive,
        payload: json!({"project_id":target}),
    }];
    for sensitive in [false, true] {
        let report = execute_batch_with_executor(&document, &mut executor, sensitive, false);
        assert_eq!(report.status, "blocked");
        assert_eq!(report.executed_step_count, 0);
        assert_eq!(
            report.blocked_by_confirmation.unwrap().risk,
            HeadlessRisk::Destructive
        );
        assert!(executor.calls.is_empty());
        assert_eq!(http_json(server.port, "/api/v1/projects", None)?.1, before);
    }
    let repaired = write_temp_json(
        "explicit-delete-approval",
        &serde_json::to_value(&document)?,
    );
    let blocked = run_headless_command(&[
        "run",
        repaired.to_str().unwrap(),
        "--execute",
        "--executor",
        "service",
        "--api-base-url",
        &base_url,
        "--allow-sensitive",
        "--json",
    ]);
    assert!(!blocked.status.success());
    assert_eq!(parse_json_output(&blocked)["status"], "blocked");
    assert_eq!(http_json(server.port, "/api/v1/projects", None)?.1, before);
    let allowed = execute_batch_with_executor(&document, &mut executor, false, true);
    assert_eq!(allowed.status, "ok", "{allowed:?}");
    assert_eq!(allowed.executed_step_count, 1);
    assert_eq!(executor.calls, ["project_delete"]);
    let after = http_json(server.port, "/api/v1/projects", None)?.1;
    let expected = before["projects"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|p| p["project_id"] != target)
        .cloned()
        .collect::<Vec<_>>();
    assert_eq!(after["projects"], json!(expected));
    for agent in [&owner, &peer] {
        assert_eq!(
            agent.request("risk-gate-final-count", "describe_agent", json!({}))?["result"]["watchdog"]
                ["total_started_execution_count"],
            0
        );
    }
    Ok(())
}
