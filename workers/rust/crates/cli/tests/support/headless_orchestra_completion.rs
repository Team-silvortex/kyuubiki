use super::agent_support::{LiveAgent, wait_for_lifecycle};
use super::{
    parse_json_output, run_headless_command, start_live_server_with_agent, write_temp_json,
};
use kyuubiki_headless_sdk::{
    HeadlessExecutionBatch, HeadlessExecutionBatchStep, HeadlessRisk, HeadlessRunReport,
    ServiceHeadlessExecutor, execute_batch_with_executor,
};
use kyuubiki_protocol::compute_operator_task_digest;
use serde_json::{Value, json};
use std::error::Error;
use std::fs;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;

fn fixture_task(id: &str) -> Value {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../../../../schemas/examples.operator-task-batch.json"
    ))
    .unwrap();
    let mut task = fixture["tasks"][0]["task_ir"].clone();
    task["task_id"] = json!(id);
    task["execution_program"]["package_version"] = json!("0.1.0");
    task["runtime_hints"]["package_version"] = json!("0.1.0");
    resign(task)
}

fn resign(mut task: Value) -> Value {
    task["integrity"]["task_digest"] = json!(compute_operator_task_digest(&task).unwrap());
    task
}

pub(super) fn bar_task(id: &str, area: f64) -> Value {
    let mut task = fixture_task(id);
    task["operator"] = json!({"id":"solve.bar_1d","family":"mechanical","kind":"solver"});
    task["input_artifact"] = json!({"length":1.0,"area":area,"youngs_modulus":210e9,
        "elements":4,"tip_force":1000.0});
    let program = &mut task["execution_program"];
    program["program_id"] = json!("solve.bar_1d");
    program["program_family"] = json!("mechanical");
    program["program_kind"] = json!("solver");
    program["package_ref"] = Value::Null;
    program["runtime_protocol"] = json!("kyuubiki.solver-rpc/v1");
    program["abi"]["kind"] = json!("solver_rpc");
    program["entrypoint"] =
        json!({"kind":"solver_method","name":"solve_bar_1d","operator_kind":"solver"});
    task["runtime_hints"] = json!({"authority_mode":"agent_local","execution_mode":"agent_native",
        "agent_fetchable":false,"cache_scope":"none","operator_kind":"solver"});
    resign(task)
}

pub(super) fn batch(task: Value, project_name: Option<&str>) -> HeadlessExecutionBatch {
    let mut steps = vec![HeadlessExecutionBatchStep {
        index: 1,
        action: "operator_task_execute".into(),
        risk: HeadlessRisk::Normal,
        payload: json!({"task":task}),
    }];
    if let Some(name) = project_name {
        steps.push(HeadlessExecutionBatchStep {
            index: 2,
            action: "project_create".into(),
            risk: HeadlessRisk::Normal,
            payload: json!({"name":name}),
        });
    }
    HeadlessExecutionBatch {
        schema_version: "kyuubiki.headless-execution-batch/v1".into(),
        exported_at: "2026-10-05T00:00:00Z".into(),
        language: "en".into(),
        workflow_id: "real-orchestra-agent-completion".into(),
        template_id: None,
        steps,
        warnings: vec![],
    }
}

fn execute(
    executor: &mut ServiceHeadlessExecutor,
    task: Value,
    project: Option<&str>,
) -> HeadlessRunReport {
    execute_batch_with_executor(&batch(task, project), executor, false, false)
}

pub(super) fn http_json(
    port: u16,
    path: &str,
    body: Option<Value>,
) -> Result<(u16, Value), Box<dyn Error>> {
    let mut stream = TcpStream::connect(("127.0.0.1", port))?;
    stream.set_read_timeout(Some(Duration::from_secs(10)))?;
    stream.set_write_timeout(Some(Duration::from_secs(10)))?;
    let payload = body
        .as_ref()
        .map(serde_json::to_vec)
        .transpose()?
        .unwrap_or_default();
    let method = if body.is_some() { "POST" } else { "GET" };
    write!(
        stream,
        "{method} {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        payload.len()
    )?;
    stream.write_all(&payload)?;
    let mut bytes = Vec::new();
    const MAX_LIVE_JSON_BYTES: u64 = 16 * 1024 * 1024;
    stream
        .take(MAX_LIVE_JSON_BYTES + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAX_LIVE_JSON_BYTES {
        return Err("live JSON response exceeds the explicit test transport limit".into());
    }
    let split = bytes
        .windows(4)
        .position(|part| part == b"\r\n\r\n")
        .ok_or("missing HTTP headers")?;
    let head = std::str::from_utf8(&bytes[..split])?;
    let status = head
        .split_whitespace()
        .nth(1)
        .ok_or("missing HTTP status")?
        .parse()?;
    let length: usize = head
        .lines()
        .find_map(|line| {
            line.split_once(':')
                .filter(|(name, _)| name.eq_ignore_ascii_case("content-length"))
                .map(|(_, length)| length.trim().parse())
        })
        .ok_or("live JSON response has no Content-Length")??;
    let data = bytes
        .get(split + 4..split + 4 + length)
        .ok_or("truncated HTTP response")?;
    Ok((status, serde_json::from_slice(data)?))
}

fn post(port: u16, route: &str, body: Value) -> Result<Value, Box<dyn Error>> {
    let (status, payload) =
        http_json(port, &format!("/api/v1/operator-tasks/{route}"), Some(body))?;
    assert_eq!(status, 200, "{route}: {payload}");
    Ok(payload)
}

pub(super) fn assert_bar(receipt: &Value) {
    assert_eq!(receipt["status"], "executed", "{receipt}");
    assert_eq!(
        receipt["result"]["operator_task_ir_status"], "executed",
        "TaskIR bypassed Agent: {receipt}"
    );
    assert_eq!(
        receipt["result"]["execution_runtime_status"],
        "agent_engine_solver_executed"
    );
    let displacement = receipt["result"]["result"]["tip_displacement"]
        .as_f64()
        .unwrap();
    // Independent closed form FL/EA, not a second call to the same solver.
    assert!((displacement / (1000.0 / (210e9 * 0.01)) - 1.0).abs() < 1e-12);
}

#[test]
fn real_orchestra_agent_chain_blocks_fails_and_recovers_without_downstream_side_effects()
-> Result<(), Box<dyn Error>> {
    let agent = LiveAgent::start()?;
    let server = start_live_server_with_agent(Some(agent.port()))?;
    let mut executor =
        ServiceHeadlessExecutor::try_new(&format!("http://127.0.0.1:{}", server.port))
            .map_err(|error| error.message)?;
    let (_, before) = http_json(server.port, "/api/v1/projects", None)?;

    let blocked = execute(
        &mut executor,
        fixture_task("real-pending"),
        Some("forbidden-after-block"),
    );
    assert_eq!(blocked.status, "blocked", "{blocked:?}\n{}", server.logs());
    assert_eq!(blocked.executed_step_count, 0);
    assert_eq!(blocked.steps.len(), 1);
    assert_eq!(
        blocked.steps[0].result_preview["execution_readiness"]["required_action"],
        "attach_operator_package_runtime"
    );
    wait_for_lifecycle(&agent, "accepting", 0)?;

    let first = execute(&mut executor, bar_task("real-bar", 0.01), None);
    assert_eq!(first.status, "ok", "{first:?}\n{}", server.logs());
    assert_eq!(first.executed_step_count, 1);
    assert_bar(&first.steps[0].result_preview);

    let failed = execute(
        &mut executor,
        bar_task("real-invalid-bar", 0.0),
        Some("forbidden-after-failure"),
    );
    assert_eq!(failed.status, "failed", "{failed:?}\n{}", server.logs());
    assert_eq!(failed.executed_step_count, 0);
    assert_eq!(failed.steps.len(), 1);
    let failure = &failed.steps[0].result_preview["failure_receipt"];
    assert_eq!(failure["reason_code"], "operator_task_solver_input_invalid");
    assert_eq!(failure["failure_stage"], "decode_solver_input");
    assert_eq!(failure["task_id"], "real-invalid-bar");
    assert_eq!(failure["recovery"]["retryable"], false);
    assert_eq!(failure["recovery"]["safe_to_continue_other_tasks"], true);
    assert_eq!(
        failure["recovery"]["required_action"],
        "fix_solver_input_artifact"
    );
    let recovery = failed
        .execution_summary
        .failure
        .as_ref()
        .expect("missing structured failure summary");
    assert_eq!(recovery.recommended_action, "fix_solver_input_artifact");
    assert!(!recovery.retryable);
    assert_eq!(recovery.step_index, 1);
    wait_for_lifecycle(&agent, "accepting", 0)?;

    let repaired = execute(&mut executor, bar_task("real-invalid-bar", 0.01), None);
    assert_eq!(repaired.status, "ok", "{repaired:?}");
    assert_eq!(repaired.executed_step_count, 1);
    assert_bar(&repaired.steps[0].result_preview);
    assert_eq!(
        repaired.steps[0].result_preview["task_id"],
        "real-invalid-bar"
    );

    let replay = execute(&mut executor, bar_task("real-bar", 0.01), None);
    assert_eq!(replay.status, "ok", "{replay:?}");
    assert_bar(&replay.steps[0].result_preview);
    assert_eq!(
        first.steps[0].result_preview,
        replay.steps[0].result_preview
    );
    let (_, after) = http_json(server.port, "/api/v1/projects", None)?;
    assert_eq!(
        before, after,
        "blocked/failed task created a downstream project"
    );

    let completed = execute(
        &mut executor,
        bar_task("real-bar-next", 0.01),
        Some("allowed-after-success"),
    );
    assert_eq!(completed.status, "ok", "{completed:?}");
    assert_eq!(completed.executed_step_count, 2);
    let (_, projects) = http_json(server.port, "/api/v1/projects", None)?;
    assert!(
        projects["projects"]
            .as_array()
            .unwrap()
            .iter()
            .any(|project| project["name"] == "allowed-after-success")
    );
    Ok(())
}

#[test]
fn real_orchestra_agent_mixed_batch_checkpoint_retains_only_unfinished_targets()
-> Result<(), Box<dyn Error>> {
    let agent = LiveAgent::start()?;
    let server = start_live_server_with_agent(Some(agent.port()))?;
    let batch = json!({"quality_execution_batch_contract":"kyuubiki.quality_execution_batch/v1",
    "tasks":[
        {"case_id":"good","task_ir":bar_task("mixed-good", 0.01)},
        {"case_id":"blocked","task_ir":fixture_task("mixed-blocked")},
        {"case_id":"bad","task_ir":bar_task("mixed-bad", 0.0)}
    ]});
    let execution = post(server.port, "execute-batch", json!({"batch":batch}))?;
    assert_eq!(
        execution["status"],
        "partial",
        "{execution}\n{}",
        server.logs()
    );
    for (field, count) in [
        ("task_count", 3),
        ("attempted_count", 3),
        ("executed_count", 1),
        ("ok_count", 1),
        ("blocked_count", 1),
        ("error_count", 1),
        ("skipped_count", 0),
    ] {
        assert_eq!(execution[field], count, "{field}: {execution}");
    }
    assert_bar(&json!({"status":"executed","result":execution["results"][0]["result"]}));
    let failure = &execution["results"][2]["failure_receipt"];
    assert_eq!(failure["failure_owner"], "agent_runtime");
    assert_eq!(failure["task_id"], "mixed-bad");
    assert_eq!(failure["reason_code"], "operator_task_solver_input_invalid");
    let checkpoint = post(
        server.port,
        "checkpoint-batch",
        json!({"batch":batch,"execution":execution}),
    )?;
    let persisted = write_temp_json("real-mixed-checkpoint", &checkpoint);
    let restored: Value = serde_json::from_slice(&fs::read(&*persisted)?)?;
    let plan = post(
        server.port,
        "resume-plan-batch",
        json!({"batch":batch,"checkpoint":restored}),
    )?;
    assert_eq!(plan["next_action"], "resolve_incomplete_cases", "{plan}");
    assert_eq!(plan["target_case_ids"], json!(["blocked", "bad"]));
    assert_eq!(plan["blocked_case_ids"], json!(["blocked"]));
    assert!(
        plan["recovery_actions"]
            .as_array()
            .unwrap()
            .contains(&json!("fix_solver_input_artifact"))
    );
    assert!(
        plan["recovery_actions"]
            .as_array()
            .unwrap()
            .contains(&json!("attach_operator_package_runtime"))
    );
    wait_for_lifecycle(&agent, "accepting", 0)?;
    Ok(())
}

#[test]
fn real_orchestra_agent_cli_blocked_exit_preserves_report() -> Result<(), Box<dyn Error>> {
    let agent = LiveAgent::start()?;
    let server = start_live_server_with_agent(Some(agent.port()))?;
    let workflow = write_temp_json(
        "real-blocked-workflow",
        &serde_json::to_value(batch(
            fixture_task("cli-pending"),
            Some("forbidden-cli-project"),
        ))?,
    );
    let output_path = workflow.parent().unwrap().join("report.json");
    let base_url = format!("http://127.0.0.1:{}", server.port);
    let output = run_headless_command(&[
        "run",
        workflow.to_str().unwrap(),
        "--execute",
        "--executor",
        "service",
        "--api-base-url",
        &base_url,
        "--json",
        "--report-out",
        output_path.to_str().unwrap(),
    ]);
    assert!(!output.status.success(), "{}", server.logs());
    assert!(
        !output.stdout.is_empty(),
        "missing CLI report: {}\n{}",
        String::from_utf8_lossy(&output.stderr),
        server.logs()
    );
    let report = parse_json_output(&output);
    assert_eq!(report["status"], "blocked", "{report}\n{}", server.logs());
    assert_eq!(report["executed_step_count"], 0);
    assert_eq!(report["steps"].as_array().unwrap().len(), 1);
    assert_eq!(
        report,
        serde_json::from_slice::<Value>(&fs::read(output_path)?)?
    );
    let error: Value = serde_json::from_slice(&output.stderr)?;
    assert_eq!(error["error"]["code"], "headless_execution_blocked");
    let (_, projects) = http_json(server.port, "/api/v1/projects", None)?;
    assert!(
        !projects["projects"]
            .as_array()
            .unwrap()
            .iter()
            .any(|project| project["name"] == "forbidden-cli-project")
    );
    wait_for_lifecycle(&agent, "accepting", 0)?;
    Ok(())
}
