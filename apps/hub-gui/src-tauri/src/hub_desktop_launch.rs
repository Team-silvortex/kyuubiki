fn run_npm(dir: &Path, args: &[&str]) -> Result<String, String> {
    let output = Command::new(npm_command())
        .current_dir(dir)
        .args(args)
        .output()
        .map_err(|error| format!("failed to run npm in {}: {error}", dir.display()))?;

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();

    if output.status.success() {
        let combined = [stdout, stderr]
            .into_iter()
            .filter(|value| !value.is_empty())
            .collect::<Vec<_>>()
            .join("\n");
        Ok(if combined.is_empty() {
            format!("npm {} succeeded in {}", args.join(" "), dir.display())
        } else {
            combined
        })
    } else {
        let combined = [stdout, stderr]
            .into_iter()
            .filter(|value| !value.is_empty())
            .collect::<Vec<_>>()
            .join("\n");
        Err(if combined.is_empty() {
            format!("npm {} failed in {}", args.join(" "), dir.display())
        } else {
            combined
        })
    }
}

fn build_host_desktop_bundles() -> Result<String, String> {
    let root = workspace_root();
    let mut lines = Vec::new();

    for (app, label) in [
        ("apps/hub-gui", "hub-gui"),
        ("apps/installer-gui", "installer-gui"),
        ("apps/workbench-gui", "workbench-gui"),
    ] {
        let dir = root.join(app);
        lines.push(format!("syncing shared desktop assets for {}", label));
        lines.push(run_npm(&dir, &["run", "sync:shared"])?);
        lines.push(format!("building host desktop bundle for {}", label));
        lines.push(run_npm(&dir, &["run", "tauri:build"])?);
    }

    Ok(lines.join("\n\n"))
}

fn spawn_background_command(mut command: Command, failure_context: &str) -> Result<(), String> {
    command
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .stdin(Stdio::null());

    command
        .spawn()
        .map_err(|error| format!("failed to {}: {error}", failure_context))?;

    Ok(())
}

fn current_platform_binary_name(crate_name: &str) -> String {
    if cfg!(target_os = "windows") {
        format!("{crate_name}.exe")
    } else {
        crate_name.to_string()
    }
}

fn installed_app_candidates(product_name: &str) -> Vec<PathBuf> {
    if !cfg!(target_os = "macos") {
        return Vec::new();
    }

    let app_name = format!("{product_name}.app");
    let mut candidates = vec![PathBuf::from("/Applications").join(&app_name)];
    if let Some(home) = std::env::var_os("HOME") {
        candidates.push(PathBuf::from(home).join("Applications").join(app_name));
    }
    candidates
}

fn built_app_candidates(app_dir: &str, product_name: &str, crate_name: &str) -> Vec<PathBuf> {
    let root = workspace_root();
    let tauri_target = root.join("apps").join(app_dir).join("src-tauri").join("target");
    let binary_name = current_platform_binary_name(crate_name);

    if cfg!(target_os = "macos") {
        let mut candidates = installed_app_candidates(product_name);
        candidates.extend([
            tauri_target
                .join("release")
                .join("bundle")
                .join("macos")
                .join(format!("{product_name}.app")),
            tauri_target
                .join("debug")
                .join("bundle")
                .join("macos")
                .join(format!("{product_name}.app")),
            tauri_target.join("release").join(&binary_name),
            tauri_target.join("debug").join(&binary_name),
        ]);
        candidates
    } else if cfg!(target_os = "windows") {
        vec![
            tauri_target.join("release").join(&binary_name),
            tauri_target.join("debug").join(&binary_name),
        ]
    } else {
        vec![
            tauri_target
                .join("release")
                .join("bundle")
                .join("appimage")
                .join(format!("{product_name}.AppImage")),
            tauri_target
                .join("debug")
                .join("bundle")
                .join("appimage")
                .join(format!("{product_name}.AppImage")),
            tauri_target.join("release").join(&binary_name),
            tauri_target.join("debug").join(&binary_name),
        ]
    }
}

fn launch_built_desktop_app(app_dir: &str, product_name: &str, crate_name: &str) -> Result<String, String> {
    let candidate = built_app_candidates(app_dir, product_name, crate_name)
        .into_iter()
        .find(|path| path.exists())
        .ok_or_else(|| format!("no built desktop app found for {}", product_name))?;

    if cfg!(target_os = "macos") {
        let mut command = Command::new("open");
        command.arg(&candidate);
        spawn_background_command(command, &format!("open {}", candidate.display()))?;
    } else if cfg!(target_os = "windows") {
        let mut command = Command::new("cmd");
        command.arg("/C").arg("start").arg("").arg(&candidate);
        spawn_background_command(command, &format!("open {}", candidate.display()))?;
    } else {
        let command = Command::new(&candidate);
        spawn_background_command(command, &format!("launch {}", candidate.display()))?;
    }

    Ok(format!("launched built {} from {}", product_name, candidate.display()))
}

fn launch_desktop_app_with_fallback(
    app_dir: &str,
    label: &str,
    product_name: &str,
    crate_name: &str,
) -> Result<String, String> {
    match launch_built_desktop_app(app_dir, product_name, crate_name) {
        Ok(message) => Ok(message),
        Err(error) => Err(format!(
            "{error}. Install {product_name} or build its desktop bundle first; Hub no longer falls back to the {label} dev shell."
        )),
    }
}

fn open_host_path(path: &Path) -> Result<String, String> {
    if cfg!(target_os = "macos") {
        let mut command = Command::new("open");
        command.arg(path);
        spawn_background_command(command, &format!("open {}", path.display()))?;
    } else if cfg!(target_os = "windows") {
        let mut command = Command::new("cmd");
        command.arg("/C").arg("start").arg("").arg(path);
        spawn_background_command(command, &format!("open {}", path.display()))?;
    } else {
        let mut command = Command::new("xdg-open");
        command.arg(path);
        spawn_background_command(command, &format!("open {}", path.display()))?;
    }

    Ok(format!("opened {}", path.display()))
}
