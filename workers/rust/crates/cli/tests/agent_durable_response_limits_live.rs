use serde_json::{Value, json};
use std::error::Error;
use std::fs;
use std::sync::Mutex;
use std::thread;

#[allow(dead_code)]
#[path = "support/agent_durable.rs"]
mod support;
use support::*;

const ELEMENTS: usize = 100_000;
static LARGE_CASE: Mutex<()> = Mutex::new(());

fn large_work(task_ir: bool) -> Result<Value, Box<dyn Error>> {
    let mut work = task('a', "large-response");
    work["params"]["elements"] = json!(ELEMENTS);
    if task_ir {
        let mut ir = native_task_ir("large-response")?;
        ir["input_artifact"] = work["params"].clone();
        ir["integrity"]["task_digest"] =
            json!(kyuubiki_protocol::compute_operator_task_digest(&ir)?);
        work["method"] = json!("run_operator_task_ir");
        work["params"] = json!({"mode":"execute", "job_id":"large-response",
            "dispatch_attempt_id":"c".repeat(32), "task_ir":ir});
    }
    Ok(work)
}

fn cached(agent: &LiveAgent, work: &Value) -> Result<Value, Box<dyn Error>> {
    let ir = &work["params"]["task_ir"];
    let response = agent.request(
        "cache",
        "fetch_operator_task_result",
        json!({
            "attempt_id":"c".repeat(32), "request_id":"original", "task_id":ir["task_id"],
            "task_digest":ir["integrity"]["task_digest"], "operator_id":ir["operator"]["id"],
            "program_id":ir["execution_program"]["program_id"]
        }),
    )?;
    Ok(successful_result(&response, "cache").clone())
}

fn descriptor(agent: &LiveAgent) -> Result<Value, Box<dyn Error>> {
    let response = agent.request("descriptor", "describe_agent", json!({}))?;
    Ok(successful_result(&response, "descriptor").clone())
}

fn assert_rejected(report: &Value) {
    assert_eq!(report["status"], "failed");
    assert_eq!(report["response"]["id"], "original");
    assert_eq!(report["response"]["ok"], false);
    assert!(report["response"]["result"].is_null());
    assert_eq!(
        report["response"]["error"]["code"],
        "durable_response_unrecordable"
    );
    let details = &report["response"]["error"]["details"];
    assert_eq!(details["reason"], "byte_limit");
    assert_eq!(details["original_response_discarded"], true);
    assert_eq!(details["execution_may_have_started"], true);
    assert_eq!(details["automatic_replay_authorized"], false);
    assert_eq!(details["external_effects_rolled_back"], false);
    assert_eq!(report["automatic_replay_authorized"], false);
    assert!(report.to_string().len() < 4096);
}

#[test]
fn oversized_native_and_task_ir_responses_are_isolated_in_sync_and_async_modes()
-> Result<(), Box<dyn Error>> {
    let _serial = LARGE_CASE
        .lock()
        .map_err(|_| "large-response suite poisoned")?;
    // One real solve at a time; large output is discarded, not kept as test evidence.
    for method in ["run_durable_agent_task", "submit_durable_agent_task"] {
        for task_ir in [false, true] {
            let mut agent = LiveAgent::start_with_durable_store()?;
            let work = large_work(task_ir)?;
            let response = agent.request("original", method, work.clone())?;
            assert_eq!(response["ok"], true);
            let failed = wait_status(&agent, 'a', "failed")?;
            assert_rejected(&failed);
            wait_for_lifecycle(&agent, "accepting", 0)?;
            let state = descriptor(&agent)?;
            assert_eq!(state["watchdog"]["total_completed_execution_count"], 0);
            assert_eq!(state["watchdog"]["total_failed_execution_count"], 1);
            assert_eq!(
                state["durable_execution"]["admission_blocked_after_write_failure"],
                false
            );
            assert_eq!(state["durable_execution"]["stored_entries"], 1);
            assert!(state["durable_execution"]["stored_bytes"].as_u64().unwrap() < 4096);
            assert_eq!(
                state["durable_execution"]["response_rejection"]["code"],
                "durable_response_unrecordable"
            );
            if task_ir {
                let cache = cached(&agent, &work)?;
                assert_eq!(cache["status"], "receipt_retained");
                assert_eq!(cache["response"], failed["response"]);
            }
            fs::write(&agent.hold_path, "large-response")?;
            for repeated in ["run_durable_agent_task", "submit_durable_agent_task"] {
                let duplicate = agent.request("duplicate", repeated, work.clone())?;
                assert_eq!(*successful_result(&duplicate, "duplicate"), failed);
            }
            // A small independent task must succeed in the same process, without clearing the ledger.
            let next = agent.request(
                "original",
                "run_durable_agent_task",
                task('b', "small-response"),
            )?;
            assert_eq!(
                successful_result(&next, "original")["response"]["result"]["max_stress"],
                10.0
            );
            wait_for_lifecycle(&agent, "accepting", 0)?;
            assert_eq!(
                descriptor(&agent)?["watchdog"]["total_completed_execution_count"],
                1
            );
            agent.stop_process()?;
            agent.start_process()?;
            assert_eq!(fetch(&agent, 'a')?, failed);
            let duplicate = agent.request("after-restart", method, work.clone())?;
            assert_eq!(*successful_result(&duplicate, "after-restart"), failed);
            assert_eq!(lifecycle(&agent, "no-replay")?["active_execution_count"], 0);
            assert_eq!(
                descriptor(&agent)?["watchdog"]["total_completed_execution_count"],
                0
            );
            if task_ir {
                assert_eq!(cached(&agent, &work)?["status"], "not_retained");
            }
        }
    }
    Ok(())
}

#[test]
fn compact_failure_receipt_survives_kill_before_transport_and_owner_release()
-> Result<(), Box<dyn Error>> {
    let _serial = LARGE_CASE
        .lock()
        .map_err(|_| "large-response suite poisoned")?;
    let mut agent = LiveAgent::start_with_durable_store()?;
    let work = large_work(false)?;
    let hold = agent.root.join("receipt.hold");
    fs::write(&hold, "a".repeat(32))?;
    let (port, submitted) = (agent.port, work.clone());
    let request = thread::spawn(move || {
        rpc_request(port, "original", "run_durable_agent_task", submitted)
            .map_err(|error| error.to_string())
    });
    let failed = wait_status(&agent, 'a', "failed")?;
    assert_rejected(&failed);
    assert!(!request.is_finished());
    let drain = agent.request(
        "drain",
        "begin_agent_drain",
        json!({"controller_id":"test", "reason":"receipt boundary"}),
    )?;
    assert_eq!(successful_result(&drain, "drain")["safe_to_replace"], false);
    assert_eq!(
        successful_result(&drain, "drain")["active_execution_count"],
        1
    );
    agent.stop_process()?;
    assert!(
        request
            .join()
            .map_err(|_| "request thread panicked")?
            .is_err()
    );
    fs::remove_file(hold)?;
    fs::write(&agent.hold_path, "large-response")?;
    agent.start_process()?;
    assert_eq!(fetch(&agent, 'a')?, failed);
    let duplicate = agent.request("repeat", "submit_durable_agent_task", work)?;
    assert_eq!(*successful_result(&duplicate, "repeat"), failed);
    assert_eq!(lifecycle(&agent, "no-replay")?["active_execution_count"], 0);
    Ok(())
}

#[test]
fn failed_compact_receipt_commit_stays_unknown_instead_of_claiming_isolation()
-> Result<(), Box<dyn Error>> {
    let _serial = LARGE_CASE
        .lock()
        .map_err(|_| "large-response suite poisoned")?;
    for method in ["run_durable_agent_task", "submit_durable_agent_task"] {
        let mut agent = LiveAgent::start_with_durable_store()?;
        let work = large_work(false)?;
        let obstruction = agent
            .root
            .join("executions")
            .join(format!("{}.receipt.next", "a".repeat(32)));
        fs::create_dir(&obstruction)?;
        let response = agent.request("original", method, work.clone())?;
        assert_eq!(response["ok"], method == "submit_durable_agent_task");
        let unknown = wait_status(&agent, 'a', "outcome_unknown")?;
        assert!(unknown["response"].is_null());
        wait_for_lifecycle(&agent, "accepting", 0)?;
        let state = descriptor(&agent)?;
        assert_eq!(state["watchdog"]["total_completed_execution_count"], 0);
        assert_eq!(state["watchdog"]["total_failed_execution_count"], 1);
        assert_eq!(
            state["durable_execution"]["admission_blocked_after_write_failure"],
            true
        );
        assert_eq!(
            agent.request("blocked", method, task('b', "small-response"))?["ok"],
            false
        );
        fs::remove_dir(obstruction)?;
        agent.stop_process()?;
        agent.start_process()?;
        assert_eq!(fetch(&agent, 'a')?, unknown);
        fs::write(&agent.hold_path, "large-response")?;
        let duplicate = agent.request("repeat", method, work)?;
        assert_eq!(*successful_result(&duplicate, "repeat"), unknown);
        assert_eq!(lifecycle(&agent, "no-replay")?["active_execution_count"], 0);
    }
    Ok(())
}
