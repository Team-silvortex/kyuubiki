use super::agent_support::LiveAgent;
use super::diagnostic_domains::DOMAINS;
use super::orchestra_completion::http_json;
use super::{Scratch, repo_root, start_live_server_with_agent};
use kyuubiki_headless_sdk::{
    TASK_DIGEST_TYPED_V2, prepare_operator_task_payload, seal_operator_task_digest,
};
use kyuubiki_protocol::{canonical_json, verify_operator_task_digest};
use serde_json::{Value, json};
use std::{error::Error, fs, process::Command};

fn elixir_authored(inputs: &Value, scratch: &Scratch) -> Result<Vec<Value>, Box<dyn Error>> {
    let path = scratch.0.join("typed-authoring-input.json");
    fs::write(&path, serde_json::to_vec(inputs)?)?;
    let output = Command::new("mix")
        .args([
            "run",
            "--no-start",
            "-r",
            "test/support/task_digest_v2_authoring.exs",
            "-e",
            "KyuubikiWeb.TestSupport.TaskDigestV2Authoring.main(System.argv())",
            "--",
        ])
        .arg(path)
        .env("MIX_ENV", "test")
        .current_dir(repo_root().join("apps/web"))
        .output()?;
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout)?;
    let json = stdout
        .lines()
        .find_map(|line| line.strip_prefix("TASK_DIGEST_V2_AUTHORED "))
        .ok_or("Elixir authoring output missing")?;
    Ok(serde_json::from_str(json)?)
}

#[test]
fn real_rust_and_elixir_typed_tasks_are_verified_independently_by_agent_and_http()
-> Result<(), Box<dyn Error>> {
    let agent = LiveAgent::start()?;
    let server = start_live_server_with_agent(Some(agent.port()))?;
    let scratch = Scratch::new()?;
    let config =
        json!({"numeric_probe":[1e-30, 1e-300, 5e-324, -0.0, 0.33333333333333337, 1, 1.0]});
    let inputs = json!(
        DOMAINS
            .iter()
            .map(|domain| json!({"operator_id":domain.diagnostic,
        "family":domain.name, "raw":domain.raw(), "config":config,
        "task_id":format!("typed-elixir-{}",domain.name)}))
            .collect::<Vec<_>>()
    );
    let elixir = elixir_authored(&inputs, &scratch)?;
    assert_eq!(elixir.len(), DOMAINS.len());

    for (index, domain) in DOMAINS.into_iter().enumerate() {
        let raw = inputs[index]["raw"].clone();
        let expected =
            kyuubiki_engine::run_extract_operator(domain.diagnostic, raw.clone(), config.clone())?;
        let mut rust = domain.task(
            &format!("typed-rust-{}", domain.name),
            domain.diagnostic,
            "extract",
            raw,
            config.clone(),
        );
        seal_operator_task_digest(&mut rust, TASK_DIGEST_TYPED_V2)?;
        for task in [rust, elixir[index].clone()] {
            verify_operator_task_digest(&task).unwrap();
            assert!(
                task["orchestration_context"]
                    .get("workflow_native_invocation_sha256")
                    .is_none()
            );
            prepare_operator_task_payload(&json!({"task":task.clone()}))?;
            let (status, prepared) = http_json(
                server.port,
                "/api/v1/operator-tasks/prepare",
                Some(json!({"task":task.clone()})),
            )?;
            assert_eq!(status, 200, "{prepared}");
            assert_eq!(prepared["task_digest"], task["integrity"]["task_digest"]);

            let response = agent.request(
                "typed-execute",
                "run_operator_task_ir",
                json!({"mode":"execute", "task_ir":task.clone()}),
            )?;
            assert_eq!(response["ok"], true, "{response}");
            assert_eq!(
                response["result"]["task_digest"],
                task["integrity"]["task_digest"]
            );
            assert_eq!(response["result"]["result"], expected);

            let mut changed = task.clone();
            changed["config"]["numeric_probe"][0] = json!(2e-30);
            assert_eq!(
                canonical_json(&changed["config"]),
                canonical_json(&task["config"])
            );
            let mut removed = task.clone();
            removed["integrity"]
                .as_object_mut()
                .unwrap()
                .remove("task_digest_encoding");
            let mut unknown = task.clone();
            unknown["integrity"]["task_digest_encoding"] = json!("future");
            for bad in [changed, removed, unknown] {
                let response = agent.request(
                    "typed-reject",
                    "run_operator_task_ir",
                    json!({"mode":"execute", "task_ir":bad.clone()}),
                )?;
                assert_eq!(response["ok"], false, "{response}");
                assert!(
                    matches!(
                        response["error"]["code"].as_str(),
                        Some("operator_task_digest_mismatch" | "operator_task_digest_invalid")
                    ),
                    "{response}"
                );
                assert_eq!(
                    response["error"]["details"]["operator_task_failure_receipt"]["failure_stage"],
                    "verify_digest"
                );
                let (status, rejected) = http_json(
                    server.port,
                    "/api/v1/operator-tasks/prepare",
                    Some(json!({"task":bad})),
                )?;
                assert_eq!(status, 422, "{rejected}");
            }
            let healthy = agent.request(
                "typed-healthy",
                "run_operator_task_ir",
                json!({"mode":"preflight", "task_ir":task.clone()}),
            )?;
            assert_eq!(healthy["ok"], true, "{healthy}");
        }
    }
    Ok(())
}

#[test]
fn real_typed_dispatch_requires_advertisement_and_recovers_original_receipts_after_orchestra_restart()
-> Result<(), Box<dyn Error>> {
    use super::live_server::start_live_server_with_described_agents;
    use super::write_ack_loss::count;

    let mut agent = LiveAgent::start()?;
    let scratch = Scratch::new()?;
    let config = json!({"numeric_probe": [1e-30, 1e-300, 5e-324, -0.0, 1, 1.0]});
    let inputs = json!(
        DOMAINS
            .iter()
            .map(|domain| json!({
                "operator_id":domain.diagnostic, "family":domain.name, "raw":domain.raw(),
                "config":config, "task_id":format!("typed-http-elixir-{}", domain.name)
            }))
            .collect::<Vec<_>>()
    );
    let elixir = elixir_authored(&inputs, &scratch)?;

    let unknown = start_live_server_with_agent(Some(agent.port()))?;
    let (status, blocked) = http_json(
        unknown.port,
        "/api/v1/operator-tasks/execute",
        Some(json!({"task":elixir[0]})),
    )?;
    assert_eq!(status, 422, "{blocked}");
    assert_eq!(blocked["error_code"], "no_matching_agent");
    assert_eq!(count(&agent)?, 0);
    drop(unknown);

    let server = start_live_server_with_described_agents(agent.port(), &scratch.0)?;
    let mut retained = Vec::new();
    for (index, domain) in DOMAINS.into_iter().enumerate() {
        let raw = inputs[index]["raw"].clone();
        let expected =
            kyuubiki_engine::run_extract_operator(domain.diagnostic, raw.clone(), config.clone())?;
        let mut rust = domain.task(
            &format!("typed-http-rust-{}", domain.name),
            domain.diagnostic,
            "extract",
            raw,
            config.clone(),
        );
        seal_operator_task_digest(&mut rust, TASK_DIGEST_TYPED_V2)?;
        for task in [rust, elixir[index].clone()] {
            let (status, completed) = http_json(
                server.port,
                "/api/v1/operator-tasks/execute",
                Some(json!({"task":task})),
            )?;
            assert_eq!(status, 200, "{completed}");
            assert_eq!(completed["status"], "executed", "{completed}");
            assert_eq!(completed["result"]["result"], expected);
            assert_eq!(completed["task_digest"], task["integrity"]["task_digest"]);

            let query =
                json!({"task_id":task["task_id"], "task_digest":task["integrity"]["task_digest"]});
            let (status, inspected) = http_json(
                server.port,
                "/api/v1/operator-tasks/inspect-dispatch",
                Some(query.clone()),
            )?;
            assert_eq!(status, 200, "{inspected}");
            assert_eq!(inspected["retained_attempt_count"], 1);
            assert_eq!(inspected["attempts"][0]["state"], "observed_executed");
            let mut query = query;
            query["attempt_id"] = inspected["attempts"][0]["attempt_id"].clone();
            retained.push((task, query, expected.clone()));
        }
    }
    assert_eq!(count(&agent)?, 10);
    drop(server);

    // Journal survives BEAM restart; successful results stay on the original live Agent only.
    let server = start_live_server_with_described_agents(agent.port(), &scratch.0)?;
    for (task, query, expected) in &retained {
        let (status, recovered) = http_json(
            server.port,
            "/api/v1/operator-tasks/fetch-dispatch-result",
            Some(query.clone()),
        )?;
        assert_eq!(status, 200, "{recovered}");
        assert_eq!(recovered["status"], "receipt_recovered", "{recovered}");
        assert_eq!(recovered["outcome"], "executed");
        assert_eq!(recovered["completion"]["result"]["result"], *expected);
        assert_eq!(
            recovered["completion"]["task_digest"],
            task["integrity"]["task_digest"]
        );
        assert_eq!(recovered["automatic_replay_authorized"], false);
        assert_eq!(recovered["publication_performed"], false);
        let mut changed = task.clone();
        changed["config"]["numeric_probe"][0] = json!(2e-30);
        let (status, rejected) = http_json(
            server.port,
            "/api/v1/operator-tasks/execute",
            Some(json!({"task":changed})),
        )?;
        assert_eq!(status, 422, "{rejected}");
        assert_eq!(rejected["error_code"], "operator_task_digest_mismatch");
    }
    assert_eq!(
        count(&agent)?,
        10,
        "receipt retrieval or rejected drift must not execute"
    );

    agent.restart_with_orchestrator(&format!("http://127.0.0.1:{}", server.port))?;
    for (_, query, _) in &retained {
        let (status, missing) = http_json(
            server.port,
            "/api/v1/operator-tasks/fetch-dispatch-result",
            Some(query.clone()),
        )?;
        assert_eq!(status, 200, "{missing}");
        assert_eq!(missing["outcome"], "unknown");
        assert!(missing["completion"].is_null());
        assert_eq!(missing["automatic_replay_authorized"], false);
        assert_eq!(missing["publication_performed"], false);
    }
    assert_eq!(
        count(&agent)?,
        0,
        "lost Agent receipts are not replay authority"
    );
    Ok(())
}
