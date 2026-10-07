use super::ack_loss_proxy::AckLossProxy;
use super::agent_support::{LiveAgent, wait_for_lifecycle};
use super::orchestra_completion::http_json;
use super::write_ack_loss::{assert_failure, count, document, run_cli};
use super::{
    Scratch, parse_json_output, run_headless_command, start_live_server_with_state, write_temp_json,
};
use kyuubiki_headless_sdk::{
    HeadlessExecutor, ServiceHeadlessExecutor, execute_batch_with_executor,
};
use serde_json::{Value, json};
use std::{error::Error, fs};

type Result<T> = std::result::Result<T, Box<dyn Error>>;

#[test]
fn real_committed_projects_with_oversized_acknowledgements_are_not_replayed() -> Result<()> {
    let owner = LiveAgent::start()?;
    let peer = LiveAgent::start()?;
    let data = Scratch::new()?;
    let server =
        start_live_server_with_state(Some(owner.port()), Some(peer.port()), Some(&data.0))?;
    let proxy = AckLossProxy::http_with_oversized_length(
        server.port,
        "POST",
        "/api/v1/projects",
        64 * 1024 * 1024,
    )?;
    let batch = document("project_create", json!({"name":"response-budget-sdk"}));
    let report = execute_batch_with_executor(
        &batch,
        &mut ServiceHeadlessExecutor::new(&format!("http://127.0.0.1:{}", proxy.port)),
        false,
        false,
    );
    let report = serde_json::to_value(report)?;
    assert_failure(&report);
    assert!(
        report["execution_summary"]["failure"]["message"]
            .as_str()
            .unwrap()
            .contains("service_response_limit_exceeded:")
    );
    // This is a distinct explicit experiment, not replay of the unknown write.
    run_cli(
        &document("project_create", json!({"name":"response-budget-cli"})),
        &proxy,
    )?;
    let capture = proxy.capture();
    assert!(capture.errors.is_empty(), "{capture:?}");
    assert_eq!(capture.requests.len(), 2);
    assert_eq!(capture.discarded.len(), 2);
    assert_eq!(capture.replies.len(), 2);
    let stored = http_json(server.port, "/api/v1/projects", None)?.1;
    let projects = stored["projects"].as_array().unwrap();
    assert_eq!(projects.len(), 2);
    for reply in &capture.discarded {
        assert!(
            projects
                .iter()
                .any(|p| p["project_id"] == reply["project"]["project_id"]
                    && p["name"] == reply["project"]["name"])
        );
    }
    assert_eq!(
        http_json(server.port, "/api/v1/jobs", None)?.1["jobs"],
        json!([])
    );
    assert_eq!(count(&owner)? + count(&peer)?, 0);
    Ok(())
}

fn assert_read_failure(report: &Value) {
    assert_eq!(report["status"], "failed");
    assert_eq!(report["executed_step_count"], 0);
    assert_eq!(report["steps"].as_array().unwrap().len(), 1);
    let failure = &report["execution_summary"]["failure"];
    assert_eq!(
        failure["error_code"],
        "kyuubiki.headless.service_response_limit_exceeded"
    );
    assert_eq!(failure["stage"], "transport");
    assert_eq!(failure["retryable"], false);
    assert_eq!(failure["retry_strategy"], "none");
    for key in ["job", "job_id", "result", "raw"] {
        assert!(report["steps"][0]["result_preview"].get(key).is_none());
    }
}

#[test]
fn real_completed_result_oversize_stops_sdk_and_cli_without_recomputation() -> Result<()> {
    let owner = LiveAgent::start()?;
    let peer = LiveAgent::start()?;
    let data = Scratch::new()?;
    let server =
        start_live_server_with_state(Some(owner.port()), Some(peer.port()), Some(&data.0))?;
    let mut direct = ServiceHeadlessExecutor::new(&format!("http://127.0.0.1:{}", server.port));
    let submitted = direct.execute_step("solve_bar_1d", 1, &json!({"model":{"length":1.0,"area":0.01,"youngs_modulus_gpa":210.0,"elements":4,"tip_force":1000.0}})).map_err(|e|e.message)?.result;
    let id = submitted["job_id"].as_str().unwrap();
    direct
        .execute_step(
            "job_wait",
            1,
            &json!({"job_id":id,"timeout_ms":5000,"interval_ms":10}),
        )
        .map_err(|e| e.message)?;
    let path = format!("/api/v1/jobs/{id}");
    let original = http_json(server.port, &path, None)?.1;
    assert_eq!(original["job"]["status"], "completed");
    assert_eq!(count(&owner)? + count(&peer)?, 1);
    let proxy =
        AckLossProxy::http_with_oversized_length(server.port, "GET", &path, 512 * 1024 * 1024)?;
    let source_document = document("job_fetch", json!({"job_id":id}));
    let result = execute_batch_with_executor(
        &source_document,
        &mut ServiceHeadlessExecutor::new(&format!("http://127.0.0.1:{}", proxy.port)),
        false,
        false,
    );
    assert_read_failure(&serde_json::to_value(result)?);
    let source = write_temp_json(
        "real-response-budget",
        &serde_json::to_value(source_document)?,
    );
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
    assert_read_failure(&report);
    assert_eq!(
        report,
        serde_json::from_slice::<Value>(&fs::read(report_path)?)?
    );
    let error: Value = serde_json::from_slice(&output.stderr)?;
    assert_eq!(error["error"]["retryable"], false);
    let capture = proxy.capture();
    assert!(capture.errors.is_empty(), "{capture:?}");
    assert_eq!(capture.requests.len(), 2);
    assert_eq!(capture.discarded, [original.clone(), original.clone()]);
    assert_eq!(capture.replies.len(), 2);
    let fetched = direct
        .execute_step("result_fetch", 1, &json!({"job_id":id}))
        .map_err(|e| e.message)?
        .result;
    assert_eq!(fetched["result"], original["result"]);
    let displacement = fetched["result"]["tip_displacement"].as_f64().unwrap();
    let expected = 1000.0 / (0.01 * 210e9);
    assert!((displacement - expected).abs() / expected < 1e-12);
    assert_eq!(http_json(server.port, &path, None)?.1, original);
    assert_eq!(
        http_json(server.port, "/api/v1/projects", None)?.1["projects"],
        json!([])
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
    assert_eq!(count(&owner)? + count(&peer)?, 1);
    Ok(())
}
