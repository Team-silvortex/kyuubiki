#[path = "../examples/durable_modal_research/evidence.rs"]
mod evidence;
#[path = "../examples/durable_modal_research/model.rs"]
mod model;
#[path = "../examples/durable_modal_research/records.rs"]
mod records;
#[path = "support/durable_modal_agent.rs"]
mod support;

use kyuubiki_headless_sdk::{DurableExecutionIntent, SdkError};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    env, fs,
    path::Path,
    process::{Command, Output},
};
use support::{Agent, Result};

fn invoke(agent: &Agent, verb: &str, root: &Path, out: &Path) -> Result<Output> {
    let binary = fs::canonicalize(
        env::var_os("KYUUBIKI_TEST_MODAL_EXAMPLE_BINARY").ok_or("example binary required")?,
    )?;
    Ok(Command::new(binary)
        .arg(verb)
        .arg(root)
        .arg(out)
        .env(
            "KYUUBIKI_RESEARCH_AGENT_ENDPOINT",
            format!("127.0.0.1:{}", agent.port),
        )
        .output()?)
}

fn unavailable(result: kyuubiki_headless_sdk::SdkResult<Value>) -> Result<()> {
    match result {
        Err(SdkError::Rpc {
            code: Some(code),
            message,
            request_id: Some(id),
            details: Some(details),
        }) => {
            assert_eq!(code, "durable_execution_unavailable");
            assert!(message.contains("duplicate JSON object key"), "{message}");
            assert!(!id.is_empty());
            assert_eq!(details["automatic_replay_authorized"], false);
            assert_eq!(
                details["required_action"],
                "inspect_execution_by_original_id"
            );
            Ok(())
        }
        Ok(_) => {
            Err("Agent normalized duplicate record fields before the SDK could reject them".into())
        }
        Err(other) => Err(format!("lost native duplicate-record diagnosis: {other}").into()),
    }
}

fn exercise(failed: bool) -> Result<()> {
    let mut agent = Agent::start()?;
    let root = agent.root.join("submission");
    let mut input = model::model(false);
    if failed {
        input["elements"][0]["density"] = json!(-1.0);
    }
    let intent = DurableExecutionIntent::new(
        &agent.store_id()?,
        &"a".repeat(32),
        "solve_modal_frame_2d",
        &input,
    )?;
    fs::create_dir(&root)?;
    records::retain(&root, "input.json", &input)?;
    records::retain(
        &root,
        "submission-intent.json",
        &json!({
            "endpoint":format!("127.0.0.1:{}", agent.port), "execution_intent":intent,
            "input_sha256":format!("{:x}", Sha256::digest(evidence::read(&root, "input.json")?.bytes))
        }),
    )?;
    let admission = agent
        .client()
        .submit_prepared_durable_agent_task(&intent, input)?;
    records::retain(&root, "admission.json", &admission)?;
    let handle = intent.handle_from_report(&evidence::read(&root, "admission.json")?.value)?;
    let original = agent.wait(&handle, if failed { "failed" } else { "completed" })?;
    let snapshot = ["input.json", "submission-intent.json", "admission.json"]
        .into_iter()
        .map(|name| Ok((name, fs::read(root.join(name))?)))
        .collect::<Result<Vec<_>>>()?;
    let path = agent
        .root
        .join("executions")
        .join(format!("{}.receipt", intent.execution_id));
    let original_bytes = fs::read(&path)?;
    let body = std::str::from_utf8(&original_bytes[65..])?;
    let (key, replacement) = if failed {
        (
            "\"reason_code\":",
            "\"reason_code\":\"contradictory\",\"reason_code\":",
        )
    } else {
        (
            "\"natural_frequency_hz\":",
            "\"natural_frequency_hz\":-1.0,\"natural_frequency_hz\":",
        )
    };
    let ambiguous = body.replacen(key, replacement, 1);
    assert_ne!(
        ambiguous, body,
        "fixture must inject a duplicate into the actual native payload"
    );
    let corrupt = format!("{:x}\n{ambiguous}", Sha256::digest(ambiguous.as_bytes())).into_bytes();
    // Recompute only this disposable test record's unkeyed checksum. This is not a signature.
    fs::write(&path, &corrupt)?;
    for phase in ["live", "restarted"] {
        unavailable(
            agent
                .client()
                .fetch_durable_agent_execution_after(&original),
        )?;
        unavailable(agent.client().inspect_durable_agent_task(&intent))?;
        let out = agent.root.join(format!("blocked-{phase}"));
        let inspected = invoke(&agent, "inspect", &root, &out)?;
        assert!(!inspected.status.success());
        let message = String::from_utf8_lossy(&inspected.stderr);
        assert!(
            message.contains("durable_execution_unavailable"),
            "{message}"
        );
        assert!(message.contains("duplicate JSON object key"), "{message}");
        assert!(!out.exists(), "invalid native record created an export");
        assert_eq!(
            fs::read(&path)?,
            corrupt,
            "inspection repaired evidence silently"
        );
        assert_eq!(
            agent
                .client()
                .inspect_durable_agent_execution(&intent.store_id, &"f".repeat(32))?["status"],
            "not_found"
        );
        agent.kill()?;
        if phase == "live" {
            agent.restart()?;
        }
    }
    fs::write(&path, &original_bytes)?;
    agent.restart()?;
    let restored = agent
        .client()
        .fetch_durable_agent_execution_after(&original)?;
    assert_eq!(restored, original);
    if failed {
        match handle.completed_result(&restored) {
            Err(SdkError::Rpc {
                code: Some(code),
                request_id: Some(id),
                details: Some(details),
                ..
            }) => {
                assert_eq!(code, original["response"]["error"]["code"]);
                assert_eq!(id, handle.request_id);
                assert_eq!(details, original["response"]["error"]["details"]);
            }
            other => panic!("restored failure changed: {other:?}"),
        }
    } else {
        let out = agent.root.join("restored-export");
        let inspected = invoke(&agent, "inspect", &root, &out)?;
        assert!(inspected.status.success(), "{inspected:?}");
        agent.kill()?;
        let verified = invoke(&agent, "verify", &root, &out)?;
        assert!(verified.status.success(), "{verified:?}");
        agent.restart()?;
    }
    for (name, bytes) in snapshot {
        assert_eq!(fs::read(root.join(name))?, bytes);
    }
    assert_eq!(fs::read(path)?, original_bytes);
    let state = agent.client().describe_agent()?.result;
    assert_eq!(state["durable_execution"]["stored_entries"], 1);
    assert_eq!(state["lifecycle"]["active_execution_count"], 0);
    Ok(())
}

#[test]
#[ignore = "requires built Agent and SDK example; run make test-sdk-modal-recovery"]
fn native_ambiguous_modal_record_is_rejected_before_sdk_export_even_after_restart() -> Result<()> {
    exercise(false)
}

#[test]
#[ignore = "requires built Agent and SDK example; run make test-sdk-modal-recovery"]
fn native_ambiguous_failure_record_never_rewrites_original_diagnostics() -> Result<()> {
    exercise(true)
}
