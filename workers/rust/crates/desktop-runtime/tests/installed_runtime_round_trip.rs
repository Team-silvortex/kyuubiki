#![cfg(unix)]

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn fixture_root() -> PathBuf {
    std::env::temp_dir().join(format!(
        "kyuubiki-installed-runtime-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ))
}

fn runtime_command(binary: &Path, root: &Path, action: &str) -> std::process::Output {
    runtime_command_with(binary, root, action, &[])
}

fn runtime_command_with(
    binary: &Path,
    root: &Path,
    action: &str,
    env: &[(&str, &str)],
) -> std::process::Output {
    Command::new(binary)
        .arg(action)
        .env("KYUUBIKI_RUNTIME_ROOT", root)
        .env("KYUUBIKI_RUNTIME_STATE_ROOT", state_root(root))
        .env("KYUUBIKI_AGENT_ENDPOINTS", "127.0.0.1:5001,127.0.0.1:5002")
        .envs(env.iter().copied())
        .output()
        .unwrap()
}

fn state_root(root: &Path) -> PathBuf {
    root.with_file_name(format!(
        "{}-state",
        root.file_name().unwrap().to_string_lossy()
    ))
}

#[test]
fn installed_runtime_can_run_agents_and_orchestrator_without_frontend() {
    let root = fixture_root();
    fs::create_dir_all(root.join("manifests")).unwrap();
    fs::create_dir_all(root.join("bin")).unwrap();
    fs::copy(
        env!("CARGO_BIN_EXE_kyuubiki-runtime-test-listener"),
        root.join("bin/service-listener"),
    )
    .unwrap();
    fs::write(
        root.join("manifests/service-launch.json"),
        r#"{
          "schema_version":"kyuubiki.service-launch/v1",
          "services":[
            {"id":"agent","command":"bin/service-listener","args":["{port}"],"cwd":"."},
            {"id":"orchestrator","command":"bin/service-listener","args":["6410"],"cwd":"."},
            {"id":"frontend","command":"bin/service-listener","args":["3000"],"cwd":"."}
          ]
        }"#,
    )
    .unwrap();
    fs::write(
        root.join("manifests/runtime-payload.json"),
        format!(
            r#"{{"schema_version":"kyuubiki.runtime-payload/v1","version":"test","platform":"{}"}}"#,
            kyuubiki_platform::Platform::current().as_str()
        ),
    )
    .unwrap();

    let runtime = Path::new(env!("CARGO_BIN_EXE_kyuubiki-runtime"));
    let options = [
        ("KYUUBIKI_ORCHESTRATOR_PORT", "6410"),
        ("KYUUBIKI_AGENT_ENDPOINTS", "127.0.0.1:6411,127.0.0.1:6412"),
        ("KYUUBIKI_RUNTIME_FRONTEND_DISABLED", "true"),
    ];
    let start = runtime_command_with(runtime, &root, "start-local", &options);
    if !start.status.success() {
        let _ = runtime_command_with(runtime, &root, "stop", &options);
        panic!(
            "installed runtime start failed: {}",
            String::from_utf8_lossy(&start.stderr)
        );
    }
    let status = runtime_command_with(runtime, &root, "status", &options);
    let rendered = String::from_utf8_lossy(&status.stdout);
    assert!(status.status.success(), "{rendered}");
    assert!(rendered.contains("orchestrator: running"));
    assert!(rendered.contains("agent[6411]: running"));
    assert!(rendered.contains("agent[6412]: running"));
    assert!(rendered.contains("frontend: disabled by runtime configuration"));
    assert!(!root.join("run/frontend.pid").exists());

    let stop = runtime_command_with(runtime, &root, "stop", &options);
    fs::remove_dir_all(&root).unwrap();
    fs::remove_dir_all(state_root(&root)).unwrap();
    assert!(
        stop.status.success(),
        "{}",
        String::from_utf8_lossy(&stop.stderr)
    );
}

#[test]
fn failed_orchestrator_start_rolls_back_new_agents() {
    let root = fixture_root();
    fs::create_dir_all(root.join("manifests")).unwrap();
    fs::create_dir_all(root.join("bin")).unwrap();
    fs::copy(
        env!("CARGO_BIN_EXE_kyuubiki-runtime-test-listener"),
        root.join("bin/service-listener"),
    )
    .unwrap();
    fs::copy(
        env!("CARGO_BIN_EXE_kyuubiki-runtime-test-listener"),
        root.join("bin/not-executable"),
    )
    .unwrap();
    fs::set_permissions(
        root.join("bin/not-executable"),
        fs::Permissions::from_mode(0o600),
    )
    .unwrap();
    fs::write(
        root.join("manifests/service-launch.json"),
        r#"{
          "schema_version":"kyuubiki.service-launch/v1",
          "services":[
            {"id":"agent","command":"bin/service-listener","args":["{port}"],"cwd":"."},
            {"id":"orchestrator","command":"bin/not-executable","args":[],"cwd":"."},
            {"id":"frontend","command":"bin/service-listener","args":["3000"],"cwd":"."}
          ]
        }"#,
    )
    .unwrap();
    fs::write(
        root.join("manifests/runtime-payload.json"),
        format!(
            r#"{{"schema_version":"kyuubiki.runtime-payload/v1","version":"test","platform":"{}"}}"#,
            kyuubiki_platform::Platform::current().as_str()
        ),
    )
    .unwrap();

    let runtime = Path::new(env!("CARGO_BIN_EXE_kyuubiki-runtime"));
    let options = [
        ("KYUUBIKI_ORCHESTRATOR_PORT", "6420"),
        ("KYUUBIKI_AGENT_ENDPOINTS", "127.0.0.1:6421,127.0.0.1:6422"),
        ("KYUUBIKI_RUNTIME_FRONTEND_DISABLED", "true"),
    ];
    let start = runtime_command_with(runtime, &root, "start-local", &options);
    assert!(!start.status.success());
    assert!(
        String::from_utf8_lossy(&start.stderr).contains("startup rollback"),
        "{}",
        String::from_utf8_lossy(&start.stderr)
    );
    let status = runtime_command_with(runtime, &root, "status", &options);
    let rendered = String::from_utf8_lossy(&status.stdout);
    assert!(rendered.contains("orchestrator: stopped"));
    assert!(rendered.contains("agent[6421]: stopped"));
    assert!(rendered.contains("agent[6422]: stopped"));
    assert!(!state_root(&root).join("run/agent-6421.pid").exists());
    assert!(!state_root(&root).join("run/agent-6422.pid").exists());

    fs::remove_dir_all(&root).unwrap();
    fs::remove_dir_all(state_root(&root)).unwrap();
}

#[test]
fn installed_runtime_starts_and_stops_without_source_toolchains() {
    let root = fixture_root();
    fs::create_dir_all(root.join("manifests")).unwrap();
    fs::create_dir_all(root.join("bin")).unwrap();
    fs::copy(
        env!("CARGO_BIN_EXE_kyuubiki-runtime-test-listener"),
        root.join("bin/service-listener"),
    )
    .unwrap();
    fs::write(
        root.join("manifests/service-launch.json"),
        r#"{
          "schema_version":"kyuubiki.service-launch/v1",
          "services":[
            {"id":"agent","command":"bin/service-listener","args":["{port}"],"cwd":"."},
            {"id":"orchestrator","command":"bin/service-listener","args":["4000"],"cwd":"."},
            {"id":"frontend","command":"bin/service-listener","args":["3000"],"cwd":"."}
          ]
        }"#,
    )
    .unwrap();
    fs::write(
        root.join("manifests/runtime-payload.json"),
        format!(
            r#"{{"schema_version":"kyuubiki.runtime-payload/v1","version":"test","platform":"{}"}}"#,
            kyuubiki_platform::Platform::current().as_str()
        ),
    )
    .unwrap();

    let runtime = Path::new(env!("CARGO_BIN_EXE_kyuubiki-runtime"));
    let start = runtime_command(runtime, &root, "start-local");
    if !start.status.success() {
        let _ = runtime_command(runtime, &root, "stop");
        panic!(
            "installed runtime start failed: {}",
            String::from_utf8_lossy(&start.stderr)
        );
    }
    let status = runtime_command(runtime, &root, "status");
    let rendered = String::from_utf8_lossy(&status.stdout);
    assert!(status.status.success(), "{rendered}");
    assert!(rendered.contains("runtime-policy: installer-managed"));
    assert!(rendered.contains("orchestrator: running"));
    assert!(rendered.contains("frontend: running"));
    assert!(rendered.contains("agent[5001]: running"));
    assert!(!rendered.contains("npm"));
    assert!(!rendered.contains("mix"));
    assert!(!rendered.contains("cargo"));
    assert!(!root.join("run").exists());
    assert!(!root.join("data").exists());

    let stop = runtime_command(runtime, &root, "stop");
    fs::remove_dir_all(&root).unwrap();
    fs::remove_dir_all(state_root(&root)).unwrap();
    assert!(
        stop.status.success(),
        "{}",
        String::from_utf8_lossy(&stop.stderr)
    );
}
