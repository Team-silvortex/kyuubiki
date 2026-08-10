use kyuubiki_protocol::{compute_operator_task_digest, validate_agent_solver_qualification_report};
use serde_json::{Value, json};
use std::error::Error;
use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const RPC_VERSION: u8 = 1;
const MAX_FRAME_BYTES: usize = 16 * 1024 * 1024;
const EXPECTED_TIP_DISPLACEMENT: f64 = 4.761_904_761_904_762e-7;
const TIP_DISPLACEMENT_TOLERANCE: f64 = 1.0e-12;

struct LiveAgent {
    child: Child,
    log_path: PathBuf,
    port: u16,
}

impl LiveAgent {
    fn start() -> Result<Self, Box<dyn Error>> {
        let port = reserve_port()?;
        let log_path =
            std::env::temp_dir().join(format!("kyuubiki-agent-task-ir-qualification-{port}.log"));
        let log = OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(&log_path)?;
        let child = Command::new(env!("CARGO_BIN_EXE_kyuubiki-cli"))
            .args(["agent", "--host", "127.0.0.1", "--port", &port.to_string()])
            .stdout(Stdio::from(log.try_clone()?))
            .stderr(Stdio::from(log))
            .spawn()?;
        let mut agent = Self {
            child,
            log_path,
            port,
        };
        agent.wait_until_ready(Duration::from_secs(30))?;
        Ok(agent)
    }

    fn wait_until_ready(&mut self, timeout: Duration) -> Result<(), Box<dyn Error>> {
        let started = Instant::now();
        while started.elapsed() < timeout {
            if TcpStream::connect(("127.0.0.1", self.port)).is_ok() {
                return Ok(());
            }
            if let Some(status) = self.child.try_wait()? {
                return Err(format!(
                    "agent exited with {status}; log:\n{}",
                    fs::read_to_string(&self.log_path).unwrap_or_default()
                )
                .into());
            }
            thread::sleep(Duration::from_millis(50));
        }
        Err(format!("agent did not listen on port {}", self.port).into())
    }

    fn request(&self, id: &str, method: &str, params: Value) -> Result<Value, Box<dyn Error>> {
        let mut stream = TcpStream::connect(("127.0.0.1", self.port))?;
        stream.set_read_timeout(Some(Duration::from_secs(30)))?;
        stream.set_write_timeout(Some(Duration::from_secs(30)))?;
        let payload = serde_json::to_vec(&json!({
            "rpc_version": RPC_VERSION,
            "id": id,
            "method": method,
            "params": params
        }))?;
        let frame_length = u32::try_from(payload.len())?;
        stream.write_all(&frame_length.to_be_bytes())?;
        stream.write_all(&payload)?;

        loop {
            let response = read_json_frame(&mut stream)?;
            if response.get("ok").is_some() {
                return Ok(response);
            }
        }
    }
}

impl Drop for LiveAgent {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = fs::remove_file(&self.log_path);
    }
}

fn reserve_port() -> Result<u16, Box<dyn Error>> {
    let listener = TcpListener::bind(("127.0.0.1", 0))?;
    Ok(listener.local_addr()?.port())
}

fn read_json_frame(stream: &mut TcpStream) -> Result<Value, Box<dyn Error>> {
    let mut header = [0_u8; 4];
    stream.read_exact(&mut header)?;
    let length = u32::from_be_bytes(header) as usize;
    if length > MAX_FRAME_BYTES {
        return Err(format!("agent frame exceeds {MAX_FRAME_BYTES} bytes").into());
    }
    let mut payload = vec![0_u8; length];
    stream.read_exact(&mut payload)?;
    Ok(serde_json::from_slice(&payload)?)
}

fn executable_solver_task_ir() -> Value {
    let mut task = json!({
        "schema_version": "kyuubiki.operator-task-ir/v1",
        "task_id": "agent-qualification-bar-1d",
        "operator": {
            "id": "solve.bar_1d",
            "family": "mechanical",
            "kind": "solver"
        },
        "descriptor_authoring": {
            "schema_version": "kyuubiki.operator-descriptor-authoring/v1",
            "mode": "rust_native",
            "runtime": "rust",
            "source": "agent_solver_task_ir_qualification",
            "hot_reloadable": false,
            "execution_language": "language_neutral"
        },
        "node": {},
        "input_artifact": {
            "length": 1.0,
            "area": 0.01,
            "youngs_modulus": 210000000000.0,
            "elements": 1,
            "tip_force": 1000.0
        },
        "config": { "qualification_case": "closed_form_axial_bar" },
        "execution_program": {
            "schema_version": "kyuubiki.operator-execution-program/v1",
            "program_id": "solve.bar_1d",
            "program_family": "mechanical",
            "program_kind": "solver",
            "operator_category_id": null,
            "package_ref": null,
            "package_version": "library-managed",
            "package_integrity": null,
            "runtime_protocol": "kyuubiki.solver-rpc/v1",
            "abi": {
                "kind": "solver_rpc",
                "input_encoding": "json",
                "output_encoding": "json"
            },
            "entrypoint": {
                "kind": "solver_method",
                "name": "solve_bar_1d",
                "operator_kind": "solver"
            },
            "bindings": {
                "input_artifact": "task.input_artifact",
                "config": "task.config",
                "output_artifact": "task.output_artifact"
            },
            "node_binding": { "node_id": null, "input_ports": [], "output_ports": [] }
        },
        "dataset_contract": {},
        "orchestration_context": {},
        "runtime_hints": {
            "authority_mode": "agent_local",
            "execution_mode": "agent_native",
            "cache_scope": "none",
            "agent_fetchable": false,
            "operator_kind": "solver"
        }
    });
    let digest = compute_operator_task_digest(&task).expect("qualification task should digest");
    task["integrity"] = json!({ "task_digest": digest });
    task
}

fn assert_success<'a>(response: &'a Value, id: &str) -> &'a Value {
    assert_eq!(response["id"], id);
    assert_eq!(response["ok"], true, "response: {response}");
    response.get("result").expect("successful result")
}

fn assert_solver_result(result: &Value) -> f64 {
    let displacement = result["result"]["tip_displacement"]
        .as_f64()
        .expect("solver result must expose tip_displacement");
    assert!((displacement - EXPECTED_TIP_DISPLACEMENT).abs() <= TIP_DISPLACEMENT_TOLERANCE);
    displacement
}

fn result_assertion(actual: f64) -> Value {
    let absolute_error = (actual - EXPECTED_TIP_DISPLACEMENT).abs();
    json!({
        "metric": "tip_displacement",
        "expected": EXPECTED_TIP_DISPLACEMENT,
        "actual": actual,
        "absolute_error": absolute_error,
        "tolerance": TIP_DISPLACEMENT_TOLERANCE,
        "passed": absolute_error <= TIP_DISPLACEMENT_TOLERANCE
    })
}

fn qualification_output_path() -> PathBuf {
    if let Some(path) = std::env::var_os("KYUUBIKI_AGENT_QUALIFICATION_OUTPUT") {
        let path = PathBuf::from(path);
        return if path.is_absolute() {
            path
        } else {
            repo_root().join(path)
        };
    }
    if let Some(output_dir) = std::env::var_os("OUTPUT_DIR") {
        let output_dir = PathBuf::from(output_dir);
        let output_dir = if output_dir.is_absolute() {
            output_dir
        } else {
            repo_root().join(output_dir)
        };
        return output_dir.join("agent-task-ir-qualification.json");
    }
    repo_root()
        .join("tmp")
        .join("agent-task-ir-qualification.json")
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../..")
}

#[test]
fn agent_executes_solver_rejects_tampering_and_recovers_over_tcp() -> Result<(), Box<dyn Error>> {
    let agent = LiveAgent::start()?;
    let initial = agent.request("qualification-describe-before", "describe_agent", json!({}))?;
    let initial_descriptor = assert_success(&initial, "qualification-describe-before");
    assert!(
        initial_descriptor["protocol"]["methods"]
            .as_array()
            .is_some_and(|methods| methods
                .iter()
                .any(|method| method == "run_operator_task_ir"))
    );

    let task = executable_solver_task_ir();
    let first = agent.request(
        "qualification-valid-before",
        "run_operator_task_ir",
        json!({ "mode": "execute", "task_ir": task.clone() }),
    )?;
    let first_result = assert_success(&first, "qualification-valid-before");
    assert_eq!(first_result["operator_task_ir_status"], "executed");
    assert_eq!(
        first_result["execution_runtime_status"],
        "agent_engine_solver_executed"
    );
    assert_eq!(
        first_result["solver_execution_capability"]["accepted"],
        true
    );
    assert_eq!(
        first_result["solver_execution_capability"]["runtime_protocol"],
        "kyuubiki.solver-rpc/v1"
    );
    assert_eq!(
        first_result["validation_receipt"]["schema_version"],
        "kyuubiki.agent-operator-task-validation/v1"
    );
    assert_eq!(first_result["validation_receipt"]["digest_verified"], true);
    assert_eq!(
        first_result["validation_receipt"]["validation_status"],
        "accepted"
    );
    assert_eq!(
        first_result["validation_receipt"]["blocked_reason"],
        Value::Null
    );
    assert_eq!(
        first_result["provenance_receipt"]["schema_version"],
        "kyuubiki.agent-operator-task-provenance/v1"
    );
    let first_displacement = assert_solver_result(first_result);

    let mut unsupported_task = task.clone();
    unsupported_task["operator"]["id"] = json!("solve.thermal_bar_1d");
    unsupported_task["execution_program"]["program_id"] = json!("solve.thermal_bar_1d");
    unsupported_task["execution_program"]["entrypoint"]["name"] = json!("solve_thermal_bar_1d");
    let unsupported_digest = compute_operator_task_digest(&unsupported_task)?;
    unsupported_task["integrity"]["task_digest"] = json!(unsupported_digest);
    let unsupported = agent.request(
        "qualification-unsupported-solver",
        "run_operator_task_ir",
        json!({ "mode": "execute", "task_ir": unsupported_task }),
    )?;
    assert_eq!(unsupported["ok"], false);
    assert_eq!(
        unsupported["error"]["code"],
        "operator_task_solver_capability_rejected"
    );
    let unsupported_failure_receipt =
        &unsupported["error"]["details"]["operator_task_failure_receipt"];
    assert_eq!(
        unsupported_failure_receipt["failure_stage"],
        "check_solver_capability"
    );
    assert_eq!(
        unsupported_failure_receipt["recovery"]["required_action"],
        "select_advertised_solver_operator"
    );

    let mut tampered_task = task.clone();
    tampered_task["config"]["qualification_case"] = json!("tampered");
    let rejected = agent.request(
        "qualification-tampered",
        "run_operator_task_ir",
        json!({ "mode": "execute", "task_ir": tampered_task }),
    )?;
    assert_eq!(rejected["ok"], false);
    assert_eq!(rejected["error"]["code"], "operator_task_digest_mismatch");
    let failure_receipt = &rejected["error"]["details"]["operator_task_failure_receipt"];
    assert_eq!(
        failure_receipt["schema_version"],
        "kyuubiki.agent-operator-task-failure/v1"
    );
    assert_eq!(failure_receipt["failure_stage"], "verify_digest");
    assert_eq!(
        failure_receipt["recovery"]["safe_to_continue_other_tasks"],
        true
    );

    let recovered = agent.request(
        "qualification-valid-after",
        "run_operator_task_ir",
        json!({ "mode": "execute", "task_ir": task }),
    )?;
    let recovered_result = assert_success(&recovered, "qualification-valid-after");
    assert_eq!(recovered_result["operator_task_ir_status"], "executed");
    let recovered_displacement = assert_solver_result(recovered_result);

    let final_state = agent.request("qualification-describe-after", "describe_agent", json!({}))?;
    let final_descriptor = assert_success(&final_state, "qualification-describe-after");
    assert_eq!(final_descriptor["watchdog"]["active_execution_count"], 0);
    assert!(final_descriptor["watchdog"]["recent_failure_count"].as_u64() >= Some(2));
    assert!(
        final_descriptor["watchdog"]["recent_failures"]
            .as_array()
            .is_some_and(|failures| failures.iter().any(|failure| {
                failure["request_id"] == "qualification-unsupported-solver"
                    && failure["reason_code"] == "operator_task_solver_capability_rejected"
            }))
    );
    assert!(
        final_descriptor["watchdog"]["recent_failures"]
            .as_array()
            .is_some_and(|failures| failures.iter().any(|failure| {
                failure["request_id"] == "qualification-tampered"
                    && failure["reason_code"] == "operator_task_digest_mismatch"
            }))
    );

    let output_path = qualification_output_path();
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)?;
    }
    let artifact = json!({
        "schema_version": "kyuubiki.agent-solver-qualification/v2",
        "generated_at_unix_ms": SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis(),
        "status": "passed",
        "transport": "tcp_framed_json",
        "rpc_version": RPC_VERSION,
        "operator_id": recovered_result["operator_id"],
        "program_kind": recovered_result["program_kind"],
        "runtime_protocol": recovered_result["runtime_protocol"],
        "task_digest": recovered_result["task_digest"],
        "stages": {
            "initial_execution": {
                "status": first_result["operator_task_ir_status"],
                "solver_execution_capability": first_result["solver_execution_capability"],
                "validation_receipt": first_result["validation_receipt"],
                "provenance_receipt": first_result["provenance_receipt"],
                "result_assertion": result_assertion(first_displacement)
            },
            "unsupported_solver_rejection": {
                "reason_code": unsupported["error"]["code"],
                "failure_receipt": unsupported_failure_receipt
            },
            "tamper_rejection": {
                "reason_code": rejected["error"]["code"],
                "failure_receipt": failure_receipt
            },
            "recovery_execution": {
                "status": recovered_result["operator_task_ir_status"],
                "solver_execution_capability": recovered_result["solver_execution_capability"],
                "validation_receipt": recovered_result["validation_receipt"],
                "provenance_receipt": recovered_result["provenance_receipt"],
                "result_assertion": result_assertion(recovered_displacement)
            }
        },
        "watchdog": final_descriptor["watchdog"]
    });
    validate_agent_solver_qualification_report(&artifact).map_err(|errors| {
        std::io::Error::other(format!(
            "agent solver qualification report is invalid: {}",
            errors.join("; ")
        ))
    })?;
    fs::write(
        &output_path,
        format!("{}\n", serde_json::to_string_pretty(&artifact)?),
    )?;
    println!("agent TaskIR qualification: {}", output_path.display());
    Ok(())
}
