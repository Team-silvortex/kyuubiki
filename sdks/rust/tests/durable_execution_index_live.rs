#[path = "../examples/durable_modal_research/checks.rs"]
mod checks;
#[path = "../examples/durable_modal_research/model.rs"]
mod model;
#[path = "support/durable_modal_agent.rs"]
mod support;

use kyuubiki_headless_sdk::{DurableExecutionIntent, SdkError};
use serde_json::{Value, json};
use std::{collections::BTreeMap, fs};
use support::{Agent, Result};

fn snapshot(agent: &Agent) -> Result<BTreeMap<String, Vec<u8>>> {
    fs::read_dir(agent.root.join("executions"))?
        .map(|entry| {
            let entry = entry?;
            Ok((
                entry.file_name().to_str().ok_or("non-UTF8 record")?.into(),
                fs::read(entry.path())?,
            ))
        })
        .collect()
}

fn unavailable(error: SdkError) {
    match error {
        SdkError::Rpc {
            code: Some(code),
            request_id: Some(id),
            details: Some(details),
            ..
        } => {
            assert_eq!(code, "durable_execution_unavailable");
            assert!(!id.is_empty());
            assert_eq!(details["automatic_replay_authorized"], false);
        }
        other => panic!("lost native index error context: {other}"),
    }
}

#[test]
#[ignore = "requires built native Agent; run make test-sdk-modal-recovery"]
fn sdk_index_recovers_mixed_native_admissions_without_promoting_or_replaying_them() -> Result<()> {
    let mut agent = Agent::start()?;
    let store = agent.store_id()?;
    assert!(
        agent
            .client()
            .list_durable_agent_executions(&store, None, 2)?
            .executions
            .is_empty()
    );
    let mut completed = Value::Null;
    let mut failed = Value::Null;
    let mut intents = Vec::new();
    // Deliberately admit out of lexical order; the final task is held inside the real solver.
    for (id, state) in [("c", "failed"), ("a", "completed"), ("b", "running")] {
        let mut input = model::model(false);
        if state == "failed" {
            input["elements"][0]["density"] = json!(-1.0);
        }
        if state == "running" {
            input["job_id"] = json!("index-interrupted");
            fs::write(agent.root.join("solver.hold"), "index-interrupted")?;
        }
        let intent =
            DurableExecutionIntent::new(&store, &id.repeat(32), "solve_modal_frame_2d", &input)?;
        let admission = agent
            .client()
            .submit_prepared_durable_agent_task(&intent, input)?;
        let handle = intent.handle_from_report(&admission)?;
        match state {
            "completed" => completed = agent.wait(&handle, state)?,
            "failed" => failed = agent.wait(&handle, state)?,
            _ => agent.held()?,
        }
        intents.push(intent);
    }
    let before = snapshot(&agent)?;
    let first = agent
        .client()
        .list_durable_agent_executions(&store, None, 2)?;
    assert_eq!(
        first
            .executions
            .iter()
            .map(|e| e.execution_id.clone())
            .collect::<Vec<_>>(),
        vec!["a".repeat(32), "b".repeat(32)]
    );
    assert_eq!(first.next_cursor, Some("b".repeat(32)));
    agent.kill()?;
    agent.restart()?;
    fs::remove_file(agent.root.join("solver.hold"))?;
    let second =
        agent
            .client()
            .list_durable_agent_executions(&store, first.next_cursor.as_deref(), 2)?;
    assert_eq!(second.executions.len(), 1);
    assert_eq!(second.executions[0].execution_id, "c".repeat(32));
    assert!(second.next_cursor.is_none());
    assert_eq!(
        agent
            .client()
            .list_durable_agent_executions(&store, None, 2)?,
        first
    );
    for entry in first.executions.iter().chain(&second.executions) {
        let intent = intents
            .iter()
            .find(|i| i.execution_id == entry.execution_id)
            .unwrap();
        assert_eq!(entry.request_digest, intent.request_digest);
        assert_eq!(entry.method, intent.method);
        let report = agent.client().inspect_durable_agent_task(intent)?;
        let handle = intent.handle_from_report(&report)?;
        assert_eq!(entry.request_id, handle.request_id);
        assert_eq!(entry.process_instance_id, handle.process_instance_id);
        match entry.execution_id.as_bytes()[0] {
            b'a' => {
                assert_eq!(report, completed);
                checks::verify(&model::model(false), handle.completed_result(&report)?)?;
            }
            b'b' => {
                assert_eq!(report["status"], "outcome_unknown");
                assert!(report["response"].is_null());
                assert!(handle.completed_result(&report).is_err());
            }
            _ => {
                assert_eq!(report, failed);
                assert!(matches!(
                    handle.completed_result(&report),
                    Err(SdkError::Rpc { .. })
                ));
            }
        }
    }
    assert_eq!(
        snapshot(&agent)?,
        before,
        "index and receipt reads must not rewrite evidence"
    );

    // Missing admission before the requested cursor must not silently disappear from recovery.
    let path = agent
        .root
        .join("executions")
        .join(format!("{}.admission", "a".repeat(32)));
    let displaced = agent.root.join("held-admission");
    fs::rename(&path, &displaced)?;
    for restart in [false, true] {
        if restart {
            agent.kill()?;
            agent.restart()?;
        }
        unavailable(
            agent
                .client()
                .list_durable_agent_executions(&store, Some(&"b".repeat(32)), 1)
                .unwrap_err(),
        );
        assert!(!path.exists(), "index must not repair evidence");
    }
    fs::rename(&displaced, &path)?;
    assert_eq!(
        agent
            .client()
            .list_durable_agent_executions(&store, None, 2)?,
        first
    );

    // Listing admission metadata is intentionally not validation of a large result receipt.
    let receipt = agent
        .root
        .join("executions")
        .join(format!("{}.receipt", "a".repeat(32)));
    let original = fs::read(&receipt)?;
    let mut corrupt = original.clone();
    corrupt[0] = if corrupt[0] == b'0' { b'1' } else { b'0' };
    fs::write(&receipt, &corrupt)?;
    assert_eq!(
        agent
            .client()
            .list_durable_agent_executions(&store, None, 2)?,
        first
    );
    unavailable(
        agent
            .client()
            .fetch_durable_agent_execution_after(&completed)
            .unwrap_err(),
    );
    assert_eq!(fs::read(&receipt)?, corrupt);
    fs::write(&receipt, original)?;
    assert_eq!(
        agent
            .client()
            .fetch_durable_agent_execution_after(&completed)?,
        completed
    );
    assert_eq!(snapshot(&agent)?, before);
    // Explicit filesystem repair is not live capacity reconciliation. Restart
    // after restoring originals so the Agent rescans its admission/byte counters.
    agent.kill()?;
    agent.restart()?;
    assert_eq!(
        agent
            .client()
            .list_durable_agent_executions(&store, None, 2)?,
        first
    );
    let state = agent.client().describe_agent()?.result;
    assert_eq!(state["durable_execution"]["stored_entries"], 3);
    assert_eq!(state["lifecycle"]["active_execution_count"], 0);
    Ok(())
}

#[test]
#[ignore = "requires built native Agent; run make test-sdk-modal-recovery"]
fn sdk_index_keeps_the_original_store_pin_when_the_endpoint_store_is_replaced() -> Result<()> {
    let mut agent = Agent::start()?;
    let original = agent.store_id()?;
    agent.kill()?;
    let path = agent.root.join("executions");
    let displaced = agent.root.join("original-executions");
    fs::rename(&path, &displaced)?;
    agent.restart()?;
    assert_ne!(agent.store_id()?, original);
    unavailable(
        agent
            .client()
            .list_durable_agent_executions(&original, None, 64)
            .unwrap_err(),
    );
    agent.kill()?;
    fs::remove_dir_all(&path)?;
    fs::rename(&displaced, &path)?;
    agent.restart()?;
    assert!(
        agent
            .client()
            .list_durable_agent_executions(&original, None, 64)?
            .executions
            .is_empty()
    );
    Ok(())
}
