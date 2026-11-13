use super::transport::Transport;
use kyuubiki_installer::{
    Platform, active_agent_binary_in, install_agent_update_package_into,
    install_operator_package_into, prepare_agent_update_package, project_operator_in,
};
use kyuubiki_protocol::{
    ProjectOperatorOperation, ProjectOperatorPin, ProjectOperatorRequest,
    build_project_operator_task,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::Read,
    net::TcpListener,
    os::unix::fs::DirBuilderExt,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, Instant},
};

static SEQUENCE: AtomicU64 = AtomicU64::new(0);
pub const CAPACITY: usize = 4;

pub struct Fixture {
    pub root: PathBuf,
    pub pin: ProjectOperatorPin,
    pub client: Transport,
    pub child: Option<Child>,
    pub agent_sha256: String,
}
impl Fixture {
    pub fn new(library: Option<&Path>) -> Self {
        Self::with_run_timeout(library, 15_000)
    }
    pub fn with_run_timeout(library: Option<&Path>, run_ms: u64) -> Self {
        assert!((100..=15_000).contains(&run_ms));
        let root = std::env::temp_dir().join(format!(
            "kyuubiki-isolation-bench-{}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        fs::DirBuilder::new().mode(0o700).create(&root).unwrap();
        // Establish cleanup ownership before staging or launching anything.
        let mut fixture = Self {
            root,
            pin: ProjectOperatorPin {
                package_id: "unused".into(),
                package_version: "0.1.0".into(),
                manifest_sha256: "0".repeat(64),
                entrypoint_sha256: "0".repeat(64),
                operators: vec![],
            },
            client: Transport::configured("127.0.0.1:1", "ssh-loopback", None, None, None).unwrap(),
            child: None,
            agent_sha256: String::new(),
        };
        let source = fixture.root.join("source");
        fs::create_dir(&source).unwrap();
        let target_library = source.join("operator.so");
        if let Some(library) = library {
            fs::copy(library, &target_library).unwrap();
        } else {
            assert!(
                Command::new(std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into()))
                    .args([
                        "--edition=2024",
                        "--crate-type=cdylib",
                        "-C",
                        "panic=abort",
                        "-C",
                        "opt-level=3"
                    ])
                    .arg(concat!(
                        env!("CARGO_MANIFEST_DIR"),
                        "/tests/fixtures/operator_process_fault.rs"
                    ))
                    .arg("-o")
                    .arg(&target_library)
                    .status()
                    .unwrap()
                    .success()
            );
        }
        let mut manifest: Value = serde_json::from_slice(
            &fs::read(
                Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("../../templates/operator-crate-template/kyuubiki-operator.json"),
            )
            .unwrap(),
        )
        .unwrap();
        manifest["entrypoint"] = json!("operator.so");
        fs::write(
            source.join("kyuubiki-operator.json"),
            serde_json::to_vec(&manifest).unwrap(),
        )
        .unwrap();
        let store = fixture.root.join("store");
        install_operator_package_into(&source, &store).unwrap();
        fixture.pin = project_operator_in(
            &store,
            ProjectOperatorRequest {
                platform: None,
                project_id: "isolation-benchmark".into(),
                scope: "b".repeat(64),
                operation: ProjectOperatorOperation::List,
            },
        )
        .unwrap()
        .installed
        .remove(0);
        prepare_agent_update_package(
            Path::new(env!("CARGO_BIN_EXE_kyuubiki-cli")),
            &fixture.root.join("agent-package"),
            env!("CARGO_PKG_VERSION"),
            Platform::current(),
        )
        .unwrap();
        install_agent_update_package_into(
            &fixture.root.join("agent-package"),
            &fixture.root.join("agent-store"),
            Platform::current(),
        )
        .unwrap();
        let binary =
            active_agent_binary_in(&fixture.root.join("agent-store"), Platform::current()).unwrap();
        fixture.agent_sha256 = file_digest(&binary);
        let reserved = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let address = reserved.local_addr().unwrap();
        drop(reserved);
        fixture.client =
            Transport::configured(&address.to_string(), "ssh-loopback", None, None, None).unwrap();
        fixture.child = Some(
            Command::new(binary)
                .env_clear()
                .env("HOME", &fixture.root)
                .env("TMPDIR", &fixture.root)
                .env("KYUUBIKI_AGENT_MAX_ACTIVE_EXECUTIONS", CAPACITY.to_string())
                .env(
                    "KYUUBIKI_OPERATOR_PROCESS_RUN_TIMEOUT_MS",
                    run_ms.to_string(),
                )
                .args([
                    "agent",
                    "--host",
                    "127.0.0.1",
                    "--port",
                    &address.port().to_string(),
                    "--agent-id",
                    "owned-isolation-benchmark",
                    "--operator-package-host-id",
                    "isolation-benchmark",
                    "--operator-packages-root",
                    store.join("packages").to_str().unwrap(),
                    "--operator-activated-package-count",
                    "1",
                ])
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .unwrap(),
        );
        let deadline = Instant::now() + Duration::from_secs(30);
        loop {
            assert!(
                fixture
                    .child
                    .as_mut()
                    .unwrap()
                    .try_wait()
                    .unwrap()
                    .is_none(),
                "owned Agent exited"
            );
            if let Ok(descriptor) = fixture.client.request("describe_agent", json!({})) {
                assert_eq!(
                    descriptor["operator_package_runtime"]["attachment"]["activated_package_count"],
                    1
                );
                break;
            }
            assert!(Instant::now() < deadline, "Agent readiness timeout");
            std::thread::sleep(Duration::from_millis(20));
        }
        fixture
    }
    pub fn pid(&self) -> u32 {
        self.child.as_ref().unwrap().id()
    }
    pub fn task(&self, id: &str, config: Value, input: Value) -> Value {
        build_project_operator_task(
            "isolation-benchmark",
            &self.pin,
            "extract.template_summary",
            id,
            input,
            config,
        )
        .unwrap()
    }
    pub fn execute(&self, id: &str, task: Value) -> Result<Value, String> {
        self.client.request_named(
            id,
            "run_operator_task_ir",
            json!({"mode":"execute", "job_id":id, "task_ir":task}),
        )
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
        let _ = fs::remove_dir_all(&self.root);
    }
}

pub fn file_digest(path: &Path) -> String {
    let mut file = fs::File::open(path).unwrap();
    let mut hash = Sha256::new();
    let mut buf = [0; 65536];
    loop {
        let len = file.read(&mut buf).unwrap();
        if len == 0 {
            break;
        }
        hash.update(&buf[..len]);
    }
    format!("{:x}", hash.finalize())
}
