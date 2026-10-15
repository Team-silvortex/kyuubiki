use serde_json::{Value, json};
use std::error::Error;
use std::fs;

#[allow(dead_code)]
#[path = "support/agent_durable.rs"]
mod support;
use support::*;

fn store_id(agent: &LiveAgent) -> Result<String, Box<dyn Error>> {
    let reply = agent.request("owner", "describe_agent", json!({}))?;
    Ok(
        successful_result(&reply, "owner")["durable_execution"]["store_id"]
            .as_str()
            .ok_or("missing stable store identity")?
            .into(),
    )
}

fn owned_task(id: &str, execution: char) -> Value {
    let mut task = task(execution, "same-job");
    task["expected_store_id"] = json!(id);
    task
}

#[test]
fn replacing_a_mesh_endpoint_cannot_redirect_fenced_tasks_or_recovery() -> Result<(), Box<dyn Error>>
{
    let mut a = LiveAgent::start_with_durable_store()?;
    let mut b = LiveAgent::start_with_durable_store()?;
    a.restart_in_mesh(
        "owner-isolation",
        vec![format!("127.0.0.1:{}", b.port)],
        1000,
    )?;
    b.restart_in_mesh(
        "owner-isolation",
        vec![format!("127.0.0.1:{}", a.port)],
        1000,
    )?;
    let original = store_id(&a)?;
    let replacement = store_id(&b)?;
    assert_ne!(original, replacement);
    fs::write(&a.hold_path, "same-job")?;
    let submitted = a.request(
        "same-request",
        "submit_durable_agent_task",
        owned_task(&original, '4'),
    )?;
    assert_eq!(
        successful_result(&submitted, "same-request")["status"],
        "running"
    );
    a.stop_process()?;

    b.stop_process()?;
    b.port = a.port; // A different ledger now occupies the exact original address.
    b.start_process()?;
    fs::write(&b.hold_path, "same-job")?;
    let other = b.request(
        "same-request",
        "submit_durable_agent_task",
        owned_task(&replacement, '4'),
    )?;
    let other = successful_result(&other, "same-request");
    assert_eq!(other["status"], "running");
    for (method, params) in [
        ("submit_durable_agent_task", owned_task(&original, '5')),
        ("run_durable_agent_task", owned_task(&original, '5')),
        (
            "fetch_durable_agent_execution",
            json!({"execution_id":"4".repeat(32), "expected_store_id":original}),
        ),
        (
            "list_durable_agent_executions",
            json!({"limit":24, "after":null, "expected_store_id":original}),
        ),
        (
            "cancel_durable_agent_execution",
            json!({"execution_id":"4".repeat(32), "expected_store_id":original,
            "process_instance_id":other["process_instance_id"], "request_digest":other["request_digest"]}),
        ),
    ] {
        // Even valid replacement-process cancellation metadata cannot bypass the owner fence.
        let reply = a.request("old-caller", method, params)?;
        assert_eq!(reply["ok"], false, "{method}: {reply}");
        assert!(
            reply["error"]["message"]
                .as_str()
                .unwrap()
                .contains("store identity mismatch")
        );
        assert_eq!(
            reply["error"]["details"]["automatic_replay_authorized"],
            false
        );
    }
    assert_eq!(fetch(&b, '5')?["status"], "not_found");
    assert_eq!(lifecycle(&b, "still-owned")?["active_execution_count"], 1);
    fs::remove_file(&b.hold_path)?;
    assert_eq!(
        wait_status(&b, '4', "completed")?["response"]["result"]["max_stress"],
        10.0
    );
    b.stop_process()?;

    fs::remove_file(&a.hold_path)?;
    a.start_process()?;
    assert_eq!(store_id(&a)?, original);
    let inspected = a.request(
        "recover",
        "fetch_durable_agent_execution",
        json!({"execution_id":"4".repeat(32), "expected_store_id":original}),
    )?;
    let recovered = successful_result(&inspected, "recover");
    assert_eq!(recovered["status"], "outcome_unknown");
    assert_eq!(recovered["store_id"], original);
    let duplicate = a.request(
        "duplicate",
        "submit_durable_agent_task",
        owned_task(&original, '4'),
    )?;
    assert_eq!(successful_result(&duplicate, "duplicate"), recovered);
    assert_eq!(lifecycle(&a, "no-replay")?["active_execution_count"], 0);
    let fresh = a.request(
        "new-id",
        "submit_durable_agent_task",
        owned_task(&original, '6'),
    )?;
    assert_eq!(successful_result(&fresh, "new-id")["status"], "running");
    assert_eq!(
        wait_status(&a, '6', "completed")?["response"]["result"]["max_stress"],
        10.0
    );
    Ok(())
}
