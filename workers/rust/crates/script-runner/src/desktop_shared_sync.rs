use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

use crate::RunnerResult;

const DESKTOP_APPS: [&str; 3] = ["hub-gui", "installer-gui", "workbench-gui"];
const SHARED_UI_FILES: [&str; 6] = [
    "desktop-shell.css",
    "desktop-shell-runtime-mesh.css",
    "platform.js",
    "runtime-status-model.js",
    "runtime-status-summary.js",
    "tauri-bridge.js",
];
const INSTALLER_PRIMARY_BUTTON_CSS: &str = "\n.desktop-shell-button-primary {\n  background: linear-gradient(180deg, rgba(255, 174, 72, 0.28), rgba(79, 84, 93, 0.96));\n  border-color: rgba(255, 174, 72, 0.34);\n}\n";

pub(crate) fn run_sync_desktop_shared(root: &Path) -> RunnerResult<u8> {
    compile_desktop_shared_typescript(root)?;
    sync_shared_assets(root)?;
    println!(
        "synced desktop shared assets to {}",
        DESKTOP_APPS.join(", ")
    );
    Ok(0)
}

fn compile_desktop_shared_typescript(root: &Path) -> RunnerResult<()> {
    let desktop_shared_dir = root.join("apps/desktop-shared");
    let tsc = tsc_bin(root);
    let status = crate::run_command(
        &desktop_shared_dir,
        tsc.to_string_lossy().as_ref(),
        [
            OsString::from("-p"),
            desktop_shared_dir.join("tsconfig.json").into_os_string(),
        ],
    )?;
    if status != 0 {
        return Err(format!(
            "desktop shared TypeScript compile failed with status {status}"
        ));
    }
    remove_if_exists(&desktop_shared_dir.join("ui/runtime-status-types.js"))?;
    Ok(())
}

fn sync_shared_assets(root: &Path) -> RunnerResult<()> {
    let brand_source = root.join("assets/brand/brand.json");
    let shared_ui_dir = root.join("apps/desktop-shared/ui");
    for app in DESKTOP_APPS {
        let shared_target_dir = root.join("apps").join(app).join("ui/shared");
        for file in SHARED_UI_FILES {
            copy_file(
                &shared_ui_dir.join(file),
                &shared_target_dir.join(file),
                "shared desktop UI asset",
            )?;
        }
        if app == "installer-gui" {
            append_file(
                &shared_target_dir.join("desktop-shell.css"),
                INSTALLER_PRIMARY_BUTTON_CSS,
            )?;
        }
        copy_file(
            &brand_source,
            &root.join("apps").join(app).join("ui/assets/brand.json"),
            "desktop brand asset",
        )?;
    }
    Ok(())
}

fn tsc_bin(root: &Path) -> PathBuf {
    root.join("apps/frontend/node_modules/.bin/tsc")
}

fn copy_file(source: &Path, target: &Path, label: &str) -> RunnerResult<()> {
    let parent = target
        .parent()
        .ok_or_else(|| format!("{label} target has no parent: {}", target.display()))?;
    fs::create_dir_all(parent)
        .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
    fs::copy(source, target).map_err(|error| {
        format!(
            "failed to copy {label} from {} to {}: {error}",
            source.display(),
            target.display()
        )
    })?;
    Ok(())
}

fn append_file(target: &Path, contents: &str) -> RunnerResult<()> {
    use std::io::Write;

    let mut file = fs::OpenOptions::new()
        .append(true)
        .open(target)
        .map_err(|error| format!("failed to open {} for append: {error}", target.display()))?;
    file.write_all(contents.as_bytes())
        .map_err(|error| format!("failed to append {}: {error}", target.display()))
}

fn remove_if_exists(target: &Path) -> RunnerResult<()> {
    match fs::remove_file(target) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(format!("failed to remove {}: {error}", target.display())),
    }
}
