use serde_json::{Value, json};
use std::error::Error;
use std::fs;
use std::path::Path;
use std::thread;
use std::time::{Duration, Instant};

#[allow(dead_code)]
#[path = "support/agent_lifecycle.rs"]
mod support;
use support::*;

struct Holds<'a>(&'a [&'a Path]);
impl Drop for Holds<'_> {
    fn drop(&mut self) {
        for path in self.0 {
            let _ = fs::remove_file(path);
        }
    }
}

fn observed_target(agent: &LiveAgent, id: &str) -> Result<Value, Box<dyn Error>> {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let state = agent.request("exact-cancel-observe", "describe_agent", json!({}))?;
        if let Some(point) = state["result"]["solver_control"]["active"]
            .as_array()
            .and_then(|points| points.iter().find(|point| point["request_id"] == id))
        {
            assert_eq!(point["cancel_requested"], false, "{state}");
            return Ok(
                json!({"process_instance_id":state["result"]["lifecycle"]["process_instance_id"],
                "request_id":point["request_id"], "generation":point["generation"], "job_id":point["job_id"]}),
            );
        }
        if Instant::now() >= deadline {
            return Err(format!("execution {id} not held: {state}").into());
        }
        thread::sleep(Duration::from_millis(10));
    }
}

fn cancel(agent: &LiveAgent, target: &Value, matched: bool) -> Result<(), Box<dyn Error>> {
    let reply = agent.request("exact-cancel", "cancel_execution", target.clone())?;
    assert_eq!(reply["ok"], true, "{reply}");
    assert_eq!(
        reply["result"],
        json!({"schema_version":"kyuubiki.agent-execution-cancellation/v1",
        "execution_target":target, "status":if matched {"requested"} else {"target_not_observed"},
        "cancel_registered":matched, "execution_terminal_confirmed":false,
        "pending_cancellation_created":false, "operator_package_cleanup_performed":false,
        "automatic_replay_authorized":false})
    );
    Ok(())
}

fn healthy(reply: &Value) {
    assert_eq!(reply["ok"], true, "{reply}");
    let displacement = reply["result"]["tip_displacement"].as_f64().unwrap();
    // Independent FL/EA and F/A checks for the fixed fixture, not a second solver call.
    assert!((displacement - 20.0 / (1000.0 * 2.0)).abs() < 1e-12);
    assert_eq!(reply["result"]["max_stress"], 10.0);
}

fn cancelled(reply: &Value, target: &Value) {
    assert_eq!(reply["ok"], false, "{reply}");
    assert!(reply["result"].is_null());
    assert_eq!(reply["id"], target["request_id"]);
    assert_eq!(reply["error"]["code"], "cancelled");
    assert_eq!(
        reply["error"]["details"]["generation"],
        target["generation"]
    );
    assert_eq!(reply["error"]["details"]["job_id"], target["job_id"]);
}

fn solve(port: u16, id: &str, job: &str) -> Result<Value, String> {
    rpc_request(port, id, "solve_bar_1d", solver_params(job)).map_err(|error| error.to_string())
}

#[test]
fn exact_cancel_preserves_same_job_sibling_peer_and_reused_request() -> Result<(), Box<dyn Error>> {
    let owner = LiveAgent::start_with_capacity("2")?;
    let peer = LiveAgent::start()?;
    let job = "exact-cancel-shared-job";
    let id = "exact-cancel-reused-request";
    thread::scope(|scope| -> Result<(), Box<dyn Error>> {
        let paths = [owner.hold_path.as_path(), peer.hold_path.as_path()];
        let holds = Holds(&paths);
        for path in paths {
            fs::write(path, job)?;
        }
        let first = scope.spawn(|| solve(owner.port, id, job));
        let original = observed_target(&owner, id)?;
        let sibling = scope.spawn(|| solve(owner.port, "exact-cancel-sibling", job));
        observed_target(&owner, "exact-cancel-sibling")?;
        let bystander = scope.spawn(|| solve(peer.port, id, job));
        let peer_target = observed_target(&peer, id)?;
        assert_eq!(original["generation"], peer_target["generation"]);
        assert_ne!(
            original["process_instance_id"],
            peer_target["process_instance_id"]
        );
        cancel(&peer, &original, false)?;
        cancel(&owner, &original, true)?;
        cancelled(
            &first.join().map_err(|_| "selected solver panicked")??,
            &original,
        );
        wait_for_lifecycle(&owner, "accepting", 1)?;
        cancel(&owner, &original, false)?;
        observed_target(&owner, "exact-cancel-sibling")?;
        observed_target(&peer, id)?;
        let rerun = scope.spawn(|| solve(owner.port, id, job));
        let current = observed_target(&owner, id)?;
        assert_ne!(current["generation"], original["generation"]);
        cancel(&owner, &original, false)?;
        assert_eq!(observed_target(&owner, id)?, current);
        drop(holds);
        for worker in [rerun, sibling, bystander] {
            healthy(&worker.join().map_err(|_| "healthy solver panicked")??);
        }
        Ok(())
    })?;
    wait_for_lifecycle(&owner, "accepting", 0)?;
    wait_for_lifecycle(&peer, "accepting", 0)?;
    Ok(())
}

#[test]
fn prior_process_cancel_cannot_hit_the_same_request_and_generation_after_agent_restart()
-> Result<(), Box<dyn Error>> {
    let mut agent = LiveAgent::start()?;
    let job = "exact-cancel-restart-job";
    let id = "exact-cancel-restart-request";
    let original = thread::scope(|scope| -> Result<Value, Box<dyn Error>> {
        let paths = [agent.hold_path.as_path()];
        let holds = Holds(&paths);
        fs::write(&agent.hold_path, job)?;
        let first = scope.spawn(|| solve(agent.port, id, job));
        let target = observed_target(&agent, id)?;
        cancel(&agent, &target, true)?;
        cancelled(
            &first.join().map_err(|_| "original solver panicked")??,
            &target,
        );
        drop(holds);
        Ok(target)
    })?;
    wait_for_lifecycle(&agent, "accepting", 0)?;
    agent.stop_process()?;
    agent.start_process()?;
    thread::scope(|scope| -> Result<(), Box<dyn Error>> {
        let paths = [agent.hold_path.as_path()];
        let holds = Holds(&paths);
        fs::write(&agent.hold_path, job)?;
        let rerun = scope.spawn(|| solve(agent.port, id, job));
        let current = observed_target(&agent, id)?;
        assert_eq!(current["generation"], original["generation"]);
        assert_ne!(
            current["process_instance_id"],
            original["process_instance_id"]
        );
        cancel(&agent, &original, false)?;
        assert_eq!(observed_target(&agent, id)?, current);
        drop(holds);
        healthy(&rerun.join().map_err(|_| "restarted solver panicked")??);
        Ok(())
    })?;
    wait_for_lifecycle(&agent, "accepting", 0)?;
    cancel(&agent, &original, false)?;
    healthy(&agent.request(id, "solve_bar_1d", solver_params(job))?);
    Ok(())
}
