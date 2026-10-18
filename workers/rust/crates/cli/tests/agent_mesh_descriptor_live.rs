use serde_json::{Value, json};
use std::error::Error;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

#[allow(dead_code)]
#[path = "support/agent_durable.rs"]
mod support;
use support::*;

struct DescriptorPeer {
    endpoint: String,
    descriptor: Arc<Mutex<Value>>,
    probes: Arc<AtomicUsize>,
    stop: Arc<AtomicBool>,
    worker: Option<thread::JoinHandle<()>>,
}

impl DescriptorPeer {
    fn start(descriptor: Value) -> Result<Self, Box<dyn Error>> {
        let listener = TcpListener::bind("127.0.0.1:0")?;
        listener.set_nonblocking(true)?;
        let endpoint = listener.local_addr()?.to_string();
        let descriptor = Arc::new(Mutex::new(descriptor));
        let published = Arc::clone(&descriptor);
        let probes = Arc::new(AtomicUsize::new(0));
        let observed = Arc::clone(&probes);
        let stop = Arc::new(AtomicBool::new(false));
        let stopped = Arc::clone(&stop);
        let worker = thread::spawn(move || {
            while !stopped.load(Ordering::SeqCst) {
                let (mut stream, _) = match listener.accept() {
                    Ok(connection) => connection,
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(5));
                        continue;
                    }
                    Err(error) => panic!("peer accept: {error}"),
                };
                stream.set_nonblocking(false).unwrap();
                stream
                    .set_read_timeout(Some(Duration::from_secs(2)))
                    .unwrap();
                stream
                    .set_write_timeout(Some(Duration::from_secs(2)))
                    .unwrap();
                let mut header = [0; 4];
                if stream.read_exact(&mut header).is_err() {
                    continue;
                }
                let length = u32::from_be_bytes(header) as usize;
                assert!((1..=16384).contains(&length));
                let mut payload = vec![0; length];
                if stream.read_exact(&mut payload).is_err() {
                    continue;
                }
                let request: Value = serde_json::from_slice(&payload).unwrap();
                assert_eq!(
                    request["method"], "describe_agent",
                    "discovery must not dispatch work"
                );
                let result = published.lock().unwrap().clone();
                let reply = serde_json::to_vec(&json!({
                    "rpc_version":1, "id":request["id"], "ok":true, "result":result
                }))
                .unwrap();
                observed.fetch_add(1, Ordering::SeqCst);
                // A forced Agent restart may abandon its in-flight discovery socket.
                if stream
                    .write_all(&(reply.len() as u32).to_be_bytes())
                    .is_ok()
                {
                    let _ = stream.write_all(&reply);
                }
            }
        });
        Ok(Self {
            endpoint,
            descriptor,
            probes,
            stop,
            worker: Some(worker),
        })
    }

    fn publish(&self, descriptor: Value) {
        *self.descriptor.lock().unwrap() = descriptor;
    }
}

impl Drop for DescriptorPeer {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(worker) = self.worker.take() {
            let outcome = worker.join();
            if !thread::panicking() {
                outcome.expect("descriptor fixture worker failed");
            }
        }
    }
}

fn descriptor() -> Value {
    let mut descriptor =
        serde_json::to_value(kyuubiki_protocol::AgentDescriptor::solver_agent_default()).unwrap();
    descriptor["runtime"] = json!({"cluster_id":"descriptor-recovery",
        "runtime_mode":"peer_mesh", "headless":true, "cluster_size":1,
        "health_score":100, "peers":[]});
    descriptor["authority"]["control_mode"] = json!("offline_mesh");
    descriptor["authority"]["authority_mode"] = json!("offline_mesh");
    descriptor["engine"]["task_source"] = json!("manual_or_mesh_dispatch");
    descriptor
}

fn wait_peer(
    agent: &LiveAgent,
    endpoint: &str,
    status: &str,
    min_failures: u64,
) -> Result<Value, Box<dyn Error>> {
    let deadline = Instant::now() + Duration::from_secs(8);
    loop {
        let response = agent.request("mesh-state", "describe_agent", json!({}))?;
        let descriptor = successful_result(&response, "mesh-state");
        if descriptor["runtime"]["peers"]
            .as_array()
            .unwrap()
            .iter()
            .any(|peer| {
                peer["address"] == endpoint
                    && peer["status"] == status
                    && peer["failure_count"].as_u64().unwrap() >= min_failures
            })
        {
            return Ok(descriptor.clone());
        }
        if Instant::now() >= deadline {
            return Err(format!(
                "expected {status} peer with {min_failures} failures: {}",
                descriptor["runtime"]
            )
            .into());
        }
        thread::sleep(Duration::from_millis(20));
    }
}

#[test]
fn incompatible_peer_hints_stay_isolated_across_restart_and_protocol_recovery()
-> Result<(), Box<dyn Error>> {
    let hint = DescriptorPeer::start(descriptor())?;
    let seed = DescriptorPeer::start(descriptor())?;
    let mut agent = LiveAgent::start_with_durable_store()?;
    agent.restart_in_mesh("descriptor-recovery", vec![seed.endpoint.clone()], 1000)?;
    wait_peer(&agent, &seed.endpoint, "healthy", 0)?;

    let task = task('a', "retained-mesh-receipt");
    let submitted = agent.request("submit", "submit_durable_agent_task", task.clone())?;
    assert_eq!(successful_result(&submitted, "submit")["status"], "running");
    let original = wait_status(&agent, 'a', "completed")?;
    assert_eq!(original["response"]["result"]["max_stress"], 10.0);
    wait_for_lifecycle(&agent, "accepting", 0)?;

    let mut valid = descriptor();
    valid["runtime"]["cluster_size"] = json!(2);
    valid["runtime"]["peers"] = json!([{"address":hint.endpoint, "status":"healthy",
        "failure_count":0, "last_seen_unix_s":u64::MAX}]);
    let mut incompatible = valid.clone();
    incompatible["protocol"]["transport"]["framing"] = json!("newline");
    seed.publish(incompatible);
    let degraded = wait_peer(&agent, &seed.endpoint, "degraded", 2)?;
    assert_eq!(degraded["runtime"]["peers"].as_array().unwrap().len(), 1);
    assert_eq!(hint.probes.load(Ordering::SeqCst), 0);
    assert_eq!(fetch(&agent, 'a')?, original);

    agent.stop_process()?;
    agent.start_process()?;
    let unavailable = wait_peer(&agent, &seed.endpoint, "unreachable", 2)?;
    assert_eq!(unavailable["runtime"]["peers"].as_array().unwrap().len(), 1);
    assert_eq!(hint.probes.load(Ordering::SeqCst), 0);

    seed.publish(valid);
    wait_peer(&agent, &seed.endpoint, "healthy", 0)?;
    let recovered = wait_peer(&agent, &hint.endpoint, "healthy", 0)?;
    assert_eq!(recovered["runtime"]["cluster_size"], 3);
    assert!(hint.probes.load(Ordering::SeqCst) > 0);
    assert_eq!(fetch(&agent, 'a')?, original);
    let mut duplicate = task;
    duplicate["expected_store_id"] = original["store_id"].clone();
    let repeated = agent.request("no-replay", "submit_durable_agent_task", duplicate)?;
    assert_eq!(successful_result(&repeated, "no-replay"), &original);
    assert_eq!(lifecycle(&agent, "idle")?["active_execution_count"], 0);
    agent.stop_process()?;
    Ok(())
}
