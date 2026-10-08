use super::agent_support::{LiveAgent, wait_for_lifecycle};
use super::diagnostic_domains::{DOMAINS, Domain};
use super::model_artifact::physical_result;
use super::transport_output::cli_with_policy;
use super::transport_research::{assert_retained, diagnostic_chain, workflow};
use super::write_ack_loss::count;
use super::{Scratch, start_live_server_with_state};
use kyuubiki_headless_sdk::ServiceHeadlessExecutor;
use kyuubiki_protocol::WorkflowGraphRunRequest;
use serde_json::{Value, json};
use std::{error::Error, fs, path::Path};

type Result<T> = std::result::Result<T, Box<dyn Error>>;

fn request(domain: Domain, input: Value, solve: bool, recover: bool) -> WorkflowGraphRunRequest {
    let req = diagnostic_chain::request(
        domain.name,
        domain.quality,
        domain.quality_config(),
        input,
        recover,
    );
    if solve {
        diagnostic_chain::with_solver(req, domain.solver)
    } else {
        req
    }
}

fn assert_ready(domain: Domain, result: &Value) {
    assert_eq!(result["failed_nodes"], json!([]), "{}", domain.name);
    assert_eq!(result["skipped_nodes"], json!([]));
    let artifacts = &result["artifacts"];
    let raw = artifacts["raw.payload"].clone();
    let expected =
        kyuubiki_engine::run_extract_operator(domain.diagnostic, raw, json!({})).unwrap();
    assert_retained(
        &artifacts["diagnose.summary"],
        &expected,
        "diagnose.summary",
    );
    let quality =
        kyuubiki_engine::run_transform_operator(domain.quality, expected, domain.quality_config())
            .unwrap();
    assert_retained(&artifacts["quality.summary"], &quality, "quality.summary");
    assert_eq!(
        artifacts["decision.summary"]["composite_quality_missing_metric_count"],
        0
    );
    assert_eq!(
        artifacts["decision.summary"]["composite_quality_ready"],
        true
    );
    assert_eq!(artifacts["independent_output.value"], json!({"value":7}));
}

fn assert_dispatches(
    client: &ServiceHeadlessExecutor,
    root: &Path,
    job_id: &str,
    domain: Domain,
    failed: bool,
) -> Result<()> {
    let prefix = format!("{job_id}:");
    let records = fs::read_dir(root.join("operator-task-dispatches"))?
        .map(|entry| -> Result<Value> {
            let envelope: Value = serde_json::from_slice(&fs::read(entry?.path())?)?;
            Ok(envelope["payload"].clone())
        })
        .collect::<Result<Vec<_>>>()?;
    let records = records
        .iter()
        .filter(|record| {
            record["task_id"]
                .as_str()
                .is_some_and(|id| id.starts_with(&prefix))
        })
        .filter(|record| {
            record["operator_id"] == domain.diagnostic || record["operator_id"] == domain.quality
        })
        .collect::<Vec<_>>();
    assert_eq!(
        records.len(),
        if failed { 1 } else { 2 },
        "{job_id}: wrong native TaskIR count"
    );
    for record in records {
        let diagnostic = record["operator_id"] == domain.diagnostic;
        let id = if diagnostic {
            domain.diagnostic
        } else {
            domain.quality
        };
        let node = if diagnostic { "diagnose" } else { "quality" };
        let task_id = format!("{job_id}:{node}:{id}");
        assert_eq!(record["task_id"], task_id);
        assert_eq!(record["program_id"], id);
        let state = if failed {
            "observed_failed"
        } else {
            "observed_executed"
        };
        assert_eq!(record["state"], state);
        let inspection = client
            .inspect_operator_task_dispatch(&task_id, record["task_digest"].as_str().unwrap())
            .map_err(|error| error.message)?;
        assert_eq!(
            inspection["attempts"].as_array().unwrap().len(),
            1,
            "hidden native replay"
        );
        assert_eq!(
            inspection["attempts"][0]["attempt_id"],
            record["attempt_id"]
        );
        assert_eq!(
            inspection["attempts"][0]["request_id"],
            record["request_id"]
        );
        assert_eq!(inspection["attempts"][0]["state"], state);
        assert_eq!(
            inspection["attempts"][0]["observation"]["status"],
            "retained_dispatch_observation"
        );
        assert_eq!(inspection["attempts"][0]["observation"]["outcome"], state);
        // Inspection is evidence only; terminal payload retrieval is a separate API.
        assert_eq!(inspection["terminal_result_available"], false);
        assert_eq!(inspection["automatic_replay_authorized"], false);
    }
    Ok(())
}

#[test]
fn real_cross_domain_native_diagnostic_quality_chains_isolate_corruption_and_recover() -> Result<()>
{
    let mut owner = LiveAgent::start()?;
    let mut peer = LiveAgent::start()?;
    let data = Scratch::new()?;
    let server =
        start_live_server_with_state(Some(owner.port()), Some(peer.port()), Some(&data.0))?;
    let url = format!("http://127.0.0.1:{}", server.port);
    owner.restart_with_orchestrator(&url)?;
    peer.restart_with_orchestrator(&url)?;
    let client = ServiceHeadlessExecutor::try_new(&url).map_err(|error| error.message)?;
    for (index, domain) in DOMAINS.into_iter().enumerate() {
        let report = cli_with_policy(
            &workflow(request(domain, domain.model(), true, false)),
            &url,
            true,
            true,
        )?;
        let healthy = physical_result(&report);
        assert_ready(domain, &healthy);
        assert_dispatches(
            &client,
            &data.0,
            report.steps[0].result_preview["job_id"].as_str().unwrap(),
            domain,
            false,
        )?;
        assert_eq!(count(&owner)? + count(&peer)?, index as u64 * 6 + 3);

        let raw = healthy["artifacts"]["raw.payload"].clone();
        let bad = domain.corrupt(raw.clone());
        let report = cli_with_policy(
            &workflow(request(domain, bad.clone(), false, true)),
            &url,
            true,
            true,
        )?;
        let partial = physical_result(&report);
        assert_eq!(partial["failed_nodes"], json!(["diagnose"]));
        assert_eq!(partial["skipped_nodes"].as_array().unwrap().len(), 3);
        for node in ["diagnose", "quality", "objective", "decision"] {
            assert!(
                partial["artifacts"]
                    .get(format!("{node}.summary"))
                    .is_none()
            );
        }
        assert_retained(&partial["artifacts"]["raw.payload"], &bad, "raw.payload");
        assert_eq!(
            partial["artifacts"]["independent_output.value"],
            json!({"value":7})
        );
        let trace = partial["node_runs"]
            .as_array()
            .unwrap()
            .iter()
            .find(|node| node["node_id"] == "diagnose")
            .unwrap();
        assert!(
            trace["error_message"]
                .as_str()
                .unwrap()
                .contains(&format!("payload.nodes[0].{}", domain.scalar))
        );
        assert_dispatches(
            &client,
            &data.0,
            report.steps[0].result_preview["job_id"].as_str().unwrap(),
            domain,
            true,
        )?;
        assert_eq!(count(&owner)? + count(&peer)?, index as u64 * 6 + 4);

        let report = cli_with_policy(
            &workflow(request(domain, raw.clone(), false, false)),
            &url,
            true,
            true,
        )?;
        let recovered = physical_result(&report);
        assert_ready(domain, &recovered);
        assert_retained(&recovered["artifacts"]["raw.payload"], &raw, "raw.payload");
        assert_dispatches(
            &client,
            &data.0,
            report.steps[0].result_preview["job_id"].as_str().unwrap(),
            domain,
            false,
        )?;
        assert_eq!(count(&owner)? + count(&peer)?, (index as u64 + 1) * 6);
        assert_eq!(partial["failed_nodes"], json!(["diagnose"]));
    }
    wait_for_lifecycle(&owner, "accepting", 0)?;
    wait_for_lifecycle(&peer, "accepting", 0)?;
    let domain = Domain {
        solver: "solve.stokes_flow_triangle_2d",
        ..DOMAINS[4]
    };
    let mut triangle = domain.model();
    triangle["nodes"].as_array_mut().unwrap().pop();
    triangle["elements"][0]
        .as_object_mut()
        .unwrap()
        .remove("node_l");
    let report = cli_with_policy(
        &workflow(request(domain, triangle, true, false)),
        &url,
        true,
        true,
    )?;
    let result = physical_result(&report);
    assert_ready(domain, &result);
    assert_eq!(
        result["artifacts"]["diagnose.summary"]["diagnostic_node_count"],
        3
    );
    assert_dispatches(
        &client,
        &data.0,
        report.steps[0].result_preview["job_id"].as_str().unwrap(),
        domain,
        false,
    )?;
    assert_eq!(
        count(&owner)? + count(&peer)?,
        33,
        "sixteen jobs must not replay"
    );
    Ok(())
}
