fn build_host_desktop_bundles() -> Result<String, String> {
    stage_release(parse_platform(None), None)?;
    Ok("desktop release staging completed; installed applications update through the Installer, not an npm build command".to_string())
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
    let shared_tauri_target = root
        .join("target")
        .join("desktop-cache")
        .join(current_desktop_platform());
    let legacy_tauri_target = root.join("apps").join(app_dir).join("src-tauri").join("target");
    let binary_name = current_platform_binary_name(crate_name);

    if cfg!(target_os = "macos") {
        let mut candidates = installed_app_candidates(product_name);
        for tauri_target in [shared_tauri_target, legacy_tauri_target] {
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
        }
        candidates
    } else if cfg!(target_os = "windows") {
        [shared_tauri_target, legacy_tauri_target]
            .into_iter()
            .flat_map(|tauri_target| {
                [
                    tauri_target.join("release").join(&binary_name),
                    tauri_target.join("debug").join(&binary_name),
                ]
            })
            .collect()
    } else {
        [shared_tauri_target, legacy_tauri_target]
            .into_iter()
            .flat_map(|tauri_target| {
                [
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
            })
            .collect()
    }
}

fn current_desktop_platform() -> &'static str {
    if cfg!(target_os = "macos") {
        "macos"
    } else if cfg!(target_os = "windows") {
        "windows"
    } else {
        "linux"
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
