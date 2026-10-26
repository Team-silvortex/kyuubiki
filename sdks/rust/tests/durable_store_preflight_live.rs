#[path = "support/durable_store_descriptor_peer.rs"]
mod descriptor_peer;
#[cfg(unix)]
#[path = "../examples/durable_modal_research/model.rs"]
mod model;
#[path = "support/durable_modal_agent.rs"]
mod support;

use kyuubiki_headless_sdk::{
    DurableAgentStoreStatus, DurableExecutionHandle, SdkError, SdkResult, SolverRpcClient,
};
use serde_json::{Value, json};
use std::{
    env, fs,
    path::{Path, PathBuf},
    process::{Command, Output},
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};
use support::{Agent, Result};

fn invoke(agent: &Agent, verb: &str, root: &Path, last: &Path) -> Result<Output> {
    invoke_at(agent.port, verb, root, last)
}

fn invoke_at(port: u16, verb: &str, root: &Path, last: &Path) -> Result<Output> {
    let binary = fs::canonicalize(
        env::var_os("KYUUBIKI_TEST_MODAL_EXAMPLE_BINARY").ok_or("example binary required")?,
    )?;
    Ok(Command::new(binary)
        .arg(verb)
        .arg(root)
        .arg(last)
        .env(
            "KYUUBIKI_RESEARCH_AGENT_ENDPOINT",
            format!("127.0.0.1:{port}"),
        )
        .output()?)
}

#[cfg(unix)]
fn completed(agent: &Agent) -> Result<(DurableExecutionHandle, Value)> {
    let admission = agent.client().submit_durable_agent_task(
        &agent.store_id()?,
        &"a".repeat(32),
        "solve_modal_frame_2d",
        model::model(false),
    )?;
    let handle = DurableExecutionHandle::from_report(&admission)?;
    let receipt = agent.wait(&handle, "completed")?;
    Ok((handle, receipt))
}

#[cfg(unix)]
#[test]
#[ignore = "requires built Agent and SDK example; run make test-sdk-modal-recovery"]
fn unavailable_store_descriptor_blocks_new_submission_before_local_records() -> Result<()> {
    let agent = Agent::start()?;
    let (handle, receipt) = completed(&agent)?;
    let store = agent.root.join("executions");
    let displaced = agent.root.join("displaced-executions");
    fs::rename(&store, &displaced)?;
    let descriptor = agent.client().describe_agent()?.result;
    assert_eq!(descriptor["durable_execution"]["available"], false);
    assert_eq!(descriptor["durable_execution"]["store_id"], handle.store_id);
    assert_eq!(
        agent.client().describe_durable_agent_store()?,
        DurableAgentStoreStatus::Unavailable {
            reason: "store_binding_unavailable".into()
        }
    );
    let destination = agent.root.join("rejected-submission");
    let output = invoke(&agent, "submit", &destination, Path::new(&"b".repeat(32)))?;
    assert!(!output.status.success());
    assert!(
        !destination.exists(),
        "unavailable store still created submission records"
    );
    assert!(String::from_utf8_lossy(&output.stderr).contains("store_binding_unavailable"));
    fs::rename(&displaced, &store)?;
    assert_eq!(
        agent.client().fetch_durable_agent_execution(&handle)?,
        receipt
    );
    assert_eq!(
        agent.client().describe_agent()?.result["durable_execution"]["stored_entries"],
        1
    );
    Ok(())
}

struct ReadHold {
    path: PathBuf,
    worker: Option<JoinHandle<SdkResult<Value>>>,
}

impl ReadHold {
    fn start(agent: &Agent, handle: &DurableExecutionHandle) -> Result<Self> {
        let path = agent.root.join("store-read.hold");
        fs::write(&path, &handle.execution_id)?;
        let client = agent.client();
        let handle = handle.clone();
        let marker = format!(
            "agent store-read fault hold acquired: {}",
            handle.execution_id
        );
        let worker = thread::spawn(move || client.fetch_durable_agent_execution(&handle));
        let held = Self {
            path,
            worker: Some(worker),
        };
        let deadline = Instant::now() + Duration::from_secs(3);
        while !fs::read_to_string(agent.root.join("agent.log"))?.contains(&marker) {
            if Instant::now() >= deadline {
                return Err("store read did not acquire hold".into());
            }
            thread::sleep(Duration::from_millis(5));
        }
        Ok(held)
    }

    fn release(mut self) -> Result<Value> {
        fs::remove_file(&self.path)?;
        Ok(self
            .worker
            .take()
            .unwrap()
            .join()
            .map_err(|_| "held reader panicked")??)
    }
}

impl Drop for ReadHold {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

#[test]
#[ignore = "requires built Agent and SDK example; run make test-sdk-modal-recovery"]
fn busy_store_observation_preserves_original_task_and_blocks_only_new_submission() -> Result<()> {
    let agent = Agent::start()?;
    let expected_store = agent.store_id()?;
    let submission = agent.root.join("submission");
    let started = invoke(&agent, "submit", &submission, Path::new(&"c".repeat(32)))?;
    assert!(
        started.status.success(),
        "{}",
        String::from_utf8_lossy(&started.stderr)
    );
    let admission: Value = serde_json::from_slice(&fs::read(submission.join("admission.json"))?)?;
    let handle = DurableExecutionHandle::from_report(&admission)?;
    assert_eq!(handle.store_id, expected_store);
    let receipt = agent.wait(&handle, "completed")?;
    let hold = ReadHold::start(&agent, &handle)?;
    let before = Instant::now();
    assert_eq!(
        agent.client().describe_durable_agent_store()?,
        DurableAgentStoreStatus::Busy
    );
    assert!(
        before.elapsed() < Duration::from_secs(2),
        "descriptor must not wait for release"
    );
    let rejected = agent.root.join("busy-submission");
    let output = invoke(&agent, "submit", &rejected, Path::new(&"d".repeat(32)))?;
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("temporarily busy"));
    assert!(!rejected.exists());
    let observed = agent
        .client()
        .fetch_durable_agent_execution_after(&receipt)
        .unwrap_err();
    match observed {
        SdkError::Rpc {
            code: Some(code),
            request_id: Some(_),
            details: Some(details),
            ..
        } => {
            assert_eq!(code, "durable_execution_busy");
            assert_eq!(details["operation_started"], false);
            assert_eq!(details["execution_state_known"], false);
            assert_eq!(details["automatic_retry"], false);
            assert_eq!(details["automatic_replay_authorized"], false);
        }
        other => panic!("busy observation was lost: {other:?}"),
    }
    let export = agent.root.join("after-busy");
    let inspected = invoke(&agent, "inspect", &submission, &export)?;
    assert!(!inspected.status.success());
    assert!(String::from_utf8_lossy(&inspected.stderr).contains("durable_execution_busy"));
    assert!(!export.exists());
    assert_eq!(hold.release()?, receipt);
    assert_eq!(
        agent.client().describe_durable_agent_store()?,
        DurableAgentStoreStatus::Ready {
            store_id: handle.store_id.clone()
        }
    );
    let inspected = invoke(&agent, "inspect", &submission, &export)?;
    assert!(
        inspected.status.success(),
        "{}",
        String::from_utf8_lossy(&inspected.stderr)
    );
    assert!(export.join("verified/result.json").is_file());
    assert_eq!(
        agent
            .client()
            .fetch_durable_agent_execution_after(&receipt)?,
        receipt
    );
    assert_eq!(
        agent.client().describe_agent()?.result["durable_execution"]["stored_entries"],
        1
    );
    Ok(())
}

#[test]
#[ignore = "requires built Agent and SDK example; run make test-sdk-modal-recovery"]
fn descriptor_preflight_reads_once_and_never_dispatches_blocked_or_malformed_work() -> Result<()> {
    let agent = Agent::start()?;
    let original = agent.client().describe_agent()?.result;
    for (fault, cause) in [
        ("admission-blocked", "write failure"),
        ("disabled", "disabled"),
        ("busy", "temporarily busy"),
        ("unavailable", "store_binding_unavailable"),
        ("missing-availability", "boolean available"),
        ("contradictory", "contradictory"),
    ] {
        let mut descriptor = original.clone();
        let policy = &mut descriptor["durable_execution"];
        match fault {
            "admission-blocked" => policy["admission_blocked_after_write_failure"] = json!(true),
            "disabled" => {
                policy["enabled"] = json!(false);
                policy["survives_agent_restart"] = json!(false);
                for key in [
                    "available",
                    "store_id",
                    "admission_blocked_after_write_failure",
                ] {
                    policy.as_object_mut().unwrap().remove(key);
                }
            }
            "busy" | "unavailable" => {
                policy["available"] = json!(false);
                policy["unavailable_reason"] = json!(if fault == "busy" {
                    "store_busy"
                } else {
                    "store_binding_unavailable"
                });
            }
            "missing-availability" => {
                policy.as_object_mut().unwrap().remove("available");
            }
            "contradictory" => policy["unavailable_reason"] = json!("store_busy"),
            _ => unreachable!(),
        }
        let peer = descriptor_peer::Peer::start(descriptor)?;
        let observed = SolverRpcClient::new("127.0.0.1", peer.port).describe_durable_agent_store();
        if let Ok(state) = observed {
            assert!(state.ready_store_id().is_err());
        }
        assert_eq!(peer.methods(), ["describe_agent"]);
        let output = agent.root.join(fault);
        let rejected = invoke_at(peer.port, "submit", &output, Path::new(&"e".repeat(32)))?;
        assert!(!rejected.status.success());
        let error = String::from_utf8_lossy(&rejected.stderr);
        assert!(error.contains(cause), "{fault}: {error}");
        assert!(!output.exists());
        assert_eq!(peer.methods(), ["describe_agent", "describe_agent"]);
    }
    assert_eq!(
        agent.client().describe_agent()?.result["durable_execution"]["stored_entries"],
        0
    );
    Ok(())
}
