use serde_json::{Value, json};
use std::error::Error;
use std::fs;
use std::thread;
use std::time::{Duration, Instant};

#[allow(dead_code)]
#[path = "support/agent_lifecycle.rs"]
mod support;
use support::*;

fn task(id: char, job: &str) -> Value {
    json!({"execution_id":id.to_string().repeat(32), "method":"solve_bar_1d", "params":solver_params(job)})
}

fn fetch(agent: &LiveAgent, id: char) -> Result<Value, Box<dyn Error>> {
    let response = agent.request(
        "fetch-durable",
        "fetch_durable_agent_execution",
        json!({"execution_id":id.to_string().repeat(32)}),
    )?;
    Ok(successful_result(&response, "fetch-durable").clone())
}

fn wait_running(agent: &LiveAgent, id: char) -> Result<(), Box<dyn Error>> {
    let deadline = Instant::now() + Duration::from_secs(10);
    while Instant::now() < deadline {
        if fetch(agent, id)?["status"] == "running" {
            return Ok(());
        }
        thread::sleep(Duration::from_millis(10));
    }
    Err("durable admission did not become visible".into())
}

#[test]
fn completed_receipt_survives_sigkill_and_retries_without_solver_replay()
-> Result<(), Box<dyn Error>> {
    let mut agent = LiveAgent::start_with_durable_store()?;
    let descriptor = agent.request("descriptor", "describe_agent", json!({}))?;
    assert_eq!(descriptor["result"]["durable_execution"]["enabled"], true);
    assert_eq!(
        descriptor["result"]["durable_execution"]["automatic_eviction"],
        false
    );
    let task = task('a', "durable-completed");
    let result = agent.request("first", "run_durable_agent_task", task.clone())?;
    let report = successful_result(&result, "first").clone();
    assert_eq!(report["status"], "completed");
    assert_eq!(report["response"]["result"]["max_stress"], 10.0);
    let original_process = report["process_instance_id"].clone();
    // A replay would enter this hold; retained reads must not invoke the solver at all.
    fs::write(&agent.hold_path, "durable-completed")?;
    for _ in 0..2 {
        agent.stop_process()?; // Child::kill is SIGKILL on Unix; no shutdown hooks.
        agent.start_process()?;
        assert_ne!(
            lifecycle(&agent, "new-process")?["process_instance_id"],
            original_process
        );
        assert_eq!(fetch(&agent, 'a')?, report);
        let retried = agent.request("retry", "run_durable_agent_task", task.clone())?;
        assert_eq!(*successful_result(&retried, "retry"), report);
        assert_eq!(lifecycle(&agent, "no-replay")?["active_execution_count"], 0);
    }
    Ok(())
}

#[test]
fn killed_inflight_execution_remains_unknown_across_restarts_and_retries()
-> Result<(), Box<dyn Error>> {
    let mut agent = LiveAgent::start_with_durable_store()?;
    fs::write(&agent.hold_path, "durable-interrupted")?;
    let port = agent.port;
    let request = thread::spawn(move || {
        rpc_request(
            port,
            "inflight",
            "run_durable_agent_task",
            task('b', "durable-interrupted"),
        )
        .map_err(|e| e.to_string())
    });
    wait_running(&agent, 'b')?;
    wait_for_lifecycle(&agent, "accepting", 1)?;
    let duplicate = agent.request(
        "duplicate-running",
        "run_durable_agent_task",
        task('b', "durable-interrupted"),
    )?;
    assert_eq!(
        successful_result(&duplicate, "duplicate-running")["status"],
        "running"
    );
    assert_eq!(lifecycle(&agent, "only-one")?["active_execution_count"], 1);
    agent.stop_process()?;
    assert!(
        request
            .join()
            .map_err(|_| "request thread panicked")?
            .is_err()
    );
    for _ in 0..2 {
        agent.start_process()?;
        let report = fetch(&agent, 'b')?;
        assert_eq!(report["status"], "outcome_unknown");
        assert_eq!(report["automatic_replay_authorized"], false);
        let duplicate = agent.request(
            "retry-unknown",
            "run_durable_agent_task",
            task('b', "durable-interrupted"),
        )?;
        assert_eq!(*successful_result(&duplicate, "retry-unknown"), report);
        assert_eq!(
            lifecycle(&agent, "no-new-execution")?["active_execution_count"],
            0
        );
        agent.stop_process()?;
    }
    Ok(())
}

#[test]
fn kill_after_receipt_sync_before_reply_recovers_original_result() -> Result<(), Box<dyn Error>> {
    let mut agent = LiveAgent::start_with_durable_store()?;
    fs::write(agent.root.join("receipt.hold"), "9".repeat(32))?;
    let port = agent.port;
    let request = thread::spawn(move || {
        rpc_request(
            port,
            "lost-reply",
            "run_durable_agent_task",
            task('9', "lost-reply-job"),
        )
        .map_err(|e| e.to_string())
    });
    let deadline = Instant::now() + Duration::from_secs(10);
    let retained = loop {
        let report = fetch(&agent, '9')?;
        if report["status"] == "completed" {
            break report;
        }
        if Instant::now() >= deadline {
            return Err("receipt was not committed".into());
        }
        thread::sleep(Duration::from_millis(10));
    };
    assert!(!request.is_finished(), "final reply must still be held");
    agent.stop_process()?;
    assert!(
        request
            .join()
            .map_err(|_| "request thread panicked")?
            .is_err()
    );
    fs::write(&agent.hold_path, "lost-reply-job")?;
    for _ in 0..2 {
        agent.start_process()?;
        assert_eq!(fetch(&agent, '9')?, retained);
        let retry = agent.request(
            "retry-lost-reply",
            "run_durable_agent_task",
            task('9', "lost-reply-job"),
        )?;
        assert_eq!(*successful_result(&retry, "retry-lost-reply"), retained);
        assert_eq!(lifecycle(&agent, "no-replay")?["active_execution_count"], 0);
        agent.stop_process()?;
    }
    Ok(())
}

#[test]
fn different_inputs_cannot_rebind_an_execution_and_failures_are_retained()
-> Result<(), Box<dyn Error>> {
    let mut agent = LiveAgent::start_with_durable_store()?;
    let mut invalid = task('c', "invalid-input");
    invalid["params"]["elements"] = json!(0);
    let reply = agent.request("bad-input", "run_durable_agent_task", invalid.clone())?;
    let report = successful_result(&reply, "bad-input").clone();
    assert_eq!(report["status"], "failed");
    assert_eq!(report["response"]["ok"], false);
    let conflict = agent.request(
        "conflict",
        "run_durable_agent_task",
        task('c', "invalid-input"),
    )?;
    assert_eq!(conflict["ok"], false);
    assert!(
        conflict["error"]["message"]
            .as_str()
            .unwrap()
            .contains("different inputs")
    );
    agent.stop_process()?;
    agent.start_process()?;
    assert_eq!(fetch(&agent, 'c')?, report);
    Ok(())
}

#[test]
fn unconfigured_store_rejects_durable_work_instead_of_falling_back_to_memory()
-> Result<(), Box<dyn Error>> {
    let agent = LiveAgent::start()?;
    let reply = agent.request(
        "unconfigured",
        "run_durable_agent_task",
        task('d', "disabled"),
    )?;
    assert_eq!(reply["ok"], false);
    assert!(
        reply["error"]["message"]
            .as_str()
            .unwrap()
            .contains("no memory fallback")
    );
    assert_eq!(
        lifecycle(&agent, "not-started")?["active_execution_count"],
        0
    );
    Ok(())
}

#[test]
fn corrupted_receipt_blocks_only_its_execution_without_replay() -> Result<(), Box<dyn Error>> {
    let mut agent = LiveAgent::start_with_durable_store()?;
    let result = agent.request("complete", "run_durable_agent_task", task('e', "corrupt"))?;
    assert_eq!(
        successful_result(&result, "complete")["status"],
        "completed"
    );
    agent.stop_process()?;
    fs::write(
        agent
            .root
            .join("executions")
            .join(format!("{}.receipt", "e".repeat(32))),
        b"broken",
    )?;
    agent.start_process()?;
    let retry = agent.request(
        "corrupt-retry",
        "run_durable_agent_task",
        task('e', "corrupt"),
    )?;
    assert_eq!(retry["ok"], false);
    let unrelated = agent.request("unrelated", "run_durable_agent_task", task('f', "healthy"))?;
    assert_eq!(
        successful_result(&unrelated, "unrelated")["status"],
        "completed"
    );
    Ok(())
}

#[test]
fn durable_wrapper_rejects_control_rpcs_and_nested_execution() -> Result<(), Box<dyn Error>> {
    let agent = LiveAgent::start_with_durable_store()?;
    for method in [
        "begin_agent_drain",
        "run_durable_agent_task",
        "fetch_durable_agent_execution",
    ] {
        let response = agent.request(
            "forbidden",
            "run_durable_agent_task",
            json!({"execution_id":"1".repeat(32), "method":method, "params":{}}),
        )?;
        assert_eq!(response["ok"], false);
    }
    assert_eq!(fetch(&agent, '1')?["status"], "not_found");
    assert_eq!(lifecycle(&agent, "still-accepting")?["state"], "accepting");
    Ok(())
}

#[test]
fn native_task_ir_uses_the_same_durable_boundary_without_changing_the_engine()
-> Result<(), Box<dyn Error>> {
    let mut agent = LiveAgent::start_with_durable_store()?;
    let mut ir: Value = serde_json::from_str(include_str!(
        "../../../../../schemas/examples.operator-task-ir.json"
    ))?;
    ir["operator"] = json!({"id":"solve.bar_1d", "family":"mechanical", "kind":"solver"});
    ir["input_artifact"] = solver_params("task-ir");
    let program = &mut ir["execution_program"];
    program["program_id"] = json!("solve.bar_1d");
    program["program_family"] = json!("mechanical");
    program["program_kind"] = json!("solver");
    program["package_ref"] = Value::Null;
    program["runtime_protocol"] = json!("kyuubiki.solver-rpc/v1");
    program["abi"]["kind"] = json!("solver_rpc");
    program["entrypoint"] =
        json!({"kind":"solver_method", "name":"solve_bar_1d", "operator_kind":"solver"});
    ir["runtime_hints"] = json!({"authority_mode":"agent_local", "execution_mode":"agent_native",
        "cache_scope":"none", "agent_fetchable":false, "operator_kind":"solver"});
    ir["integrity"] = json!({"task_digest":kyuubiki_protocol::compute_operator_task_digest(&ir)?});
    let payload = json!({"execution_id":"8".repeat(32), "method":"run_operator_task_ir",
        "params":{"mode":"execute", "task_ir":ir}});
    let response = agent.request("native-task-ir", "run_durable_agent_task", payload)?;
    let report = successful_result(&response, "native-task-ir").clone();
    assert_eq!(report["status"], "completed", "{report}");
    let result = &report["response"]["result"]["result"];
    assert_eq!(result["max_stress"], 10.0);
    assert!((result["tip_displacement"].as_f64().unwrap() - 0.01).abs() < 1e-12);
    agent.stop_process()?;
    agent.start_process()?;
    assert_eq!(fetch(&agent, '8')?, report);
    Ok(())
}

#[test]
fn exact_execution_cancellation_remains_effective_and_its_receipt_survives_restart()
-> Result<(), Box<dyn Error>> {
    let mut agent = LiveAgent::start_with_durable_store()?;
    fs::write(&agent.hold_path, "durable-cancel")?;
    let port = agent.port;
    let request = thread::spawn(move || {
        rpc_request(
            port,
            "durable-cancel-request",
            "run_durable_agent_task",
            task('7', "durable-cancel"),
        )
        .map_err(|e| e.to_string())
    });
    wait_running(&agent, '7')?;
    wait_for_lifecycle(&agent, "accepting", 1)?;
    let descriptor = agent.request("active", "describe_agent", json!({}))?;
    let active = descriptor["result"]["solver_control"]["active"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["request_id"] == "durable-cancel-request")
        .unwrap();
    let cancel = agent.request("cancel", "cancel_execution", json!({
        "process_instance_id":descriptor["result"]["lifecycle"]["process_instance_id"],
        "request_id":active["request_id"], "generation":active["generation"], "job_id":active["job_id"]}))?;
    assert_eq!(
        successful_result(&cancel, "cancel")["cancel_registered"],
        true
    );
    let response = request.join().map_err(|_| "cancel request panicked")??;
    let report = successful_result(&response, "durable-cancel-request").clone();
    assert_eq!(report["status"], "failed");
    assert_eq!(report["response"]["error"]["code"], "cancelled");
    agent.stop_process()?;
    agent.start_process()?;
    assert_eq!(fetch(&agent, '7')?, report);
    Ok(())
}
