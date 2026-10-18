use super::agent_support::{LiveAgent, wait_for_lifecycle};
use super::diagnostic_domains::{DOMAINS, Domain};
use super::domain_diagnostics::{assert_dispatches, assert_ready_with_skips, request};
use super::live_server::{
    OwnedPostgresScope, start_live_server_with_typed_native_gate,
    start_live_server_with_typed_postgres_gate,
};
use super::orchestra_completion::http_json;
use super::write_ack_loss::count;
use super::{LiveServer, Scratch, start_live_server_with_native_gate};
use kyuubiki_headless_sdk::{HeadlessExecutor, ServiceHeadlessExecutor};
use kyuubiki_protocol::{
    TASK_DIGEST_TYPED_V2, WorkflowNativeTaskDigestEncoding, compute_operator_task_digest,
};
use serde_json::{Value, json};
use std::{
    error::Error,
    fs,
    path::Path,
    thread,
    time::{Duration, Instant},
};

type Result<T> = std::result::Result<T, Box<dyn Error>>;

fn http(server: &LiveServer, path: &str, body: Option<Value>) -> Result<(u16, Value)> {
    http_json(server.port, path, body)
        .map_err(|error| format!("workflow HTTP {path} failed: {error}\n{}", server.logs()).into())
}

fn records(root: &Path, job: &str) -> Result<Vec<Value>> {
    let mut result = fs::read_dir(root.join("operator-task-dispatches"))?
        .map(|entry| -> Result<Value> {
            Ok(serde_json::from_slice::<Value>(&fs::read(entry?.path())?)?["payload"].clone())
        })
        .collect::<Result<Vec<_>>>()?;
    result.retain(|record| record["job_id"] == job);
    Ok(result)
}

fn wait_gate(server: &LiveServer, root: &Path, job: &str, domain: Domain) -> Result<Value> {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if let Ok(bytes) = fs::read(root.join("native-recovery-held.json")) {
            let held: Value = serde_json::from_slice(&bytes).map_err(|error| {
                format!(
                    "native gate snapshot invalid ({} bytes): {error}",
                    bytes.len()
                )
            })?;
            assert_eq!(held["job_id"], job);
            assert_eq!(held["task_id"], format!("{job}:quality:{}", domain.quality));
            return Ok(held);
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
        let (_, value) = http(server, &format!("/api/v1/jobs/{job}"), None)?;
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

fn insert_branch(req: &mut Value, chosen: bool) {
    let artifact = "artifact/result_summary";
    let rejected = if chosen { "if_false" } else { "if_true" };
    let port = |id| json!({"id":id,"artifact_type":artifact});
    req["graph"]["nodes"].as_array_mut().unwrap().extend([
        json!({"id":"recovery-branch","kind":"condition",
            "config":{"predicate":{"operator":if chosen {"truthy"} else {"falsy"}},"on_error":"skip"},
            "inputs":[port("payload")],"outputs":[port("if_true"),port("if_false")]}),
        json!({"id":"recovery-join","kind":"transform","operator_id":"transform.first_available",
            "inputs":[port("left"),port("right")],"outputs":[port("summary")]}),
        json!({"id":"unchosen","kind":"transform","operator_id":"transform.first_available",
            "inputs":[port("payload")],"outputs":[port("summary")]}),
        json!({"id":"unchosen-output","kind":"output","inputs":[port("summary")],"outputs":[]}),
    ]);
    let edges = req["graph"]["edges"].as_array_mut().unwrap();
    let edge = edges
        .iter_mut()
        .find(|edge| edge["from"]["node"] == "diagnose" && edge["to"]["node"] == "quality")
        .unwrap();
    edge["from"] = json!({"node":"recovery-join","port":"summary"});
    edges.extend([
        json!({"id":"diagnose-branch","artifact_type":artifact,"from":{"node":"diagnose","port":"summary"},"to":{"node":"recovery-branch","port":"payload"}}),
        json!({"id":"true-join","artifact_type":artifact,"from":{"node":"recovery-branch","port":"if_true"},"to":{"node":"recovery-join","port":"left"}}),
        json!({"id":"false-join","artifact_type":artifact,"from":{"node":"recovery-branch","port":"if_false"},"to":{"node":"recovery-join","port":"right"}}),
        json!({"id":"branch-unchosen","artifact_type":artifact,"from":{"node":"recovery-branch","port":rejected},"to":{"node":"unchosen","port":"payload"}}),
        json!({"id":"unchosen-output","artifact_type":artifact,"from":{"node":"unchosen","port":"summary"},"to":{"node":"unchosen-output","port":"summary"}}),
    ]);
}

fn run_round(
    domain: Domain,
    lose_receipt: bool,
    solve: bool,
    branch: Option<bool>,
    typed: bool,
    postgres_url: Option<&str>,
) -> Result<()> {
    let mut owner = LiveAgent::start()?;
    let peer = LiveAgent::start()?;
    let data = Scratch::new()?;
    let mut postgres_scope = postgres_url.map(|url| OwnedPostgresScope::new(url, &data.0));
    let database = data.0.join("receipt-recovery.sqlite3");
    fs::write(
        data.0.join("native-recovery-gate"),
        "pause-before-next-native-admission",
    )?;
    let start_server = |agent_port, peer_port, root: &Path, database: &Path| {
        if let Some(url) = postgres_url {
            start_live_server_with_typed_postgres_gate(agent_port, peer_port, root, url)
        } else if typed {
            start_live_server_with_typed_native_gate(agent_port, peer_port, root, database)
        } else {
            start_live_server_with_native_gate(agent_port, peer_port, root, database)
        }
    };
    let server = start_server(owner.port(), peer.port(), &data.0, &database)?;
    let mut input = if solve { domain.model() } else { domain.raw() };
    let witness = json!([1, 1.0, -0.0, 0.0, 1e-30, 1.0000000000000002]);
    if typed {
        input["precision_witness"] = witness.clone();
    }
    let mut authored = request(domain, input, solve, false);
    if typed {
        authored.graph.defaults.native_task_digest_encoding =
            WorkflowNativeTaskDigestEncoding::TypedIeee754V2;
    }
    let mut req = serde_json::to_value(authored)?;
    if typed {
        for node in req["graph"]["nodes"].as_array_mut().unwrap() {
            if node["id"] == "quality" {
                node["config"]["precision_witness"] = witness.clone();
            }
        }
    }
    req["graph"]["nodes"][1]["placement_tags"] = json!(["owned-live-owner"]);
    if solve {
        for node in req["graph"]["nodes"].as_array_mut().unwrap() {
            if node["id"] == "solve" {
                node["placement_tags"] = json!(["owned-live-owner"]);
            }
        }
    }
    if let Some(chosen) = branch {
        insert_branch(&mut req, chosen);
    }
    let (status, admitted) = http(&server, "/api/v1/workflows/graph/jobs", Some(req))?;
    assert_eq!(status, 202, "{admitted}");
    let job = admitted["job"]["job_id"]
        .as_str()
        .ok_or("missing job")?
        .to_owned();
    let held = wait_gate(&server, &data.0, &job, domain)?;
    if typed {
        assert_eq!(
            held["task_ir"]["integrity"]["task_digest_encoding"],
            TASK_DIGEST_TYPED_V2
        );
        assert_eq!(held["task_ir"]["config"]["precision_witness"], witness);
        assert_eq!(
            held["task_digest"],
            compute_operator_task_digest(&held["task_ir"])?
        );
        let values = held["task_ir"]["config"]["precision_witness"]
            .as_array()
            .unwrap();
        assert!(values[0].is_i64() && values[1].is_f64());
        assert_eq!(values[2].as_f64().unwrap().to_bits(), (-0.0f64).to_bits());
        let diagnostic: Value =
            serde_json::from_slice(&fs::read(data.0.join("native-recovery-diagnostic.json"))?)?;
        assert_eq!(
            diagnostic["integrity"]["task_digest_encoding"],
            TASK_DIGEST_TYPED_V2
        );
        assert_eq!(
            diagnostic["integrity"]["task_digest"],
            compute_operator_task_digest(&diagnostic)?
        );
        if !solve {
            let input = &diagnostic["input_artifact"]["precision_witness"];
            assert_eq!(input, &witness);
            assert!(input[0].is_i64() && input[1].is_f64());
            assert_eq!(input[2].as_f64().unwrap().to_bits(), (-0.0f64).to_bits());
        }
    }
    wait_for_lifecycle(&owner, "accepting", 0)?;
    let original = http(&server, &format!("/api/v1/jobs/{job}"), None)?.1;
    assert_eq!(original["job"]["status"], "solving");
    assert_eq!(original["result"]["recovery"]["generation"], 1);
    assert_eq!(
        original["result"]["recovery"]["native_result_checkpoint_count"], 1,
        "the completed diagnosis is durable before the next admission"
    );
    assert_eq!(
        original["result"]["recovery"]["solve_checkpoint_count"],
        usize::from(solve)
    );
    assert_eq!(
        original["result"]["recovery"]["solve_checkpoint_policy"]["max_bytes"],
        2_097_152
    );
    assert_eq!(
        original["result"]["recovery"]["branch_checkpoint_count"],
        usize::from(branch.is_some())
    );
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
    assert_eq!(count(&owner)?, if solve { 2 } else { 1 });
    assert_eq!(count(&peer)?, 0);
    // Lose the real BEAM after successful diagnosis, with scoring not yet admitted.
    drop(server);
    fs::remove_file(data.0.join("native-recovery-gate"))?;
    if lose_receipt {
        owner.stop_process()?;
        owner.start_process()?;
    }
    let restarted = start_server(owner.port(), peer.port(), &data.0, &database)?;
    let ended = wait_terminal(&restarted, &job)?;
    let recovery = &ended["result"]["recovery"];
    assert_eq!(recovery["generation"], 2, "{ended}");
    assert_eq!(recovery["attempt"], 2);
    if typed {
        assert_eq!(
            recovery["native_task_digest_encoding"],
            TASK_DIGEST_TYPED_V2
        );
    }
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
    {
        // Losing the Agent's volatile receipt is recoverable from the verified durable
        // Orchestra checkpoint. Neither that receipt nor completed solves may be replayed.
        assert_eq!(ended["job"]["status"], "completed", "{ended}");
        assert_eq!(recovery["state"], "completed");
        assert_eq!(recovery["envelope_retained"], false);
        assert_eq!(recovery["solve_checkpoint_count"], 0);
        assert_eq!(recovery["branch_checkpoint_count"], 0);
        assert_eq!(recovery["native_result_checkpoint_count"], 0);
        let full = http(&restarted, &format!("/api/v1/results/{job}"), None)?.1;
        let skipped: &[&str] = if branch.is_some() {
            &["unchosen", "unchosen-output"]
        } else {
            &[]
        };
        assert_ready_with_skips(domain, &full["result"], skipped);
        if let Some(chosen) = branch {
            let selected = if chosen { "if_true" } else { "if_false" };
            assert_eq!(
                full["result"]["branch_decisions"],
                json!([{
                    "node_id":"recovery-branch","chosen_output":if chosen {"if_true"} else {"if_false"},
                    "predicate_result":chosen
                }])
            );
            assert_eq!(
                full["result"]["skipped_nodes"],
                json!(["unchosen", "unchosen-output"])
            );
            assert!(
                full["result"]["artifacts"]
                    .get("unchosen.summary")
                    .is_none()
            );
            assert!(
                full["result"]["artifacts"]
                    .get("unchosen-output.summary")
                    .is_none()
            );
            let join = full["result"]["node_runs"]
                .as_array()
                .unwrap()
                .iter()
                .find(|run| run["node_id"] == "recovery-join")
                .ok_or("missing rebuilt join trace")?;
            let sources = json!([format!("recovery-branch.{selected}")]);
            assert_eq!(join["status"], "completed");
            assert_eq!(join["consumed_artifacts"], sources);
            let lineage = full["result"]["artifact_lineage"]
                .as_array()
                .unwrap()
                .iter()
                .find(|entry| entry["artifact_key"] == "recovery-join.summary")
                .ok_or("missing rebuilt join lineage")?;
            assert_eq!(lineage["source_artifacts"], sources);
            assert_eq!(
                full["result"]["artifacts"]["recovery-join.summary"],
                full["result"]["artifacts"]["diagnose.summary"]
            );
            let quality = full["result"]["node_runs"]
                .as_array()
                .unwrap()
                .iter()
                .find(|run| run["node_id"] == "quality")
                .ok_or("missing downstream scoring trace")?;
            assert_eq!(
                quality["consumed_artifacts"],
                json!(["recovery-join.summary"])
            );
        }
        assert_dispatches(&sdk, &data.0, &job, domain, false)?;
        if typed {
            let scoring = records(&data.0, &job)?
                .into_iter()
                .find(|record| record["operator_id"] == domain.quality)
                .ok_or("missing scoring dispatch")?;
            assert_eq!(
                scoring["task_digest"], held["task_digest"],
                "restart changed typed scoring identity"
            );
        }
        assert_eq!(
            count(&owner)? + count(&peer)?,
            if lose_receipt {
                1
            } else if solve {
                3
            } else {
                2
            },
            "completed solve and diagnosis must not execute again"
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
    drop(restarted);
    if let Some(scope) = postgres_scope.as_mut() {
        scope.cleanup()?;
    }
    Ok(())
}

#[test]
fn real_native_workflow_restart_reuses_original_success_receipt_without_duplicate_execution()
-> Result<()> {
    for domain in DOMAINS {
        run_round(domain, false, false, None, false, None)?;
    }
    Ok(())
}

#[test]
fn real_native_workflow_restart_with_lost_agent_receipt_uses_checkpoint_without_replay()
-> Result<()> {
    run_round(DOMAINS[0], true, false, None, false, None)
}

#[test]
fn real_solved_workflow_restart_reuses_durable_artifacts_before_native_receipt_recovery()
-> Result<()> {
    for domain in DOMAINS {
        run_round(domain, false, true, None, false, None)?;
    }
    Ok(())
}

#[test]
fn real_branch_workflow_restart_retains_route_before_native_receipt_recovery() -> Result<()> {
    for (domain, chosen) in [(DOMAINS[0], true), (DOMAINS[1], false)] {
        run_round(domain, false, true, Some(chosen), false, None)?;
    }
    Ok(())
}

#[test]
fn real_typed_solved_branch_workflow_restart_preserves_numeric_identity_without_reexecution()
-> Result<()> {
    for (index, domain) in DOMAINS.into_iter().enumerate() {
        run_round(domain, false, true, Some(index % 2 == 0), true, None)?;
    }
    Ok(())
}

#[test]
fn real_typed_workflow_lost_agent_receipt_recovers_after_solve_and_branch_restore() -> Result<()> {
    run_round(DOMAINS[0], true, true, Some(true), true, None)
}

#[test]
fn real_typed_raw_workflow_restart_preserves_input_numeric_identity() -> Result<()> {
    run_round(DOMAINS[0], false, false, None, true, None)
}

#[test]
#[ignore = "requires an explicitly provisioned disposable PostgreSQL test database"]
fn real_typed_postgres_solved_branch_restart_preserves_identity_without_reexecution() -> Result<()>
{
    let url = std::env::var("KYUUBIKI_HEADLESS_LIVE_TEST_POSTGRES_URL")?;
    for (index, domain) in DOMAINS.into_iter().enumerate() {
        run_round(domain, false, true, Some(index % 2 == 0), true, Some(&url))?;
    }
    Ok(())
}

#[test]
#[ignore = "requires an explicitly provisioned disposable PostgreSQL test database"]
fn real_typed_postgres_lost_agent_receipt_recovers_after_checkpoint_restore() -> Result<()> {
    let url = std::env::var("KYUUBIKI_HEADLESS_LIVE_TEST_POSTGRES_URL")?;
    run_round(DOMAINS[0], true, true, Some(true), true, Some(&url))
}

#[test]
#[ignore = "requires an explicitly provisioned disposable PostgreSQL test database"]
fn real_typed_postgres_raw_restart_preserves_input_numeric_identity() -> Result<()> {
    let url = std::env::var("KYUUBIKI_HEADLESS_LIVE_TEST_POSTGRES_URL")?;
    run_round(DOMAINS[0], false, false, None, true, Some(&url))
}
