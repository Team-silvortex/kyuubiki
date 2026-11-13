#[allow(dead_code)]
#[path = "support/agent_lifecycle.rs"]
mod support;
use serde_json::{Value, json};
use std::{
    error::Error,
    fs,
    path::Path,
    thread,
    time::{Duration, Instant},
};
use support::*;

const TOKEN: &str = "test-only-pending-cancel-token-0123456789abcdef";

struct Hold<'a>(&'a Path);
impl Drop for Hold<'_> {
    fn drop(&mut self) {
        let _ = fs::remove_file(self.0);
    }
}

fn call(agent: &LiveAgent, id: &str, method: &str, params: Value) -> Value {
    let response = agent.request(id, method, params).unwrap();
    assert_eq!(response["id"], id);
    response
}

fn describe(agent: &LiveAgent) -> Value {
    let reply = call(agent, "pending-describe", "describe_agent", json!({}));
    successful_result(&reply, "pending-describe").clone()
}

fn count(agent: &LiveAgent, expected: usize) {
    assert_eq!(
        describe(agent)["solver_control"]["pending_job_cancellations"]["count"],
        expected
    );
}

fn cancel(agent: &LiveAgent, job: &str) {
    let reply = call(
        agent,
        "register-cancel",
        "cancel_job",
        json!({"job_id":job}),
    );
    let result = successful_result(&reply, "register-cancel");
    assert_eq!(result["job_id"], job);
    assert_eq!(result["cancel_registered"], true);
    assert_eq!(result["execution_terminal_confirmed"], false);
}

fn rejected(reply: &Value, code: &str) {
    assert_eq!(reply["ok"], false, "{reply}");
    assert!(reply["result"].is_null());
    assert_eq!(reply["error"]["code"], code, "{reply}");
}

fn healthy(reply: &Value) {
    assert_eq!(reply["ok"], true, "{reply}");
    assert!((reply["result"]["tip_displacement"].as_f64().unwrap() - 0.01).abs() < 1e-12);
    assert_eq!(reply["result"]["max_stress"], 10.0);
}

fn target(agent: &LiveAgent, id: &str) -> Value {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let descriptor = describe(agent);
        if let Some(active) = descriptor["solver_control"]["active"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["request_id"] == id)
        {
            assert_eq!(active["cancel_requested"], false);
            return json!({"process_instance_id":descriptor["lifecycle"]["process_instance_id"],
                "request_id":id,"job_id":active["job_id"],"generation":active["generation"]});
        }
        assert!(Instant::now() < deadline, "held execution never registered");
        thread::sleep(Duration::from_millis(5));
    }
}

fn active_recovery_at_capacity(agent: &LiveAgent, limit: usize) {
    let job = "running-with-full-pending-registry";
    thread::scope(|scope| {
        let hold = Hold(&agent.hold_path);
        fs::write(hold.0, job).unwrap();
        let first = scope.spawn(|| call(agent, "active-first", "solve_bar_1d", solver_params(job)));
        let first_target = target(agent, "active-first");
        let second =
            scope.spawn(|| call(agent, "active-second", "solve_bar_1d", solver_params(job)));
        let second_target = target(agent, "active-second");
        let excess = call(
            agent,
            "not-admitted",
            "solve_bar_1d",
            solver_params("pending-0"),
        );
        rejected(&excess, "agent_at_capacity");
        count(agent, limit);
        let exact = call(agent, "cancel-exact", "cancel_execution", first_target);
        let receipt = successful_result(&exact, "cancel-exact");
        assert_eq!(receipt["cancel_registered"], true);
        assert_eq!(receipt["pending_cancellation_created"], false);
        assert_eq!(receipt["operator_package_cleanup_performed"], false);
        rejected(&first.join().unwrap(), "cancelled");
        assert_eq!(target(agent, "active-second"), second_target);
        count(agent, limit);
        cancel(agent, job);
        rejected(&second.join().unwrap(), "cancelled");
        count(agent, limit);
        drop(hold);
    });
    wait_for_lifecycle(agent, "accepting", 0).unwrap();
    healthy(&call(
        agent,
        "same-job-next",
        "solve_bar_1d",
        solver_params(job),
    ));
    count(agent, limit);
}

#[test]
#[ignore = "fills an owned TLS Agent's pending cancellation registry; no daily services or credentials"]
fn tls_pending_cancellation_capacity_preserves_active_recovery_and_one_shot_intents()
-> Result<(), Box<dyn Error>> {
    let cert = rcgen::generate_simple_self_signed(vec!["localhost".into()])?;
    let mut agent = LiveAgent::start_with_capacity("2")?;
    agent.restart_with_api_security(&cert.cert.pem(), &cert.signing_key.serialize_pem(), TOKEN)?;
    let descriptor = describe(&agent);
    assert_eq!(descriptor["connection_access"]["authenticated"], true);
    assert_eq!(descriptor["connection_access"]["encrypted"], true);
    let policy = &descriptor["solver_control"]["pending_job_cancellations"];
    assert_eq!(policy["max_count"], 1024);
    assert_eq!(policy["max_job_id_bytes"], 256);
    assert_eq!(policy["on_capacity"], "reject_new_pending_without_eviction");
    assert_eq!(policy["survives_process_restart"], false);
    let limit = policy["max_count"].as_u64().unwrap() as usize;
    count(&agent, 0);
    for invalid in [
        String::new(),
        "x".repeat(257),
        "\u{00e9}".repeat(129),
        "bad\njob".into(),
    ] {
        let reply = call(
            &agent,
            "invalid-id",
            "cancel_job",
            json!({"job_id":invalid}),
        );
        rejected(&reply, "operator_package_job_id_invalid");
    }
    count(&agent, 0);
    for n in 0..limit {
        cancel(&agent, &format!("pending-{n}"));
    }
    count(&agent, limit);
    for n in 0..32 {
        let reply = call(
            &agent,
            "over-limit",
            "cancel_job",
            json!({"job_id":format!("rejected-{n}")}),
        );
        rejected(&reply, "pending_cancellation_capacity_exceeded");
        let details = &reply["error"]["details"];
        for field in [
            "cancel_registered",
            "pending_cancellation_created",
            "execution_terminal_confirmed",
            "operator_package_cleanup_performed",
            "automatic_replay_authorized",
        ] {
            assert_eq!(details[field], false, "{field}");
        }
    }
    for _ in 0..3 {
        cancel(&agent, "pending-0");
    }
    count(&agent, limit);
    active_recovery_at_capacity(&agent, limit);

    let drain = call(
        &agent,
        "drain",
        "begin_agent_drain",
        json!({"controller_id":"pending-cancel-test","reason":"verify no intent consumption on rejected admission"}),
    );
    let generation = successful_result(&drain, "drain")["drain_generation"].clone();
    let blocked = call(
        &agent,
        "draining-job",
        "solve_bar_1d",
        solver_params("pending-0"),
    );
    rejected(&blocked, "agent_draining");
    count(&agent, limit);
    let resume = call(
        &agent,
        "resume",
        "resume_agent_admission",
        json!({"controller_id":"pending-cancel-test","drain_generation":generation}),
    );
    successful_result(&resume, "resume");

    for n in 0..limit {
        let id = format!("consume-{n}");
        let reply = call(
            &agent,
            &id,
            "solve_bar_1d",
            solver_params(&format!("pending-{n}")),
        );
        rejected(&reply, "cancelled");
        assert_eq!(reply["error"]["details"]["job_id"], format!("pending-{n}"));
    }
    count(&agent, 0);
    for job in ["pending-0", "pending-1023", "rejected-0"] {
        healthy(&call(
            &agent,
            "after-consumption",
            "solve_bar_1d",
            solver_params(job),
        ));
    }
    cancel(&agent, "intent-before-restart");
    count(&agent, 1);
    wait_for_lifecycle(&agent, "accepting", 0)?;
    agent.stop_process()?;
    agent.start_process()?;
    count(&agent, 0);
    healthy(&call(
        &agent,
        "after-restart",
        "solve_bar_1d",
        solver_params("intent-before-restart"),
    ));
    let logs = fs::read_to_string(&agent.log_path)?;
    assert!(!logs.contains(TOKEN) && !logs.contains("PRIVATE KEY"));
    println!(
        "pending cancellation: 1024 retained/consumed, 32 rejected, active/exact cancellation and drain/capacity fencing passed over TLS; restart is explicitly nondurable"
    );
    Ok(())
}
