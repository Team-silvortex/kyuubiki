use std::fs;
use std::path::Path;

use crate::{Platform, escape_json};

pub(crate) fn write_release_scripts(release_dir: &Path, platform: Platform) -> Result<(), String> {
    let scripts_dir = release_dir.join("scripts");

    if platform == Platform::Windows {
        write_text_file(
            &scripts_dir.join("start.cmd"),
            "@echo off\r\ncd /d %~dp0\\..\\..\r\nzsh ./scripts/kyuubiki start\r\n",
        )?;
        write_text_file(
            &scripts_dir.join("stop.cmd"),
            "@echo off\r\ncd /d %~dp0\\..\\..\r\nzsh ./scripts/kyuubiki stop\r\n",
        )?;
        write_text_file(
            &scripts_dir.join("status.cmd"),
            "@echo off\r\ncd /d %~dp0\\..\\..\r\nzsh ./scripts/kyuubiki status\r\n",
        )?;
        write_text_file(
            &scripts_dir.join("export-db.cmd"),
            "@echo off\r\ncd /d %~dp0\\..\\..\r\nzsh ./scripts/kyuubiki export-db > .\\exports\\kyuubiki-database.json\r\n",
        )?;
    } else {
        write_text_file(
            &scripts_dir.join("start.sh"),
            "#!/bin/zsh\nset -e\ncd \"$(dirname \"$0\")/../..\"\nzsh ./scripts/kyuubiki start\n",
        )?;
        write_text_file(
            &scripts_dir.join("stop.sh"),
            "#!/bin/zsh\nset -e\ncd \"$(dirname \"$0\")/../..\"\nzsh ./scripts/kyuubiki stop\n",
        )?;
        write_text_file(
            &scripts_dir.join("status.sh"),
            "#!/bin/zsh\nset -e\ncd \"$(dirname \"$0\")/../..\"\nzsh ./scripts/kyuubiki status\n",
        )?;
        write_text_file(
            &scripts_dir.join("export-db.sh"),
            "#!/bin/zsh\nset -e\ncd \"$(dirname \"$0\")/../..\"\nzsh ./scripts/kyuubiki export-db > ./dist/exports/kyuubiki-database.json\n",
        )?;
    }

    Ok(())
}

pub(crate) fn build_release_manifest(
    root: &Path,
    release_dir: &Path,
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
        release_dir = escape_json(&release_dir.display().to_string()),
        workspace = escape_json(&root.display().to_string()),
    )
}

pub(crate) fn build_launch_manifest(root: &Path, platform: Platform) -> String {
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
        workspace = escape_json(&root.display().to_string()),
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
