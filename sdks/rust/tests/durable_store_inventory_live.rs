#[path = "../examples/durable_modal_research/checks.rs"]
mod checks;
#[path = "../examples/durable_modal_research/model.rs"]
mod model;
#[path = "support/durable_modal_agent.rs"]
mod support;

use kyuubiki_headless_sdk::{DurableAgentStoreStatus, DurableExecutionIntent, SdkError};
use serde_json::{Value, json};
use std::fs;
use support::{Agent, Result};

fn intent(agent: &Agent, id: &str, input: &Value) -> Result<DurableExecutionIntent> {
    Ok(DurableExecutionIntent::new(
        &agent.store_id()?,
        &id.repeat(32),
        "solve_modal_frame_2d",
        input,
    )?)
}

fn complete(agent: &Agent, id: &str) -> Result<Value> {
    let input = model::model(false);
    let task = intent(agent, id, &input)?;
    let admission = agent
        .client()
        .submit_prepared_durable_agent_task(&task, input.clone())?;
    let handle = task.handle_from_report(&admission)?;
    let report = agent.wait(&handle, "completed")?;
    checks::verify(&input, handle.completed_result(&report)?)?;
    Ok(report)
}

fn assert_fenced(agent: &Agent, task: &DurableExecutionIntent) -> Result<()> {
    let error = agent
        .client()
        .submit_prepared_durable_agent_task(task, model::model(false))
        .unwrap_err();
    match error {
        SdkError::Rpc {
            code: Some(code),
            details: Some(details),
            message,
            ..
        } => {
            assert_eq!(code, "durable_execution_unavailable");
            assert!(message.contains("inventory"));
            assert_eq!(details["automatic_replay_authorized"], false);
        }
        other => panic!("inventory drift must not become definite capacity rejection: {other}"),
    }
    let status = agent.client().describe_durable_agent_store()?;
    assert_eq!(
        status,
        DurableAgentStoreStatus::AdmissionBlocked {
            store_id: task.store_id.clone()
        }
    );
    assert!(status.ready_store_id().is_err());
    assert_eq!(
        agent.client().inspect_durable_agent_task(task)?["status"],
        "not_found"
    );
    for suffix in ["admission", "receipt", "admission.next", "receipt.next"] {
        assert!(
            !agent
                .root
                .join("executions")
                .join(format!("{}.{}", task.execution_id, suffix))
                .exists()
        );
    }
    Ok(())
}

#[test]
#[ignore = "requires built native Agent; run make test-sdk-modal-recovery"]
fn restored_native_inventory_blocks_new_sdk_work_until_explicit_restart() -> Result<()> {
    let mut agent = Agent::start()?;
    let completed = complete(&agent, "a")?;
    let store = agent.store_id()?;
    agent.kill()?;
    let mut originals = Vec::new();
    for suffix in ["admission", "receipt"] {
        let path = agent
            .root
            .join("executions")
            .join(format!("{}.{}", "a".repeat(32), suffix));
        let held = agent.root.join(format!("held-{suffix}"));
        let bytes = fs::read(&path)?;
        fs::rename(&path, &held)?;
        originals.push((path, held, bytes));
    }
    agent.restart()?;
    assert_eq!(agent.store_id()?, store);
    assert_eq!(
        agent.client().describe_agent()?.result["durable_execution"]["stored_entries"],
        0
    );
    for (path, held, _) in &originals {
        fs::rename(held, path)?;
    }
    // Readiness is a descriptor snapshot, not a reservation or live directory scan.
    assert!(matches!(
        agent.client().describe_durable_agent_store()?,
        DurableAgentStoreStatus::Ready { .. }
    ));
    let fresh = intent(&agent, "b", &model::model(false))?;
    assert_fenced(&agent, &fresh)?;
    assert_eq!(
        agent
            .client()
            .fetch_durable_agent_execution_after(&completed)?,
        completed
    );
    let index = agent
        .client()
        .list_durable_agent_executions(&store, None, 64)?;
    assert_eq!(index.executions.len(), 1);
    assert_eq!(index.executions[0].execution_id, "a".repeat(32));
    let state = agent.client().describe_agent()?.result;
    assert_eq!(state["durable_execution"]["stored_entries"], 0);
    assert_eq!(state["lifecycle"]["active_execution_count"], 0);
    for (path, _, bytes) in &originals {
        assert_eq!(&fs::read(path)?, bytes);
    }
    agent.kill()?;
    agent.restart()?;
    assert_eq!(
        agent
            .client()
            .describe_durable_agent_store()?
            .ready_store_id()?,
        store
    );
    let state = agent.client().describe_agent()?.result;
    assert_eq!(state["durable_execution"]["stored_entries"], 1);
    assert_eq!(
        state["durable_execution"]["stored_bytes"].as_u64(),
        Some(
            originals
                .iter()
                .map(|(_, _, bytes)| bytes.len() as u64)
                .sum()
        )
    );
    assert_eq!(
        agent.client().inspect_durable_agent_task(&fresh)?["status"],
        "not_found"
    );
    complete(&agent, "c")?; // Explicit new identity, never automatic resubmission.
    assert_eq!(
        agent
            .client()
            .fetch_durable_agent_execution_after(&completed)?,
        completed
    );
    Ok(())
}

#[test]
#[ignore = "requires built native Agent; run make test-sdk-modal-recovery"]
fn inventory_fence_preserves_inflight_modal_completion_and_original_receipt_reads() -> Result<()> {
    let mut agent = Agent::start()?;
    let completed = complete(&agent, "a")?;
    let physical_input = model::model(false);
    let mut input = physical_input.clone();
    input["job_id"] = json!("inventory-active");
    fs::write(agent.root.join("solver.hold"), "inventory-active")?;
    let active = intent(&agent, "b", &input)?;
    let admission = agent
        .client()
        .submit_prepared_durable_agent_task(&active, input.clone())?;
    let handle = active.handle_from_report(&admission)?;
    agent.held()?;
    let path = agent
        .root
        .join("executions")
        .join(format!("{}.receipt", "a".repeat(32)));
    let original = fs::read(&path)?;
    let mut changed = original.clone();
    changed.push(b' ');
    fs::write(&path, &changed)?;
    let fresh = intent(&agent, "c", &model::model(false))?;
    assert_fenced(&agent, &fresh)?;
    assert_eq!(fs::read(&path)?, changed);
    fs::remove_file(agent.root.join("solver.hold"))?;
    let healthy = agent.wait(&handle, "completed")?;
    // job_id scopes the Agent hold, not SolveModalFrame2dRequest's physical input.
    // The intent/handle still bind the complete dispatch, including that job ID.
    checks::verify(&physical_input, handle.completed_result(&healthy)?)?;
    fs::write(&path, &original)?;
    assert_fenced(&agent, &fresh)?;
    assert_eq!(
        agent
            .client()
            .fetch_durable_agent_execution_after(&completed)?,
        completed
    );
    agent.kill()?;
    agent.restart()?;
    assert_eq!(
        agent
            .client()
            .fetch_durable_agent_execution_after(&healthy)?,
        healthy
    );
    assert_eq!(
        agent.client().inspect_durable_agent_task(&fresh)?["status"],
        "not_found"
    );
    complete(&agent, "d")?;
    assert_eq!(fs::read(&path)?, original);
    Ok(())
}
