#[path = "support/runtime_fixture.rs"]
mod support;

use support::RuntimeFixture;

#[test]
fn installed_runtime_starts_and_stops_without_source_toolchains() {
    let runtime = RuntimeFixture::new();
    runtime.success("start-local");
    let status = runtime.success("status");
    assert!(status.contains("runtime-policy: installer-managed"));
    for label in [
        "orchestrator".to_string(),
        "frontend".to_string(),
        format!("agent[{}]", runtime.ports[2]),
        format!("agent[{}]", runtime.ports[3]),
    ] {
        assert!(status.contains(&format!("{label}: running")), "{status}");
    }
    for toolchain in ["npm", "mix", "cargo"] {
        assert!(!status.contains(toolchain), "{status}");
    }
    assert!(!runtime.root.join("run").exists());
    assert!(!runtime.root.join("data").exists());
    runtime.success("stop");
    runtime.assert_stopped();
}

#[test]
fn installed_runtime_can_run_agents_and_orchestrator_without_frontend() {
    let runtime = RuntimeFixture::new();
    let start = runtime
        .command("start-local")
        .env("KYUUBIKI_RUNTIME_FRONTEND_DISABLED", "true")
        .output()
        .unwrap();
    assert!(start.status.success(), "{}", RuntimeFixture::render(&start));
    let status = runtime
        .command("status")
        .env("KYUUBIKI_RUNTIME_FRONTEND_DISABLED", "true")
        .output()
        .unwrap();
    let rendered = RuntimeFixture::render(&status);
    assert!(rendered.contains("orchestrator: running"), "{rendered}");
    assert!(rendered.contains(&format!("agent[{}]: running", runtime.ports[2])));
    assert!(rendered.contains("frontend: disabled by runtime configuration"));
    assert!(!runtime.pid_path(1).exists());
    runtime.success("stop");
    runtime.assert_stopped();
}

#[test]
fn failed_orchestrator_start_rolls_back_new_agents() {
    let runtime = RuntimeFixture::new();
    runtime.set_service_args("orchestrator", &["--exit"]);
    let start = runtime.run("start-local");
    let rendered = RuntimeFixture::render(&start);
    assert!(!start.status.success());
    assert!(rendered.contains("startup rollback"), "{rendered}");
    assert!(
        rendered.contains("exited") || rendered.contains("already exited"),
        "{rendered}"
    );
    runtime.assert_stopped();
}
