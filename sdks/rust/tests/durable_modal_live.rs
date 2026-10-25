#[path = "../examples/durable_modal_research/checks.rs"]
mod checks;
#[path = "../examples/durable_modal_research/model.rs"]
mod model;
#[path = "support/durable_modal_agent.rs"]
mod support;

use kyuubiki_headless_sdk::DurableExecutionHandle;
use serde_json::{Value, json};
use std::{fs, io::Write};
use support::{Agent, Result};

#[test]
#[ignore = "requires a built native Agent; run make test-sdk-modal-recovery"]
fn direct_modal_rpc_validates_real_progress_and_terminal_envelopes() -> Result<()> {
    let agent = Agent::start()?;
    let input = model::model(false);
    let solved = agent.client().solve_modal_frame_2d(input.clone())?;
    checks::verify(&input, &solved.result)?;
    assert!(!solved.progress_frames.is_empty());
    let id = &solved.progress_frames[0]["id"];
    assert!(
        solved
            .progress_frames
            .iter()
            .all(|frame| frame["id"] == *id && frame["rpc_version"] == 1)
    );
    let mut invalid = input;
    invalid["elements"][0]["density"] = json!(-1.0);
    match agent.client().solve_modal_frame_2d(invalid) {
        Err(kyuubiki_headless_sdk::SdkError::Rpc {
            code: Some(code),
            request_id: Some(id),
            details: Some(details),
            ..
        }) => {
            assert_eq!(details["request_id"], id);
            assert_eq!(details["reason_code"], code);
        }
        _ => panic!("direct Agent failure must preserve its structured identity and cause"),
    }
    Ok(())
}

fn disk_roundtrip(agent: &Agent, name: &str, value: &Value) -> Result<Value> {
    let path = agent.root.join(name);
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)?;
    file.write_all(&serde_json::to_vec_pretty(value)?)?;
    file.sync_all()?;
    Ok(serde_json::from_slice(&fs::read(path)?)?)
}

#[test]
#[ignore = "requires a built native Agent; run make test-sdk-modal-recovery"]
fn official_sdk_recovers_committed_heterogeneous_modes_after_agent_kill() -> Result<()> {
    let mut agent = Agent::start()?;
    let mut reference = Value::Null;
    for reversed in [false, true] {
        let input = model::model(reversed);
        let id = if reversed { "b" } else { "a" }.repeat(32);
        let prefix = if reversed { "renumbered" } else { "original" };
        let input = disk_roundtrip(&agent, &format!("{prefix}-input.json"), &input)?;
        let admission = agent.client().submit_durable_agent_task(
            &agent.store_id()?,
            &id,
            "solve_modal_frame_2d",
            input.clone(),
        )?;
        let handle = DurableExecutionHandle::from_report(&admission)?;
        let handle: DurableExecutionHandle = serde_json::from_value(disk_roundtrip(
            &agent,
            &format!("{prefix}-handle.json"),
            &serde_json::to_value(handle)?,
        )?)?;
        let completed = agent.wait(&handle, "completed")?;
        let result = disk_roundtrip(
            &agent,
            &format!("{prefix}-result.json"),
            handle.completed_result(&completed)?,
        )?;
        let checked = checks::verify(&input, &result)?;
        for (pointer, value) in [
            ("/modes/0/eigenvalue_rad_s_squared", json!(999.0)),
            ("/modes/0/shape", json!(vec![0.0; 9])),
            ("/modes/0/natural_frequency_hz", json!(0.0)),
            ("/modes/0/period_s", json!(0.0)),
            ("/modes/1", result["modes"][0].clone()),
            ("/modes", json!([])),
            ("/total_mass", json!(1.0)),
        ] {
            let mut corrupt = result.clone();
            *corrupt.pointer_mut(pointer).unwrap() = value;
            assert!(checks::verify(&input, &corrupt).is_err(), "{pointer}");
        }
        if reversed {
            for (a, b) in checked["independent_roots"]
                .as_array()
                .unwrap()
                .iter()
                .zip(reference.as_array().unwrap())
            {
                assert!((a.as_f64().unwrap() / b.as_f64().unwrap() - 1.0).abs() < 1e-10);
            }
        } else {
            reference = checked["independent_roots"].clone();
        }
        disk_roundtrip(&agent, &format!("{prefix}-verification.json"), &checked)?;
        agent.kill()?;
        agent.restart()?;
        let recovered = agent
            .client()
            .fetch_durable_agent_execution_after(&completed)?;
        assert_eq!(recovered, completed);
        assert_eq!(handle.completed_result(&recovered)?, &result);
        checks::verify(&input, handle.completed_result(&recovered)?)?;
        let description = agent.client().describe_agent()?.result;
        let new_process = description["lifecycle"]["process_instance_id"]
            .as_str()
            .ok_or("missing process identity")?;
        assert_ne!(new_process, handle.process_instance_id);
        let mut changed = input;
        changed["elements"][0]["density"] = json!(99.0);
        assert!(
            agent
                .client()
                .submit_durable_agent_task(&handle.store_id, &id, "solve_modal_frame_2d", changed)
                .is_err()
        );
        let mut wrong_store = handle.clone();
        wrong_store.store_id = "f".repeat(32);
        assert!(
            agent
                .client()
                .fetch_durable_agent_execution(&wrong_store)
                .is_err()
        );
        assert_eq!(
            agent.client().fetch_durable_agent_execution(&handle)?,
            completed
        );
    }
    Ok(())
}

#[test]
#[ignore = "requires a built native Agent; run make test-sdk-modal-recovery"]
fn official_sdk_refuses_a_replacement_store_at_the_original_endpoint() -> Result<()> {
    let mut agent = Agent::start()?;
    let admission = agent.client().submit_durable_agent_task(
        &agent.store_id()?,
        &"e".repeat(32),
        "solve_modal_frame_2d",
        model::model(false),
    )?;
    let handle = DurableExecutionHandle::from_report(&admission)?;
    let completed = agent.wait(&handle, "completed")?;
    agent.kill()?;
    let store = agent.root.join("executions");
    let retained = agent.root.join("original-executions");
    fs::rename(&store, &retained)?;
    agent.restart()?;
    assert_ne!(agent.store_id()?, handle.store_id);
    assert!(
        agent
            .client()
            .fetch_durable_agent_execution(&handle)
            .is_err()
    );
    assert_eq!(
        agent.client().describe_agent()?.result["durable_execution"]["stored_entries"],
        0
    );
    agent.kill()?;
    fs::remove_dir_all(&store)?;
    fs::rename(retained, store)?;
    agent.restart()?;
    assert_eq!(
        agent.client().fetch_durable_agent_execution(&handle)?,
        completed
    );
    checks::verify(&model::model(false), handle.completed_result(&completed)?)?;
    Ok(())
}

#[test]
#[ignore = "requires built Agent and SDK example; run make test-sdk-modal-recovery"]
fn research_example_exports_from_a_new_process_after_agent_restart() -> Result<()> {
    let binary = std::fs::canonicalize(
        std::env::var_os("KYUUBIKI_TEST_MODAL_EXAMPLE_BINARY")
            .ok_or("set KYUUBIKI_TEST_MODAL_EXAMPLE_BINARY to the SDK example")?,
    )?;
    let mut agent = Agent::start()?;
    let submission = agent.root.join("research");
    let output = agent.root.join("export");
    let invoke = |verb: &str, last: &str, port: u16| {
        std::process::Command::new(&binary)
            .arg(verb)
            .arg(&submission)
            .arg(last)
            .env(
                "KYUUBIKI_RESEARCH_AGENT_ENDPOINT",
                format!("127.0.0.1:{port}"),
            )
            .output()
    };
    let start = invoke("submit", &"f".repeat(32), agent.port)?;
    assert!(
        start.status.success(),
        "{}",
        String::from_utf8_lossy(&start.stderr)
    );
    let admission: Value = serde_json::from_slice(&fs::read(submission.join("admission.json"))?)?;
    let handle = DurableExecutionHandle::from_report(&admission)?;
    let completed = agent.wait(&handle, "completed")?;
    agent.kill()?;
    agent.restart()?;
    let inspect = invoke(
        "inspect",
        output.to_str().ok_or("non UTF-8 path")?,
        agent.port,
    )?;
    assert!(
        inspect.status.success(),
        "{}",
        String::from_utf8_lossy(&inspect.stderr)
    );
    let verified = output.join("verified");
    let report: Value = serde_json::from_slice(&fs::read(verified.join("verification.json"))?)?;
    assert_eq!(report["passed"], true);
    let result: Value = serde_json::from_slice(&fs::read(verified.join("result.json"))?)?;
    assert_eq!(&result, handle.completed_result(&completed)?);
    checks::verify(&model::model(false), &result)?;
    let repeated = invoke("inspect", output.to_str().unwrap(), agent.port)?;
    assert!(
        !repeated.status.success(),
        "an export must not overwrite old evidence"
    );
    let original_report = fs::read(verified.join("verification.json"))?;
    fs::write(submission.join("input.json"), b"{}")?;
    let corrupt = invoke(
        "inspect",
        agent.root.join("tampered-export").to_str().unwrap(),
        agent.port,
    )?;
    assert!(
        !corrupt.status.success(),
        "changed input must not qualify a result"
    );
    assert_eq!(
        fs::read(verified.join("verification.json"))?,
        original_report
    );
    Ok(())
}

#[test]
#[ignore = "requires a built native Agent; run make test-sdk-modal-recovery"]
fn official_sdk_does_not_publish_or_replay_interrupted_modal_work() -> Result<()> {
    let mut agent = Agent::start()?;
    fs::write(agent.root.join("solver.hold"), "held-modal-research")?;
    let mut input = model::model(false);
    input["job_id"] = json!("held-modal-research");
    let admission = agent.client().submit_durable_agent_task(
        &agent.store_id()?,
        &"c".repeat(32),
        "solve_modal_frame_2d",
        input.clone(),
    )?;
    let handle = DurableExecutionHandle::from_report(&admission)?;
    agent.held()?;
    assert_eq!(agent.wait(&handle, "running")?["response"], Value::Null);
    agent.kill()?;
    agent.restart()?;
    let unknown = agent.client().fetch_durable_agent_execution(&handle)?;
    assert_eq!(unknown["status"], "outcome_unknown");
    assert_eq!(unknown["response"], Value::Null);
    assert!(handle.completed_result(&unknown).is_err());
    // Even an explicit duplicate must only read the original durable outcome.
    assert_eq!(
        agent.client().submit_durable_agent_task(
            &handle.store_id,
            &handle.execution_id,
            &handle.method,
            input
        )?,
        unknown
    );
    assert_eq!(
        agent.client().describe_agent()?.result["lifecycle"]["active_execution_count"],
        0
    );
    fs::remove_file(agent.root.join("solver.hold"))?;
    let fresh = agent.client().submit_durable_agent_task(
        &handle.store_id,
        &"d".repeat(32),
        "solve_modal_frame_2d",
        model::model(false),
    )?;
    let fresh_handle = DurableExecutionHandle::from_report(&fresh)?;
    let completed = agent.wait(&fresh_handle, "completed")?;
    checks::verify(
        &model::model(false),
        fresh_handle.completed_result(&completed)?,
    )?;
    assert_eq!(
        agent.client().fetch_durable_agent_execution(&handle)?,
        unknown
    );
    Ok(())
}
