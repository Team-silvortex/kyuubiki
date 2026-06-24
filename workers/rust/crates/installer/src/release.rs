use std::fs;
use std::path::Path;

use crate::{Platform, escape_json};

pub(crate) fn write_release_scripts(release_dir: &Path, platform: Platform) -> Result<(), String> {
    for (relative, contents) in expected_release_script_contents(platform) {
        write_text_file(&release_dir.join(relative), &contents)?;
    }

    Ok(())
}

pub(crate) fn expected_release_script_contents(platform: Platform) -> Vec<(String, String)> {
    if platform == Platform::Windows {
        let node_path = format!(
            ".\\dist\\{}\\runtimes\\{}\\node\\node.exe",
            platform.as_str(),
            platform.as_str()
        );
        return vec![
            (
                "scripts/start.cmd".to_string(),
                windows_runtime_script(&node_path, "start"),
            ),
            (
                "scripts/stop.cmd".to_string(),
                windows_runtime_script(&node_path, "stop"),
            ),
            (
                "scripts/status.cmd".to_string(),
                windows_runtime_script(&node_path, "status"),
            ),
            (
                "scripts/export-db.cmd".to_string(),
                windows_runtime_script_with_redirect(
                    &node_path,
                    "export-db",
                    ".\\dist\\windows\\exports\\kyuubiki-database.json",
                ),
            ),
        ];
    }

    let node_path = format!(
        "./dist/{}/runtimes/{}/node/bin/node",
        platform.as_str(),
        platform.as_str()
    );
    vec![
        (
            "scripts/start.sh".to_string(),
            unix_runtime_script(&node_path, "start"),
        ),
        (
            "scripts/stop.sh".to_string(),
            unix_runtime_script(&node_path, "stop"),
        ),
        (
            "scripts/status.sh".to_string(),
            unix_runtime_script(&node_path, "status"),
        ),
        (
            "scripts/export-db.sh".to_string(),
            unix_runtime_script_with_redirect(
                &node_path,
                "export-db",
                &format!(
                    "./dist/{}/exports/kyuubiki-database.json",
                    platform.as_str()
                ),
            ),
        ),
    ]
}

fn unix_runtime_script(node_path: &str, command: &str) -> String {
    format!(
        "#!/usr/bin/env sh\nset -e\ncd \"$(dirname \"$0\")/../..\"\nNODE_BIN=\"{node_path}\"\nif [ ! -x \"$NODE_BIN\" ]; then NODE_BIN=\"node\"; fi\n\"$NODE_BIN\" ./scripts/kyuubiki-runtime.mjs {command}\n"
    )
}

fn unix_runtime_script_with_redirect(node_path: &str, command: &str, output_path: &str) -> String {
    format!(
        "#!/usr/bin/env sh\nset -e\ncd \"$(dirname \"$0\")/../..\"\nNODE_BIN=\"{node_path}\"\nif [ ! -x \"$NODE_BIN\" ]; then NODE_BIN=\"node\"; fi\n\"$NODE_BIN\" ./scripts/kyuubiki-runtime.mjs {command} > {output_path}\n"
    )
}

fn windows_runtime_script(node_path: &str, command: &str) -> String {
    format!(
        "@echo off\r\ncd /d %~dp0\\..\\..\r\nset NODE_BIN={node_path}\r\nif not exist \"%NODE_BIN%\" set NODE_BIN=node\r\n\"%NODE_BIN%\" .\\scripts\\kyuubiki-runtime.mjs {command}\r\n"
    )
}

fn windows_runtime_script_with_redirect(
    node_path: &str,
    command: &str,
    output_path: &str,
) -> String {
    format!(
        "@echo off\r\ncd /d %~dp0\\..\\..\r\nset NODE_BIN={node_path}\r\nif not exist \"%NODE_BIN%\" set NODE_BIN=node\r\n\"%NODE_BIN%\" .\\scripts\\kyuubiki-runtime.mjs {command} > {output_path}\r\n"
    )
}

pub(crate) fn build_release_manifest(
    _root: &Path,
    _release_dir: &Path,
    platform: Platform,
) -> String {
    format!(
        concat!(
            "{{\n",
            "  \"schema_version\": \"kyuubiki.release/v1\",\n",
            "  \"platform\": \"{platform}\",\n",
            "  \"release_dir\": \"{release_dir}\",\n",
            "  \"workspace\": \"{workspace}\",\n",
            "  \"layout\": [\n",
            "    \"bin\",\n",
            "    \"config\",\n",
            "    \"data\",\n",
            "    \"desktop\",\n",
            "    \"logs\",\n",
            "    \"exports\",\n",
            "    \"manifests\",\n",
            "    \"runtimes\",\n",
            "    \"scripts\"\n",
            "  ],\n",
            "  \"recommended_flow\": [\n",
            "    \"scripts/start\",\n",
            "    \"scripts/status\",\n",
            "    \"scripts/export-db\"\n",
            "  ]\n",
            "}}\n"
        ),
        platform = platform.as_str(),
        release_dir = escape_json(portable_release_dir()),
        workspace = portable_workspace_hint(),
    )
}

pub(crate) fn build_launch_manifest(_root: &Path, platform: Platform) -> String {
    format!(
        concat!(
            "{{\n",
            "  \"schema_version\": \"kyuubiki.launch/v1\",\n",
            "  \"platform\": \"{platform}\",\n",
            "  \"shell\": \"{shell}\",\n",
            "  \"workspace\": \"{workspace}\",\n",
            "  \"entrypoint\": \"{entry}\",\n",
            "  \"deployment_profiles\": [\"local\", \"cloud\", \"distributed\"],\n",
            "  \"agent_discovery\": [\"static\", \"manifest\"],\n",
            "  \"services\": [\n",
            "    {{\"name\": \"frontend\", \"port\": 3000}},\n",
            "    {{\"name\": \"orchestrator\", \"port\": 4000}},\n",
            "    {{\"name\": \"agent-1\", \"port\": 5001}},\n",
            "    {{\"name\": \"agent-2\", \"port\": 5002}}\n",
            "  ]\n",
            "}}\n"
        ),
        platform = platform.as_str(),
        shell = platform.default_shell(),
        workspace = portable_workspace_hint(),
        entry = escape_json(platform.entrypoint_command()),
    )
}

pub(crate) fn build_release_readme(platform: Platform) -> String {
    format!(
        concat!(
            "Kyuubiki portable release scaffold\n\n",
            "Platform: {platform}\n\n",
            "This directory is a repo-local deployment scaffold.\n",
            "It is intentionally lightweight and does not install host-level packages.\n\n",
            "Directory shape:\n",
            "- bin/        component binaries or launch helpers\n",
            "- config/     environment and deployment configuration\n",
            "- data/       local runtime state\n",
            "- desktop/    desktop-shell packaging placeholders\n",
            "- logs/       runtime logs\n",
            "- manifests/  release and launch manifests\n",
            "- runtimes/   installer-managed embedded language/runtime payloads\n",
            "- scripts/    operator entry points\n",
            "- exports/    snapshots and operator exports\n\n",
            "Suggested flow:\n",
            "1. copy config/.env.example to config/.env.local when packaging externally\n",
            "2. use scripts/start to launch services\n",
            "3. use scripts/export-db to snapshot persisted data\n"
        ),
        platform = platform.as_str()
    )
}

pub(crate) fn build_desktop_readme() -> String {
    concat!(
        "Desktop packaging placeholders\n\n",
        "hub-gui/\n",
        "  Reserved for the Tauri Hub shell build output or packaged bundle references.\n",
        "  Contains a manifest.json packaging descriptor.\n\n",
        "installer-gui/\n",
        "  Reserved for the Tauri installer GUI build output or packaged bundle references.\n",
        "  Contains a manifest.json packaging descriptor.\n\n",
        "workbench-gui/\n",
        "  Reserved for the Tauri desktop workbench shell build output or packaged bundle references.\n",
        "  Contains a manifest.json packaging descriptor.\n"
    )
    .to_string()
}

pub(crate) fn build_desktop_app_manifest(app: &str, platform: Platform) -> String {
    let (product_name, source_dir, target_dir, build_command) = match app {
        "hub-gui" => (
            "Kyuubiki Hub",
            "apps/hub-gui",
            "apps/hub-gui/src-tauri/target",
            "make build-hub-gui",
        ),
        "installer-gui" => (
            "Kyuubiki Installer",
            "apps/installer-gui",
            "apps/installer-gui/src-tauri/target",
            "make build-installer-gui",
        ),
        _ => (
            "Kyuubiki Workbench",
            "apps/workbench-gui",
            "apps/workbench-gui/src-tauri/target",
            "make build-workbench-gui",
        ),
    };

    format!(
        concat!(
            "{{\n",
            "  \"schema_version\": \"kyuubiki.desktop-package/v1\",\n",
            "  \"app\": \"{app}\",\n",
            "  \"product_name\": \"{product_name}\",\n",
            "  \"platform\": \"{platform}\",\n",
            "  \"expected_bundle_kinds\": [{bundle_kinds}],\n",
            "  \"source_dir\": \"{source_dir}\",\n",
            "  \"tauri_target_dir\": \"{target_dir}\",\n",
            "  \"build_command\": \"{build_command}\",\n",
            "  \"notes\": \"Use the platform-scoped desktop packaging flow to populate this placeholder.\"\n",
            "}}\n"
        ),
        app = app,
        product_name = product_name,
        platform = platform.as_str(),
        bundle_kinds = platform.desktop_bundle_kinds_json(),
        source_dir = source_dir,
        target_dir = target_dir,
        build_command = build_command,
    )
}

fn write_text_file(path: &Path, contents: &str) -> Result<(), String> {
    fs::write(path, contents)
        .map_err(|error| format!("failed to write {}: {error}", path.display()))
}

fn portable_release_dir() -> &'static str {
    "."
}

fn portable_workspace_hint() -> &'static str {
    "../.."
}
