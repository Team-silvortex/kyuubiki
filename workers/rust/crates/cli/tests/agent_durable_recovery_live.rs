use serde_json::{Value, json};
use std::{
    error::Error,
    fs, thread,
    time::{Duration, Instant},
};

#[allow(dead_code)]
#[path = "support/agent_lifecycle.rs"]
mod support;
use support::*;

fn submit(agent: &LiveAgent, id: char, job: &str) -> Result<Value, Box<dyn Error>> {
    let response = agent.request("reused-request", "submit_durable_agent_task",
        json!({"execution_id":id.to_string().repeat(32), "method":"solve_bar_1d", "params":solver_params(job)}))?;
    Ok(successful_result(&response, "reused-request").clone())
}

fn query(report: &Value) -> Value {
    json!({"execution_id":report["execution_id"], "process_instance_id":report["process_instance_id"], "request_digest":report["request_digest"]})
}

fn inspect(agent: &LiveAgent, id: char, expected: &str) -> Result<Value, Box<dyn Error>> {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let response = agent.request(
            "inspect",
            "fetch_durable_agent_execution",
            json!({"execution_id":id.to_string().repeat(32)}),
        )?;
        let report = successful_result(&response, "inspect");
        if report["status"] == expected {
            return Ok(report.clone());
        }
        if Instant::now() > deadline {
            return Err(format!("expected {expected}: {report}").into());
        }
        thread::sleep(Duration::from_millis(10));
    }
}

#[test]
fn index_and_exact_owner_cancel_survive_restart_without_poisoning_reused_job_and_request()
-> Result<(), Box<dyn Error>> {
    let mut agent = LiveAgent::start_with_durable_store()?;
    fs::write(&agent.hold_path, "shared-job")?;
    let first = submit(&agent, 'a', "shared-job")?;
    let mut wrong = query(&first);
    wrong["request_digest"] = json!("0".repeat(64));
    assert_eq!(
        agent.request("wrong", "cancel_durable_agent_execution", wrong)?["ok"],
        false
    );
    assert_eq!(inspect(&agent, 'a', "running")?["status"], "running");
    let cancel = agent.request("cancel", "cancel_durable_agent_execution", query(&first))?;
    assert_eq!(successful_result(&cancel, "cancel")["status"], "requested");
    assert_eq!(cancel["result"]["execution_terminal_confirmed"], false);
    let failed = inspect(&agent, 'a', "failed")?;
    assert_eq!(failed["response"]["error"]["code"], "cancelled");
    wait_for_lifecycle(&agent, "accepting", 0)?;
    agent.stop_process()?;
    agent.start_process()?;
    let index = agent.request(
        "list",
        "list_durable_agent_executions",
        json!({"limit":24,"after":null}),
    )?;
    assert_eq!(
        index["result"]["executions"][0]["execution_id"],
        "a".repeat(32)
    );
    assert!(index["result"]["executions"][0].get("response").is_none());
    assert_eq!(inspect(&agent, 'a', "failed")?, failed);
    let second = submit(&agent, 'b', "shared-job")?;
    assert_eq!(second["status"], "running");
    let old = agent.request(
        "old-cancel",
        "cancel_durable_agent_execution",
        query(&first),
    )?;
    assert_eq!(old["result"]["status"], "not_running");
    assert_eq!(old["result"]["cancel_registered"], false);
    assert_eq!(inspect(&agent, 'b', "running")?["status"], "running");
    fs::remove_file(&agent.hold_path)?;
    assert_eq!(
        inspect(&agent, 'b', "completed")?["response"]["result"]["max_stress"],
        10.0
    );
    Ok(())
}

#[test]
fn interrupted_admission_is_discoverable_but_cannot_cancel_new_owner_or_replay()
-> Result<(), Box<dyn Error>> {
    let mut agent = LiveAgent::start_with_durable_store()?;
    fs::write(&agent.hold_path, "interrupted")?;
    let first = submit(&agent, 'c', "interrupted")?;
    agent.stop_process()?;
    agent.start_process()?;
    let unknown = inspect(&agent, 'c', "outcome_unknown")?;
    assert_eq!(unknown["automatic_replay_authorized"], false);
    let second = submit(&agent, 'd', "interrupted")?;
    assert_eq!(second["status"], "running");
    let old = agent.request(
        "cancel-old",
        "cancel_durable_agent_execution",
        query(&first),
    )?;
    assert_eq!(old["result"]["status"], "not_running");
    assert_eq!(inspect(&agent, 'd', "running")?["status"], "running");
    let index = agent.request("list", "list_durable_agent_executions", json!({"limit":1}))?;
    assert_eq!(index["result"]["next_cursor"], "c".repeat(32));
    let page = agent.request(
        "next",
        "list_durable_agent_executions",
        json!({"limit":1,"after":"c".repeat(32)}),
    )?;
    assert_eq!(
        page["result"]["executions"][0]["execution_id"],
        "d".repeat(32)
    );
    assert!(page["result"]["next_cursor"].is_null());
    fs::remove_file(&agent.hold_path)?;
    inspect(&agent, 'd', "completed")?;
    assert_eq!(inspect(&agent, 'c', "outcome_unknown")?, unknown);
    Ok(())
}

#[test]
fn recovery_controls_are_advertised_and_fail_closed_without_a_store() -> Result<(), Box<dyn Error>>
{
    let agent = LiveAgent::start()?;
    let descriptor = agent.request("descriptor", "describe_agent", json!({}))?;
    assert_eq!(
        descriptor["result"]["durable_execution"]["recovery_controls"]["cancel_method"],
        "cancel_durable_agent_execution"
    );
    for (method, params) in [
        ("list_durable_agent_executions", json!({"limit":24})),
        (
            "cancel_durable_agent_execution",
            json!({"execution_id":"a".repeat(32), "request_digest":"b".repeat(64), "process_instance_id":"process"}),
        ),
    ] {
        assert_eq!(agent.request("disabled", method, params)?["ok"], false);
    }
    Ok(())
}
