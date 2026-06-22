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
    let dir = std::env::temp_dir().join(format!("kyuubiki-rust-headless-live-{unique}"));
    fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join(format!("{prefix}.json"));
    fs::write(
        &path,
        format!(
            "{}\n",
            serde_json::to_string_pretty(payload).expect("serialize temp json")
        ),
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
    let root = repo_root();
    let web_root = root.join("apps/web");
    let server_script = web_root.join("test/support/headless_live_server.exs");

    let mut child = Command::new("mix")
        .arg("run")
        .arg(server_script)
        .current_dir(&web_root)
        .env("MIX_ENV", "test")
        .env("KYUUBIKI_STORAGE_BACKEND", "sqlite")
        .env("KYUUBIKI_DEPLOYMENT_MODE", "local")
        .env("KYUUBIKI_HEADLESS_LIVE_SCENARIO", "electrostatic_quad_summary")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    let stdout = child.stdout.take().expect("stdout pipe");
    let stderr = child.stderr.take().expect("stderr pipe");
    let stdout_log = Arc::new(Mutex::new(String::new()));
    let stderr_log = Arc::new(Mutex::new(String::new()));
    let (ready_tx, ready_rx) = mpsc::channel();

    spawn_log_reader(BufReader::new(stdout), Arc::clone(&stdout_log), Some(ready_tx));
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

fn assert_command_ok(output: &std::process::Output, logs: &str) {
    assert!(
        output.status.success(),
        "command failed\nstdout:\n{}\nstderr:\n{}\nserver:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
        logs
    );
}

fn parse_json_output(output: &std::process::Output) -> Value {
    serde_json::from_slice(&output.stdout).expect("parse command json output")
}

fn electrostatic_plane_quad_input_artifacts() -> Value {
    json!({
        "electrostatic_model": {
            "nodes": [
                { "id": "n0", "x": 0.0, "y": 0.0, "fix_potential": true, "potential": 10.0, "charge_density": 0.0 },
                { "id": "n1", "x": 1.0, "y": 0.0, "fix_potential": true, "potential": 0.0, "charge_density": 0.0 },
                { "id": "n2", "x": 1.0, "y": 1.0, "fix_potential": true, "potential": 0.0, "charge_density": 0.0 },
                { "id": "n3", "x": 0.0, "y": 1.0, "fix_potential": true, "potential": 10.0, "charge_density": 0.0 }
            ],
            "elements": [
                {
                    "id": "epq0",
                    "node_i": 0,
                    "node_j": 1,
                    "node_k": 2,
                    "node_l": 3,
                    "thickness": 0.05,
                    "permittivity": 2.0
                }
            ]
        }
    })
}

#[test]
fn rust_headless_cli_executes_live_service_health_and_workflow_submit() {
    let server = start_live_server().expect("start live server");
    let base_url = format!("http://127.0.0.1:{}", server.port);

    let health_path = write_temp_json(
        "service-health",
        &json!({
            "schema_version": "kyuubiki.headless-workflow/v1",
            "exported_at": "2026-06-22T00:00:00Z",
            "language": "en",
            "workflow": {
                "id": "workflow.live.service-health",
                "steps": [
                    { "action": "service_health", "payload": {} }
                ]
            }
        }),
    );

    let health_output = run_headless_command(&[
        "run",
        health_path.to_str().expect("health path"),
        "--json",
        "--execute",
        "--executor",
        "service",
        "--api-base-url",
        &base_url,
    ]);
    assert_command_ok(&health_output, &server.logs());
    let health_payload = parse_json_output(&health_output);
    assert_eq!(health_payload["status"], "ok");
    assert_eq!(
        health_payload["steps"][0]["result_preview"]["service"],
        "kyuubiki-orchestrator"
    );
    assert_eq!(health_payload["steps"][0]["result_preview"]["status"], "ok");

    let workflow_path = write_temp_json(
        "workflow-submit",
        &json!({
            "schema_version": "kyuubiki.headless-workflow/v1",
            "exported_at": "2026-06-22T00:00:00Z",
            "language": "en",
            "workflow": {
                "id": "workflow.live.electrostatic-plane-quad",
                "steps": [
                    {
                        "action": "workflow_submit_catalog",
                        "payload": {
                            "workflow_id": "workflow.electrostatic-plane-quad-2d",
                            "input_artifacts": electrostatic_plane_quad_input_artifacts()
                        }
                    },
                    {
                        "action": "job_wait",
                        "payload": {
                            "job_id": "{{steps.1.result.job_id}}",
                            "interval_ms": 20,
                            "timeout_ms": 5000
                        }
                    },
                    {
                        "action": "result_fetch",
                        "payload": {
                            "job_id": "{{steps.1.result.job_id}}"
                        }
                    }
                ]
            }
        }),
    );

    let workflow_output = run_headless_command(&[
        "run",
        workflow_path.to_str().expect("workflow path"),
        "--json",
        "--execute",
        "--executor",
        "service",
        "--api-base-url",
        &base_url,
    ]);
    assert_command_ok(&workflow_output, &server.logs());
    let workflow_payload = parse_json_output(&workflow_output);
    assert_eq!(workflow_payload["status"], "ok");
    assert_eq!(workflow_payload["executed_step_count"], 3);
    assert!(
        workflow_payload["steps"][0]["result_preview"]["job_id"]
            .as_str()
            .is_some_and(|value| !value.is_empty())
    );
    assert_eq!(
        workflow_payload["steps"][2]["result_preview"]["result"]["workflow_id"],
        "workflow.electrostatic-plane-quad-2d"
    );

    let _ = fs::remove_file(health_path);
    let _ = fs::remove_file(workflow_path);
}
