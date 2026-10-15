use serde_json::{Value, json};
use std::error::Error;
use std::fs;
use std::net::TcpListener;
use std::thread;
use std::time::{Duration, Instant};

#[allow(dead_code)]
#[path = "support/agent_durable.rs"]
mod support;
use support::*;

fn endpoint(agent: &LiveAgent) -> String {
    format!("127.0.0.1:{}", agent.port)
}

fn describe(agent: &LiveAgent) -> Result<Value, Box<dyn Error>> {
    let response = agent.request("mesh-descriptor", "describe_agent", json!({}))?;
    Ok(successful_result(&response, "mesh-descriptor").clone())
}

fn wait_mesh(
    agent: &LiveAgent,
    condition: impl Fn(&Value) -> bool,
) -> Result<Value, Box<dyn Error>> {
    let deadline = Instant::now() + Duration::from_secs(15);
    loop {
        let descriptor = describe(agent)?;
        assert_eq!(descriptor["runtime"]["runtime_mode"], "peer_mesh");
        assert_eq!(descriptor["authority"]["authority_mode"], "offline_mesh");
        assert!(descriptor["authority"]["orchestrator_id"].is_null());
        assert!(descriptor["authority"]["orchestrator_session_id"].is_null());
        if condition(&descriptor) {
            return Ok(descriptor);
        }
        if Instant::now() >= deadline {
            return Err(format!("mesh did not converge: {descriptor}").into());
        }
        thread::sleep(Duration::from_millis(30));
    }
}

fn healthy_three(agent: &LiveAgent) -> Result<(), Box<dyn Error>> {
    let descriptor = wait_mesh(agent, |d| {
        d["runtime"]["cluster_size"] == 3 && d["runtime"]["health_score"] == 100
    })?;
    assert_eq!(descriptor["runtime"]["peers"].as_array().unwrap().len(), 2);
    assert_eq!(descriptor["peer_discovery"]["task_replication"], false);
    assert_eq!(
        descriptor["peer_discovery"]["cluster_id_is_authentication"],
        false
    );
    assert_eq!(
        descriptor["peer_discovery"]["probe_order"],
        "retained_fifo_new_candidates_at_tail"
    );
    assert_eq!(
        descriptor["peer_discovery"]["max_probe_rounds_for_retained_peer"],
        8
    );
    Ok(())
}

fn run_bar(agent: &LiveAgent, id: char) -> Result<(), Box<dyn Error>> {
    let response = agent.request(
        "same-transport",
        "submit_durable_agent_task",
        task(id, "same-job"),
    )?;
    assert_eq!(
        successful_result(&response, "same-transport")["status"],
        "running"
    );
    let result = wait_status(agent, id, "completed")?;
    assert_eq!(result["response"]["result"]["max_stress"], 10.0);
    wait_for_lifecycle(agent, "accepting", 0)?;
    Ok(())
}

#[test]
fn three_native_mesh_agents_survive_peer_loss_without_cross_peer_task_replay()
-> Result<(), Box<dyn Error>> {
    let mut a = LiveAgent::start_with_durable_store()?;
    let mut b = LiveAgent::start_with_durable_store()?;
    let mut c = LiveAgent::start_with_durable_store()?;
    // Only one seed each. Discovery must build the ring without Orchestra or a full seed list.
    a.restart_in_mesh("live-research", vec![endpoint(&b)], 1000)?;
    b.restart_in_mesh("live-research", vec![endpoint(&c)], 1000)?;
    c.restart_in_mesh("live-research", vec![endpoint(&a)], 1000)?;
    for agent in [&a, &b, &c] {
        healthy_three(agent)?;
    }
    for (agent, id) in [(&a, 'a'), (&b, 'b'), (&c, 'c')] {
        run_bar(agent, id)?;
    }

    fs::write(&a.hold_path, "interrupted-mesh-job")?;
    let response = a.request(
        "held",
        "submit_durable_agent_task",
        task('d', "interrupted-mesh-job"),
    )?;
    let admitted = successful_result(&response, "held").clone();
    assert_eq!(admitted["status"], "running");
    assert_eq!(lifecycle(&a, "held-owner")?["active_execution_count"], 1);
    a.stop_process()?;
    let lost = endpoint(&a);
    for agent in [&b, &c] {
        wait_mesh(agent, |d| {
            d["runtime"]["peers"]
                .as_array()
                .unwrap()
                .iter()
                .any(|peer| peer["address"] == lost && peer["status"] == "degraded")
        })?;
        assert_eq!(fetch(agent, 'd')?["status"], "not_found");
    }
    run_bar(&b, 'e')?;
    run_bar(&c, 'f')?;

    fs::remove_file(&a.hold_path)?;
    a.start_process()?;
    for agent in [&a, &b, &c] {
        healthy_three(agent)?;
    }
    let unknown = fetch(&a, 'd')?;
    assert_eq!(unknown["status"], "outcome_unknown");
    assert_eq!(
        unknown["process_instance_id"],
        admitted["process_instance_id"]
    );
    assert_eq!(unknown["automatic_replay_authorized"], false);
    let duplicate = a.request(
        "reconnect",
        "submit_durable_agent_task",
        task('d', "interrupted-mesh-job"),
    )?;
    assert_eq!(*successful_result(&duplicate, "reconnect"), unknown);
    assert_eq!(lifecycle(&a, "no-replay")?["active_execution_count"], 0);
    for agent in [&b, &c] {
        assert_eq!(fetch(agent, 'd')?["status"], "not_found");
        assert_eq!(
            lifecycle(agent, "no-migration")?["active_execution_count"],
            0
        );
    }
    run_bar(&a, '1')?;
    Ok(())
}

#[test]
fn mesh_task_identity_is_agent_local_and_foreign_cancellation_is_rejected()
-> Result<(), Box<dyn Error>> {
    let mut a = LiveAgent::start_with_durable_store()?;
    let mut b = LiveAgent::start_with_durable_store()?;
    a.restart_in_mesh("cancel-isolation", vec![endpoint(&b)], 1000)?;
    b.restart_in_mesh("cancel-isolation", vec![endpoint(&a)], 1000)?;
    for agent in [&a, &b] {
        fs::write(&agent.hold_path, "same-job")?;
    }
    let mut receipts = Vec::new();
    // Even identical caller IDs are scoped to an Agent, not a distributed exactly-once ledger.
    for agent in [&a, &b] {
        let response = agent.request(
            "same-transport",
            "submit_durable_agent_task",
            task('2', "same-job"),
        )?;
        receipts.push(successful_result(&response, "same-transport").clone());
    }
    assert_ne!(
        receipts[0]["process_instance_id"],
        receipts[1]["process_instance_id"]
    );
    let foreign = b.request("foreign-cancel", "cancel_durable_agent_execution", json!({
        "execution_id":receipts[0]["execution_id"], "process_instance_id":receipts[0]["process_instance_id"],
        "request_digest":receipts[0]["request_digest"]
    }))?;
    assert_eq!(foreign["ok"], false);
    assert!(
        foreign["error"]["message"]
            .as_str()
            .unwrap()
            .contains("identity mismatch")
    );
    for agent in [&a, &b] {
        assert_eq!(
            lifecycle(agent, "still-active")?["active_execution_count"],
            1
        );
        fs::remove_file(&agent.hold_path)?;
        assert_eq!(
            wait_status(agent, '2', "completed")?["response"]["result"]["max_stress"],
            10.0
        );
    }
    Ok(())
}

#[test]
fn learned_peer_retirement_preserves_seed_and_original_task_recovery() -> Result<(), Box<dyn Error>>
{
    let mut a = LiveAgent::start_with_durable_store()?;
    let mut b = LiveAgent::start_with_durable_store()?;
    let mut c = LiveAgent::start_with_durable_store()?;
    a.restart_in_mesh("candidate-retirement", vec![endpoint(&b)], 1000)?;
    b.restart_in_mesh("candidate-retirement", vec![endpoint(&c)], 1000)?;
    c.restart_in_mesh("candidate-retirement", vec![endpoint(&b)], 1000)?;
    // A only learns C from B. B must retain C because C is its configured seed.
    wait_mesh(&a, |d| {
        d["runtime"]["peers"]
            .as_array()
            .unwrap()
            .iter()
            .any(|peer| peer["address"] == endpoint(&c) && peer["status"] == "healthy")
    })?;
    fs::write(&c.hold_path, "retired-owner-job")?;
    let admitted = c.request(
        "retired-owner",
        "submit_durable_agent_task",
        task('7', "retired-owner-job"),
    )?;
    let admitted = successful_result(&admitted, "retired-owner").clone();
    assert_eq!(admitted["status"], "running");
    c.stop_process()?;
    let retired = endpoint(&c);
    let discovery = wait_mesh(&a, |d| {
        d["runtime"]["peers"]
            .as_array()
            .unwrap()
            .iter()
            .all(|peer| peer["address"] != retired)
    })?;
    assert_eq!(discovery["runtime"]["cluster_size"], 2);
    let policy = &discovery["peer_discovery"];
    assert_eq!(policy["learned_peer_failure_threshold"], 3);
    assert_eq!(policy["learned_peer_cooldown_ms"], 60_000);
    assert_eq!(policy["candidate_retirement_scope"], "discovery_only");
    assert_eq!(
        policy["health_score_scope"],
        "retained_candidates_not_task_recovery"
    );
    wait_mesh(&b, |d| {
        d["runtime"]["peers"]
            .as_array()
            .unwrap()
            .iter()
            .any(|peer| {
                peer["address"] == retired && peer["failure_count"].as_u64().unwrap_or(0) >= 3
            })
    })?;
    assert_eq!(fetch(&a, '7')?["status"], "not_found");
    assert_eq!(fetch(&b, '7')?["status"], "not_found");
    run_bar(&a, '8')?;

    fs::remove_file(&c.hold_path)?;
    c.start_process()?;
    wait_mesh(&b, |d| {
        d["runtime"]["peers"]
            .as_array()
            .unwrap()
            .iter()
            .any(|peer| peer["address"] == retired && peer["status"] == "healthy")
    })?;
    let previous = describe(&a)?["runtime"]["peers"][0]["last_seen_unix_s"].clone();
    let after_gossip = wait_mesh(&a, |d| {
        d["runtime"]["peers"][0]["last_seen_unix_s"] != previous
    })?;
    assert!(
        after_gossip["runtime"]["peers"]
            .as_array()
            .unwrap()
            .iter()
            .all(|peer| { peer["address"] != retired }),
        "gossip bypassed retirement cooldown"
    );

    // Discovery retirement is not loss of ownership, a ledger deletion or replay permission.
    let original = c.request(
        "recover-retired",
        "fetch_durable_agent_execution",
        json!({
            "execution_id":admitted["execution_id"], "expected_store_id":admitted["store_id"]
        }),
    )?;
    let original = successful_result(&original, "recover-retired");
    assert_eq!(original["status"], "outcome_unknown");
    assert_eq!(original["store_id"], admitted["store_id"]);
    assert_eq!(
        original["process_instance_id"],
        admitted["process_instance_id"]
    );
    let mut duplicate = task('7', "retired-owner-job");
    duplicate["expected_store_id"] = admitted["store_id"].clone();
    let duplicate = c.request("no-replay", "submit_durable_agent_task", duplicate)?;
    assert_eq!(successful_result(&duplicate, "no-replay"), original);
    assert_eq!(
        lifecycle(&c, "still-not-replayed")?["active_execution_count"],
        0
    );
    assert_eq!(fetch(&a, '7')?["status"], "not_found");
    assert_eq!(fetch(&b, '7')?["status"], "not_found");
    Ok(())
}

#[test]
fn foreign_cluster_seed_cannot_introduce_its_peer_hints() -> Result<(), Box<dyn Error>> {
    let mut a = LiveAgent::start()?;
    let mut b = LiveAgent::start()?;
    let reserved = TcpListener::bind("127.0.0.1:0")?;
    let foreign_hint = reserved.local_addr()?.to_string();
    b.restart_in_mesh("foreign", vec![foreign_hint], 1000)?;
    a.restart_in_mesh("local", vec![endpoint(&b)], 1000)?;
    let descriptor = wait_mesh(&a, |d| {
        d["runtime"]["peers"][0]["failure_count"]
            .as_u64()
            .unwrap_or(0)
            >= 2
    })?;
    assert_eq!(descriptor["runtime"]["cluster_size"], 2);
    assert_eq!(descriptor["runtime"]["health_score"], 0);
    assert_eq!(descriptor["runtime"]["peers"][0]["address"], endpoint(&b));
    assert_eq!(descriptor["runtime"]["peers"][0]["status"], "unreachable");
    assert_eq!(descriptor["runtime"]["peers"].as_array().unwrap().len(), 1);
    Ok(())
}

#[cfg(unix)]
#[test]
fn slow_discovery_peers_do_not_hide_ready_peers_or_block_work_and_shutdown()
-> Result<(), Box<dyn Error>> {
    let mut a = LiveAgent::start_with_durable_store()?;
    let mut b = LiveAgent::start_with_durable_store()?;
    b.restart_in_mesh("slow-discovery", vec![endpoint(&a)], 1000)?;
    // Keep seven loopback listeners open without accepting or replying. Hostname sorting
    // puts the healthy literal-IP seed first, so this isolates delayed publication.
    let stalled = (0..7)
        .map(|_| TcpListener::bind("127.0.0.1:0"))
        .collect::<Result<Vec<_>, _>>()?;
    let mut seeds = vec![endpoint(&b)];
    for listener in &stalled {
        seeds.push(format!("localhost:{}", listener.local_addr()?.port()));
    }
    a.restart_in_mesh("slow-discovery", seeds, 300_000)?;
    let deadline = Instant::now() + Duration::from_secs(3);
    let ready = loop {
        let descriptor = describe(&a)?;
        if descriptor["runtime"]["peers"]
            .as_array()
            .unwrap()
            .iter()
            .any(|peer| peer["address"] == endpoint(&b) && peer["status"] == "healthy")
        {
            break descriptor;
        }
        if Instant::now() >= deadline {
            return Err("ready peer remained hidden behind stalled discovery peers".into());
        }
        thread::sleep(Duration::from_millis(20));
    };
    assert_eq!(ready["peer_discovery"]["max_concurrent_probes"], 2);
    assert_eq!(
        ready["peer_discovery"]["state_publication"],
        "after_each_probe_completion"
    );
    run_bar(&a, '9')?;
    a.signal(libc::SIGTERM)?;
    assert!(a.wait_for_exit(Duration::from_secs(5))?.success());
    run_bar(&b, '0')?;
    Ok(())
}

#[cfg(unix)]
#[test]
fn mesh_shutdown_interrupts_a_five_minute_discovery_interval() -> Result<(), Box<dyn Error>> {
    let mut a = LiveAgent::start()?;
    let mut b = LiveAgent::start()?;
    b.restart_in_mesh("shutdown", vec![endpoint(&a)], 1000)?;
    a.restart_in_mesh("shutdown", vec![endpoint(&b)], 300_000)?;
    wait_mesh(&a, |d| d["runtime"]["health_score"] == 100)?;
    a.signal(libc::SIGTERM)?;
    assert!(a.wait_for_exit(Duration::from_secs(5))?.success());
    run_bar(&b, '3')?;
    Ok(())
}
