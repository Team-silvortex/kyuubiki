use std::path::Path;

use crate::{
    Platform, build_embedded_runtime_manifest, build_launch_manifest, build_release_manifest,
    embedded_runtime_report, expected_release_script_contents, linux_desktop_dependency_plan,
    workspace_root,
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
fn release_scripts_prefer_embedded_node_runtime() {
    let macos_scripts = expected_release_script_contents(Platform::Macos);
    let start_script = macos_scripts
        .iter()
        .find(|(path, _)| path == "scripts/start.sh")
        .map(|(_, contents)| contents)
        .unwrap();
    assert!(start_script.contains("dist/macos/runtimes/macos/node/bin/node"));
    assert!(start_script.contains("NODE_BIN=\"node\""));

    let windows_scripts = expected_release_script_contents(Platform::Windows);
    let status_script = windows_scripts
        .iter()
        .find(|(path, _)| path == "scripts/status.cmd")
        .map(|(_, contents)| contents)
        .unwrap();
    assert!(status_script.contains("dist\\windows\\runtimes\\windows\\node\\node.exe"));
    assert!(status_script.contains("set NODE_BIN=node"));
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
