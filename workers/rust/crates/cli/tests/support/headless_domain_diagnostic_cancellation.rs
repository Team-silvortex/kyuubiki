use super::agent_support::{LiveAgent, wait_for_lifecycle};
use super::diagnostic_domains::{DOMAINS, Domain};
use super::orchestra_cancellation::{assert_cancelled, cancel_execution};
use super::orchestra_completion::http_json;
use super::start_live_server_with_agent;
use super::transport_research::assert_retained;
use super::write_ack_loss::count;
use kyuubiki_headless_sdk::{HeadlessExecutor, ServiceHeadlessExecutor};
use serde_json::{Value, json};
use std::error::Error;

fn scan_payload(domain: Domain) -> Value {
    let mut raw = domain.raw();
    // Repeat computed records to exercise reducer cadence, not a larger physical mesh.
    for group in ["nodes", "elements"] {
        let records = raw[group].as_array().unwrap();
        raw[group] = json!(
            (0..129)
                .map(|index| records[index % records.len()].clone())
                .collect::<Vec<_>>()
        );
    }
    raw
}

#[test]
fn real_cross_domain_native_diagnostic_cancellation_preserves_attempts_and_requires_explicit_rerun()
-> Result<(), Box<dyn Error>> {
    let agent =
        LiveAgent::start_with_solver_hold("result_diagnostics", "run_operator_task_ir", "1")?;
    let server = start_live_server_with_agent(Some(agent.port()))?;
    let mut executor =
        ServiceHeadlessExecutor::try_new(&format!("http://127.0.0.1:{}", server.port))
            .map_err(|error| error.message)?;
    let (_, before) = http_json(server.port, "/api/v1/projects", None)?;
    for (index, domain) in DOMAINS.into_iter().enumerate() {
        let payload = scan_payload(domain);
        let expected =
            kyuubiki_engine::run_extract_operator(domain.diagnostic, payload.clone(), json!({}))
                .unwrap();
        let task = domain.task(
            &format!("cancel-native-{}", domain.name),
            domain.diagnostic,
            "extract",
            payload,
            json!({}),
        );
        let report = cancel_execution(&agent, server.port, &task, Some("result_diagnostics"))?;
        assert_cancelled(&report, &task, "execute_diagnostics");
        assert!(
            report.steps[0].result_preview["failure_receipt"]["message"]
                .as_str()
                .unwrap()
                .contains("result_diagnostics after 64 steps")
        );
        wait_for_lifecycle(&agent, "accepting", 0)?;
        assert_eq!(http_json(server.port, "/api/v1/projects", None)?.1, before);
        assert_eq!(count(&agent)?, index as u64 * 2 + 1);
        let task_id = task["task_id"].as_str().unwrap();
        let digest = task["integrity"]["task_digest"].as_str().unwrap();
        let cancelled = executor
            .inspect_operator_task_dispatch(task_id, digest)
            .map_err(|error| error.message)?;
        assert_eq!(cancelled["attempts"].as_array().unwrap().len(), 1);
        assert_eq!(cancelled["attempts"][0]["state"], "observed_failed");
        assert_eq!(cancelled["terminal_result_available"], false);
        let old = cancelled["attempts"][0]["attempt_id"].clone();

        let recovered = executor
            .execute_step("operator_task_execute", 1, &json!({"task":task}))
            .map_err(|error| error.message)?;
        assert_eq!(recovered.status, "executed");
        assert_eq!(recovered.result["task_digest"], digest);
        assert_eq!(
            recovered.result["result"]["operator_task_ir_status"],
            "executed"
        );
        assert_retained(
            &recovered.result["result"]["result"],
            &expected,
            "diagnose.summary",
        );
        wait_for_lifecycle(&agent, "accepting", 0)?;
        let retained = executor
            .inspect_operator_task_dispatch(task_id, digest)
            .map_err(|error| error.message)?;
        let attempts = retained["attempts"].as_array().unwrap();
        assert_eq!(attempts.len(), 2);
        assert!(
            attempts.iter().any(
                |attempt| attempt["attempt_id"] == old && attempt["state"] == "observed_failed"
            )
        );
        assert!(
            attempts
                .iter()
                .any(|attempt| attempt["attempt_id"] != old
                    && attempt["state"] == "observed_executed")
        );
        assert_eq!(count(&agent)?, (index as u64 + 1) * 2);
    }
    assert_eq!(count(&agent)?, 10);
    Ok(())
}
