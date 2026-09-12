#[path = "support/runtime_fixture.rs"]
mod support;

use std::fs;
use std::net::TcpListener;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use support::RuntimeFixture;

fn assert_alive(pid: u32) {
    assert!(
        kyuubiki_platform::process_instance_token(pid)
            .unwrap()
            .is_some(),
        "process {pid} unexpectedly exited"
    );
}

#[test]
fn background_processes_survive_launcher_exit_and_duplicate_start_is_idempotent() {
    let runtime = RuntimeFixture::new();
    runtime.success("start-local");
    let pids: Vec<_> = (0..4).map(|index| runtime.pid(index)).collect();
    runtime.success("start-local");
    for (index, pid) in pids.iter().enumerate() {
        assert_eq!(*pid, runtime.pid(index));
        assert_alive(*pid);
    }
    assert!(
        runtime
            .success("status")
            .contains("GUI exit does not stop services")
    );
    runtime.success("restart-local");
    for (index, pid) in pids.iter().enumerate() {
        assert_ne!(*pid, runtime.pid(index));
    }
    runtime.success("stop");
    runtime.success("stop");
    runtime.assert_stopped();
}

#[test]
fn concurrent_stop_cannot_interrupt_an_inflight_start_and_status_remains_readable() {
    let runtime = RuntimeFixture::new();
    let first = runtime
        .command("start-local")
        .env("KYUUBIKI_TEST_LISTENER_DELAY_MS", "400")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    runtime.wait_for_record(2);
    let status = runtime.run("status");
    let stop = runtime.run("stop");
    let started = first.wait_with_output().unwrap();
    assert!(
        started.status.success(),
        "{}",
        RuntimeFixture::render(&started)
    );
    assert!(
        status.status.success(),
        "{}",
        RuntimeFixture::render(&status)
    );
    assert!(
        RuntimeFixture::render(&status).contains("starting")
            || RuntimeFixture::render(&status).contains("running")
    );
    assert!(!stop.status.success());
    assert!(
        RuntimeFixture::render(&stop).contains("busy"),
        "{}",
        RuntimeFixture::render(&stop)
    );
    runtime.success("stop");
    runtime.assert_stopped();
}

#[test]
fn unmanaged_port_blocks_start_before_any_agent_is_spawned_and_is_never_stopped() {
    let runtime = RuntimeFixture::new();
    let unrelated = TcpListener::bind(("127.0.0.1", runtime.ports[0])).unwrap();
    assert!(runtime.success("status").contains("orchestrator: blocked"));
    let start = runtime.run("start-local");
    assert!(!start.status.success());
    assert!(RuntimeFixture::render(&start).contains("unmanaged"));
    for index in 0..4 {
        assert!(!runtime.pid_path(index).exists());
    }
    let stop = runtime.run("stop");
    assert!(!stop.status.success());
    assert!(RuntimeFixture::render(&stop).contains("unmanaged"));
    assert!(unrelated.local_addr().is_ok());
    drop(unrelated);
    runtime.success("start-local");
    runtime.success("stop");
    runtime.assert_stopped();
}

#[test]
fn scoped_stop_and_failed_start_preserve_preexisting_agents_and_frontend() {
    let runtime = RuntimeFixture::new();
    runtime.success("start-local");
    let pids: Vec<_> = (1..4).map(|index| runtime.pid(index)).collect();
    let stop = runtime
        .command("stop")
        .env("KYUUBIKI_RUNTIME_ORCHESTRATOR_ONLY", "true")
        .output()
        .unwrap();
    assert!(stop.status.success(), "{}", RuntimeFixture::render(&stop));
    assert!(!runtime.pid_path(0).exists());
    runtime.set_service_args("orchestrator", &["--exit"]);
    let started = Instant::now();
    let failure = runtime.run("start-local");
    assert!(!failure.status.success());
    assert!(
        started.elapsed() < Duration::from_secs(10),
        "failed startup waited for the readiness timeout"
    );
    for (offset, pid) in pids.iter().enumerate() {
        assert_eq!(*pid, runtime.pid(offset + 1));
        assert_alive(*pid);
    }
    runtime.success("stop");
    runtime.assert_stopped();
}

#[test]
fn legacy_live_pid_and_reused_pid_tokens_never_authorize_termination() {
    let runtime = RuntimeFixture::new();
    runtime.success("start-local");
    let pid = runtime.pid(0);
    let path = runtime.pid_path(0).with_extension("process.json");
    let original = fs::read(&path).unwrap();
    let mut record: serde_json::Value = serde_json::from_slice(&original).unwrap();
    record["instance"] = serde_json::json!("stale-process-incarnation");
    fs::write(&path, record.to_string()).unwrap();
    let mismatch = runtime
        .command("stop")
        .env("KYUUBIKI_RUNTIME_ORCHESTRATOR_ONLY", "true")
        .output();
    fs::write(&path, &original).unwrap();
    let mismatch = mismatch.unwrap();
    assert!(!mismatch.status.success());
    assert!(RuntimeFixture::render(&mismatch).contains("ownership mismatch"));
    assert_alive(pid);

    fs::remove_file(&path).unwrap();
    let legacy = runtime
        .command("stop")
        .env("KYUUBIKI_RUNTIME_ORCHESTRATOR_ONLY", "true")
        .output();
    fs::write(&path, &original).unwrap();
    let legacy = legacy.unwrap();
    assert!(!legacy.status.success());
    assert!(RuntimeFixture::render(&legacy).contains("unverified live process"));
    assert_alive(pid);
    runtime.success("stop");
    runtime.assert_stopped();
}

#[test]
fn stop_continues_other_owned_services_after_one_ownership_failure() {
    let runtime = RuntimeFixture::new();
    runtime.success("start-local");
    let pid = runtime.pid(0);
    let path = runtime.pid_path(0).with_extension("process.json");
    let original = fs::read(&path).unwrap();
    fs::write(&path, "invalid receipt").unwrap();
    let stopped = runtime.command("stop").output();
    fs::write(&path, original).unwrap();
    let stopped = stopped.unwrap();
    assert!(!stopped.status.success());
    assert!(RuntimeFixture::render(&stopped).contains("stop incomplete"));
    assert_alive(pid);
    for index in 1..4 {
        assert!(!runtime.pid_path(index).exists());
    }
    assert!(
        runtime
            .state
            .join("run")
            .join(format!("runtime-mode-{}.txt", runtime.ports[0]))
            .exists()
    );
    runtime.success("stop");
    runtime.assert_stopped();
}

#[test]
fn dead_pid_records_are_reconciled_without_trusting_a_bare_pid() {
    let runtime = RuntimeFixture::new();
    let mut dead = Command::new(env!("CARGO_BIN_EXE_kyuubiki-runtime-test-listener"))
        .arg("--exit")
        .spawn()
        .unwrap();
    let dead_pid = dead.id();
    dead.wait().unwrap();
    fs::create_dir_all(runtime.state.join("run")).unwrap();
    fs::write(runtime.pid_path(0), dead_pid.to_string()).unwrap();
    runtime.success("start-local");
    assert_ne!(dead_pid, runtime.pid(0));
    runtime.success("stop");
    runtime.assert_stopped();
}

#[test]
fn terminated_controller_releases_lock_and_registered_services_are_recoverable() {
    let runtime = RuntimeFixture::new();
    let mut controller = runtime
        .command("start-local")
        .env("KYUUBIKI_TEST_LISTENER_DELAY_MS", "800")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    runtime.wait_for_record(2);
    controller.kill().unwrap();
    controller.wait().unwrap();
    let orphan = runtime.pid(2);
    assert_alive(orphan);
    runtime.success("restart-local");
    assert_ne!(orphan, runtime.pid(2));
    assert!(
        kyuubiki_platform::process_instance_token(orphan)
            .unwrap()
            .is_none()
    );
    runtime.success("stop");
    runtime.assert_stopped();
}

#[test]
fn missing_mode_record_does_not_relabel_live_local_processes_as_cloud() {
    let runtime = RuntimeFixture::new();
    runtime.success("start-local");
    let pids: Vec<_> = (0..4).map(|index| runtime.pid(index)).collect();
    fs::remove_file(
        runtime
            .state
            .join("run")
            .join(format!("runtime-mode-{}.txt", runtime.ports[0])),
    )
    .unwrap();
    let changed = runtime
        .command("start-cloud")
        .env("KYUUBIKI_DEPLOYMENT_MODE", "local")
        .env(
            "DATABASE_URL",
            "postgresql://127.0.0.1/unused-lifecycle-fixture",
        )
        .output()
        .unwrap();
    assert!(!changed.status.success());
    assert!(RuntimeFixture::render(&changed).contains("different deployment mode"));
    for (index, pid) in pids.iter().enumerate() {
        assert_eq!(*pid, runtime.pid(index));
        assert_alive(*pid);
    }
    runtime.success("stop");
    runtime.assert_stopped();
}
