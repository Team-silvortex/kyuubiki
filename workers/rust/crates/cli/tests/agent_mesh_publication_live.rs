use serde_json::{Value, json};
use std::error::Error;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

#[allow(dead_code)]
#[path = "support/agent_durable.rs"]
mod support;
use support::*;

fn private_file(path: &Path, bytes: &[u8]) -> Result<(), Box<dyn Error>> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(path)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    Ok(())
}

fn record_path(agent: &LiveAgent, execution: char, suffix: &str) -> PathBuf {
    agent
        .root
        .join("executions")
        .join(format!("{}.{suffix}", execution.to_string().repeat(32)))
}

fn submit(agent: &LiveAgent, execution: char, job: &str) -> Result<Value, Box<dyn Error>> {
    let response = agent.request(
        "original",
        "submit_durable_agent_task",
        task(execution, job),
    )?;
    Ok(successful_result(&response, "original").clone())
}

fn assert_no_replay(
    agent: &LiveAgent,
    execution: char,
    job: &str,
    expected: &Value,
) -> Result<(), Box<dyn Error>> {
    for method in ["submit_durable_agent_task", "run_durable_agent_task"] {
        let response = agent.request("duplicate", method, task(execution, job))?;
        assert_eq!(successful_result(&response, "duplicate"), expected);
    }
    assert_eq!(lifecycle(agent, "no-replay")?["active_execution_count"], 0);
    assert_eq!(expected["automatic_replay_authorized"], false);
    Ok(())
}

#[test]
fn staging_collision_retains_evidence_and_mesh_neighbor_survives_until_restart()
-> Result<(), Box<dyn Error>> {
    let mut a = LiveAgent::start_with_durable_store()?;
    let mut b = LiveAgent::start_with_durable_store()?;
    a.restart_in_mesh(
        "publication-collision",
        vec![format!("127.0.0.1:{}", b.port)],
        1000,
    )?;
    b.restart_in_mesh(
        "publication-collision",
        vec![format!("127.0.0.1:{}", a.port)],
        1000,
    )?;
    let descriptor = a.request("policy", "describe_agent", json!({}))?;
    let durable = &successful_result(&descriptor, "policy")["durable_execution"];
    assert_eq!(
        durable["record_publication"]["overwrites_existing_record"],
        false
    );
    assert_eq!(
        durable["record_publication"]["hard_link_support_required"],
        true
    );

    fs::write(&a.hold_path, "held")?;
    let running = submit(&a, 'a', "held")?;
    assert_eq!(running["status"], "running");
    let pending = record_path(&a, 'a', "receipt.next");
    private_file(&pending, b"preexisting staging evidence")?;
    fs::remove_file(&a.hold_path)?;
    let unknown = wait_status(&a, 'a', "outcome_unknown")?;
    wait_for_lifecycle(&a, "accepting", 0)?;
    assert_eq!(fs::read(&pending)?, b"preexisting staging evidence");
    assert!(!record_path(&a, 'a', "receipt").exists());
    assert_no_replay(&a, 'a', "held", &unknown)?;
    let blocked = a.request("blocked", "submit_durable_agent_task", task('b', "new"))?;
    assert_eq!(blocked["ok"], false);
    assert_eq!(blocked["error"]["code"], "durable_execution_unavailable");
    assert_eq!(
        blocked["error"]["details"]["automatic_replay_authorized"],
        false
    );
    assert!(!record_path(&a, 'b', "admission").exists());
    submit(&b, 'c', "neighbor")?;
    let neighbor = wait_status(&b, 'c', "completed")?;
    assert_eq!(neighbor["response"]["result"]["max_stress"], 10.0);

    a.stop_process()?;
    a.start_process()?;
    assert!(!pending.exists());
    assert_eq!(fetch(&a, 'a')?, unknown);
    assert_no_replay(&a, 'a', "held", &unknown)?;
    assert_eq!(fetch(&b, 'c')?, neighbor);
    submit(&a, 'b', "new")?;
    assert_eq!(
        wait_status(&a, 'b', "completed")?["response"]["result"]["max_stress"],
        10.0
    );
    Ok(())
}

#[test]
fn forced_restart_recovers_published_names_but_never_promotes_pending_receipts()
-> Result<(), Box<dyn Error>> {
    let mut agent = LiveAgent::start_with_durable_store()?;
    agent.restart_in_mesh("publication-restart", vec![], 1000)?;
    for (execution, stage) in [('a', "admission"), ('b', "receipt"), ('c', "pending_only")] {
        if stage == "admission" {
            fs::write(&agent.hold_path, "interrupted")?;
        }
        let running = submit(&agent, execution, "interrupted")?;
        assert_eq!(running["status"], "running");
        let original = if stage == "admission" {
            running
        } else {
            let completed = wait_status(&agent, execution, "completed")?;
            wait_for_lifecycle(&agent, "accepting", 0)?;
            completed
        };
        let suffix = if stage == "admission" {
            "admission"
        } else {
            "receipt"
        };
        let final_path = record_path(&agent, execution, suffix);
        let pending = record_path(&agent, execution, &format!("{suffix}.next"));
        let retained_bytes = fs::read(&final_path)?;
        // Construct exact on-disk publication states, then kill the real process.
        // This is not a physical power-loss test or a scheduler-timed commit race.
        if stage == "pending_only" {
            fs::rename(&final_path, &pending)?;
        } else {
            fs::hard_link(&final_path, &pending)?;
        }
        agent.stop_process()?;
        if stage == "admission" {
            fs::remove_file(&agent.hold_path)?;
        }
        for _ in 0..2 {
            agent.start_process()?;
            let recovered = fetch(&agent, execution)?;
            for field in [
                "execution_id",
                "request_id",
                "request_digest",
                "process_instance_id",
                "store_id",
            ] {
                assert_eq!(recovered[field], original[field]);
            }
            assert!(!pending.exists());
            if stage == "receipt" {
                assert_eq!(recovered, original);
            } else {
                assert_eq!(recovered["status"], "outcome_unknown");
                assert!(recovered["response"].is_null());
            }
            if stage == "pending_only" {
                assert!(!final_path.exists());
            } else {
                assert_eq!(fs::read(&final_path)?, retained_bytes);
            }
            assert_no_replay(&agent, execution, "interrupted", &recovered)?;
            let index = agent.request(
                "index",
                "list_durable_agent_executions",
                json!({"limit":24}),
            )?;
            assert!(
                successful_result(&index, "index")["executions"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|record| record["execution_id"] == execution.to_string().repeat(32))
            );
            agent.stop_process()?;
        }
        agent.start_process()?;
    }
    submit(&agent, 'd', "healthy")?;
    assert_eq!(
        wait_status(&agent, 'd', "completed")?["response"]["result"]["max_stress"],
        10.0
    );
    Ok(())
}
