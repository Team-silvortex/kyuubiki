use kyuubiki_headless_sdk::{
    HeadlessExecutionBatch, HeadlessExecutionBatchStep, HeadlessRisk, ServiceHeadlessExecutor,
    execute_batch_with_executor, prepare_operator_task_payload,
};
use kyuubiki_protocol::compute_operator_task_digest;
use serde_json::{Value, json};
use std::error::Error;
use std::fs;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

#[allow(dead_code)]
#[path = "support/agent_lifecycle.rs"]
mod support;
use support::*;

static SCRATCH_SEQUENCE: AtomicU64 = AtomicU64::new(0);

struct Scratch(PathBuf);

impl Scratch {
    fn new() -> Result<Self, Box<dyn Error>> {
        let root = std::env::temp_dir().join(format!(
            "kyuubiki-headless-completion-{}-{}-{}",
            std::process::id(),
            SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos(),
            SCRATCH_SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root)?;
        Ok(Self(root))
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn golden_task() -> Value {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../../../schemas/examples.operator-task-batch.json"
    ))
    .unwrap();
    fixture["tasks"][0]["task_ir"].clone()
}

fn bar_task(area: f64) -> Value {
    let mut task = golden_task();
    task["task_id"] = json!("headless-agent-bar");
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
    task["integrity"]["task_digest"] = json!(compute_operator_task_digest(&task).unwrap());
    task
}

fn batch(task: Value, later_side_effect: bool) -> HeadlessExecutionBatch {
    let mut steps = vec![HeadlessExecutionBatchStep {
        index: 1,
        action: "operator_task_execute".into(),
        risk: HeadlessRisk::Normal,
        payload: json!({"task":task}),
    }];
    if later_side_effect {
        steps.push(HeadlessExecutionBatchStep {
            index: 2,
            action: "project_create".into(),
            risk: HeadlessRisk::Normal,
            payload: json!({"name":"must-not-be-created"}),
        });
    }
    HeadlessExecutionBatch {
        schema_version: "kyuubiki.headless-execution-batch/v1".into(),
        exported_at: "2026-10-05T00:00:00Z".into(),
        language: "en".into(),
        workflow_id: "real-agent-completion-regression".into(),
        template_id: None,
        steps,
        warnings: vec![],
    }
}

fn read_http_task(stream: &mut TcpStream) -> Result<Value, Box<dyn Error>> {
    stream.set_read_timeout(Some(Duration::from_secs(5)))?;
    let mut bytes = Vec::new();
    loop {
        let mut buffer = [0; 1024];
        let count = stream.read(&mut buffer)?;
        if count == 0 || bytes.len() + count > 64 * 1024 {
            return Err("incomplete or oversized HTTP task".into());
        }
        bytes.extend_from_slice(&buffer[..count]);
        if let Some(split) = bytes.windows(4).position(|part| part == b"\r\n\r\n") {
            let header = std::str::from_utf8(&bytes[..split])?;
            if !header.starts_with("POST /api/v1/operator-tasks/execute HTTP/1.1\r\n") {
                return Err("unexpected downstream side effect reached the server".into());
            }
            let length: usize = header
                .lines()
                .find_map(|line| line.strip_prefix("Content-Length: "))
                .ok_or("missing Content-Length")?
                .parse()?;
            if bytes.len() >= split + 4 + length {
                return Ok(serde_json::from_slice(
                    &bytes[split + 4..split + 4 + length],
                )?);
            }
        }
    }
}

#[test]
fn headless_http_receipts_from_a_real_agent_block_fail_and_recover() -> Result<(), Box<dyn Error>> {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    listener.set_nonblocking(true)?;
    let address = listener.local_addr()?;
    let agent = LiveAgent::start()?;
    // This test adapter only wraps real Agent TCP replies in the current HTTP
    // envelope. It is not a replacement Orchestra or an installed qualification.
    let server = thread::spawn(move || -> Result<Vec<Value>, String> {
        let mut replies = Vec::new();
        for index in 0..4 {
            let deadline = Instant::now() + Duration::from_secs(10);
            let mut stream = loop {
                match listener.accept() {
                    Ok((stream, _)) => break stream,
                    Err(error)
                        if error.kind() == std::io::ErrorKind::WouldBlock
                            && Instant::now() < deadline =>
                    {
                        thread::sleep(Duration::from_millis(5))
                    }
                    Err(error) => return Err(error.to_string()),
                }
            };
            stream
                .set_nonblocking(false)
                .map_err(|error| error.to_string())?;
            let payload = read_http_task(&mut stream).map_err(|error| error.to_string())?;
            let prepared = prepare_operator_task_payload(&payload)?;
            let response = agent
                .request(
                    &format!("sdk-agent-{index}"),
                    "run_operator_task_ir",
                    json!({"mode":"execute","task_ir":payload["task"]}),
                )
                .map_err(|error| error.to_string())?;
            wait_for_lifecycle(&agent, "accepting", 0).map_err(|error| error.to_string())?;
            let (status, receipt) = if response["ok"] == true {
                let mut receipt = prepared;
                receipt["status"] = json!("executed");
                receipt["result"] = response["result"].clone();
                ("200 OK", receipt)
            } else {
                ("422 Unprocessable Entity", response["error"].clone())
            };
            let body = serde_json::to_vec(&receipt).map_err(|error| error.to_string())?;
            let header = format!(
                "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            );
            stream
                .write_all(header.as_bytes())
                .map_err(|error| error.to_string())?;
            stream.write_all(&body).map_err(|error| error.to_string())?;
            replies.push(response);
        }
        Ok(replies)
    });
    let mut executor = ServiceHeadlessExecutor::try_new(&format!("http://{address}"))
        .map_err(|error| error.message)?;
    let blocked =
        execute_batch_with_executor(&batch(golden_task(), true), &mut executor, false, false);
    let first =
        execute_batch_with_executor(&batch(bar_task(0.01), false), &mut executor, false, false);
    let failure =
        execute_batch_with_executor(&batch(bar_task(0.0), true), &mut executor, false, false);
    let replay =
        execute_batch_with_executor(&batch(bar_task(0.01), false), &mut executor, false, false);
    let replies = server.join().map_err(|_| "HTTP adapter panicked")??;
    assert_eq!(replies.len(), 4);
    assert_eq!(blocked.status, "blocked");
    assert_eq!(blocked.executed_step_count, 0);
    assert_eq!(blocked.steps.len(), 1);
    assert_eq!(
        blocked.steps[0].result_preview["result"]["execution_readiness"]["required_action"],
        "attach_operator_package_runtime"
    );
    assert_eq!(failure.status, "failed");
    assert_eq!(failure.executed_step_count, 0);
    assert_eq!(failure.steps.len(), 1);
    assert!(
        failure.steps[0].result_preview["error"]
            .as_str()
            .unwrap()
            .contains("operator_task_solver_input_invalid")
    );
    for report in [&first, &replay] {
        assert_eq!(report.status, "ok");
        assert_eq!(report.executed_step_count, 1);
        let displacement = report.steps[0].result_preview["result"]["result"]["tip_displacement"]
            .as_f64()
            .unwrap();
        // Independent axial-bar closed form, not comparison with the same solver.
        assert!((displacement / (1000.0 / (210e9 * 0.01)) - 1.0).abs() < 1e-12);
    }
    assert_eq!(
        first.steps[0].result_preview,
        replay.steps[0].result_preview
    );
    Ok(())
}

#[test]
fn native_cli_blocked_task_returns_nonzero_and_persists_the_receipt() -> Result<(), Box<dyn Error>>
{
    let scratch = Scratch::new()?;
    let workflow = scratch.0.join("workflow.json");
    let output_path = scratch.0.join("run-report.json");
    fs::write(&workflow, serde_json::to_vec(&batch(golden_task(), true))?)?;
    let output = Command::new(env!("CARGO_BIN_EXE_kyuubiki-headless"))
        .args([
            "run",
            workflow.to_str().unwrap(),
            "--execute",
            "--executor",
            "mock",
            "--execution-posture",
            "preview",
            "--json",
            "--report-out",
            output_path.to_str().unwrap(),
        ])
        .output()?;
    assert!(!output.status.success());
    let receipt: Value = serde_json::from_slice(&output.stdout)?;
    assert_eq!(receipt["status"], "blocked");
    assert_eq!(receipt["executed_step_count"], 0);
    assert_eq!(receipt["steps"].as_array().unwrap().len(), 1);
    assert_eq!(
        serde_json::from_slice::<Value>(&fs::read(output_path)?)?,
        receipt
    );
    let message = String::from_utf8_lossy(&output.stderr);
    let error: Value = serde_json::from_str(&message)?;
    assert_eq!(error["error"]["code"], "headless_execution_blocked");
    assert_eq!(error["error"]["stage"], "execution");
    assert_eq!(error["error"]["retryable"], false);
    assert!(message.contains("headless execution blocked at step 1 (operator_task_execute)"));
    assert!(message.contains("attach_operator_package_runtime"));
    Ok(())
}

#[test]
fn native_cli_blocked_confirmation_is_not_success_but_dry_run_remains_planning()
-> Result<(), Box<dyn Error>> {
    let scratch = Scratch::new()?;
    let workflow = scratch.0.join("confirmation-workflow.json");
    let mut document = batch(golden_task(), true);
    document.steps[0].action = "project_delete".into();
    document.steps[0].risk = HeadlessRisk::Destructive;
    document.steps[0].payload = json!({"project_id":"review-required"});
    fs::write(&workflow, serde_json::to_vec(&document)?)?;
    let execute = Command::new(env!("CARGO_BIN_EXE_kyuubiki-headless"))
        .args([
            "run",
            workflow.to_str().unwrap(),
            "--execute",
            "--executor",
            "mock",
            "--execution-posture",
            "preview",
            "--json",
        ])
        .output()?;
    assert!(!execute.status.success());
    let report: Value = serde_json::from_slice(&execute.stdout)?;
    assert_eq!(report["status"], "blocked");
    assert_eq!(report["executed_step_count"], 0);
    assert_eq!(report["blocked_by_confirmation"]["index"], 1);
    assert_eq!(report["steps"].as_array().unwrap().len(), 1);
    let dry = Command::new(env!("CARGO_BIN_EXE_kyuubiki-headless"))
        .args(["run", workflow.to_str().unwrap(), "--json"])
        .output()?;
    assert!(dry.status.success());
    let plan: Value = serde_json::from_slice(&dry.stdout)?;
    assert_eq!(plan["mode"], "dry_run");
    assert_eq!(plan["status"], "blocked");
    Ok(())
}
