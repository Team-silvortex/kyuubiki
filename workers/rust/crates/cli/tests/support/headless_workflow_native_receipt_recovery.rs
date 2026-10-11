use super::agent_support::{LiveAgent, wait_for_lifecycle};
use super::diagnostic_domains::{DOMAINS, Domain};
use super::domain_diagnostics::{assert_dispatches, assert_ready, request};
use super::orchestra_completion::http_json;
use super::write_ack_loss::count;
use super::{LiveServer, Scratch, start_live_server_with_native_gate};
use kyuubiki_headless_sdk::{HeadlessExecutor, ServiceHeadlessExecutor};
use serde_json::{Value, json};
use std::{
    error::Error,
    fs,
    path::Path,
    thread,
    time::{Duration, Instant},
};

type Result<T> = std::result::Result<T, Box<dyn Error>>;

fn records(root: &Path, job: &str) -> Result<Vec<Value>> {
    let mut result = fs::read_dir(root.join("operator-task-dispatches"))?
        .map(|entry| -> Result<Value> {
            Ok(serde_json::from_slice::<Value>(&fs::read(entry?.path())?)?["payload"].clone())
        })
        .collect::<Result<Vec<_>>>()?;
    result.retain(|record| record["job_id"] == job);
    Ok(result)
}

fn wait_gate(server: &LiveServer, root: &Path, job: &str, domain: Domain) -> Result<()> {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if let Ok(bytes) = fs::read(root.join("native-recovery-held.json")) {
            let held: Value = serde_json::from_slice(&bytes)?;
            assert_eq!(held["job_id"], job);
            assert_eq!(held["task_id"], format!("{job}:quality:{}", domain.quality));
            return Ok(());
        }
        if Instant::now() >= deadline {
            return Err(format!(
                "next node never reached admission boundary: {}",
                server.logs()
            )
            .into());
        }
        thread::sleep(Duration::from_millis(10));
    }
}

fn wait_terminal(server: &LiveServer, job: &str) -> Result<Value> {
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        let (_, value) = http_json(server.port, &format!("/api/v1/jobs/{job}"), None)?;
        if matches!(
            value["job"]["status"].as_str(),
            Some("completed" | "failed" | "cancelled")
        ) {
            return Ok(value);
        }
        if Instant::now() >= deadline {
            return Err(format!("recovery did not finish: {value}\n{}", server.logs()).into());
        }
        thread::sleep(Duration::from_millis(50));
    }
}

fn run_round(domain: Domain, lose_receipt: bool) -> Result<()> {
    let mut owner = LiveAgent::start()?;
    let peer = LiveAgent::start()?;
    let data = Scratch::new()?;
    let database = data.0.join("receipt-recovery.sqlite3");
    fs::write(
        data.0.join("native-recovery-gate"),
        "pause-before-next-native-admission",
    )?;
    let server = start_live_server_with_native_gate(owner.port(), peer.port(), &data.0, &database)?;
    let mut req = serde_json::to_value(request(domain, domain.raw(), false, false))?;
    req["graph"]["nodes"][1]["placement_tags"] = json!(["owned-live-owner"]);
    let (status, admitted) = http_json(server.port, "/api/v1/workflows/graph/jobs", Some(req))?;
    assert_eq!(status, 202, "{admitted}");
    let job = admitted["job"]["job_id"]
        .as_str()
        .ok_or("missing job")?
        .to_owned();
    wait_gate(&server, &data.0, &job, domain)?;
    wait_for_lifecycle(&owner, "accepting", 0)?;
    let original = http_json(server.port, &format!("/api/v1/jobs/{job}"), None)?.1;
    assert_eq!(original["job"]["status"], "solving");
    assert_eq!(original["result"]["recovery"]["generation"], 1);
    let retained = records(&data.0, &job)?;
    assert_eq!(
        retained.len(),
        1,
        "next node must not have a journal reservation"
    );
    let record = &retained[0];
    assert_eq!(record["operator_id"], domain.diagnostic);
    assert_eq!(record["state"], "observed_executed");
    let path = data
        .0
        .join("operator-task-dispatches")
        .join(format!("{}.json", record["attempt_id"].as_str().unwrap()));
    let bytes = fs::read(&path)?;
    assert_eq!(count(&owner)?, 1);
    assert_eq!(count(&peer)?, 0);
    // Lose the real BEAM after successful diagnosis, with scoring not yet admitted.
    drop(server);
    fs::remove_file(data.0.join("native-recovery-gate"))?;
    if lose_receipt {
        owner.stop_process()?;
        owner.start_process()?;
    }
    let restarted =
        start_live_server_with_native_gate(owner.port(), peer.port(), &data.0, &database)?;
    let ended = wait_terminal(&restarted, &job)?;
    let recovery = &ended["result"]["recovery"];
    assert_eq!(recovery["generation"], 2, "{ended}");
    assert_eq!(recovery["attempt"], 2);
    assert_eq!(
        recovery["envelope_sha256"],
        original["result"]["recovery"]["envelope_sha256"]
    );
    assert_eq!(
        fs::read(&path)?,
        bytes,
        "receipt retrieval must not rewrite dispatch evidence"
    );
    let mut sdk = ServiceHeadlessExecutor::try_new(&format!("http://127.0.0.1:{}", restarted.port))
        .map_err(|error| error.message)?;
    if lose_receipt {
        assert_eq!(ended["job"]["status"], "failed", "{ended}");
        assert_eq!(recovery["state"], "recovery_blocked");
        assert_eq!(recovery["envelope_retained"], true);
        assert_eq!(
            recovery["history"].as_array().unwrap().last().unwrap()["next_action"],
            "restore_original_native_receipt_or_explicit_resubmit"
        );
        assert!(
            ended["job"]["message"]
                .as_str()
                .unwrap()
                .contains("native_workflow_recovery_receipt_unavailable")
        );
        assert_eq!(records(&data.0, &job)?, retained);
        assert_eq!(count(&owner)? + count(&peer)?, 0);
        for node in ["diagnose", "quality", "objective", "decision"] {
            assert!(
                ended["result"]["artifacts"]
                    .get(format!("{node}.summary"))
                    .is_none()
            );
        }
        assert!(
            sdk.execute_step(
                "job_wait",
                1,
                &json!({"job_id":job,"timeout_ms":1000,"interval_ms":10})
            )
            .is_err()
        );
    } else {
        assert_eq!(ended["job"]["status"], "completed", "{ended}");
        assert_eq!(recovery["state"], "completed");
        assert_eq!(recovery["envelope_retained"], false);
        let full = http_json(restarted.port, &format!("/api/v1/results/{job}"), None)?.1;
        assert_ready(domain, &full["result"]);
        assert_dispatches(&sdk, &data.0, &job, domain, false)?;
        assert_eq!(
            count(&owner)? + count(&peer)?,
            2,
            "completed diagnosis must not execute again"
        );
        let waited = sdk
            .execute_step(
                "job_wait",
                1,
                &json!({"job_id":job,"timeout_ms":1000,"interval_ms":10}),
            )
            .map_err(|error| error.message)?;
        assert_eq!(waited.result["status"], "completed");
    }
    Ok(())
}

#[test]
fn real_native_workflow_restart_reuses_original_success_receipt_without_duplicate_execution()
-> Result<()> {
    for domain in DOMAINS {
        run_round(domain, false)?;
    }
    Ok(())
}

#[test]
fn real_native_workflow_restart_with_lost_agent_receipt_blocks_without_replay() -> Result<()> {
    run_round(DOMAINS[0], true)
}
