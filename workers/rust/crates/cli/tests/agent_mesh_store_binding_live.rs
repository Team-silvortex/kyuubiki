#![cfg(unix)]

use serde_json::{Value, json};
use std::error::Error;
use std::fs;
use std::os::unix::fs::DirBuilderExt;

#[allow(dead_code)]
#[path = "support/agent_durable.rs"]
mod support;
use support::*;

fn descriptor(agent: &LiveAgent) -> Result<Value, Box<dyn Error>> {
    let reply = agent.request("descriptor", "describe_agent", json!({}))?;
    Ok(successful_result(&reply, "descriptor").clone())
}

fn owned_task(store: &Value, execution: char) -> Value {
    let mut params = task(execution, "held-job");
    params["expected_store_id"] = store.clone();
    params
}

#[test]
fn live_mesh_store_replacement_is_unavailable_not_missing_or_redirected()
-> Result<(), Box<dyn Error>> {
    let mut a = LiveAgent::start_with_durable_store()?;
    let mut b = LiveAgent::start_with_durable_store()?;
    a.restart_in_mesh("store-binding", vec![format!("127.0.0.1:{}", b.port)], 1000)?;
    b.restart_in_mesh("store-binding", vec![format!("127.0.0.1:{}", a.port)], 1000)?;
    let owner = descriptor(&a)?["durable_execution"]["store_id"].clone();
    let reply = a.request(
        "completed",
        "submit_durable_agent_task",
        owned_task(&owner, 'a'),
    )?;
    assert_eq!(successful_result(&reply, "completed")["status"], "running");
    let completed = wait_status(&a, 'a', "completed")?;
    wait_for_lifecycle(&a, "accepting", 0)?;

    fs::write(&a.hold_path, "held-job")?;
    let reply = a.request("held", "submit_durable_agent_task", owned_task(&owner, 'b'))?;
    let running = successful_result(&reply, "held");
    assert_eq!(running["status"], "running");
    let root = a.root.join("executions");
    let saved = a.root.join("relocated-executions");
    fs::rename(&root, &saved)?;
    fs::DirBuilder::new().mode(0o700).create(&root)?;
    fs::copy(
        b.root.join("executions/owner.lock"),
        root.join("owner.lock"),
    )?;
    // Matching admission bytes must not allow the original worker to commit to this store.
    let admission = format!("{}.admission", "b".repeat(32));
    fs::copy(saved.join(&admission), root.join(&admission))?;
    let foreign_owner = fs::read(root.join("owner.lock"))?;
    let admission_bytes = fs::read(root.join(&admission))?;

    let state = descriptor(&a)?;
    assert_eq!(state["runtime"]["runtime_mode"], "peer_mesh");
    assert_eq!(state["durable_execution"]["available"], false);
    assert_eq!(
        state["durable_execution"]["unavailable_reason"],
        "store_binding_unavailable"
    );
    for (method, params) in [
        ("submit_durable_agent_task", owned_task(&owner, 'c')),
        ("run_durable_agent_task", owned_task(&owner, 'c')),
        (
            "fetch_durable_agent_execution",
            json!({"execution_id":"a".repeat(32), "expected_store_id":owner}),
        ),
        (
            "fetch_durable_agent_execution",
            json!({"execution_id":"c".repeat(32)}),
        ),
        (
            "list_durable_agent_executions",
            json!({"limit":24, "expected_store_id":owner}),
        ),
        (
            "cancel_durable_agent_execution",
            json!({"execution_id":"b".repeat(32), "expected_store_id":owner,
            "process_instance_id":running["process_instance_id"], "request_digest":running["request_digest"]}),
        ),
    ] {
        let reply = a.request("detached", method, params)?;
        assert_eq!(reply["ok"], false, "{method}: {reply}");
        assert_eq!(reply["error"]["code"], "durable_execution_unavailable");
        assert_eq!(
            reply["error"]["details"]["automatic_replay_authorized"],
            false
        );
        assert!(
            reply["error"]["message"]
                .as_str()
                .unwrap()
                .contains("binding unavailable")
        );
    }
    assert_eq!(lifecycle(&a, "still-running")?["active_execution_count"], 1);
    fs::remove_file(&a.hold_path)?;
    wait_for_lifecycle(&a, "accepting", 0)?;
    assert_eq!(fs::read_dir(&root)?.count(), 2);
    assert_eq!(fs::read(root.join("owner.lock"))?, foreign_owner);
    assert_eq!(fs::read(root.join(&admission))?, admission_bytes);
    assert!(!saved.join(format!("{}.receipt", "b".repeat(32))).exists());
    assert_eq!(descriptor(&b)?["durable_execution"]["available"], true);
    let reply = b.request(
        "neighbor",
        "submit_durable_agent_task",
        task('d', "neighbor"),
    )?;
    assert_eq!(successful_result(&reply, "neighbor")["status"], "running");
    assert_eq!(
        wait_status(&b, 'd', "completed")?["response"]["result"]["max_stress"],
        10.0
    );

    a.stop_process()?;
    fs::remove_dir_all(&root)?;
    fs::rename(&saved, &root)?;
    a.start_process()?;
    assert_eq!(descriptor(&a)?["durable_execution"]["available"], true);
    assert_eq!(fetch(&a, 'a')?, completed);
    let unknown = fetch(&a, 'b')?;
    assert_eq!(unknown["store_id"], owner);
    assert_eq!(unknown["status"], "outcome_unknown");
    assert_eq!(unknown["automatic_replay_authorized"], false);
    let duplicate = a.request(
        "duplicate",
        "submit_durable_agent_task",
        owned_task(&owner, 'b'),
    )?;
    assert_eq!(successful_result(&duplicate, "duplicate"), &unknown);
    assert_eq!(lifecycle(&a, "not-replayed")?["active_execution_count"], 0);
    let reply = a.request(
        "fresh",
        "submit_durable_agent_task",
        owned_task(&owner, 'e'),
    )?;
    assert_eq!(successful_result(&reply, "fresh")["status"], "running");
    assert_eq!(
        wait_status(&a, 'e', "completed")?["response"]["result"]["max_stress"],
        10.0
    );
    Ok(())
}
