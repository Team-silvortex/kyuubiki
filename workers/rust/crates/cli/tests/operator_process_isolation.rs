#![cfg(unix)]
mod support;
use kyuubiki_installer::{install_operator_package_into, project_operator_in};
use kyuubiki_protocol::{
    ProjectOperatorOperation, ProjectOperatorPin, ProjectOperatorRequest,
    build_project_operator_task,
};
use serde_json::{Value, json};
use std::{
    fs,
    io::Read,
    net::{TcpListener, TcpStream},
    path::PathBuf,
    process::{Command, Stdio},
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use support::operator_package::{LiveAgent, template_package_root};

static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(0);

struct Fixture {
    root: PathBuf,
    pin: ProjectOperatorPin,
}
impl Fixture {
    fn new(constructor: Option<&str>) -> Self {
        let root = std::env::temp_dir().join(format!(
            "kyuubiki-process-isolation-{}-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        let source = root.join("source");
        fs::create_dir_all(&source).unwrap();
        let file = format!("fault{}", std::env::consts::DLL_SUFFIX);
        let mut compiler =
            Command::new(std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into()));
        compiler
            .args(["--edition=2024", "--crate-type=cdylib", "-C", "panic=abort"])
            .arg(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/operator_process_fault.rs"
            ))
            .arg("-o")
            .arg(source.join(&file));
        if let Some(mode) = constructor {
            compiler.arg("--cfg").arg(mode);
        }
        assert!(compiler.status().unwrap().success());
        let mut manifest: Value = serde_json::from_slice(
            &fs::read(template_package_root().join("kyuubiki-operator.json")).unwrap(),
        )
        .unwrap();
        manifest["entrypoint"] = json!(file);
        fs::write(
            source.join("kyuubiki-operator.json"),
            serde_json::to_vec(&manifest).unwrap(),
        )
        .unwrap();
        let store = root.join("store");
        install_operator_package_into(&source, &store).unwrap();
        let pin = project_operator_in(
            &store,
            ProjectOperatorRequest {
                platform: None,
                project_id: "isolation".into(),
                scope: "a".repeat(64),
                operation: ProjectOperatorOperation::List,
            },
        )
        .unwrap()
        .installed
        .remove(0);
        Self { root, pin }
    }
    fn start(&self) -> LiveAgent {
        LiveAgent::start_fault_deployment(&self.root.join("store/packages")).unwrap()
    }
    fn run(&self, agent: &LiveAgent, id: &str, fault: &str) -> Value {
        self.try_run(agent, id, fault)
            .expect("Agent must survive native faults")
    }
    fn try_run(&self, agent: &LiveAgent, id: &str, fault: &str) -> Result<Value, String> {
        self.run_config(agent, id, json!({"fault":fault}))
    }
    fn run_config(&self, agent: &LiveAgent, id: &str, mut config: Value) -> Result<Value, String> {
        config["marker"] = json!(self.root.join(format!("{id}.pid")));
        let task = build_project_operator_task(
            "isolation",
            &self.pin,
            "extract.template_summary",
            id,
            config,
            json!({}),
        )
        .unwrap();
        agent
            .request(
                id,
                "run_operator_task_ir",
                json!({"mode":"execute", "task_ir":task,"job_id":id}),
            )
            .map_err(|e| e.to_string())
    }
    fn helper_config(&self, fault: &str) -> (TcpListener, Value) {
        let helper = self.root.join("helper");
        if !helper.exists() {
            assert!(
                Command::new(std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into()))
                    .args(["--edition=2024", "--crate-type=bin"])
                    .arg(concat!(
                        env!("CARGO_MANIFEST_DIR"),
                        "/tests/fixtures/operator_process_helper.rs"
                    ))
                    .arg("-o")
                    .arg(&helper)
                    .status()
                    .unwrap()
                    .success()
            );
        }
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        listener.set_nonblocking(true).unwrap();
        let config = json!({"fault":fault,"helper_path":helper,
            "helper_address":listener.local_addr().unwrap().to_string()});
        (listener, config)
    }
    fn wait_pid(&self, id: &str) -> i32 {
        let start = Instant::now();
        loop {
            if let Ok(pid) = fs::read_to_string(self.root.join(format!("{id}.pid"))) {
                if let Ok(pid) = pid.parse() {
                    return pid;
                }
            }
            assert!(
                start.elapsed().as_millis() < 1500,
                "native call never entered"
            );
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    }
}

fn helper_connection(listener: &TcpListener) -> TcpStream {
    let start = Instant::now();
    let mut stream = loop {
        match listener.accept() {
            Ok((stream, _)) => break stream,
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                assert!(
                    start.elapsed() < Duration::from_secs(5),
                    "helper did not connect"
                );
                std::thread::sleep(Duration::from_millis(10));
            }
            Err(e) => panic!("helper connection failed: {e}"),
        }
    };
    stream
        .set_read_timeout(Some(Duration::from_millis(1500)))
        .unwrap();
    let mut pid = [0; 4];
    stream.read_exact(&mut pid).unwrap();
    assert_ne!(u32::from_be_bytes(pid), std::process::id());
    stream
}

fn assert_helper_stopped(mut stream: TcpStream) {
    // EOF proves its owned connection closed even if init has not yet reaped a zombie.
    assert_eq!(
        stream
            .read(&mut [0])
            .expect("native descendant remained alive"),
        0
    );
}

#[test]
#[ignore = "starts an owned native worker without a dedicated process group"]
fn private_worker_rejects_shared_process_group_before_reading_input() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_kyuubiki-cli"))
        .args(["__native-operator-process-v1", "127.0.0.1:9"])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let started = Instant::now();
    let status = loop {
        let status = child.try_wait().unwrap();
        if status.is_some() || started.elapsed() > Duration::from_secs(2) {
            break status;
        }
        std::thread::sleep(Duration::from_millis(10));
    };
    // Always clean up the owned child, including when the early guard is broken.
    let _ = child.kill();
    let _ = child.wait();
    assert!(
        status.is_some_and(|s| !s.success()),
        "worker entered IPC without group ownership"
    );
}

#[test]
#[ignore = "compiles fault helpers and force-stops an owned test Agent"]
fn parent_loss_stops_owned_native_process_group() {
    let f = Fixture::new(None);
    let agent = f.start();
    let (listener, config) = f.helper_config("hang");
    std::thread::scope(|scope| {
        let work = scope.spawn(|| f.run_config(&agent, "parent-loss-tree", config));
        let stream = helper_connection(&listener);
        let pid = f.wait_pid("parent-loss-tree");
        assert_eq!(
            unsafe { libc::kill(agent.process_id() as i32, libc::SIGKILL) },
            0
        );
        assert!(work.join().unwrap().is_err());
        assert_gone(pid);
        assert_helper_stopped(stream);
    });
}

#[test]
#[ignore = "compiles fault helpers and starts isolated local Agents"]
fn descendants_stop_on_success_crash_timeout_and_cancellation() {
    let f = Fixture::new(None);
    let agent = f.start();
    for (id, fault, expected) in [
        ("tree-success", "healthy", None),
        ("tree-crash", "abort", Some("operator_process_crashed")),
        ("tree-timeout", "hang", Some("operator_process_timeout")),
        ("tree-cancel", "hang", Some("cancelled")),
    ] {
        let (listener, config) = f.helper_config(fault);
        std::thread::scope(|scope| {
            let work = scope.spawn(|| f.run_config(&agent, id, config));
            let stream = helper_connection(&listener);
            if id == "tree-cancel" {
                assert_eq!(
                    agent
                        .request("cancel-tree", "cancel_job", json!({"job_id":id}))
                        .unwrap()["ok"],
                    true
                );
            }
            let result = work.join().unwrap().unwrap();
            if let Some(code) = expected {
                assert_eq!(result["error"]["code"], code, "{result}");
            } else {
                assert_eq!(result["ok"], true, "{result}");
            }
            assert_gone(f.wait_pid(id));
            assert_helper_stopped(stream);
        });
        assert_eq!(f.run(&agent, &format!("after-{id}"), "healthy")["ok"], true);
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

#[test]
#[ignore = "compiles fault cdylibs and starts isolated local Agents"]
fn native_crash_and_hang_do_not_poison_agent_or_next_execution() {
    let f = Fixture::new(None);
    let agent = f.start();
    for (fault, code) in [
        ("abort", "operator_process_crashed"),
        ("hang", "operator_process_timeout"),
        ("invalid_pointer", "operator_process_crashed"),
        ("free_hang", "operator_process_timeout"),
    ] {
        let started = Instant::now();
        let result = f.run(&agent, fault, fault);
        assert_eq!(result["ok"], false, "{result}");
        assert_eq!(result["error"]["code"], code, "{result}");
        assert_eq!(
            result["error"]["details"]["operator_task_failure_receipt"]["recovery"]["retryable"],
            false
        );
        assert!(started.elapsed().as_secs() < 10);
        assert_gone(f.wait_pid(fault));
        let describe = agent
            .request("describe", "describe_agent", json!({}))
            .unwrap();
        assert_eq!(describe["ok"], true);
        let healthy = f.run(&agent, &format!("after-{fault}"), "healthy");
        assert_eq!(healthy["ok"], true, "{healthy}");
        assert_eq!(healthy["result"]["result"]["summary"]["sum"], 14);
        assert_eq!(
            healthy["result"]["result"]["summary"]["secret_inherited"],
            false
        );
        assert_gone(f.wait_pid(&format!("after-{fault}")));
        assert_eq!(
            healthy["result"]["operator_package_execution"]["isolation"]["mode"],
            "per_call_process"
        );
        assert_eq!(
            healthy["result"]["operator_package_execution"]["isolation"]["parent_loss_cleanup"],
            "owned_process_group"
        );
    }
}

#[test]
#[ignore = "compiles fault cdylibs and starts isolated local Agents"]
fn native_result_contract_rejection_preserves_task_identity_and_recovery() {
    let f = Fixture::new(None);
    let agent = f.start();
    for fault in ["wrong_result_id", "empty_result_id", "ambiguous_success"] {
        let rejected = f.run(&agent, fault, fault);
        assert_eq!(rejected["ok"], false, "{rejected}");
        assert!(rejected["result"].is_null(), "invalid result was published");
        assert_eq!(
            rejected["error"]["code"], "operator_package_dispatch_failed",
            "{rejected}"
        );
        let receipt = &rejected["error"]["details"]["operator_task_failure_receipt"];
        assert_eq!(receipt["operator_id"], "extract.template_summary");
        assert_eq!(receipt["task_id"], fault);
        assert_eq!(receipt["recovery"]["retryable"], false);
        assert_gone(f.wait_pid(fault));
        let healthy = f.run(&agent, &format!("after-{fault}"), "healthy");
        assert_eq!(healthy["ok"], true, "{healthy}");
        assert_eq!(
            healthy["result"]["result"]["operator_id"],
            "extract.template_summary"
        );
        assert_eq!(healthy["result"]["result"]["summary"]["sum"], 14);
    }
}

#[test]
#[ignore = "compiles fault cdylibs and starts isolated local Agents"]
fn disk_identity_change_after_probe_cannot_execute_under_old_pin() {
    let f = Fixture::new(None);
    let agent = f.start();
    let path = f
        .root
        .join("store/packages")
        .join(&f.pin.package_id)
        .join("kyuubiki-operator.json");
    let original = fs::read(&path).unwrap();
    let mut changed = original.clone();
    changed.push(b'\n');
    fs::write(&path, changed).unwrap();
    let rejected = f.run(&agent, "changed", "healthy");
    assert_eq!(
        rejected["error"]["code"], "operator_package_identity_mismatch",
        "{rejected}"
    );
    assert!(
        !f.root.join("changed.pid").exists(),
        "entrypoint ran before checking generation identity"
    );
    fs::write(&path, original).unwrap();
    assert_eq!(f.run(&agent, "unchanged", "healthy")["ok"], true);
}

fn assert_gone(pid: i32) {
    let start = Instant::now();
    while unsafe { libc::kill(pid, 0) } == 0 {
        assert!(
            start.elapsed().as_secs() < 3,
            "native child {pid} still exists"
        );
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    assert_eq!(
        std::io::Error::last_os_error().raw_os_error(),
        Some(libc::ESRCH)
    );
}

#[test]
#[ignore = "compiles fault cdylibs and starts isolated local Agents"]
fn cancellation_kills_native_call_and_releases_execution_capacity() {
    let f = Fixture::new(None);
    let agent = f.start();
    std::thread::scope(|scope| {
        let work = scope.spawn(|| f.run(&agent, "cancel-native", "hang"));
        let pid = f.wait_pid("cancel-native");
        let start = Instant::now();
        let cancelled = agent
            .request("cancel", "cancel_job", json!({"job_id":"cancel-native"}))
            .unwrap();
        assert_eq!(cancelled["ok"], true, "{cancelled}");
        let result = work.join().unwrap();
        assert_eq!(result["error"]["code"], "cancelled", "{result}");
        assert_eq!(
            result["error"]["details"]["operator_task_failure_receipt"]["failure_stage"],
            "execute_external_process"
        );
        assert!(start.elapsed().as_millis() < 1500);
        assert_gone(pid);
    });
    assert_eq!(f.run(&agent, "after-cancel", "healthy")["ok"], true);
}

#[test]
#[ignore = "compiles fault cdylibs and force-stops an owned test Agent"]
fn force_stopping_parent_does_not_leave_native_call_running() {
    let f = Fixture::new(None);
    let agent = f.start();
    std::thread::scope(|scope| {
        let work = scope.spawn(|| f.try_run(&agent, "orphan-probe", "hang"));
        let pid = f.wait_pid("orphan-probe");
        assert_eq!(
            unsafe { libc::kill(agent.process_id() as i32, libc::SIGKILL) },
            0
        );
        assert!(work.join().unwrap().is_err());
        assert_gone(pid);
    });
}

#[test]
#[ignore = "compiles fault cdylibs and starts isolated local Agents"]
fn constructor_crash_and_hang_leave_recovery_control_available() {
    for mode in ["constructor_abort", "constructor_hang"] {
        let f = Fixture::new(Some(mode));
        let started = Instant::now();
        let agent = f.start();
        assert!(started.elapsed().as_secs() < 10);
        let describe = agent
            .request("describe", "describe_agent", json!({}))
            .unwrap();
        assert_eq!(describe["ok"], true);
        assert_eq!(
            describe["result"]["operator_package_runtime"]["status"], "activation_failed",
            "{describe}"
        );
        assert_eq!(describe["result"]["deployment_readiness"]["ready"], false);
    }
}
