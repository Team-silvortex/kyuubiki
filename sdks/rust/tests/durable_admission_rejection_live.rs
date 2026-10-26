#[path = "support/durable_ack_loss.rs"]
mod ack_loss;
#[path = "support/durable_modal_agent.rs"]
mod support;

use kyuubiki_headless_sdk::{DurableExecutionHandle, DurableExecutionIntent, SdkError};
use serde_json::{Value, json};
use std::{
    thread,
    time::{Duration, Instant},
};
use support::{Agent, Result};

fn fill(agent: &Agent) -> Result<Value> {
    let descriptor = agent.client().describe_agent()?.result;
    let maximum = descriptor["durable_execution"]["max_entries"]
        .as_u64()
        .ok_or("missing entry limit")?;
    assert!((1..=1024).contains(&maximum), "fixture must remain bounded");
    let store = agent.store_id()?;
    let mut last = Value::Null;
    for index in 0..maximum {
        // Small invalid solver inputs create real failed receipts without a costly solve.
        last = agent.client().submit_durable_agent_task(
            &store,
            &format!("{index:032x}"),
            "solve_bar_1d",
            json!({}),
        )?;
    }
    let handle = DurableExecutionHandle::from_report(&last)?;
    let receipt = agent.wait(&handle, "failed")?;
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let state = agent.client().describe_agent()?.result;
        if state["durable_execution"]["stored_entries"] == maximum
            && state["lifecycle"]["active_execution_count"] == 0
        {
            break;
        }
        if Instant::now() >= deadline {
            return Err("capacity fixture did not settle".into());
        }
        thread::sleep(Duration::from_millis(10));
    }
    Ok(receipt)
}

#[test]
#[ignore = "requires built native Agent; run make test-sdk-modal-recovery"]
fn full_store_rejection_keeps_reason_and_identity_instead_of_a_schema_error() -> Result<()> {
    let mut agent = Agent::start()?;
    let original = fill(&agent)?;
    let intent = DurableExecutionIntent::new(
        &agent.store_id()?,
        &"f".repeat(32),
        "solve_bar_1d",
        &json!({}),
    )?;
    let error = agent
        .client()
        .submit_prepared_durable_agent_task(&intent, json!({}))
        .unwrap_err();
    assert!(
        error.to_string().contains("capacity"),
        "lost native rejection: {error}"
    );
    let SdkError::DurableAdmissionRejected { report } = error else {
        panic!("expected explicit rejection: {error}");
    };
    assert_eq!(report["execution_id"], intent.execution_id);
    assert_eq!(report["request_digest"], intent.request_digest);
    assert_eq!(report["reason"], "store_capacity");
    assert_eq!(report["execution_started"], false);
    assert!(DurableExecutionHandle::from_report(&report).is_err());
    assert_eq!(
        agent.client().inspect_durable_agent_task(&intent)?["status"],
        "not_found"
    );
    assert_eq!(
        agent
            .client()
            .fetch_durable_agent_execution_after(&original)?,
        original
    );

    // An explicit duplicate observation remains valid even when the store is full.
    let existing = DurableExecutionHandle::from_report(&original)?;
    assert_eq!(
        agent.client().submit_durable_agent_task(
            &existing.store_id,
            &existing.execution_id,
            &existing.method,
            json!({})
        )?,
        original
    );

    // The relay sees the real refusal, but a client whose reply was dropped cannot know it.
    let lost = DurableExecutionIntent::new(
        &agent.store_id()?,
        &"e".repeat(32),
        "solve_bar_1d",
        &json!({}),
    )?;
    let relay = ack_loss::Relay::start(agent.port, false)?;
    let uncertain = relay
        .client()
        .submit_prepared_durable_agent_task(&lost, json!({}));
    assert!(matches!(
        uncertain,
        Err(SdkError::Io(_) | SdkError::Timeout(_))
    ));
    assert_eq!(relay.dropped_receipt()["status"], "not_admitted");
    assert_eq!(
        relay.client().inspect_durable_agent_task(&lost)?["status"],
        "not_found"
    );
    relay.assert_one_submit();
    drop(relay);

    let state = agent.client().describe_agent()?.result;
    let count = state["durable_execution"]["stored_entries"].clone();
    assert_eq!(count, state["durable_execution"]["max_entries"]);
    agent.kill()?;
    agent.restart()?;
    assert_eq!(
        agent
            .client()
            .fetch_durable_agent_execution_after(&original)?,
        original
    );
    for pending in [&intent, &lost] {
        assert_eq!(
            agent.client().inspect_durable_agent_task(pending)?["status"],
            "not_found"
        );
    }
    let restarted = agent.client().describe_agent()?.result;
    assert_eq!(restarted["durable_execution"]["stored_entries"], count);
    assert_eq!(restarted["lifecycle"]["active_execution_count"], 0);
    Ok(())
}
