#[path = "../examples/durable_modal_research/checks.rs"]
mod checks;
#[path = "../examples/durable_modal_research/model.rs"]
mod model;
#[path = "support/durable_ack_loss.rs"]
mod relay;
#[path = "support/durable_modal_agent.rs"]
mod support;

use kyuubiki_headless_sdk::{
    DurableExecutionHandle, DurableExecutionIntent, SdkError, SolverRpcLimits,
};
use relay::Relay;
use serde_json::{Value, json};
use std::{env, fs, io::Write, path::Path, process::Command, time::Duration};
use support::{Agent, Result};

fn prepare(agent: &Agent, input: &Value) -> Result<DurableExecutionIntent> {
    let intent = DurableExecutionIntent::new(
        &agent.store_id()?,
        &"a".repeat(32),
        "solve_modal_frame_2d",
        input,
    )?;
    let mut file = fs::File::create_new(agent.root.join("intent.json"))?;
    file.write_all(&serde_json::to_vec(&intent)?)?;
    file.sync_all()?;
    Ok(serde_json::from_slice(&fs::read(
        agent.root.join("intent.json"),
    )?)?)
}

#[test]
#[ignore = "requires a built native Agent; run make test-sdk-modal-recovery"]
fn lost_admission_recovers_only_original_input_after_agent_restart() -> Result<()> {
    let mut agent = Agent::start()?;
    let relay = Relay::start(agent.port, false)?;
    let input = model::model(false);
    let intent = prepare(&agent, &input)?;
    assert!(
        relay
            .client()
            .submit_prepared_durable_agent_task(&intent, input.clone())
            .is_err()
    );
    // Only the test relay saw this acknowledgement. SDK recovery has just its intent.
    let handle = intent.handle_from_report(&relay.dropped_receipt())?;
    let completed = agent.wait(&handle, "completed")?;
    agent.kill()?;
    agent.restart()?;
    let recovered = relay.client().inspect_durable_agent_task(&intent)?;
    assert_eq!(recovered, completed);
    let reconciled = intent.handle_from_report(&recovered)?;
    assert_eq!(reconciled, handle);
    let result = reconciled.completed_result(&recovered)?;
    checks::verify(&input, result)?;
    let mut different = input;
    different["elements"][0]["density"] = json!(99.0);
    let wrong = DurableExecutionIntent::new(
        &intent.store_id,
        &intent.execution_id,
        &intent.method,
        &different,
    )?;
    assert!(relay.client().inspect_durable_agent_task(&wrong).is_err());
    assert_eq!(
        relay.client().inspect_durable_agent_task(&intent)?,
        completed
    );
    relay.assert_one_submit();
    Ok(())
}

#[test]
#[ignore = "requires a built native Agent; run make test-sdk-modal-recovery"]
fn lost_admission_and_interrupted_solver_remain_unknown_without_replay() -> Result<()> {
    let mut agent = Agent::start()?;
    let relay = Relay::start(agent.port, false)?;
    fs::write(agent.root.join("solver.hold"), "held-admission-loss")?;
    let mut input = model::model(false);
    input["job_id"] = json!("held-admission-loss");
    let intent = prepare(&agent, &input)?;
    assert!(
        relay
            .client()
            .submit_prepared_durable_agent_task(&intent, input)
            .is_err()
    );
    intent.handle_from_report(&relay.dropped_receipt())?;
    agent.held()?;
    agent.kill()?;
    agent.restart()?;
    fs::remove_file(agent.root.join("solver.hold"))?;
    for _ in 0..3 {
        let report = relay.client().inspect_durable_agent_task(&intent)?;
        assert_eq!(report["status"], "outcome_unknown");
        assert_eq!(report["response"], Value::Null);
        assert!(
            intent
                .handle_from_report(&report)?
                .completed_result(&report)
                .is_err()
        );
    }
    assert_eq!(
        agent.client().describe_agent()?.result["lifecycle"]["active_execution_count"],
        0
    );
    relay.assert_one_submit();
    Ok(())
}

#[test]
#[ignore = "requires a built native Agent; run make test-sdk-modal-recovery"]
fn request_lost_before_admission_is_missing_not_permission_to_retry() -> Result<()> {
    let mut agent = Agent::start()?;
    let relay = Relay::start(agent.port, true)?;
    let input = model::model(false);
    let intent = prepare(&agent, &input)?;
    assert!(
        relay
            .client()
            .submit_prepared_durable_agent_task(&intent, input)
            .is_err()
    );
    agent.kill()?;
    agent.restart()?;
    for _ in 0..3 {
        let report = relay.client().inspect_durable_agent_task(&intent)?;
        assert_eq!(report["status"], "not_found");
        assert!(intent.handle_from_report(&report).is_err());
    }
    assert_eq!(
        agent.client().describe_agent()?.result["durable_execution"]["stored_entries"],
        0
    );
    relay.assert_one_submit();
    Ok(())
}

#[test]
#[ignore = "requires a built native Agent; run make test-sdk-modal-recovery"]
fn lost_admission_does_not_promote_a_failed_solver_to_success() -> Result<()> {
    let mut agent = Agent::start()?;
    let relay = Relay::start(agent.port, false)?;
    let mut input = model::model(false);
    input["elements"][0]["density"] = json!(-1.0);
    let intent = prepare(&agent, &input)?;
    assert!(
        relay
            .client()
            .submit_prepared_durable_agent_task(&intent, input)
            .is_err()
    );
    let handle = intent.handle_from_report(&relay.dropped_receipt())?;
    let failed = agent.wait(&handle, "failed")?;
    agent.kill()?;
    agent.restart()?;
    let report = relay.client().inspect_durable_agent_task(&intent)?;
    assert_eq!(report, failed);
    assert!(report["response"]["error"].is_object());
    assert_failure(&intent.handle_from_report(&report)?, &report);
    relay.assert_one_submit();
    Ok(())
}

fn assert_failure(handle: &DurableExecutionHandle, report: &Value) {
    let error = handle.completed_result(report).unwrap_err();
    match error {
        SdkError::Rpc {
            code: Some(code),
            message,
            request_id: Some(id),
            details: Some(details),
        } => {
            assert_eq!(code, report["response"]["error"]["code"]);
            assert_eq!(message, report["response"]["error"]["message"]);
            assert_eq!(id, handle.request_id);
            assert_eq!(details, report["response"]["error"]["details"]);
            assert!(details.is_object());
        }
        other => panic!("original solver cause was lost: {other}"),
    }
}

#[test]
#[ignore = "requires a built native Agent; run make test-sdk-modal-recovery"]
fn timed_out_admission_recovers_the_original_failure_after_process_restart() -> Result<()> {
    let mut agent = Agent::start()?;
    let relay = Relay::start_with_delay(agent.port, false, Duration::from_millis(400))?;
    let mut input = model::model(false);
    input["elements"][0]["density"] = json!(-1.0);
    let intent = prepare(&agent, &input)?;
    let client = relay.client().with_limits(SolverRpcLimits {
        io_timeout: Duration::from_millis(100),
        call_timeout: Duration::from_secs(2),
        ..Default::default()
    })?;
    assert!(matches!(
        client.submit_prepared_durable_agent_task(&intent, input),
        Err(SdkError::Timeout(_))
    ));
    let handle = intent.handle_from_report(&relay.dropped_receipt())?;
    let failed = agent.wait(&handle, "failed")?;
    agent.kill()?;
    agent.restart()?;
    let recovered = relay.client().inspect_durable_agent_task(&intent)?;
    assert_eq!(recovered, failed);
    assert_eq!(
        agent
            .client()
            .fetch_durable_agent_execution_after(&failed)?,
        failed
    );
    assert_failure(&intent.handle_from_report(&recovered)?, &recovered);
    relay.assert_one_submit();
    Ok(())
}

fn read(root: &Path, name: &str) -> Result<Value> {
    Ok(serde_json::from_slice(&fs::read(root.join(name))?)?)
}

#[test]
#[ignore = "requires built Agent and SDK example; run make test-sdk-modal-recovery"]
fn example_reconciles_missing_ack_but_never_ignores_corrupt_saved_ack() -> Result<()> {
    let binary = fs::canonicalize(
        env::var_os("KYUUBIKI_TEST_MODAL_EXAMPLE_BINARY").ok_or("SDK example binary required")?,
    )?;
    let mut agent = Agent::start()?;
    let relay = Relay::start(agent.port, false)?;
    let submission = agent.root.join("research");
    let invoke = |verb: &str, last: &str| {
        Command::new(&binary)
            .arg(verb)
            .arg(&submission)
            .arg(last)
            .env(
                "KYUUBIKI_RESEARCH_AGENT_ENDPOINT",
                format!("127.0.0.1:{}", relay.port),
            )
            .output()
    };
    let submitted = invoke("submit", &"e".repeat(32))?;
    assert!(
        !submitted.status.success(),
        "test must lose the actual acknowledgement"
    );
    assert!(!submission.join("admission.json").exists());
    let intent: DurableExecutionIntent = serde_json::from_value(
        read(&submission, "submission-intent.json")?["execution_intent"].clone(),
    )?;
    intent.validate_input(&read(&submission, "input.json")?)?;
    let original_ack = relay.dropped_receipt();
    let handle = intent.handle_from_report(&original_ack)?;
    let completed = agent.wait(&handle, "completed")?;
    agent.kill()?;
    agent.restart()?;
    let output = agent.root.join("recovered");
    let result = invoke("inspect", output.to_str().unwrap())?;
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let verified = output.join("verified");
    let verification = read(&verified, "verification.json")?;
    assert_eq!(verification["passed"], true);
    assert_eq!(verification["admission_response_available"], false);
    assert_eq!(
        &read(&verified, "result.json")?,
        handle.completed_result(&completed)?
    );
    let mut wrong = original_ack;
    wrong["process_instance_id"] = json!("different-admission");
    for (name, bad_ack) in [
        ("corrupt", b"{".to_vec()),
        ("contradictory", serde_json::to_vec(&wrong)?),
    ] {
        fs::write(submission.join("admission.json"), bad_ack)?;
        let rejected = agent.root.join(name);
        let result = invoke("inspect", rejected.to_str().unwrap())?;
        assert!(
            !result.status.success(),
            "bad admission must not downgrade to intent-only recovery"
        );
        assert!(!rejected.join("result.json").exists());
        assert!(!rejected.join("verification.json").exists());
        assert!(!rejected.join("verified").exists());
    }
    fs::remove_file(submission.join("admission.json"))?;
    let mut changed = read(&submission, "submission-intent.json")?;
    changed["execution_intent"]["request_digest"] = json!("f".repeat(64));
    fs::write(
        submission.join("submission-intent.json"),
        serde_json::to_vec(&changed)?,
    )?;
    let rejected = agent.root.join("changed-intent");
    assert!(
        !invoke("inspect", rejected.to_str().unwrap())?
            .status
            .success()
    );
    assert!(
        !rejected.exists(),
        "input check must precede exporting a receipt"
    );
    assert_eq!(verification, read(&verified, "verification.json")?);
    relay.assert_one_submit();
    Ok(())
}
