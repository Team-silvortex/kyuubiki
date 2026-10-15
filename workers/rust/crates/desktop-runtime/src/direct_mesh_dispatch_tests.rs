use super::*;
use std::net::TcpListener;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, AtomicUsize, Ordering},
};
use std::thread;

struct Peer {
    endpoint: String,
    executions: Arc<AtomicUsize>,
    stop: Arc<AtomicBool>,
    join: Option<thread::JoinHandle<()>>,
    descriptor: Arc<Mutex<Value>>,
}

impl Peer {
    fn start(behavior: &'static str, health: u8) -> Self {
        Self::start_with_descriptor(behavior, accepting_descriptor(health))
    }

    fn start_with_descriptor(behavior: &'static str, descriptor: Value) -> Self {
        let descriptor = Arc::new(Mutex::new(descriptor));
        let advertised = Arc::clone(&descriptor);
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let endpoint = listener.local_addr().unwrap().to_string();
        listener.set_nonblocking(true).unwrap();
        let stop = Arc::new(AtomicBool::new(false));
        let executions = Arc::new(AtomicUsize::new(0));
        let stopped = Arc::clone(&stop);
        let calls = Arc::clone(&executions);
        let join = thread::spawn(move || {
            while !stopped.load(Ordering::Acquire) {
                let (mut stream, _) = match listener.accept() {
                    Ok(socket) => socket,
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(5));
                        continue;
                    }
                    Err(error) => panic!("fake mesh peer accept: {error}"),
                };
                stream.set_nonblocking(false).unwrap();
                stream
                    .set_read_timeout(Some(Duration::from_secs(2)))
                    .unwrap();
                stream
                    .set_write_timeout(Some(Duration::from_secs(2)))
                    .unwrap();
                let request = read_frame(&mut stream, "fake-peer").unwrap();
                let describe = request["method"] == "describe_agent";
                if !describe {
                    calls.fetch_add(1, Ordering::AcqRel);
                    if behavior == "lost-reply" {
                        continue;
                    }
                }
                let failure = (describe && behavior == "unavailable")
                    || (!describe && behavior == "solver-error");
                let response = if failure {
                    json!({"rpc_version":1, "id":request["id"], "ok":false,
                        "error":{"code":"test-failure", "message":"test failure"}})
                } else {
                    json!({"rpc_version":1, "id":request["id"], "ok":true,
                        "result":if describe {advertised.lock().unwrap().clone()} else {json!({"max_stress":10.0})}})
                };
                let bytes = serde_json::to_vec(&response).unwrap();
                stream
                    .write_all(&(bytes.len() as u32).to_be_bytes())
                    .unwrap();
                stream.write_all(&bytes).unwrap();
            }
        });
        Self {
            endpoint,
            executions,
            stop,
            join: Some(join),
            descriptor,
        }
    }

    fn count(&self) -> usize {
        self.executions.load(Ordering::Acquire)
    }
}

fn accepting_descriptor(health: u8) -> Value {
    let mut descriptor =
        serde_json::to_value(kyuubiki_protocol::AgentDescriptor::solver_agent_default()).unwrap();
    descriptor["runtime"]["health_score"] = json!(health);
    descriptor["execution_admission"] = json!({
        "schema_version":"kyuubiki.agent-execution-admission/v1", "configured":true,
        "max_active_executions":1, "active_execution_count":0
    });
    descriptor
}

#[test]
fn online_but_ineligible_peer_is_skipped_before_new_task_dispatch() {
    let mut draining = accepting_descriptor(100);
    draining["lifecycle"]["state"] = json!("quiescent");
    draining["lifecycle"]["accepting_new_work"] = json!(false);
    draining["lifecycle"]["quiescent"] = json!(true);
    draining["lifecycle"]["safe_to_replace"] = json!(true);
    draining["lifecycle"]["drain_generation"] = json!(1);
    draining["lifecycle"]["drain_owner_id"] = json!("controller");
    draining["lifecycle"]["drain_reason"] = json!("rolling replacement");
    draining["lifecycle"]["drain_started_unix_ms"] = json!(1);
    let mut active_draining = draining.clone();
    active_draining["lifecycle"]["state"] = json!("draining");
    active_draining["lifecycle"]["quiescent"] = json!(false);
    active_draining["lifecycle"]["safe_to_replace"] = json!(false);
    active_draining["lifecycle"]["active_execution_count"] = json!(1);
    active_draining["execution_admission"]["max_active_executions"] = json!(2);
    active_draining["execution_admission"]["active_execution_count"] = json!(1);
    let mut full = accepting_descriptor(100);
    full["execution_admission"]["active_execution_count"] = json!(1);
    full["lifecycle"]["active_execution_count"] = json!(1);
    let mut unsupported = accepting_descriptor(100);
    unsupported["protocol"]["methods"] = json!(["ping", "describe_agent"]);
    let mut missing = accepting_descriptor(100);
    missing.as_object_mut().unwrap().remove("lifecycle");
    for mode in ["first_reachable", "healthiest"] {
        for descriptor in [&draining, &active_draining, &full, &unsupported, &missing] {
            let first = Peer::start_with_descriptor("success", descriptor.clone());
            let second = Peer::start("success", 10);
            let result = solve(
                "solve_bar_1d",
                json!({}),
                &[first.endpoint.clone(), second.endpoint.clone()],
                mode,
            )
            .unwrap();
            assert_eq!(
                result.endpoint, second.endpoint,
                "{mode}: ineligible first peer selected"
            );
            assert_eq!(first.count(), 0);
            assert_eq!(second.count(), 1);
        }
    }
}

#[test]
fn no_eligible_peer_means_no_dispatch_and_explicit_rejection_reasons() {
    let mut full = accepting_descriptor(100);
    full["execution_admission"]["active_execution_count"] = json!(1);
    let first = Peer::start_with_descriptor("success", full);
    let second = Peer::start_with_descriptor("success", json!({"runtime":{"health_score":100}}));
    let error = solve(
        "solve_bar_1d",
        json!({}),
        &[first.endpoint.clone(), second.endpoint.clone()],
        "healthiest",
    )
    .unwrap_err();
    assert!(error.contains("no task dispatched"));
    assert!(error.contains("execution_at_capacity"));
    assert!(error.contains("invalid_solver_role"));
    assert_eq!(first.count() + second.count(), 0);
}

#[test]
fn new_tasks_recheck_capacity_instead_of_reusing_old_eligibility() {
    let first = Peer::start("success", 100);
    let second = Peer::start("success", 10);
    let endpoints = [first.endpoint.clone(), second.endpoint.clone()];
    for (index, active) in [1, 0, 1].into_iter().enumerate() {
        first.descriptor.lock().unwrap()["execution_admission"]["active_execution_count"] =
            json!(active);
        let result = solve(
            "solve_bar_1d",
            json!({"job_id":format!("distinct-job-{index}")}),
            &endpoints,
            "healthiest",
        )
        .unwrap();
        assert_eq!(
            result.endpoint,
            if active == 0 {
                &first.endpoint
            } else {
                &second.endpoint
            }
            .as_str()
        );
    }
    assert_eq!(first.count(), 1);
    assert_eq!(second.count(), 2);
}

impl Drop for Peer {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(join) = self.join.take() {
            join.join().unwrap();
        }
    }
}

#[test]
fn legacy_mesh_solve_never_replays_after_lost_reply_or_solver_failure() {
    for mode in ["first_reachable", "healthiest"] {
        for behavior in ["lost-reply", "solver-error"] {
            let first = Peer::start(behavior, 100);
            let second = Peer::start("success", 50);
            let result = solve(
                "solve_bar_1d",
                json!({}),
                &[first.endpoint.clone(), second.endpoint.clone()],
                mode,
            );
            assert_eq!(first.count(), 1);
            assert_eq!(second.count(), 0, "replayed {behavior} in {mode}");
            assert!(
                result
                    .unwrap_err()
                    .contains("automatic cross-agent replay disabled")
            );
        }
    }
}

#[test]
fn descriptor_selection_can_skip_unavailable_peers_before_dispatch() {
    let unavailable = Peer::start("unavailable", 100);
    let healthy = Peer::start("success", 50);
    let result = solve(
        "solve_bar_1d",
        json!({}),
        &[unavailable.endpoint.clone(), healthy.endpoint.clone()],
        "first_reachable",
    )
    .unwrap();
    assert_eq!(result.endpoint, healthy.endpoint);
    assert_eq!(result.result["max_stress"], 10.0);
    assert_eq!(unavailable.count(), 0);
    assert_eq!(healthy.count(), 1);
}

#[test]
fn failed_discovery_does_not_fall_back_to_blind_execution() {
    let first = Peer::start("unavailable", 100);
    let second = Peer::start("unavailable", 50);
    let result = solve(
        "solve_bar_1d",
        json!({}),
        &[first.endpoint.clone(), second.endpoint.clone()],
        "healthiest",
    );
    assert_eq!(first.count() + second.count(), 0);
    assert!(result.is_err());
}
