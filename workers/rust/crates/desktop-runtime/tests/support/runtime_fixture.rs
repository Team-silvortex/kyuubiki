#![allow(dead_code)]

use std::fs;
use std::net::TcpListener;
use std::path::PathBuf;
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, MutexGuard};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

pub struct RuntimeFixture {
    pub base: PathBuf,
    pub root: PathBuf,
    pub state: PathBuf,
    pub ports: [u16; 4],
    _port_handoff: MutexGuard<'static, ()>,
}

impl RuntimeFixture {
    pub fn new() -> Self {
        static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(0);
        // Serializing fixture port handoff avoids another fixture's probes consuming a
        // reserved ephemeral port. Concurrent lifecycle calls are exercised inside a fixture.
        static PORT_HANDOFF: Mutex<()> = Mutex::new(());
        let port_handoff = PORT_HANDOFF
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let sequence = NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed);
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let base = std::env::temp_dir().join(format!(
            "kyuubiki-lifecycle-{}-{stamp}-{sequence}",
            std::process::id()
        ));
        let root = base.join("payload");
        let state = base.join("state");
        fs::create_dir_all(root.join("manifests")).unwrap();
        fs::create_dir_all(root.join("bin")).unwrap();
        let reservations: Vec<_> = (0..4)
            .map(|_| TcpListener::bind(("127.0.0.1", 0)).unwrap())
            .collect();
        let ports = std::array::from_fn(|index| reservations[index].local_addr().unwrap().port());
        let executable = if cfg!(windows) {
            "service-listener.exe"
        } else {
            "service-listener"
        };
        fs::copy(
            env!("CARGO_BIN_EXE_kyuubiki-runtime-test-listener"),
            root.join("bin").join(executable),
        )
        .unwrap();
        let command = format!("bin/{executable}");
        let manifest = serde_json::json!({
            "schema_version": "kyuubiki.service-launch/v1",
            "services": [
                {"id": "agent", "command": command, "args": ["{port}"], "cwd": "."},
                {"id": "orchestrator", "command": command, "args": [ports[0].to_string()], "cwd": "."},
                {"id": "frontend", "command": command, "args": ["{port}"], "cwd": "."}
            ]
        });
        fs::write(
            root.join("manifests/service-launch.json"),
            manifest.to_string(),
        )
        .unwrap();
        fs::write(
            root.join("manifests/runtime-payload.json"),
            serde_json::json!({
                "schema_version": "kyuubiki.runtime-payload/v1", "version": "test",
                "platform": kyuubiki_platform::Platform::current().as_str()
            })
            .to_string(),
        )
        .unwrap();
        Self {
            base,
            root,
            state,
            ports,
            _port_handoff: port_handoff,
        }
    }

    pub fn command(&self, action: &str) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_kyuubiki-runtime"));
        command
            .arg(action)
            .env("KYUUBIKI_RUNTIME_ROOT", &self.root)
            .env("KYUUBIKI_RUNTIME_STATE_ROOT", &self.state)
            .env("KYUUBIKI_ORCHESTRATOR_PORT", self.ports[0].to_string())
            .env("KYUUBIKI_FRONTEND_PORT", self.ports[1].to_string())
            .env(
                "KYUUBIKI_AGENT_ENDPOINTS",
                format!("127.0.0.1:{},127.0.0.1:{}", self.ports[2], self.ports[3]),
            )
            .env("KYUUBIKI_AGENT_DISCOVERY", "static")
            .env("KYUUBIKI_RUNTIME_ORCHESTRATOR_ONLY", "false")
            .env("KYUUBIKI_RUNTIME_FRONTEND_DISABLED", "false");
        command
    }

    pub fn run(&self, action: &str) -> Output {
        self.command(action).output().unwrap()
    }

    pub fn success(&self, action: &str) -> String {
        let output = self.run(action);
        assert!(
            output.status.success(),
            "{action}: {}",
            Self::render(&output)
        );
        String::from_utf8(output.stdout).unwrap()
    }

    pub fn render(output: &Output) -> String {
        format!(
            "{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
    }

    pub fn pid_path(&self, index: usize) -> PathBuf {
        let stem = match index {
            0 => "orchestrator",
            1 => "frontend",
            _ => "agent",
        };
        self.state
            .join("run")
            .join(format!("{stem}-{}.pid", self.ports[index]))
    }

    pub fn pid(&self, index: usize) -> u32 {
        fs::read_to_string(self.pid_path(index))
            .unwrap()
            .trim()
            .parse()
            .unwrap()
    }

    pub fn set_service_args(&self, service: &str, args: &[&str]) {
        let path = self.root.join("manifests/service-launch.json");
        let mut manifest: serde_json::Value =
            serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        let entry = manifest["services"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|entry| entry["id"] == service)
            .unwrap();
        entry["args"] = serde_json::json!(args);
        fs::write(path, manifest.to_string()).unwrap();
    }

    pub fn wait_for_record(&self, index: usize) {
        let deadline = Instant::now() + Duration::from_secs(5);
        while !fs::read(self.pid_path(index).with_extension("process.json"))
            .ok()
            .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(&bytes).ok())
            .is_some_and(|record| record["instance"].is_string() && record["pid"].is_u64())
        {
            assert!(Instant::now() < deadline, "process record did not appear");
            std::thread::sleep(Duration::from_millis(20));
        }
    }

    pub fn assert_stopped(&self) {
        for index in 0..4 {
            assert!(!self.pid_path(index).exists());
            assert!(!self.pid_path(index).with_extension("process.json").exists());
            assert!(
                TcpListener::bind(("127.0.0.1", self.ports[index])).is_ok(),
                "port {} is still occupied",
                self.ports[index]
            );
        }
    }
}

impl Drop for RuntimeFixture {
    fn drop(&mut self) {
        // Never remove ownership records while an owned service could still be running.
        if self
            .command("stop")
            .output()
            .is_ok_and(|output| output.status.success())
        {
            let _ = fs::remove_dir_all(&self.base);
        }
    }
}
