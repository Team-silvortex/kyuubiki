#[path = "../examples/durable_modal_research/model.rs"]
mod model;
#[path = "support/durable_modal_agent.rs"]
mod support;

use kyuubiki_headless_sdk::{DurableExecutionHandle, DurableExecutionIntent, SdkError};
use serde_json::{Value, json};
use std::{
    env, fs,
    path::Path,
    process::{Command, Output},
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

fn unavailable(error: SdkError) {
    match error {
        SdkError::Rpc {
            code: Some(code),
            message,
            request_id: Some(id),
            details: Some(details),
        } => {
            assert_eq!(code, "durable_execution_unavailable");
            assert!(!message.trim().is_empty());
            assert!(!id.trim().is_empty());
            assert_eq!(details["automatic_replay_authorized"], false);
            assert_eq!(
                details["required_action"],
                "inspect_execution_by_original_id"
            );
        }
        other => panic!("lost unavailable-store failure context: {other}"),
    }
}

fn check_blocked(
    agent: &Agent,
    intent: &DurableExecutionIntent,
    submission: &Path,
    output: &str,
) -> Result<()> {
    unavailable(
        agent
            .client()
            .inspect_durable_agent_task(intent)
            .unwrap_err(),
    );
    let destination = agent.root.join(output);
    let inspected = invoke(agent, "inspect", submission, &destination)?;
    assert!(!inspected.status.success());
    assert!(String::from_utf8_lossy(&inspected.stderr).contains("durable_execution_unavailable"));
    assert!(
        !destination.exists(),
        "invalid remote evidence created an export"
    );
    let unrelated = DurableExecutionIntent::new(
        &intent.store_id,
        &"f".repeat(32),
        &intent.method,
        &model::model(false),
    )?;
    let absent = agent.client().inspect_durable_agent_task(&unrelated)?;
    assert_eq!(absent["status"], "not_found");
    assert!(unrelated.handle_from_report(&absent).is_err());
    Ok(())
}

#[test]
#[ignore = "requires built Agent and SDK example; run make test-sdk-modal-recovery"]
fn missing_or_corrupt_native_records_block_export_until_original_records_are_restored() -> Result<()>
{
    let mut agent = Agent::start()?;
    let submission = agent.root.join("submission");
    let started = invoke(&agent, "submit", &submission, Path::new(&"a".repeat(32)))?;
    assert!(
        started.status.success(),
        "{}",
        String::from_utf8_lossy(&started.stderr)
    );
    let retained: Value =
        serde_json::from_slice(&fs::read(submission.join("submission-intent.json"))?)?;
    let intent: DurableExecutionIntent =
        serde_json::from_value(retained["execution_intent"].clone())?;
    let admission: Value = serde_json::from_slice(&fs::read(submission.join("admission.json"))?)?;
    let handle = intent.handle_from_report(&admission)?;
    let completed = agent.wait(&handle, "completed")?;
    let snapshot = || -> Result<Vec<Vec<u8>>> {
        ["input.json", "submission-intent.json", "admission.json"]
            .iter()
            .map(|name| fs::read(submission.join(name)).map_err(Into::into))
            .collect()
    };
    let local_before = snapshot()?;
    let store = agent.root.join("executions");
    let admission_path = store.join(format!("{}.admission", intent.execution_id));
    let receipt_path = store.join(format!("{}.receipt", intent.execution_id));
    let admission_bytes = fs::read(&admission_path)?;
    let receipt_bytes = fs::read(&receipt_path)?;
    let displaced = agent.root.join("held-admission");

    fs::rename(&admission_path, &displaced)?;
    check_blocked(&agent, &intent, &submission, "missing-live")?;
    agent.kill()?;
    agent.restart()?;
    check_blocked(&agent, &intent, &submission, "orphan-after-restart")?;
    assert_eq!(fs::read(&receipt_path)?, receipt_bytes);
    assert_eq!(fs::read(&displaced)?, admission_bytes);
    fs::rename(&displaced, &admission_path)?;

    // Alter only this disposable test ledger, and explicitly restore exact originals.
    for (index, (path, original)) in [
        (&admission_path, &admission_bytes),
        (&receipt_path, &receipt_bytes),
    ]
    .into_iter()
    .enumerate()
    {
        let mut corrupt = original.clone();
        corrupt[0] = if corrupt[0] == b'0' { b'1' } else { b'0' };
        fs::write(path, &corrupt)?;
        check_blocked(&agent, &intent, &submission, &format!("corrupt-{index}"))?;
        assert_eq!(
            fs::read(path)?,
            corrupt,
            "inspection must not repair records"
        );
        fs::write(path, original)?;
    }
    agent.kill()?;
    agent.restart()?;
    assert_eq!(
        agent
            .client()
            .fetch_durable_agent_execution_after(&completed)?,
        completed
    );
    let exported = agent.root.join("recovered-export");
    let restored = invoke(&agent, "inspect", &submission, &exported)?;
    assert!(
        restored.status.success(),
        "{}",
        String::from_utf8_lossy(&restored.stderr)
    );
    assert!(exported.join("verified/result.json").is_file());
    assert!(
        invoke(&agent, "verify", &submission, &exported)?
            .status
            .success()
    );
    assert_eq!(snapshot()?, local_before);
    assert_eq!(fs::read(&admission_path)?, admission_bytes);
    assert_eq!(fs::read(&receipt_path)?, receipt_bytes);
    let state = agent.client().describe_agent()?.result;
    assert_eq!(state["durable_execution"]["stored_entries"], 1);
    assert_eq!(state["lifecycle"]["active_execution_count"], 0);
    Ok(())
}

#[test]
#[ignore = "requires built native Agent; run make test-sdk-modal-recovery"]
fn interrupted_admission_loss_remains_unavailable_then_unknown_without_replay() -> Result<()> {
    let mut agent = Agent::start()?;
    fs::write(agent.root.join("solver.hold"), "interrupted")?;
    let mut input = model::model(false);
    input["job_id"] = json!("interrupted");
    let intent = DurableExecutionIntent::new(
        &agent.store_id()?,
        &"b".repeat(32),
        "solve_modal_frame_2d",
        &input,
    )?;
    let admission = agent
        .client()
        .submit_prepared_durable_agent_task(&intent, input)?;
    let handle = intent.handle_from_report(&admission)?;
    agent.held()?;
    agent.kill()?;
    agent.restart()?;
    fs::remove_file(agent.root.join("solver.hold"))?;
    let unknown = agent.client().inspect_durable_agent_task(&intent)?;
    assert_eq!(unknown["status"], "outcome_unknown");
    let path = agent
        .root
        .join("executions")
        .join(format!("{}.admission", intent.execution_id));
    let displaced = agent.root.join("held-admission");
    let bytes = fs::read(&path)?;
    fs::rename(&path, &displaced)?;
    unavailable(
        agent
            .client()
            .inspect_durable_agent_task(&intent)
            .unwrap_err(),
    );
    assert!(!path.exists());
    fs::rename(&displaced, &path)?;
    let restored = agent
        .client()
        .fetch_durable_agent_execution_after(&unknown)?;
    assert_eq!(restored, unknown);
    assert_eq!(restored["response"], Value::Null);
    assert_eq!(restored["resumable"], false);
    assert!(handle.completed_result(&restored).is_err());
    assert_eq!(fs::read(path)?, bytes);
    let state = agent.client().describe_agent()?.result;
    assert_eq!(state["durable_execution"]["stored_entries"], 1);
    assert_eq!(state["lifecycle"]["active_execution_count"], 0);
    Ok(())
}

fn read_json(path: &Path) -> Result<Value> {
    Ok(serde_json::from_slice(&fs::read(path)?)?)
}

fn assert_idle(agent: &Agent) -> Result<()> {
    let state = agent.client().describe_agent()?.result;
    assert_eq!(state["durable_execution"]["stored_entries"], 1);
    assert_eq!(state["durable_execution"]["active_reservations"], 0);
    assert_eq!(state["lifecycle"]["active_execution_count"], 0);
    Ok(())
}

fn assert_lost_terminal(agent: &Agent, retained: &Value) -> Result<Value> {
    let handle = DurableExecutionHandle::from_report(retained)?;
    // Identity-only inspection has no saved terminal payload to compare against.
    let observed = agent.client().fetch_durable_agent_execution(&handle)?;
    assert_eq!(observed["status"], "outcome_unknown");
    assert_eq!(observed["response"], Value::Null);
    assert_eq!(observed["resumable"], false);
    assert_eq!(observed["automatic_replay_authorized"], false);
    assert_eq!(DurableExecutionHandle::from_report(&observed)?, handle);
    assert!(handle.completed_result(&observed).is_err());
    match agent.client().fetch_durable_agent_execution_after(retained) {
        Err(SdkError::Transport(message)) => {
            assert!(message.contains("terminal observation changed"));
            assert!(message.contains("preserve original evidence"));
        }
        other => panic!("a known terminal must not silently regress: {other:?}"),
    }
    assert_idle(agent)?;
    Ok(observed)
}

#[test]
#[ignore = "requires built Agent and SDK example; run make test-sdk-modal-recovery"]
fn completed_receipt_loss_preserves_offline_export_and_blocks_terminal_regression() -> Result<()> {
    let mut agent = Agent::start()?;
    let submission = agent.root.join("submission");
    let started = invoke(&agent, "submit", &submission, Path::new(&"c".repeat(32)))?;
    assert!(started.status.success(), "{started:?}");
    let admission = read_json(&submission.join("admission.json"))?;
    let handle = DurableExecutionHandle::from_report(&admission)?;
    let completed = agent.wait(&handle, "completed")?;
    let export = agent.root.join("original-export");
    let inspected = invoke(&agent, "inspect", &submission, &export)?;
    assert!(inspected.status.success(), "{inspected:?}");

    // A separately retained completed acknowledgement is a valid SDK caller history.
    // Do not overwrite the example's original nonterminal admission to simulate it.
    let terminal_submission = agent.root.join("terminal-submission");
    fs::create_dir(&terminal_submission)?;
    for name in ["input.json", "submission-intent.json"] {
        fs::copy(submission.join(name), terminal_submission.join(name))?;
    }
    fs::write(
        terminal_submission.join("admission.json"),
        serde_json::to_vec_pretty(&completed)?,
    )?;
    let originals = [
        submission.join("input.json"),
        submission.join("submission-intent.json"),
        submission.join("admission.json"),
        terminal_submission.join("admission.json"),
        export.join("receipt.json"),
        export.join("verified/result.json"),
        export.join("verified/verification.json"),
    ]
    .into_iter()
    .map(|path| Ok((path.clone(), fs::read(path)?)))
    .collect::<Result<Vec<_>>>()?;
    let ledger = agent.root.join("executions");
    let receipt = ledger.join(format!("{}.receipt", handle.execution_id));
    let original_receipt = fs::read(&receipt)?;
    let admission_path = ledger.join(format!("{}.admission", handle.execution_id));
    let original_admission = fs::read(&admission_path)?;
    let displaced = agent.root.join("held-completed-receipt");
    fs::rename(&receipt, &displaced)?;

    for phase in ["live", "restarted"] {
        let unknown = assert_lost_terminal(&agent, &completed)?;
        for (name, root) in [("initial", &submission), ("terminal", &terminal_submission)] {
            let destination = agent.root.join(format!("{phase}-{name}"));
            let output = invoke(&agent, "inspect", root, &destination)?;
            assert!(!output.status.success());
            let error = String::from_utf8_lossy(&output.stderr);
            assert!(
                error.contains(if name == "terminal" {
                    "terminal observation changed"
                } else {
                    "execution is not completed"
                }),
                "{error}"
            );
            // Keep the raw observation for diagnosis, never promote it to verified output.
            assert_eq!(read_json(&destination.join("receipt.json"))?, unknown);
            assert!(!destination.join("verified").exists());
            assert!(!destination.join(".verification-pending").exists());
        }
        assert!(
            !receipt.exists(),
            "read-only checks recreated the lost receipt"
        );
        assert_eq!(fs::read(&displaced)?, original_receipt);
        assert_eq!(fs::read(&admission_path)?, original_admission);
        agent.kill()?;
        let verified = invoke(&agent, "verify", &submission, &export)?;
        assert!(verified.status.success(), "{verified:?}");
        for (path, bytes) in &originals {
            assert_eq!(&fs::read(path)?, bytes, "{}", path.display());
        }
        if phase == "live" {
            agent.restart()?;
            let process = agent.client().describe_agent()?.result;
            assert_ne!(
                process["lifecycle"]["process_instance_id"],
                handle.process_instance_id
            );
        }
    }

    fs::rename(&displaced, &receipt)?;
    agent.restart()?;
    assert_eq!(
        agent
            .client()
            .fetch_durable_agent_execution_after(&completed)?,
        completed
    );
    let restored = agent.root.join("restored-export");
    let output = invoke(&agent, "inspect", &terminal_submission, &restored)?;
    assert!(output.status.success(), "{output:?}");
    assert!(
        invoke(&agent, "verify", &terminal_submission, &restored)?
            .status
            .success()
    );
    for (path, bytes) in &originals {
        assert_eq!(&fs::read(path)?, bytes, "{}", path.display());
    }
    assert_eq!(fs::read(&receipt)?, original_receipt);
    assert_eq!(fs::read(&admission_path)?, original_admission);
    assert_idle(&agent)?;
    Ok(())
}

#[test]
#[ignore = "requires built native Agent; run make test-sdk-modal-recovery"]
fn failed_receipt_loss_preserves_original_failure_after_explicit_restoration() -> Result<()> {
    let mut agent = Agent::start()?;
    let mut input = model::model(false);
    input["elements"][0]["density"] = json!(-1.0);
    let admission = agent.client().submit_durable_agent_task(
        &agent.store_id()?,
        &"d".repeat(32),
        "solve_modal_frame_2d",
        input,
    )?;
    let handle = DurableExecutionHandle::from_report(&admission)?;
    let failed = agent.wait(&handle, "failed")?;
    let saved = agent.root.join("retained-failure.json");
    fs::write(&saved, serde_json::to_vec_pretty(&failed)?)?;
    let bytes = fs::read(&saved)?;
    let retained = read_json(&saved)?;
    let original_error = retained["response"]["error"].clone();
    assert!(original_error["details"].is_object());
    let receipt = agent
        .root
        .join("executions")
        .join(format!("{}.receipt", handle.execution_id));
    let original_receipt = fs::read(&receipt)?;
    let displaced = agent.root.join("held-failed-receipt");
    fs::rename(&receipt, &displaced)?;
    assert_lost_terminal(&agent, &retained)?;
    agent.kill()?;
    agent.restart()?;
    assert_lost_terminal(&agent, &retained)?;
    assert!(!receipt.exists());
    assert_eq!(fs::read(&saved)?, bytes);
    assert_eq!(fs::read(&displaced)?, original_receipt);
    agent.kill()?;
    fs::rename(&displaced, &receipt)?;
    agent.restart()?;
    let restored = agent
        .client()
        .fetch_durable_agent_execution_after(&retained)?;
    assert_eq!(restored, retained);
    match handle.completed_result(&restored) {
        Err(SdkError::Rpc {
            code: Some(code),
            message,
            request_id: Some(id),
            details: Some(details),
        }) => {
            assert_eq!(code, original_error["code"]);
            assert_eq!(message, original_error["message"]);
            assert_eq!(id, handle.request_id);
            assert_eq!(details, original_error["details"]);
        }
        other => panic!("restored failure lost its original diagnostics: {other:?}"),
    }
    assert_eq!(fs::read(&saved)?, bytes);
    assert_eq!(fs::read(receipt)?, original_receipt);
    assert_idle(&agent)?;
    Ok(())
}

fn reject_owner_rebootstrap(agent: &mut Agent, retained: &Value, missing: bool) -> Result<Value> {
    let handle = DurableExecutionHandle::from_report(retained)?;
    agent.kill()?;
    let store = agent.root.join("executions");
    let owner = store.join("owner.lock");
    let original_owner = fs::read(&owner)?;
    let snapshot = fs::read_dir(&store)?
        .map(|entry| {
            let path = entry?.path();
            let bytes = fs::read(&path)?;
            Ok((path, bytes))
        })
        .collect::<Result<Vec<_>>>()?;
    let displaced = agent.root.join("original-owner.lock");
    if missing {
        fs::rename(&owner, &displaced)?;
    } else {
        fs::write(&owner, b"")?;
    }
    for _ in 0..2 {
        let error = agent
            .restart()
            .expect_err("damaged owner identity must block startup");
        assert!(
            error
                .to_string()
                .contains("refusing identity reinitialization"),
            "{error}"
        );
        match agent.client().fetch_durable_agent_execution_after(retained) {
            Err(SdkError::Io(error)) => {
                assert_eq!(error.kind(), std::io::ErrorKind::ConnectionRefused);
            }
            other => panic!("failed startup must not expose a replacement observation: {other:?}"),
        }
        if missing {
            assert!(!owner.exists(), "failed startup created a replacement lock");
        } else {
            assert!(
                fs::read(&owner)?.is_empty(),
                "failed startup generated an identity"
            );
        }
        assert_eq!(
            fs::read_dir(&store)?.count(),
            snapshot.len() - usize::from(missing)
        );
        for (path, bytes) in &snapshot {
            if *path != owner {
                assert_eq!(fs::read(path)?, *bytes);
            }
        }
    }
    if missing {
        fs::rename(displaced, &owner)?;
    } else {
        fs::write(&owner, &original_owner)?;
    }
    agent.restart()?;
    assert_eq!(agent.store_id()?, handle.store_id);
    let restored = agent
        .client()
        .fetch_durable_agent_execution_after(retained)?;
    for (path, bytes) in snapshot {
        assert_eq!(fs::read(path)?, bytes);
    }
    assert_idle(agent)?;
    Ok(restored)
}

#[test]
#[ignore = "requires built Agent and SDK example; run make test-sdk-modal-recovery"]
fn lost_native_owner_blocks_restart_then_original_identity_recovers_verified_export() -> Result<()>
{
    for missing in [false, true] {
        let mut agent = Agent::start()?;
        let submission = agent.root.join("submission");
        let started = invoke(&agent, "submit", &submission, Path::new(&"a".repeat(32)))?;
        assert!(started.status.success(), "{started:?}");
        let admission = read_json(&submission.join("admission.json"))?;
        let handle = DurableExecutionHandle::from_report(&admission)?;
        let completed = agent.wait(&handle, "completed")?;
        let before = ["input.json", "submission-intent.json", "admission.json"]
            .into_iter()
            .map(|name| Ok((name, fs::read(submission.join(name))?)))
            .collect::<Result<Vec<_>>>()?;
        assert_eq!(
            reject_owner_rebootstrap(&mut agent, &completed, missing)?,
            completed
        );
        let export = agent.root.join("restored-export");
        let inspected = invoke(&agent, "inspect", &submission, &export)?;
        assert!(inspected.status.success(), "{inspected:?}");
        agent.kill()?;
        let verified = invoke(&agent, "verify", &submission, &export)?;
        assert!(verified.status.success(), "{verified:?}");
        for (name, bytes) in before {
            assert_eq!(fs::read(submission.join(name))?, bytes);
        }
    }
    Ok(())
}

#[test]
#[ignore = "requires built native Agent; run make test-sdk-modal-recovery"]
fn lost_native_owner_never_relabels_failed_or_interrupted_work() -> Result<()> {
    for missing in [false, true] {
        for failed in [false, true] {
            let mut agent = Agent::start()?;
            let mut input = model::model(false);
            if failed {
                input["elements"][0]["density"] = json!(-1.0);
            } else {
                input["job_id"] = json!("owner-interrupted");
                fs::write(agent.root.join("solver.hold"), "owner-interrupted")?;
            }
            let admission = agent.client().submit_durable_agent_task(
                &agent.store_id()?,
                &"b".repeat(32),
                "solve_modal_frame_2d",
                input,
            )?;
            let handle = DurableExecutionHandle::from_report(&admission)?;
            let retained = if failed {
                agent.wait(&handle, "failed")?
            } else {
                agent.held()?;
                agent.client().fetch_durable_agent_execution(&handle)?
            };
            let restored = reject_owner_rebootstrap(&mut agent, &retained, missing)?;
            if failed {
                assert_eq!(restored, retained);
                assert!(matches!(
                    handle.completed_result(&restored),
                    Err(SdkError::Rpc { .. })
                ));
            } else {
                assert_eq!(restored["status"], "outcome_unknown");
                assert!(restored["response"].is_null());
                assert_eq!(restored["automatic_replay_authorized"], false);
                assert!(
                    !agent
                        .root
                        .join("executions")
                        .join(format!("{}.receipt", handle.execution_id))
                        .exists()
                );
            }
        }
    }
    Ok(())
}
