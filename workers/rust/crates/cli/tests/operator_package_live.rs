use kyuubiki_protocol::compute_operator_task_digest;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::error::Error;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

const MAX_FRAME_BYTES: usize = 16 * 1024 * 1024;

struct LiveAgent {
    child: Child,
    log_path: PathBuf,
    port: u16,
}

impl LiveAgent {
    fn start(packages_root: &Path) -> Result<Self, Box<dyn Error>> {
        let port = reserve_port()?;
        let log_path =
            std::env::temp_dir().join(format!("kyuubiki-operator-package-live-{port}.log"));
        let log = OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(&log_path)?;
        let child = Command::new(env!("CARGO_BIN_EXE_kyuubiki-cli"))
            .args([
                "agent",
                "--host",
                "127.0.0.1",
                "--port",
                &port.to_string(),
                "--agent-id",
                "operator-package-live-agent",
                "--operator-package-host-id",
                "operator-package-live-host",
                "--operator-packages-root",
                packages_root.to_str().ok_or("packages root is not utf-8")?,
                "--operator-activated-package-count",
                "1",
            ])
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
            "rpc_version": 1,
            "id": id,
            "method": method,
            "params": params
        }))?;
        stream.write_all(&u32::try_from(payload.len())?.to_be_bytes())?;
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

#[test]
#[ignore = "requires prebuilt operator template cdylib"]
fn live_agent_loads_executes_rejects_tamper_and_recovers() {
    let packages_root = template_packages_root();
    let library = packages_root
        .join("operator-crate-template/target/debug")
        .join(format!(
            "{}kyuubiki_operator_template{}",
            std::env::consts::DLL_PREFIX,
            std::env::consts::DLL_SUFFIX
        ));
    assert!(
        library.is_file(),
        "prebuilt cdylib missing: {}",
        library.display()
    );
    let entrypoint_sha256 = sha256_file(&library).expect("hash template cdylib");
    let agent = LiveAgent::start(&packages_root).expect("start package-enabled Agent");

    let descriptor = agent
        .request("describe", "describe_agent", json!({}))
        .expect("describe Agent");
    assert_eq!(
        descriptor["result"]["operator_package_runtime"]["attachment"]["activated_package_count"],
        1
    );

    let task = external_operator_task(&entrypoint_sha256);
    let executed = agent
        .request(
            "execute-valid",
            "run_operator_task_ir",
            json!({ "mode": "execute", "task_ir": task }),
        )
        .expect("execute external operator");
    assert_eq!(executed["ok"], true, "response: {executed}");
    let result = &executed["result"];
    assert_eq!(
        result["execution_runtime_status"],
        "external_operator_package_executed"
    );
    assert_eq!(
        result["operator_package_execution"]["integrity_verified"],
        true
    );
    assert_eq!(
        result["operator_package_execution"]["entrypoint_sha256"],
        entrypoint_sha256
    );
    assert_eq!(result["result"]["summary"]["count"], 3);
    assert_eq!(result["result"]["summary"]["sum"], 14.0);

    let mut tampered = external_operator_task(&"0".repeat(64));
    refresh_task_digest(&mut tampered);
    let rejected = agent
        .request(
            "execute-tampered",
            "run_operator_task_ir",
            json!({ "mode": "execute", "task_ir": tampered }),
        )
        .expect("reject tampered package identity");
    assert_eq!(rejected["ok"], false);
    assert_eq!(
        rejected["error"]["code"],
        "operator_package_identity_mismatch"
    );
    assert_eq!(
        rejected["error"]["details"]["operator_task_failure_receipt"]["failure_stage"],
        "verify_package_integrity"
    );

    let recovered = agent
        .request(
            "execute-recovered",
            "run_operator_task_ir",
            json!({
                "mode": "execute",
                "task_ir": external_operator_task(&entrypoint_sha256)
            }),
        )
        .expect("execute after isolated failure");
    assert_eq!(recovered["ok"], true, "response: {recovered}");
    assert_eq!(recovered["result"]["result"]["summary"]["max"], 8.0);
}

fn external_operator_task(entrypoint_sha256: &str) -> Value {
    let mut task = json!({
        "schema_version": "kyuubiki.operator-task-ir/v1",
        "task_id": "operator-package-live-template-summary",
        "operator": {
            "id": "extract.template_summary",
            "family": "template_summary",
            "kind": "extract"
        },
        "descriptor_authoring": {
            "schema_version": "kyuubiki.operator-descriptor-authoring/v1",
            "mode": "rust_native",
            "runtime": "rust",
            "source": "operator_package_live",
            "hot_reloadable": false,
            "execution_language": "language_neutral"
        },
        "node": {},
        "input_artifact": { "values": [2.0, 4.0, 8.0] },
        "config": { "qualification": "live_agent_dynamic_host" },
        "execution_program": {
            "schema_version": "kyuubiki.operator-execution-program/v1",
            "program_id": "extract.template_summary",
            "program_family": "template_summary",
            "program_kind": "extract",
            "operator_category_id": null,
            "package_ref": "bundle://operator.template.summary",
            "package_version": "0.1.0",
            "package_integrity": {
                "algorithm": "sha256",
                "digest": entrypoint_sha256
            },
            "runtime_protocol": "kyuubiki.operator-execution/v1",
            "abi": {
                "kind": "operator_task",
                "input_encoding": "json",
                "output_encoding": "json"
            },
            "entrypoint": {
                "kind": "operator_id",
                "name": "extract.template_summary",
                "operator_kind": "extract"
            },
            "bindings": {
                "input_artifact": "task.input_artifact",
                "config": "task.config",
                "output_artifact": "task.output_artifact"
            },
            "node_binding": { "node_id": null, "input_ports": [], "output_ports": [] }
        },
        "dataset_contract": {},
        "orchestration_context": { "project_id": "operator-sdk-qualification" },
        "runtime_hints": {
            "authority_mode": "agent_local",
            "execution_mode": "local_bundle",
            "cache_scope": "agent",
            "agent_fetchable": false,
            "operator_kind": "extract",
            "package_ref": "bundle://operator.template.summary",
            "package_version": "0.1.0"
        }
    });
    refresh_task_digest(&mut task);
    task
}

fn refresh_task_digest(task: &mut Value) {
    let digest = compute_operator_task_digest(task).expect("digest external operator task");
    task["integrity"] = json!({ "task_digest": digest });
}

fn template_packages_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../templates")
        .canonicalize()
        .expect("template packages root")
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
        return Err(format!("Agent frame exceeds {MAX_FRAME_BYTES} bytes").into());
    }
    let mut payload = vec![0_u8; length];
    stream.read_exact(&mut payload)?;
    Ok(serde_json::from_slice(&payload)?)
}

fn sha256_file(path: &Path) -> Result<String, Box<dyn Error>> {
    let mut file = File::open(path)?;
    let mut digest = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        digest.update(&buffer[..read]);
    }
    Ok(format!("{:x}", digest.finalize()))
}
