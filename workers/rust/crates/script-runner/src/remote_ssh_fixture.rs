use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

type RunnerResult<T> = Result<T, String>;

pub(crate) fn run_remote_ssh_fixture(root: &Path, args: Vec<OsString>) -> RunnerResult<u8> {
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        print_help();
        return Ok(0);
    }
    if !args.is_empty() {
        return Err("remote-ssh-fixture does not accept positional arguments".to_string());
    }

    let fixture = RemoteSshFixture::new(root);
    fixture.prepare_runtime()?;
    fixture.ensure_key()?;
    fixture.compose(&["up", "-d", "--build"])?;
    let _cleanup = FixtureCleanup { fixture: &fixture };
    fixture.probe()
}

struct RemoteSshFixture {
    root: PathBuf,
    fixture_dir: PathBuf,
    runtime_dir: PathBuf,
    key_path: PathBuf,
    known_hosts_path: PathBuf,
    compose_file: PathBuf,
}

struct FixtureCleanup<'a> {
    fixture: &'a RemoteSshFixture,
}

impl Drop for FixtureCleanup<'_> {
    fn drop(&mut self) {
        let _ = self.fixture.compose_quiet(&["down"]);
    }
}

impl RemoteSshFixture {
    fn new(root: &Path) -> Self {
        let fixture_dir = root.join("tests/integration/remote-ssh-fixture");
        let runtime_dir = fixture_dir.join("runtime");
        Self {
            root: root.to_path_buf(),
            key_path: runtime_dir.join("client_key"),
            known_hosts_path: runtime_dir.join("known_hosts"),
            compose_file: fixture_dir.join("compose.yaml"),
            fixture_dir,
            runtime_dir,
        }
    }

    fn prepare_runtime(&self) -> RunnerResult<()> {
        std::fs::create_dir_all(self.runtime_dir.join("workspace")).map_err(|error| {
            format!(
                "failed to create fixture runtime {}: {error}",
                self.runtime_dir.display()
            )
        })
    }

    fn ensure_key(&self) -> RunnerResult<()> {
        if self.key_path.is_file() {
            return Ok(());
        }
        let status = Command::new("ssh-keygen")
            .args(["-t", "ed25519", "-N", "", "-f"])
            .arg(&self.key_path)
            .args(["-C", "kyuubiki-remote-ssh-fixture"])
            .current_dir(&self.root)
            .stdout(Stdio::null())
            .status()
            .map_err(|error| format!("failed to run ssh-keygen: {error}"))?;
        if !status.success() {
            return Err("ssh-keygen failed for remote ssh fixture".to_string());
        }
        Ok(())
    }

    fn compose(&self, args: &[&str]) -> RunnerResult<()> {
        let status = compose_command(&self.compose_file, args)
            .current_dir(&self.fixture_dir)
            .status()
            .map_err(|error| format!("failed to run docker compose: {error}"))?;
        if !status.success() {
            return Err(format!("docker compose {} failed", args.join(" ")));
        }
        Ok(())
    }

    fn compose_quiet(&self, args: &[&str]) -> RunnerResult<()> {
        let status = compose_command(&self.compose_file, args)
            .current_dir(&self.fixture_dir)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map_err(|error| format!("failed to run docker compose: {error}"))?;
        if status.success() {
            Ok(())
        } else {
            Err(format!("docker compose {} failed", args.join(" ")))
        }
    }

    fn probe(&self) -> RunnerResult<u8> {
        for _ in 0..30 {
            if self.probe_once()? {
                println!("remote ssh fixture probe ok");
                return Ok(0);
            }
            thread::sleep(Duration::from_secs(1));
        }
        eprintln!("remote ssh fixture did not become reachable");
        Ok(1)
    }

    fn probe_once(&self) -> RunnerResult<bool> {
        let output = Command::new("ssh")
            .args(["-i"])
            .arg(&self.key_path)
            .args(["-o"])
            .arg(format!(
                "UserKnownHostsFile={}",
                self.known_hosts_path.display()
            ))
            .args([
                "-o",
                "StrictHostKeyChecking=accept-new",
                "-o",
                "ConnectTimeout=2",
                "-p",
                "2222",
                "kyuubiki-fixture@127.0.0.1",
                "cd /tmp/kyuubiki-fixture && printf '%s' 'kyuubiki-remote-ok'",
            ])
            .current_dir(&self.root)
            .output()
            .map_err(|error| format!("failed to run ssh fixture probe: {error}"))?;
        Ok(output.status.success()
            && String::from_utf8_lossy(&output.stdout).contains("kyuubiki-remote-ok"))
    }
}

fn compose_command(compose_file: &Path, args: &[&str]) -> Command {
    let mut command = Command::new("docker");
    command.args(["compose", "-f"]);
    command.arg(compose_file);
    command.args(args);
    command
}

fn print_help() {
    println!(
        "remote-ssh-fixture\n\n\
Usage:\n  ./scripts/kyuubiki remote-ssh-fixture\n\n\
Starts the local Docker SSH fixture, probes it through SSH, and tears it down."
    );
}
