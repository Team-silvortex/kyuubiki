use serde_json::{Value, json};
use std::error::Error;
use std::fs;

#[allow(dead_code)]
#[path = "support/agent_durable.rs"]
mod support;
use support::*;

fn owner(agent: &LiveAgent) -> Result<Value, Box<dyn Error>> {
    let response = agent.request("owner", "describe_agent", json!({}))?;
    let descriptor = successful_result(&response, "owner");
    assert_eq!(descriptor["runtime"]["runtime_mode"], "peer_mesh");
    Ok(descriptor["durable_execution"]["store_id"].clone())
}

fn owned_task(owner: &Value, execution: char, job: &str) -> Value {
    let mut params = task(execution, job);
    params["expected_store_id"] = owner.clone();
    params
}

fn unavailable(agent: &LiveAgent, method: &str, params: Value) -> Result<(), Box<dyn Error>> {
    let response = agent.request("missing-record", method, params)?;
    assert_eq!(response["ok"], false, "{method}: {response}");
    assert_eq!(response["error"]["code"], "durable_execution_unavailable");
    assert_eq!(
        response["error"]["details"]["automatic_replay_authorized"],
        false
    );
    assert!(response.get("result").is_none_or(Value::is_null));
    Ok(())
}

fn reject_missing(
    agent: &LiveAgent,
    owner: &Value,
    execution: char,
    job: &str,
) -> Result<(), Box<dyn Error>> {
    for method in ["submit_durable_agent_task", "run_durable_agent_task"] {
        unavailable(agent, method, owned_task(owner, execution, job))?;
    }
    for params in [
        json!({"execution_id":execution.to_string().repeat(32)}),
        json!({"execution_id":execution.to_string().repeat(32), "expected_store_id":owner}),
    ] {
        unavailable(agent, "fetch_durable_agent_execution", params)?;
    }
    for after in [Value::Null, json!("f".repeat(32))] {
        unavailable(
            agent,
            "list_durable_agent_executions",
            json!({"limit":24, "after":after, "expected_store_id":owner}),
        )?;
    }
    Ok(())
}

fn complete(agent: &LiveAgent, owner: &Value, execution: char) -> Result<Value, Box<dyn Error>> {
    let response = agent.request(
        "healthy-work",
        "submit_durable_agent_task",
        owned_task(owner, execution, "healthy"),
    )?;
    assert_eq!(
        successful_result(&response, "healthy-work")["status"],
        "running"
    );
    let completed = wait_status(agent, execution, "completed")?;
    assert_eq!(completed["response"]["result"]["max_stress"], 10.0);
    Ok(completed)
}

#[test]
fn missing_mesh_admission_never_reexecutes_with_spare_capacity_or_hides_from_recovery()
-> Result<(), Box<dyn Error>> {
    let mut a = LiveAgent::start_with_capacity("2")?;
    let mut b = LiveAgent::start_with_durable_store()?;
    a.restart_in_mesh(
        "missing-admission",
        vec![format!("127.0.0.1:{}", b.port)],
        1000,
    )?;
    b.restart_in_mesh(
        "missing-admission",
        vec![format!("127.0.0.1:{}", a.port)],
        1000,
    )?;
    let store = owner(&a)?;
    let sibling = complete(&a, &store, 'b')?;
    wait_for_lifecycle(&a, "accepting", 0)?;
    fs::write(&a.hold_path, "held-original")?;
    let submitted = a.request(
        "original",
        "submit_durable_agent_task",
        owned_task(&store, 'a', "held-original"),
    )?;
    let running = successful_result(&submitted, "original");
    assert_eq!(running["status"], "running");
    let admission = a
        .root
        .join("executions")
        .join(format!("{}.admission", "a".repeat(32)));
    let retained_fixture = a.root.join("original-admission.fixture");
    fs::rename(&admission, &retained_fixture)?;
    reject_missing(&a, &store, 'a', "held-original")?;
    unavailable(
        &a,
        "cancel_durable_agent_execution",
        json!({
            "execution_id":"a".repeat(32), "expected_store_id":store,
            "process_instance_id":running["process_instance_id"], "request_digest":running["request_digest"]
        }),
    )?;
    assert_eq!(lifecycle(&a, "only-original")?["active_execution_count"], 1);
    assert!(!admission.exists());

    // Spare solver capacity cannot override stale ledger accounting. Only new
    // admissions on this Agent are fenced; prior reads and the healthy peer survive.
    for method in ["submit_durable_agent_task", "run_durable_agent_task"] {
        unavailable(&a, method, owned_task(&store, 'e', "fresh-blocked"))?;
    }
    let descriptor = a.request("inventory-fence", "describe_agent", json!({}))?;
    assert_eq!(
        successful_result(&descriptor, "inventory-fence")["durable_execution"]["admission_blocked_after_inventory_change"],
        true
    );
    assert_eq!(fetch(&a, 'e')?["status"], "not_found");
    assert_eq!(
        lifecycle(&a, "no-new-reservation")?["active_execution_count"],
        1
    );
    complete(&b, &owner(&b)?, 'c')?;
    assert_eq!(fetch(&a, 'b')?, sibling);
    fs::remove_file(&a.hold_path)?;
    wait_for_lifecycle(&a, "accepting", 0)?;
    reject_missing(&a, &store, 'a', "held-original")?;
    assert!(
        !a.root
            .join("executions")
            .join(format!("{}.receipt", "a".repeat(32)))
            .exists()
    );

    // Restore only the test fixture, then force-restart and inspect; never re-run the old ID.
    a.stop_process()?;
    fs::rename(retained_fixture, admission)?;
    a.start_process()?;
    let unknown = fetch(&a, 'a')?;
    assert_eq!(unknown["status"], "outcome_unknown");
    assert_eq!(unknown["request_id"], "original");
    assert_eq!(unknown["request_digest"], running["request_digest"]);
    assert_eq!(unknown["automatic_replay_authorized"], false);
    let duplicate = a.request(
        "retry",
        "submit_durable_agent_task",
        owned_task(&store, 'a', "held-original"),
    )?;
    assert_eq!(successful_result(&duplicate, "retry"), &unknown);
    assert_eq!(lifecycle(&a, "no-replay")?["active_execution_count"], 0);
    assert_eq!(fetch(&a, 'b')?, sibling);
    assert_eq!(fetch(&a, 'e')?["status"], "not_found");
    complete(&a, &store, 'd')?;
    Ok(())
}

#[test]
fn orphan_mesh_receipt_survives_restart_as_an_error_until_original_admission_is_restored()
-> Result<(), Box<dyn Error>> {
    let mut a = LiveAgent::start_with_durable_store()?;
    let mut b = LiveAgent::start_with_durable_store()?;
    a.restart_in_mesh(
        "orphan-receipt",
        vec![format!("127.0.0.1:{}", b.port)],
        1000,
    )?;
    b.restart_in_mesh(
        "orphan-receipt",
        vec![format!("127.0.0.1:{}", a.port)],
        1000,
    )?;
    let store = owner(&a)?;
    let completed = complete(&a, &store, 'a')?;
    wait_for_lifecycle(&a, "accepting", 0)?;
    let admission = a
        .root
        .join("executions")
        .join(format!("{}.admission", "a".repeat(32)));
    let receipt = a
        .root
        .join("executions")
        .join(format!("{}.receipt", "a".repeat(32)));
    let receipt_bytes = fs::read(&receipt)?;
    let retained_fixture = a.root.join("original-admission.fixture");
    fs::rename(&admission, &retained_fixture)?;
    for _ in 0..2 {
        reject_missing(&a, &store, 'a', "healthy")?;
        assert_eq!(lifecycle(&a, "no-replay")?["active_execution_count"], 0);
        assert!(!admission.exists());
        assert_eq!(fs::read(&receipt)?, receipt_bytes);
        a.stop_process()?;
        a.start_process()?;
    }
    complete(&a, &store, 'b')?;
    complete(&b, &owner(&b)?, 'c')?;
    a.stop_process()?;
    fs::rename(retained_fixture, admission)?;
    a.start_process()?;
    assert_eq!(fetch(&a, 'a')?, completed);
    let duplicate = a.request(
        "restored",
        "submit_durable_agent_task",
        owned_task(&store, 'a', "healthy"),
    )?;
    assert_eq!(successful_result(&duplicate, "restored"), &completed);
    assert_eq!(
        lifecycle(&a, "not-reexecuted")?["active_execution_count"],
        0
    );
    let index = a.request(
        "restored-index",
        "list_durable_agent_executions",
        json!({"limit":24, "expected_store_id":store}),
    )?;
    assert_eq!(
        successful_result(&index, "restored-index")["executions"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    Ok(())
}
