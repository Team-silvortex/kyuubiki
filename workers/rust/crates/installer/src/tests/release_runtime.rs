use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::{
    Platform, build_embedded_runtime_manifest, build_launch_manifest, build_release_manifest,
    build_service_launch_manifest, embedded_runtime_report, expected_release_script_contents,
    find_workspace_root_from, linux_desktop_dependency_plan, release_env_path, workspace_root,
};

#[test]
fn release_manifest_contains_expected_schema() {
    let manifest = build_release_manifest(
        Path::new("/tmp/workspace"),
        Path::new("/tmp/dist/macos"),
        Platform::Macos,
    );
    assert!(manifest.contains("\"schema_version\": \"kyuubiki.release/v1\""));
    assert!(manifest.contains("\"platform\": \"macos\""));
    assert!(manifest.contains("\"release_dir\": \".\""));
    assert!(manifest.contains("\"workspace\": \"../..\""));
}

#[test]
fn launch_manifest_uses_portable_entrypoints() {
    let macos_manifest = build_launch_manifest(Path::new("/tmp/workspace"), Platform::Macos);
    let windows_manifest = build_launch_manifest(Path::new("/tmp/workspace"), Platform::Windows);
    assert!(macos_manifest.contains("\"entrypoint\": \"./scripts/start.sh\""));
    assert!(windows_manifest.contains("\"entrypoint\": \"./scripts/start.cmd\""));
    assert!(windows_manifest.contains("\"shell\": \"cmd\""));
}

#[test]
fn release_scripts_require_native_runtime_controller() {
    let macos_scripts = expected_release_script_contents(Platform::Macos);
    let start_script = macos_scripts
        .iter()
        .find(|(path, _)| path == "scripts/start.sh")
        .map(|(_, contents)| contents)
        .unwrap();
    assert!(start_script.contains("dist/macos/bin/kyuubiki-runtime"));
    assert!(start_script.contains("RUNTIME_BIN="));
    assert!(!start_script.contains("node"));

    let windows_scripts = expected_release_script_contents(Platform::Windows);
    let status_script = windows_scripts
        .iter()
        .find(|(path, _)| path == "scripts/status.cmd")
        .map(|(_, contents)| contents)
        .unwrap();
    assert!(status_script.contains("dist\\windows\\bin\\kyuubiki-runtime.exe"));
    assert!(status_script.contains("set RUNTIME_BIN="));
    assert!(!status_script.contains("node"));
}

#[test]
fn embedded_runtime_manifest_declares_self_host_payloads() {
    let root = workspace_root();
    let manifest = build_embedded_runtime_manifest(&root, Platform::Linux).unwrap();
    assert!(manifest.contains("\"schema_version\": \"kyuubiki.embedded-runtimes/v1\""));
    assert!(manifest.contains("\"id\": \"elixir-otp\""));
    assert!(manifest.contains("\"id\": \"node\""));
    assert!(manifest.contains("\"required_for_self_host\": true"));
    assert!(manifest.contains("\"source_contract\": \"config/toolchains.json#/elixir\""));
}

#[test]
fn embedded_runtime_report_renders_contract_summary() {
    let report = embedded_runtime_report().unwrap();
    let rendered = report.render();
    assert!(rendered.contains("kyuubiki embedded runtimes"));
    assert!(rendered.contains("elixir-otp"));
    assert!(rendered.contains("node"));
}

#[test]
fn service_launch_manifest_never_falls_back_to_source_tools() {
    for platform in [Platform::Macos, Platform::Linux, Platform::Windows] {
        let rendered = build_service_launch_manifest(platform);
        let manifest: serde_json::Value = serde_json::from_str(&rendered).unwrap();
        assert_eq!(manifest["schema_version"], "kyuubiki.service-launch/v1");
        assert_eq!(manifest["policy"]["source_fallback"], false);
        let services = manifest["services"].as_array().unwrap();
        assert_eq!(services.len(), 3);
        assert!(services.iter().any(|service| service["id"] == "agent"));
        let orchestrator = services
            .iter()
            .find(|service| service["id"] == "orchestrator")
            .unwrap();
        assert_eq!(orchestrator["args"], serde_json::json!(["start"]));
        assert_ne!(orchestrator["args"], serde_json::json!(["daemon"]));
        assert!(
            rendered.contains("services/frontend/server.js"),
            "{platform:?}"
        );
        for forbidden in ["npm run dev", "mix run", "cargo run", "apps/frontend"] {
            assert!(!rendered.contains(forbidden), "{platform:?}: {forbidden}");
        }
    }
}

#[test]
fn linux_desktop_dependency_plan_declares_tauri_ubuntu_prerequisites() {
    let plan = linux_desktop_dependency_plan();
    assert_eq!(
        plan.schema_version,
        "kyuubiki.linux-desktop-dependencies/v1"
    );
    assert!(plan.node_runtime.contains("node-v20.19.2-linux-x64"));
    assert!(
        plan.apt_packages
            .iter()
            .any(|package| package == "libwebkit2gtk-4.1-dev")
    );
    assert!(
        plan.apt_packages
            .iter()
            .any(|package| package == "libgtk-3-dev")
    );
    assert!(
        plan.apt_packages
            .iter()
            .any(|package| package == "librsvg2-dev")
    );
    assert!(
        plan.apt_packages
            .iter()
            .any(|package| package == "patchelf")
    );
    assert_eq!(
        plan.preflight_command,
        "make desktop-linux-remote-preflight"
    );
    assert!(plan.render().contains("installer-managed remote execution"));
}

#[test]
fn release_staging_uses_example_when_local_override_is_absent() {
    let root = std::env::temp_dir().join(format!(
        "kyuubiki-release-env-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ));
    fs::create_dir_all(&root).expect("fixture root");
    fs::write(
        root.join(".env.example"),
        "KYUUBIKI_STORAGE_BACKEND=sqlite\n",
    )
    .expect("example env");
    assert_eq!(release_env_path(&root), root.join(".env.example"));

    fs::write(root.join(".env.local"), "KYUUBIKI_STORAGE_BACKEND=sqlite\n").expect("local env");
    assert_eq!(release_env_path(&root), root.join(".env.local"));
    fs::remove_dir_all(root).expect("remove fixture");
}

#[test]
fn workspace_discovery_is_independent_of_the_compilation_directory() {
    let root = std::env::temp_dir().join(format!(
        "kyuubiki-workspace-discovery-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ));
    let nested = root.join("cache/target/release");
    fs::create_dir_all(root.join("workers/rust")).expect("rust marker parent");
    fs::create_dir_all(root.join("config")).expect("config marker parent");
    fs::create_dir_all(&nested).expect("nested executable parent");
    fs::write(
        root.join(".env.example"),
        "KYUUBIKI_STORAGE_BACKEND=sqlite\n",
    )
    .expect("environment marker");
    fs::write(root.join("workers/rust/Cargo.toml"), "[workspace]\n").expect("workspace marker");
    fs::write(root.join("config/toolchains.json"), "{}\n").expect("toolchain marker");

    assert_eq!(find_workspace_root_from(&nested), Some(root.clone()));
    fs::remove_dir_all(root).expect("remove fixture");
}
