use kyuubiki_protocol::compute_operator_task_digest;
use serde_json::{Value, json};
use std::error::Error;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex, mpsc};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

struct LiveServer {
    child: Child,
    stdout_log: Arc<Mutex<String>>,
    stderr_log: Arc<Mutex<String>>,
    port: u16,
}

impl LiveServer {
    fn logs(&self) -> String {
        let stdout = self.stdout_log.lock().expect("stdout log").clone();
        let stderr = self.stderr_log.lock().expect("stderr log").clone();
        format!("stdout:\n{stdout}\nstderr:\n{stderr}")
    }
}

impl Drop for LiveServer {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../..")
        .canonicalize()
        .expect("repo root")
}

fn write_temp_json(prefix: &str, payload: &Value) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("kyuubiki-operator-task-live-{unique}"));
    fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join(format!("{prefix}.json"));
    fs::write(
        &path,
        serde_json::to_vec_pretty(payload).expect("serialize json"),
    )
    .expect("write temp json");
    path
}

fn spawn_log_reader(
    mut reader: BufReader<impl std::io::Read + Send + 'static>,
    log: Arc<Mutex<String>>,
    ready_tx: Option<mpsc::Sender<u16>>,
) {
    thread::spawn(move || {
        let mut line = String::new();
        loop {
            line.clear();
            match reader.read_line(&mut line) {
                Ok(0) => break,
                Ok(_) => {
                    if let Ok(mut current) = log.lock() {
                        current.push_str(&line);
                    }
                    if let Some(tx) = ready_tx.as_ref() {
                        if let Some(port) = line
                            .trim()
                            .strip_prefix("HEADLESS_LIVE_SERVER_READY ")
                            .and_then(|value| value.parse::<u16>().ok())
                        {
                            let _ = tx.send(port);
                        }
                    }
                }
                Err(_) => break,
            }
        }
    });
}

fn start_live_server() -> Result<LiveServer, Box<dyn Error>> {
    let web_root = repo_root().join("apps/web");
    let server_script = web_root.join("test/support/headless_live_server.exs");
    let mut child = Command::new("mix")
        .arg("run")
        .arg(server_script)
        .current_dir(&web_root)
        .env("MIX_ENV", "test")
        .env("KYUUBIKI_STORAGE_BACKEND", "sqlite")
        .env("KYUUBIKI_DEPLOYMENT_MODE", "local")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    let stdout = child.stdout.take().expect("stdout pipe");
    let stderr = child.stderr.take().expect("stderr pipe");
    let stdout_log = Arc::new(Mutex::new(String::new()));
    let stderr_log = Arc::new(Mutex::new(String::new()));
    let (ready_tx, ready_rx) = mpsc::channel();
    spawn_log_reader(
        BufReader::new(stdout),
        Arc::clone(&stdout_log),
        Some(ready_tx),
    );
    spawn_log_reader(BufReader::new(stderr), Arc::clone(&stderr_log), None);
    let port = ready_rx.recv_timeout(Duration::from_secs(60))?;
    Ok(LiveServer {
        child,
        stdout_log,
        stderr_log,
        port,
    })
}

fn run_headless_command(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_kyuubiki-headless"))
        .args(args)
        .output()
        .expect("run kyuubiki-headless")
}

fn executable_operator_task_ir() -> Value {
    let mut task = json!({
        "schema_version": "kyuubiki.operator-task-ir/v1",
        "task_id": "operator-task-live-thermal-shock",
        "operator": {
            "id": "transform.evaluate_material_thermal_shock",
            "family": "material_margin",
            "kind": "transform"
        },
        "descriptor_authoring": {
            "schema_version": "kyuubiki.operator-descriptor-authoring/v1",
            "mode": "rust_native",
            "runtime": "rust",
            "source": "operator_task_live",
            "hot_reloadable": false,
            "execution_language": "language_neutral"
        },
        "node": {},
        "input_artifact": {
            "temperature_delta": 160.0,
            "thermal_expansion": 1.2e-5,
            "youngs_modulus": 70000000000.0,
            "poisson_ratio": 0.33,
            "yield_strength": 320000000.0
        },
        "config": { "constraint_factor": 0.7 },
        "execution_program": {
            "schema_version": "kyuubiki.operator-execution-program/v1",
            "program_id": "transform.evaluate_material_thermal_shock",
            "program_family": "material_margin",
            "program_kind": "transform",
            "operator_category_id": null,
            "package_ref": null,
            "package_version": "library-managed",
            "package_integrity": null,
            "runtime_protocol": "kyuubiki.operator-execution/v1",
            "abi": { "kind": "operator_task", "input_encoding": "json", "output_encoding": "json" },
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
            "source_ref": null,
            "package_ref": null,
            "package_version": "library-managed",
            "placement_tags": [],
            "required_capabilities": [],
            "cache_scope": "job",
            "agent_fetchable": true,
            "operator_kind": "transform"
        }
    });
    let digest = compute_operator_task_digest(&task).expect("digest task");
    task["integrity"] = json!({ "task_digest": digest });
    task
}

#[test]
fn rust_headless_cli_executes_operator_task_through_control_plane() {
    let server = start_live_server().expect("start live server");
    let base_url = format!("http://127.0.0.1:{}", server.port);
    let workflow_path = write_temp_json(
        "operator-task-execute",
        &json!({
            "schema_version": "kyuubiki.headless-workflow/v1",
            "exported_at": "2026-07-01T00:00:00Z",
            "language": "en",
            "workflow": {
                "id": "workflow.live.operator-task-execute",
                "steps": [{
                    "action": "operator_task_execute",
                    "payload": { "task": executable_operator_task_ir() }
                }]
            }
        }),
    );
    let output = run_headless_command(&[
        "run",
        workflow_path.to_str().expect("workflow path"),
        "--json",
        "--execute",
        "--executor",
        "service",
        "--api-base-url",
        &base_url,
    ]);
    assert!(
        output.status.success(),
        "command failed\nstdout:\n{}\nstderr:\n{}\nserver:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
        server.logs()
    );
    let payload: Value = serde_json::from_slice(&output.stdout).expect("parse output");
    assert_eq!(payload["status"], "ok", "payload: {payload}");
    assert_eq!(payload["steps"][0]["result_preview"]["status"], "executed");
    assert!(
        payload["steps"][0]["result_preview"]["result"]["material_thermal_shock_status"]
            .as_str()
            .is_some()
    );
    let _ = fs::remove_file(workflow_path);
}
