use serde_json::{Value, json};
use std::error::Error;
use std::fs;

#[allow(dead_code)]
#[path = "support/agent_durable.rs"]
mod support;
use support::*;

fn submit(agent: &LiveAgent, id: char, job: &str) -> Result<Value, Box<dyn Error>> {
    let response = agent.request("original", "submit_durable_agent_task", task(id, job))?;
    Ok(successful_result(&response, "original").clone())
}

fn assert_panicked(report: &Value) {
    assert_eq!(report["status"], "failed");
    assert_eq!(report["response"]["id"], "original");
    assert_eq!(report["response"]["ok"], false);
    assert!(report["response"]["result"].is_null());
    assert_eq!(
        report["response"]["error"]["code"],
        "durable_execution_panicked"
    );
    assert_eq!(
        report["response"]["error"]["details"],
        json!({
            "phase":"async_worker_unwind", "execution_may_have_started":true,
            "partial_result_discarded":true, "external_effects_rolled_back":false,
            "automatic_replay_authorized":false
        })
    );
    assert_eq!(report["automatic_replay_authorized"], false);
    assert!(!report.to_string().contains("injected durable worker panic"));
}

#[test]
fn caught_worker_panic_discards_result_and_allows_only_new_explicit_execution()
-> Result<(), Box<dyn Error>> {
    let agent = LiveAgent::start_with_durable_panic(&"a".repeat(32))?;
    assert_eq!(submit(&agent, 'a', "panicking-job")?["status"], "running");
    let failed = wait_status(&agent, 'a', "failed")?;
    assert_panicked(&failed);
    wait_for_lifecycle(&agent, "accepting", 0)?;
    let descriptor = agent.request("descriptor", "describe_agent", json!({}))?;
    let descriptor = successful_result(&descriptor, "descriptor");
    assert_eq!(
        descriptor["durable_execution"]["panic_fault_injection"]["enabled"],
        true
    );
    let watchdog = &descriptor["watchdog"];
    assert_eq!(watchdog["total_completed_execution_count"], 0);
    assert_eq!(watchdog["total_failed_execution_count"], 1);
    assert!(
        watchdog["recent_failures"]
            .as_array()
            .unwrap()
            .iter()
            .any(|failure| failure["request_id"] == "original"
                && failure["reason_code"] == "durable_execution_panicked")
    );
    assert!(
        fs::read_to_string(&agent.log_path)?
            .contains("injected durable worker panic after dispatch")
    );
    for method in ["submit_durable_agent_task", "run_durable_agent_task"] {
        let duplicate = agent.request("repeat", method, task('a', "panicking-job"))?;
        assert_eq!(*successful_result(&duplicate, "repeat"), failed);
    }
    let conflict = agent.request(
        "conflict",
        "submit_durable_agent_task",
        task('a', "changed"),
    )?;
    assert_eq!(conflict["ok"], false);
    fs::write(&agent.hold_path, "panicking-job")?;
    assert_eq!(submit(&agent, 'b', "panicking-job")?["status"], "running");
    let cancel = agent.request("old-cancel", "cancel_durable_agent_execution", json!({
        "execution_id":failed["execution_id"], "process_instance_id":failed["process_instance_id"],
        "request_digest":failed["request_digest"]
    }))?;
    assert_eq!(
        successful_result(&cancel, "old-cancel")["status"],
        "not_running"
    );
    assert_eq!(lifecycle(&agent, "new-owner")?["active_execution_count"], 1);
    fs::remove_file(&agent.hold_path)?;
    assert_eq!(
        wait_status(&agent, 'b', "completed")?["response"]["result"]["max_stress"],
        10.0
    );
    wait_for_lifecycle(&agent, "accepting", 0)?;
    Ok(())
}

#[test]
fn caught_worker_panic_receipt_survives_kill_before_owner_release_and_two_restarts()
-> Result<(), Box<dyn Error>> {
    let mut agent = LiveAgent::start_with_durable_panic(&"a".repeat(32))?;
    let hold = agent.root.join("receipt.hold");
    fs::write(&hold, "a".repeat(32))?;
    assert_eq!(
        submit(&agent, 'a', "kill-after-panic")?["status"],
        "running"
    );
    let failed = wait_status(&agent, 'a', "failed")?;
    assert_panicked(&failed);
    let drain = agent.request(
        "drain",
        "begin_agent_drain",
        json!({"controller_id":"test", "reason":"test"}),
    )?;
    let draining = successful_result(&drain, "drain");
    assert_eq!(draining["active_execution_count"], 1);
    assert_eq!(draining["safe_to_replace"], false);
    agent.stop_process()?;
    fs::remove_file(&hold)?;
    for _ in 0..2 {
        agent.start_process()?;
        assert_eq!(fetch(&agent, 'a')?, failed);
        for method in ["submit_durable_agent_task", "run_durable_agent_task"] {
            let duplicate = agent.request("repeat", method, task('a', "kill-after-panic"))?;
            assert_eq!(*successful_result(&duplicate, "repeat"), failed);
        }
        assert_eq!(lifecycle(&agent, "no-replay")?["active_execution_count"], 0);
        agent.stop_process()?;
    }
    Ok(())
}

#[test]
fn caught_worker_panic_with_failed_receipt_commit_stays_unknown_after_restart()
-> Result<(), Box<dyn Error>> {
    let mut agent = LiveAgent::start_with_durable_panic(&"a".repeat(32))?;
    let obstruction = agent
        .root
        .join("executions")
        .join(format!("{}.receipt.next", "a".repeat(32)));
    fs::create_dir(&obstruction)?;
    assert_eq!(
        submit(&agent, 'a', "panic-commit-failed")?["status"],
        "running"
    );
    let unknown = wait_status(&agent, 'a', "outcome_unknown")?;
    assert!(unknown["response"].is_null());
    assert_eq!(unknown["automatic_replay_authorized"], false);
    wait_for_lifecycle(&agent, "accepting", 0)?;
    fs::remove_dir(&obstruction)?;
    let blocked = agent.request("blocked", "submit_durable_agent_task", task('b', "new"))?;
    assert_eq!(blocked["ok"], false);
    assert_eq!(fetch(&agent, 'b')?["status"], "not_found");
    agent.stop_process()?;
    agent.start_process()?;
    assert_eq!(fetch(&agent, 'a')?, unknown);
    for method in ["submit_durable_agent_task", "run_durable_agent_task"] {
        let duplicate = agent.request("repeat", method, task('a', "panic-commit-failed"))?;
        assert_eq!(*successful_result(&duplicate, "repeat"), unknown);
    }
    assert_eq!(lifecycle(&agent, "no-replay")?["active_execution_count"], 0);
    assert_eq!(submit(&agent, 'b', "new")?["status"], "running");
    assert_eq!(
        wait_status(&agent, 'b', "completed")?["response"]["result"]["max_stress"],
        10.0
    );
    Ok(())
}
