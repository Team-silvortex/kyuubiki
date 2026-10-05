use super::agent_support::{LiveAgent, wait_for_lifecycle};
use super::orchestra_completion::{assert_bar, bar_task, http_json};
use super::start_live_server_with_agents;
use kyuubiki_headless_sdk::{HeadlessExecutor, ServiceHeadlessExecutor};
use serde_json::{Value, json};
use std::error::Error;
use std::fs;
use std::path::Path;
use std::thread;
use std::time::{Duration, Instant};

struct HoldMarkers<'a>(&'a Path, &'a Path);
impl Drop for HoldMarkers<'_> {
    fn drop(&mut self) {
        let _ = fs::remove_file(self.0);
        let _ = fs::remove_file(self.1);
    }
}

fn task_result(port: u16, task: Value) -> Result<Value, String> {
    let mut executor = ServiceHeadlessExecutor::try_new(&format!("http://127.0.0.1:{port}"))
        .map_err(|e| e.message)?;
    let outcome = executor
        .execute_step("operator_task_execute", 1, &json!({"task": task}))
        .map_err(|e| e.message)?;
    if outcome.status != "executed" {
        return Err(format!("unexpected outcome: {outcome:?}"));
    }
    Ok(outcome.result)
}

fn active(agent: &LiveAgent, job: &str) -> Result<Value, Box<dyn Error>> {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let reply = agent.request("routing-describe", "describe_agent", json!({}))?;
        if let Some(entry) = reply["result"]["solver_control"]["active"]
            .as_array()
            .and_then(|entries| entries.iter().find(|entry| entry["job_id"] == job))
        {
            return Ok(entry.clone());
        }
        if Instant::now() > deadline {
            return Err(format!("held task {job} missing: {reply}").into());
        }
        thread::sleep(Duration::from_millis(10));
    }
}

fn spring() -> Value {
    json!({"nodes":[
        {"id":"fixed","x":0.0,"y":0.0,"fix_x":true,"fix_y":true,"load_x":0.0,"load_y":0.0},
        {"id":"loaded","x":1.0,"y":0.0,"fix_x":false,"fix_y":true,"load_x":1000.0,"load_y":0.0}
    ],"elements":[{"id":"spring","node_i":0,"node_j":1,"stiffness":25000.0}]})
}

fn assert_spring(result: &Value) {
    let displacement = result["nodes"][1]["ux"]
        .as_f64()
        .expect("spring displacement");
    assert!((displacement - 0.04).abs() < 1e-12, "{result}");
    assert_eq!(result["nodes"][0]["ux"], 0.0);
    assert_eq!(result["nodes"][1]["uy"], 0.0);
    assert!((result["elements"][0]["force"].as_f64().unwrap() - 1000.0).abs() < 1e-9);
    assert!((result["total_strain_energy"].as_f64().unwrap() - 20.0).abs() < 1e-10);
}

#[test]
fn real_two_agent_public_job_cancel_targets_owner_and_does_not_poison_peer()
-> Result<(), Box<dyn Error>> {
    let owner = LiveAgent::start()?;
    let peer = LiveAgent::start()?;
    let server = start_live_server_with_agents(Some(owner.port()), Some(peer.port()))?;
    thread::scope(|scope| -> Result<(), Box<dyn Error>> {
        // Saturate both known slots so the generated job identity can be held deterministically.
        let markers = HoldMarkers(&owner.hold_path, &peer.hold_path);
        fs::write(&owner.hold_path, "routing-owner-blocker")?;
        fs::write(&peer.hold_path, "routing-peer-blocker")?;
        let first =
            scope.spawn(|| task_result(server.port, bar_task("routing-owner-blocker", 0.01)));
        active(&owner, "routing-owner-blocker")?;
        let second =
            scope.spawn(|| task_result(server.port, bar_task("routing-peer-blocker", 0.01)));
        active(&peer, "routing-peer-blocker")?;

        let (status, admitted) =
            http_json(server.port, "/api/v1/fem/spring-2d/jobs", Some(spring()))?;
        assert_eq!(status, 202, "{admitted}");
        let job = admitted["job"]["job_id"]
            .as_str()
            .ok_or("missing generated job id")?;
        // Switch only the owner's marker: its predecessor completes, then the queued job is held.
        fs::write(&owner.hold_path, job)?;
        assert_bar(&first.join().map_err(|_| "owner blocker panicked")??);
        let held = active(&owner, job)?;
        assert_eq!(held["cancel_requested"], false);

        let (status, cancelled) = http_json(
            server.port,
            &format!("/api/v1/jobs/{job}/cancel"),
            Some(json!({})),
        )?;
        assert_eq!(status, 200, "{cancelled}");
        assert_eq!(cancelled["job"]["status"], "cancelled");
        assert_eq!(cancelled["cancellation"]["status"], "requested");
        assert_eq!(cancelled["cancellation"]["target_count"], 1);
        assert_eq!(
            cancelled["cancellation"]["targets"][0]["agent_id"],
            "owned-live-agent"
        );
        assert_eq!(
            cancelled["cancellation"]["execution_terminal_confirmed"],
            false
        );
        wait_for_lifecycle(&owner, "accepting", 0)?;
        assert_eq!(
            active(&peer, "routing-peer-blocker")?["cancel_requested"],
            false
        );
        let (_, stored) = http_json(server.port, &format!("/api/v1/jobs/{job}"), None)?;
        assert_eq!(stored["job"]["status"], "cancelled");
        assert_eq!(stored["job"]["has_result"], false);

        drop(markers);
        assert_bar(&second.join().map_err(|_| "peer blocker panicked")??);
        wait_for_lifecycle(&peer, "accepting", 0)?;
        // Reusing that job identity directly on the bystander must not consume a stray pending cancel.
        let mut input = spring();
        input["job_id"] = json!(job);
        let direct = peer.request("routing-peer-same-job", "solve_spring_2d", input)?;
        assert_eq!(direct["ok"], true, "{direct}");
        assert_spring(&direct["result"]);
        wait_for_lifecycle(&peer, "accepting", 0)?;

        let (status, next) = http_json(server.port, "/api/v1/fem/spring-2d/jobs", Some(spring()))?;
        assert_eq!(status, 202);
        let next_job = next["job"]["job_id"].as_str().ok_or("missing next job")?;
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            let (_, result) = http_json(server.port, &format!("/api/v1/jobs/{next_job}"), None)?;
            if result["job"]["status"] == "completed" {
                assert_eq!(result["job"]["has_result"], true);
                assert_spring(&result["result"]);
                break;
            }
            if Instant::now() > deadline {
                return Err(format!("next job did not complete: {result}").into());
            }
            thread::sleep(Duration::from_millis(10));
        }
        Ok(())
    })
}
