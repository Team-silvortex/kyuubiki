use super::agent_support::{LiveAgent, wait_for_lifecycle};
use super::diagnostic_domains::DOMAINS;
use super::domain_diagnostics::{assert_ready, request};
use super::orchestra_completion::http_json;
use super::transport_research::assert_retained;
use super::write_ack_loss::count;
use super::{Scratch, start_live_server_with_state};
use kyuubiki_protocol::canonical_json;
use serde_json::{Value, json};
use std::{error::Error, fs};

#[test]
fn real_native_workflow_distinguishes_tiny_inputs_under_the_same_task_name()
-> Result<(), Box<dyn Error>> {
    let mut owner = LiveAgent::start()?;
    let mut peer = LiveAgent::start()?;
    let data = Scratch::new()?;
    let server =
        start_live_server_with_state(Some(owner.port()), Some(peer.port()), Some(&data.0))?;
    let url = format!("http://127.0.0.1:{}", server.port);
    owner.restart_with_orchestrator(&url)?;
    peer.restart_with_orchestrator(&url)?;
    let domain = DOMAINS[0];
    let raw = [1e-30, 2e-30]
        .into_iter()
        .map(|temperature| {
            let mut model = domain.model();
            for node in model["nodes"].as_array_mut().unwrap() {
                node["fix_temperature"] = json!(true);
                node["temperature"] = json!(temperature);
                node["heat_load"] = json!(0.0);
            }
            kyuubiki_engine::run_solve_operator(domain.solver, model).unwrap()
        })
        .collect::<Vec<_>>();
    assert_ne!(raw[0], raw[1]);
    assert_eq!(canonical_json(&raw[0]), canonical_json(&raw[1]));

    for (index, raw) in raw.into_iter().enumerate() {
        let req = serde_json::to_value(request(domain, raw.clone(), false, false))?;
        let (status, result) = http_json(server.port, "/api/v1/workflows/graph/run", Some(req))?;
        assert_eq!(status, 200, "{result}");
        assert_ready(domain, &result);
        assert_retained(&result["artifacts"]["raw.payload"], &raw, "raw.payload");
        assert_eq!(count(&owner)? + count(&peer)?, (index as u64 + 1) * 2);
    }

    let records = fs::read_dir(data.0.join("operator-task-dispatches"))?
        .map(|entry| -> Result<Value, Box<dyn Error>> {
            Ok(serde_json::from_slice::<Value>(&fs::read(entry?.path())?)?["payload"].clone())
        })
        .collect::<Result<Vec<_>, _>>()?;
    assert_eq!(records.len(), 4);
    for (node, id) in [("diagnose", domain.diagnostic), ("quality", domain.quality)] {
        let task_id = format!("workflow:{node}:{id}");
        let matching = records
            .iter()
            .filter(|record| record["task_id"] == task_id)
            .collect::<Vec<_>>();
        assert_eq!(matching.len(), 2, "task names must not supply separation");
        assert_ne!(
            matching[0]["task_digest"], matching[1]["task_digest"],
            "rounded values must not alias a native workflow invocation"
        );
        assert!(
            matching
                .iter()
                .all(|record| record["state"] == "observed_executed")
        );
    }
    wait_for_lifecycle(&owner, "accepting", 0)?;
    wait_for_lifecycle(&peer, "accepting", 0)?;
    assert_eq!(count(&owner)? + count(&peer)?, 4);
    Ok(())
}
