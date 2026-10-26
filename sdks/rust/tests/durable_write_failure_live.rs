#[path = "../examples/durable_modal_research/evidence.rs"]
mod evidence;
#[path = "../examples/durable_modal_research/model.rs"]
mod model;
#[path = "../examples/durable_modal_research/records.rs"]
mod records;
#[path = "support/durable_modal_agent.rs"]
mod support;

use kyuubiki_headless_sdk::{
    DurableAgentStoreStatus, DurableExecutionHandle, DurableExecutionIntent, SdkError,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    env, fs,
    io::Write,
    path::Path,
    process::{Command, Output},
    thread,
    time::{Duration, Instant},
};
use support::{Agent, Result};

fn invoke(agent: &Agent, verb: &str, root: &Path, last: &Path) -> Result<Output> {
    let binary = fs::canonicalize(
        env::var_os("KYUUBIKI_TEST_MODAL_EXAMPLE_BINARY").ok_or("example binary required")?,
    )?;
    Ok(Command::new(binary)
        .arg(verb)
        .arg(root)
        .arg(last)
        .env(
            "KYUUBIKI_RESEARCH_AGENT_ENDPOINT",
            format!("127.0.0.1:{}", agent.port),
        )
        .output()?)
}

fn success(output: Output) {
    assert!(output.status.success(), "{output:?}");
}

fn create_pending(path: &Path) -> Result<Vec<u8>> {
    let bytes = b"incomplete test-owned record; never a receipt".to_vec();
    let mut options = fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(path)?;
    file.write_all(&bytes)?;
    file.sync_all()?;
    Ok(bytes)
}

fn await_blocked(agent: &Agent, store_id: &str, entries: u64) -> Result<()> {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let state = agent.client().describe_agent()?.result;
        let store = DurableAgentStoreStatus::from_descriptor(&state)?;
        if store
            == (DurableAgentStoreStatus::AdmissionBlocked {
                store_id: store_id.into(),
            })
            && state["lifecycle"]["active_execution_count"] == 0
            && state["durable_execution"]["active_reservations"] == 0
        {
            assert!(store.ready_store_id().is_err());
            assert_eq!(state["durable_execution"]["stored_entries"], entries);
            return Ok(());
        }
        if Instant::now() >= deadline {
            return Err(format!(
                "native write failure did not fence admission and release work: {state}"
            )
            .into());
        }
        thread::sleep(Duration::from_millis(10));
    }
}

fn inspect_uncertain(
    agent: &Agent,
    intent: &DurableExecutionIntent,
    root: &Path,
    destination: &Path,
    status: &str,
) -> Result<Value> {
    let observed = agent.client().inspect_durable_agent_task(intent)?;
    assert_eq!(observed["status"], status);
    assert_eq!(observed["automatic_replay_authorized"], false);
    assert!(observed["response"].is_null());
    if status == "outcome_unknown" {
        let handle = intent.handle_from_report(&observed)?;
        assert!(handle.completed_result(&observed).is_err());
        assert_eq!(observed["resumable"], false);
    } else {
        assert!(intent.handle_from_report(&observed).is_err());
    }
    let output = invoke(agent, "inspect", root, destination)?;
    assert!(!output.status.success(), "{output:?}");
    assert_eq!(evidence::read(destination, "receipt.json")?.value, observed);
    assert!(!destination.join("verified").exists());
    assert!(!destination.join(".verification-pending").exists());
    Ok(observed)
}

fn exercise_write_failure(stage: &str) -> Result<()> {
    let mut agent = Agent::start()?;
    let store_id = agent.store_id()?;
    let original_root = agent.root.join("original-study");
    success(invoke(
        &agent,
        "submit",
        &original_root,
        Path::new(&"a".repeat(32)),
    )?);
    let original_admission = evidence::read(&original_root, "admission.json")?.value;
    let original_handle = DurableExecutionHandle::from_report(&original_admission)?;
    let original = agent.wait(&original_handle, "completed")?;
    let ledger = agent.root.join("executions");
    let original_path = ledger.join(format!("{}.receipt", original_handle.execution_id));
    let original_bytes = fs::read(&original_path)?;

    let id = "b".repeat(32);
    let pending = ledger.join(format!("{id}.{stage}.next"));
    let pending_bytes = create_pending(&pending)?;
    let submission = agent.root.join("uncertain-study");
    let (entries, status) = if stage == "admission" {
        // The real example's ready preflight cannot predict a later filesystem failure.
        let output = invoke(&agent, "submit", &submission, Path::new(&id))?;
        assert!(!output.status.success());
        assert!(String::from_utf8_lossy(&output.stderr).contains("durable_execution_unavailable"));
        assert!(!submission.join("admission.json").exists());
        assert!(!ledger.join(format!("{id}.admission")).exists());
        (1, "not_found")
    } else {
        let mut input = model::model(false);
        input["job_id"] = json!("write-failure-study");
        let intent = DurableExecutionIntent::new(&store_id, &id, "solve_modal_frame_2d", &input)?;
        fs::create_dir(&submission)?;
        records::retain(&submission, "input.json", &input)?;
        records::retain(
            &submission,
            "submission-intent.json",
            &json!({
                "endpoint":format!("127.0.0.1:{}", agent.port), "execution_intent":intent,
                "input_sha256":format!("{:x}", Sha256::digest(evidence::read(&submission, "input.json")?.bytes))
            }),
        )?;
        fs::write(agent.root.join("solver.hold"), "write-failure-study")?;
        let admission = agent
            .client()
            .submit_prepared_durable_agent_task(&intent, input)?;
        records::retain(&submission, "admission.json", &admission)?;
        agent.held()?;
        assert!(!ledger.join(format!("{id}.receipt")).exists());
        fs::remove_file(agent.root.join("solver.hold"))?;
        (2, "outcome_unknown")
    };
    let retained = evidence::read(&submission, "submission-intent.json")?.value;
    let intent: DurableExecutionIntent =
        serde_json::from_value(retained["execution_intent"].clone())?;
    let snapshot = fs::read_dir(&submission)?
        .map(|entry| {
            let path = entry?.path();
            Ok((path.clone(), fs::read(path)?))
        })
        .collect::<Result<Vec<_>>>()?;
    await_blocked(&agent, &store_id, entries)?;
    assert_eq!(
        fs::read(&pending)?,
        pending_bytes,
        "failed writer removed someone else's pending record"
    );
    let unknown = inspect_uncertain(
        &agent,
        &intent,
        &submission,
        &agent.root.join("blocked-inspect"),
        status,
    )?;

    let rejected_root = agent.root.join("blocked-new-study");
    let output = invoke(&agent, "submit", &rejected_root, Path::new(&"c".repeat(32)))?;
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("write failure"));
    assert!(
        !rejected_root.exists(),
        "blocked preflight left new local evidence"
    );
    // Bypassing the example preflight still cannot bypass the native admission fence.
    match agent.client().submit_durable_agent_task(
        &store_id,
        &"c".repeat(32),
        "solve_modal_frame_2d",
        model::model(false),
    ) {
        Err(SdkError::Rpc {
            code: Some(code),
            message,
            details: Some(details),
            ..
        }) => {
            assert_eq!(code, "durable_execution_unavailable");
            assert!(message.contains("previous write failure"));
            assert_eq!(details["automatic_replay_authorized"], false);
            assert_eq!(
                details["required_action"],
                "inspect_execution_by_original_id"
            );
        }
        other => panic!(
            "write failure must not become a safe capacity rejection or execution: {other:?}"
        ),
    }
    assert!(
        !ledger
            .join(format!("{}.admission", "c".repeat(32)))
            .exists()
    );
    assert_eq!(
        agent
            .client()
            .fetch_durable_agent_execution_after(&original)?,
        original
    );
    let old_export = agent.root.join("old-result-while-blocked");
    success(invoke(&agent, "inspect", &original_root, &old_export)?);
    success(invoke(&agent, "verify", &original_root, &old_export)?);
    await_blocked(&agent, &store_id, entries)?;
    assert_eq!(fs::read(&pending)?, pending_bytes);
    assert!(!ledger.join(format!("{id}.receipt")).exists());

    agent.kill()?;
    agent.restart()?;
    assert!(
        !pending.exists(),
        "startup should discard, not promote, the pending file"
    );
    assert_eq!(
        agent.client().describe_durable_agent_store()?,
        DurableAgentStoreStatus::Ready {
            store_id: store_id.clone()
        }
    );
    let restarted = inspect_uncertain(
        &agent,
        &intent,
        &submission,
        &agent.root.join("restart-inspect"),
        status,
    )?;
    assert_eq!(
        restarted, unknown,
        "ready store changed the original uncertain outcome"
    );
    assert!(!ledger.join(format!("{id}.receipt")).exists());
    assert_eq!(fs::read(&original_path)?, original_bytes);
    for (path, bytes) in &snapshot {
        assert_eq!(&fs::read(path)?, bytes);
    }

    // This is an explicitly new study, never an automatic retry of the uncertain ID.
    let fresh = agent.root.join("fresh-study");
    success(invoke(
        &agent,
        "submit",
        &fresh,
        Path::new(&"d".repeat(32)),
    )?);
    let fresh_handle =
        DurableExecutionHandle::from_report(&evidence::read(&fresh, "admission.json")?.value)?;
    agent.wait(&fresh_handle, "completed")?;
    let fresh_export = agent.root.join("fresh-export");
    success(invoke(&agent, "inspect", &fresh, &fresh_export)?);
    success(invoke(&agent, "verify", &fresh, &fresh_export)?);
    assert_eq!(agent.client().inspect_durable_agent_task(&intent)?, unknown);
    assert_eq!(
        agent
            .client()
            .fetch_durable_agent_execution_after(&original)?,
        original
    );
    assert_eq!(fs::read(original_path)?, original_bytes);
    let state = agent.client().describe_agent()?.result;
    assert_eq!(state["durable_execution"]["stored_entries"], entries + 1);
    assert_eq!(state["lifecycle"]["active_execution_count"], 0);
    assert_eq!(state["durable_execution"]["active_reservations"], 0);
    Ok(())
}

#[test]
#[ignore = "requires built Agent and SDK example; run make test-sdk-modal-recovery"]
fn native_admission_write_failure_fences_new_work_without_inventing_a_receipt() -> Result<()> {
    exercise_write_failure("admission")
}

#[test]
#[ignore = "requires built Agent and SDK example; run make test-sdk-modal-recovery"]
fn native_receipt_write_failure_preserves_unknown_without_replay_after_restart() -> Result<()> {
    exercise_write_failure("receipt")
}
