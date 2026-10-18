#[path = "support/runtime_fixture.rs"]
mod support;

use std::fs;
use support::RuntimeFixture;

const KEY: &str = "KYUUBIKI_AGENT_ALLOWED_PEER_IPS";

#[test]
fn invalid_network_restart_preserves_existing_service_incarnations_and_records() {
    let runtime = RuntimeFixture::new();
    runtime.success("start-local");
    let pids: Vec<_> = (0..4).map(|index| runtime.pid(index)).collect();
    let records: Vec<_> = (0..4)
        .map(|index| fs::read(runtime.pid_path(index).with_extension("process.json")).unwrap())
        .collect();
    let retained = runtime.state.join("agent-executions/retained-sentinel");
    fs::write(&retained, "must survive rejected configuration").unwrap();
    // Models the real Agent rejecting its startup settings if preflight is missed.
    runtime.set_service_args("agent", &["--exit"]);
    let result = runtime
        .command("restart-local")
        .env(KEY, "*")
        .output()
        .unwrap();
    assert!(
        !result.status.success(),
        "{}",
        RuntimeFixture::render(&result)
    );
    for (index, pid) in pids.iter().enumerate() {
        assert_eq!(
            *pid,
            runtime.pid(index),
            "restart must reject before stopping services"
        );
        assert!(
            kyuubiki_platform::process_instance_token(*pid)
                .unwrap()
                .is_some()
        );
        assert_eq!(
            records[index],
            fs::read(runtime.pid_path(index).with_extension("process.json")).unwrap()
        );
    }
    assert!(RuntimeFixture::render(&result).contains(KEY));
    assert_eq!(
        fs::read_to_string(retained).unwrap(),
        "must survive rejected configuration"
    );
    runtime.success("stop");
    runtime.assert_stopped();
}

#[test]
fn invalid_network_start_cannot_publish_any_service_or_create_execution_storage() {
    let runtime = RuntimeFixture::new();
    let result = runtime
        .command("start-local")
        .env(KEY, "192.0.2.0/24")
        .output()
        .unwrap();
    assert!(
        !result.status.success(),
        "{}",
        RuntimeFixture::render(&result)
    );
    assert!(RuntimeFixture::render(&result).contains(KEY));
    for index in 0..4 {
        assert!(!runtime.pid_path(index).exists());
    }
    assert!(!runtime.state.join("agent-executions").exists());
    runtime.assert_stopped();
}

#[test]
fn invalid_network_settings_do_not_hide_status_or_prevent_explicit_stop() {
    let runtime = RuntimeFixture::new();
    runtime.success("start-local");
    let status = runtime
        .command("status")
        .env(KEY, "secret.example.invalid")
        .output()
        .unwrap();
    let rendered = RuntimeFixture::render(&status);
    assert!(status.status.success(), "{rendered}");
    assert!(rendered.contains("agent-network-config: invalid"));
    assert!(rendered.contains("orchestrator: running"));
    assert!(rendered.contains("frontend: running"));
    assert!(!rendered.contains("secret.example.invalid"));
    let stop = runtime.command("stop").env(KEY, "*").output().unwrap();
    assert!(stop.status.success(), "{}", RuntimeFixture::render(&stop));
    runtime.assert_stopped();
}

#[test]
fn explicit_network_config_is_reported_without_claiming_it_is_running_policy() {
    let runtime = RuntimeFixture::new();
    runtime.success("start-local");
    let pids: Vec<_> = (0..4).map(|index| runtime.pid(index)).collect();
    let start = runtime
        .command("start-local")
        .env(KEY, "192.0.2.1,::ffff:192.0.2.1")
        .output()
        .unwrap();
    let rendered = RuntimeFixture::render(&start);
    assert!(start.status.success(), "{rendered}");
    assert!(rendered.contains("explicit-peers=1"));
    assert!(rendered.contains("running-policy=not-observed"));
    assert!(rendered.contains("authentication=false"));
    for (index, pid) in pids.iter().enumerate() {
        assert_eq!(*pid, runtime.pid(index));
    }
    runtime.success("stop");
    runtime.assert_stopped();
}

#[test]
fn orchestrator_only_start_does_not_validate_an_unlaunched_agent_policy() {
    let runtime = RuntimeFixture::new();
    let started = runtime
        .command("start-local")
        .env("KYUUBIKI_RUNTIME_ORCHESTRATOR_ONLY", "true")
        .env(KEY, "*")
        .output()
        .unwrap();
    assert!(
        started.status.success(),
        "{}",
        RuntimeFixture::render(&started)
    );
    assert!(runtime.pid_path(0).exists());
    for index in 1..4 {
        assert!(!runtime.pid_path(index).exists());
    }
    runtime.success("stop");
    runtime.assert_stopped();
}

#[test]
fn distributed_restart_without_mode_record_stops_preexisting_local_agents() {
    let runtime = RuntimeFixture::new();
    runtime.success("start-local");
    let pids: Vec<_> = (0..4).map(|index| runtime.pid(index)).collect();
    let mode_path = runtime
        .state
        .join("run")
        .join(format!("runtime-mode-{}.txt", runtime.ports[0]));
    fs::remove_file(&mode_path).unwrap();
    let restart = runtime
        .command("restart-distributed")
        .env("KYUUBIKI_DEPLOYMENT_MODE", "local")
        .env(
            "DATABASE_URL",
            "postgresql://127.0.0.1/unused-lifecycle-fixture",
        )
        .env(KEY, "*")
        .output()
        .unwrap();
    let old_processes_gone = pids.iter().all(|pid| {
        kyuubiki_platform::process_instance_token(*pid)
            .unwrap()
            .is_none()
    });
    let agent_records_gone = (2..4).all(|index| {
        !runtime.pid_path(index).exists()
            && !runtime
                .pid_path(index)
                .with_extension("process.json")
                .exists()
    });
    let final_mode = fs::read_to_string(&mode_path).unwrap_or_default();
    // Clean all fixture-owned services even if a regression left local Agents behind.
    fs::write(&mode_path, "local\n").unwrap();
    runtime.success("stop");
    runtime.assert_stopped();
    assert!(
        restart.status.success(),
        "{}",
        RuntimeFixture::render(&restart)
    );
    assert_eq!(final_mode.trim(), "distributed");
    assert!(old_processes_gone);
    assert!(agent_records_gone);
}
