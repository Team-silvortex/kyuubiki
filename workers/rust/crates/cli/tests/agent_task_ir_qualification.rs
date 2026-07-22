use kyuubiki_protocol::compute_operator_task_digest;
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

fn executable_task_ir() -> Value {
    let mut task = json!({
        "schema_version": "kyuubiki.operator-task-ir/v1",
        "task_id": "agent-qualification-thermal-shock",
        "operator": {
            "id": "transform.evaluate_material_thermal_shock",
            "family": "material_thermal_shock",
            "kind": "transform"
        },
        "descriptor_authoring": {
            "schema_version": "kyuubiki.operator-descriptor-authoring/v1",
            "mode": "rust_native",
            "runtime": "rust",
            "source": "agent_task_ir_qualification",
            "hot_reloadable": false,
            "execution_language": "language_neutral"
        },
        "node": {},
        "input_artifact": {
            "candidates": {
                "alloy": {
                    "temperature_delta": 160.0,
                    "thermal_expansion": 1.2e-5,
                    "youngs_modulus": 70000000000.0,
                    "poisson_ratio": 0.33,
                    "yield_strength": 320000000.0
                },
                "ceramic": {
                    "temperature_delta": 160.0,
                    "thermal_expansion": 8.0e-6,
                    "youngs_modulus": 300000000000.0,
                    "poisson_ratio": 0.22,
                    "tensile_strength": 180000000.0,
                    "fracture_toughness": 3000000.0,
                    "flaw_size": 0.001
                }
            }
        },
        "config": { "constraint_factor": 0.7 },
        "execution_program": {
            "schema_version": "kyuubiki.operator-execution-program/v1",
            "program_id": "transform.evaluate_material_thermal_shock",
            "program_family": "material_thermal_shock",
            "program_kind": "transform",
            "operator_category_id": null,
            "package_ref": null,
            "package_version": "library-managed",
            "package_integrity": null,
            "runtime_protocol": "kyuubiki.operator-execution/v1",
            "abi": {
                "kind": "operator_task",
                "input_encoding": "json",
                "output_encoding": "json"
            },
            "entrypoint": {
                "kind": "operator_id",
                "name": "transform.evaluate_material_thermal_shock",
                "operator_kind": "transform"
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
            "authority_mode": "central_operator_library",
            "execution_mode": "orchestra_fetch",
            "cache_scope": "job",
            "agent_fetchable": true,
            "operator_kind": "transform"
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

fn qualification_output_path() -> PathBuf {
    if let Some(path) = std::env::var_os("KYUUBIKI_AGENT_QUALIFICATION_OUTPUT") {
        return PathBuf::from(path);
    }
    if let Some(output_dir) = std::env::var_os("OUTPUT_DIR") {
        return PathBuf::from(output_dir).join("agent-task-ir-qualification.json");
    }
    repo_root()
        .join("tmp")
        .join("agent-task-ir-qualification.json")
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../..")
}

#[test]
fn agent_executes_rejects_tampering_and_recovers_over_tcp() -> Result<(), Box<dyn Error>> {
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

    let task = executable_task_ir();
    let first = agent.request(
        "qualification-valid-before",
        "run_operator_task_ir",
        json!({ "mode": "execute", "task_ir": task.clone() }),
    )?;
    let first_result = assert_success(&first, "qualification-valid-before");
    assert_eq!(first_result["operator_task_ir_status"], "executed");
    assert_eq!(
        first_result["validation_receipt"]["schema_version"],
        "kyuubiki.agent-operator-task-validation/v1"
    );
    assert_eq!(first_result["validation_receipt"]["digest_verified"], true);
    assert_eq!(
        first_result["provenance_receipt"]["schema_version"],
        "kyuubiki.agent-operator-task-provenance/v1"
    );
    assert_eq!(
        first_result["result"]["material_thermal_shock_best_candidate_id"],
        "alloy"
    );

    let mut tampered_task = task.clone();
    tampered_task["config"]["constraint_factor"] = json!(0.9);
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
    assert_eq!(
        recovered_result["result"]["material_thermal_shock_pass_count"],
        1
    );

    let final_state = agent.request("qualification-describe-after", "describe_agent", json!({}))?;
    let final_descriptor = assert_success(&final_state, "qualification-describe-after");
    assert_eq!(final_descriptor["watchdog"]["active_execution_count"], 0);
    assert!(final_descriptor["watchdog"]["recent_failure_count"].as_u64() >= Some(1));
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
        "schema_version": "kyuubiki.agent-solver-qualification/v1",
        "generated_at_unix_ms": SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis(),
        "status": "passed",
        "transport": "tcp_framed_json",
        "rpc_version": RPC_VERSION,
        "operator_id": recovered_result["operator_id"],
        "task_digest": recovered_result["task_digest"],
        "stages": {
            "initial_execution": {
                "status": first_result["operator_task_ir_status"],
                "validation_receipt": first_result["validation_receipt"],
                "provenance_receipt": first_result["provenance_receipt"]
            },
            "tamper_rejection": {
                "reason_code": rejected["error"]["code"],
                "failure_receipt": failure_receipt
            },
            "recovery_execution": {
                "status": recovered_result["operator_task_ir_status"],
                "validation_receipt": recovered_result["validation_receipt"],
                "provenance_receipt": recovered_result["provenance_receipt"]
            }
        },
        "watchdog": final_descriptor["watchdog"]
    });
    fs::write(
        &output_path,
        format!("{}\n", serde_json::to_string_pretty(&artifact)?),
    )?;
    println!("agent TaskIR qualification: {}", output_path.display());
    Ok(())
}
