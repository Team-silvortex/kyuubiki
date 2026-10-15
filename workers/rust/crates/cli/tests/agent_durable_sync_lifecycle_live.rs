use serde_json::{Value, json};
use std::error::Error;
use std::fs;
use std::io::Write;
use std::net::{Shutdown, TcpStream};
use std::thread;
use std::time::{Duration, Instant};

#[allow(dead_code)]
#[path = "support/agent_durable.rs"]
mod support;
use support::*;

fn watchdog(agent: &LiveAgent) -> Result<Value, Box<dyn Error>> {
    let descriptor = agent.request("descriptor", "describe_agent", json!({}))?;
    Ok(successful_result(&descriptor, "descriptor")["watchdog"].clone())
}

fn failed_once(state: &Value, reason: &str) {
    assert_eq!(state["total_completed_execution_count"], 0);
    assert_eq!(state["total_failed_execution_count"], 1);
    let failure = state["recent_failures"]
        .as_array()
        .unwrap()
        .iter()
        .find(|failure| failure["request_id"] == "original")
        .expect("original failure diagnostic");
    assert_eq!(failure["reason_code"], reason);
    assert_eq!(failure["job_id"], "sync-job");
}

#[test]
fn synchronous_failed_commit_preserves_unknown_identity_and_never_counts_success()
-> Result<(), Box<dyn Error>> {
    let mut agent = LiveAgent::start_with_durable_store()?;
    let work = task('a', "sync-job");
    let obstruction = agent
        .root
        .join("executions")
        .join(format!("{}.receipt.next", "a".repeat(32)));
    fs::create_dir(&obstruction)?;
    let response = agent.request("original", "run_durable_agent_task", work.clone())?;
    assert_eq!(response["ok"], false);
    assert_eq!(response["error"]["code"], "durable_execution_unavailable");
    wait_for_lifecycle(&agent, "accepting", 0)?;
    failed_once(&watchdog(&agent)?, "durable_receipt_commit_failed");
    let unknown = fetch(&agent, 'a')?;
    assert_eq!(unknown["status"], "outcome_unknown");
    assert!(unknown["response"].is_null());
    for method in ["run_durable_agent_task", "submit_durable_agent_task"] {
        let duplicate = agent.request("duplicate", method, work.clone())?;
        assert_eq!(*successful_result(&duplicate, "duplicate"), unknown);
    }
    failed_once(&watchdog(&agent)?, "durable_receipt_commit_failed");
    assert_eq!(
        agent.request("blocked", "run_durable_agent_task", task('b', "sync-job"))?["ok"],
        false
    );
    assert_eq!(fetch(&agent, 'b')?["status"], "not_found");
    fs::remove_dir(obstruction)?;
    agent.stop_process()?;
    agent.start_process()?;
    assert_eq!(fetch(&agent, 'a')?, unknown);
    let result = agent.request("original", "run_durable_agent_task", task('b', "sync-job"))?;
    assert_eq!(
        successful_result(&result, "original")["response"]["result"]["max_stress"],
        10.0
    );
    wait_for_lifecycle(&agent, "accepting", 0)?;
    assert_eq!(watchdog(&agent)?["total_completed_execution_count"], 1);
    assert_eq!(watchdog(&agent)?["total_failed_execution_count"], 0);
    Ok(())
}

#[test]
fn synchronous_committed_success_keeps_its_slot_until_reply_delivery() -> Result<(), Box<dyn Error>>
{
    let agent = LiveAgent::start_with_durable_store()?;
    let hold = agent.root.join("receipt.hold");
    fs::write(&hold, "a".repeat(32))?;
    let port = agent.port;
    let request = thread::spawn(move || {
        rpc_request(
            port,
            "original",
            "run_durable_agent_task",
            task('a', "sync-job"),
        )
        .map_err(|e| e.to_string())
    });
    let completed = wait_status(&agent, 'a', "completed")?;
    assert_eq!(completed["response"]["result"]["tip_displacement"], 0.01);
    assert!(!request.is_finished());
    let state = watchdog(&agent)?;
    assert_eq!(state["total_completed_execution_count"], 0);
    assert_eq!(state["total_failed_execution_count"], 0);
    let drain = agent.request(
        "drain",
        "begin_agent_drain",
        json!({"controller_id":"test", "reason":"verify durable delivery ownership"}),
    )?;
    assert_eq!(successful_result(&drain, "drain")["safe_to_replace"], false);
    assert_eq!(
        successful_result(&drain, "drain")["active_execution_count"],
        1
    );
    fs::remove_file(hold)?;
    let response = request.join().map_err(|_| "request thread panicked")??;
    assert_eq!(*successful_result(&response, "original"), completed);
    wait_for_lifecycle(&agent, "quiescent", 0)?;
    assert_eq!(watchdog(&agent)?["total_completed_execution_count"], 1);
    assert_eq!(watchdog(&agent)?["total_failed_execution_count"], 0);
    let descriptor = agent.request("descriptor", "describe_agent", json!({}))?;
    assert_eq!(
        successful_result(&descriptor, "descriptor")["durable_execution"]["synchronous_execution"],
        json!({
            "method":"run_durable_agent_task", "completion_boundary":"receipt_commit_then_transport_delivery",
            "commit_failure_code":"durable_receipt_commit_failed", "unwind_diagnostic_code":"durable_execution_aborted",
            "unwind_commits_failure_receipt":false
        })
    );
    Ok(())
}

#[test]
fn synchronous_cancellation_and_failed_commit_preserve_the_original_failure()
-> Result<(), Box<dyn Error>> {
    let agent = LiveAgent::start_with_durable_store()?;
    fs::write(&agent.hold_path, "sync-job")?;
    let obstruction = agent
        .root
        .join("executions")
        .join(format!("{}.receipt.next", "a".repeat(32)));
    fs::create_dir(&obstruction)?;
    let port = agent.port;
    let request = thread::spawn(move || {
        rpc_request(
            port,
            "original",
            "run_durable_agent_task",
            task('a', "sync-job"),
        )
        .map_err(|e| e.to_string())
    });
    wait_status(&agent, 'a', "running")?;
    let deadline = Instant::now() + Duration::from_secs(5);
    let target = loop {
        let descriptor = agent.request("descriptor", "describe_agent", json!({}))?;
        let descriptor = successful_result(&descriptor, "descriptor");
        if let Some(active) = descriptor["solver_control"]["active"]
            .as_array()
            .unwrap()
            .iter()
            .find(|entry| entry["request_id"] == "original")
        {
            break json!({"process_instance_id":descriptor["lifecycle"]["process_instance_id"],
                "request_id":"original", "generation":active["generation"], "job_id":"sync-job"});
        }
        if Instant::now() >= deadline {
            return Err("synchronous owner was not observed".into());
        }
        thread::sleep(Duration::from_millis(10));
    };
    let cancel = agent.request("cancel", "cancel_execution", target)?;
    assert_eq!(successful_result(&cancel, "cancel")["status"], "requested");
    let response = request.join().map_err(|_| "request thread panicked")??;
    assert_eq!(response["ok"], false);
    assert_eq!(fetch(&agent, 'a')?["status"], "outcome_unknown");
    wait_for_lifecycle(&agent, "accepting", 0)?;
    failed_once(&watchdog(&agent)?, "cancelled");
    Ok(())
}

#[test]
fn disconnected_synchronous_task_keeps_a_failed_receipt_and_releases_capacity()
-> Result<(), Box<dyn Error>> {
    for task_ir in [false, true] {
        let agent = LiveAgent::start_with_durable_store()?;
        fs::write(&agent.hold_path, "sync-job")?;
        let work = if task_ir {
            json!({"execution_id":"a".repeat(32), "method":"run_operator_task_ir",
            "params":{"mode":"execute", "job_id":"sync-job", "task_ir":native_task_ir("sync-job")?}})
        } else {
            task('a', "sync-job")
        };
        let mut stream = TcpStream::connect(("127.0.0.1", agent.port))?;
        stream.set_read_timeout(Some(Duration::from_secs(10)))?;
        stream.set_write_timeout(Some(Duration::from_secs(10)))?;
        let payload = serde_json::to_vec(&json!({"rpc_version":RPC_VERSION, "id":"original",
            "method":"run_durable_agent_task", "params":work}))?;
        stream.write_all(&u32::try_from(payload.len())?.to_be_bytes())?;
        stream.write_all(&payload)?;
        assert!(read_json_frame(&mut stream)?.get("ok").is_none());
        stream.shutdown(Shutdown::Both)?;
        drop(stream);
        let failed = wait_status(&agent, 'a', "failed")?;
        assert_eq!(failed["response"]["error"]["code"], "cancelled");
        assert!(failed["response"]["result"].is_null());
        wait_for_lifecycle(&agent, "accepting", 0)?;
        failed_once(&watchdog(&agent)?, "cancelled");
        let duplicate = agent.request("duplicate", "run_durable_agent_task", work)?;
        assert_eq!(*successful_result(&duplicate, "duplicate"), failed);
        fs::remove_file(&agent.hold_path)?;
        let next = agent.request("original", "run_durable_agent_task", task('b', "sync-job"))?;
        assert_eq!(successful_result(&next, "original")["status"], "completed");
        wait_for_lifecycle(&agent, "accepting", 0)?;
        assert_eq!(watchdog(&agent)?["total_completed_execution_count"], 1);
        assert_eq!(watchdog(&agent)?["total_failed_execution_count"], 1);
    }
    Ok(())
}
