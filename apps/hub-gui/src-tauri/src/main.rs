use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::process::Stdio;

use kyuubiki_desktop_runtime::{
    hot_service_start as desktop_hot_service_start,
    hot_service_status as desktop_hot_service_status,
    hot_service_stop as desktop_hot_service_stop,
    read_runtime_log as read_shared_runtime_log,
    service_restart as desktop_service_restart,
    service_start as desktop_service_start,
    service_status as desktop_service_status,
    service_stop as desktop_service_stop,
    HotServiceMode,
    ServiceMode,
};
use kyuubiki_installer::{
    doctor_report as build_doctor_report, parse_platform, stage_release, validate_env_file, Platform,
};
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
struct ServiceStatusPayload {
    rendered: String,
}

#[derive(Serialize)]
struct HubEnvironmentPayload {
    hub_role: String,
    workbench_url: String,
    orchestrator_url: String,
    deployment_mode: String,
    host_platform: String,
    installer_gui_hint: String,
    workbench_gui_hint: String,
}

#[derive(Serialize)]
struct RuntimeLogPayload {
    service: String,
    rendered: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ServicePayload {
    mode: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct LogPayload {
    service: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PlatformPayload {
    platform: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProjectBundlePayload {
    path: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProjectBundleOutputPayload {
    path: String,
    out: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProjectBundleComparePayload {
    left_path: String,
    right_path: String,
}

fn resolve_service_mode(mode: Option<&str>) -> ServiceMode {
    match mode {
        Some("cloud") => ServiceMode::Cloud,
        Some("distributed") => ServiceMode::Distributed,
        Some("default") => ServiceMode::Default,
        _ => ServiceMode::Local,
    }
}

fn resolve_hot_service_mode(mode: Option<&str>) -> HotServiceMode {
    match mode {
        Some("cloud") => HotServiceMode::Cloud,
        Some("distributed") => HotServiceMode::Distributed,
        _ => HotServiceMode::Local,
    }
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .unwrap_or_else(|_| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.."))
}

fn npm_command() -> &'static str {
    if cfg!(target_os = "windows") {
        "npm.cmd"
    } else {
        "npm"
    }
}

fn required_icon_patterns(platform: Platform) -> &'static [&'static str] {
    match platform {
        Platform::Macos => &[".png", ".icns"],
        Platform::Linux => &[".png"],
        Platform::Windows => &[".png", ".ico"],
    }
}

fn expected_bundle_kinds(platform: Platform) -> &'static [&'static str] {
    match platform {
        Platform::Macos => &["app", "dmg"],
        Platform::Linux => &["appimage", "deb", "rpm"],
        Platform::Windows => &["msi", "nsis"],
    }
}

fn has_icon_with_suffix(dir: &Path, suffix: &str) -> bool {
    fs::read_dir(dir)
        .ok()
        .into_iter()
        .flat_map(|entries| entries.flatten())
        .any(|entry| {
            entry
                .path()
                .file_name()
                .and_then(|name| name.to_str())
                .map(|name| name.ends_with(suffix))
                .unwrap_or(false)
        })
}

fn verify_icons(root: &Path, app: &str, platform: Platform) -> Result<String, String> {
    let icon_dir = root.join("apps").join(app).join("src-tauri").join("icons");
    for suffix in required_icon_patterns(platform) {
        if !has_icon_with_suffix(&icon_dir, suffix) {
            return Err(format!(
                "missing {} icon input for {} under {}",
                suffix,
                app,
                icon_dir.display()
            ));
        }
    }

    Ok(format!("ok: {} icon inputs for {}", app, platform.as_str()))
}

fn verify_desktop_platform(platform: Platform) -> Result<String, String> {
    let root = workspace_root();
    let desktop_root = root.join("dist").join(platform.as_str()).join("desktop");
    if !desktop_root.is_dir() {
        return Err(format!(
            "missing staged desktop directory: {}",
            desktop_root.display()
        ));
    }

    let mut lines = Vec::new();
    for app in ["hub-gui", "installer-gui", "workbench-gui"] {
        let manifest_path = desktop_root.join(app).join("manifest.json");
        let manifest = fs::read_to_string(&manifest_path).map_err(|error| {
            format!("failed to read {}: {error}", manifest_path.display())
        })?;

        for kind in expected_bundle_kinds(platform) {
            if !manifest.contains(kind) {
                return Err(format!(
                    "missing bundle kind {} in {}",
                    kind,
                    manifest_path.display()
                ));
            }
        }

        lines.push(verify_icons(&root, app, platform)?);
    }

    lines.push(format!(
        "desktop release verification passed for {}",
        platform.as_str()
    ));
    Ok(lines.join("\n"))
}

fn desktop_runtime_stage_status(platform: Platform) -> &'static str {
    let root = workspace_root().join("dist").join(platform.as_str());
    if root.join("bin").is_dir() && root.join("config").is_dir() && root.join("desktop").is_dir() {
        "present"
    } else {
        "missing"
    }
}

fn desktop_manifest_status(platform: Platform, app: &str) -> &'static str {
    let manifest = workspace_root()
        .join("dist")
        .join(platform.as_str())
        .join("desktop")
        .join(app)
        .join("manifest.json");

    if manifest.is_file() {
        "present"
    } else {
        "missing"
    }
}

fn desktop_icon_status(platform: Platform, app: &str) -> String {
    let icon_dir = workspace_root()
        .join("apps")
        .join(app)
        .join("src-tauri")
        .join("icons");

    match platform {
        Platform::Macos => {
            if has_icon_with_suffix(&icon_dir, ".png") && has_icon_with_suffix(&icon_dir, ".icns") {
                "ready (.png + .icns)".to_string()
            } else {
                "missing macOS icons".to_string()
            }
        }
        Platform::Linux => {
            if has_icon_with_suffix(&icon_dir, ".png") {
                "ready (.png)".to_string()
            } else {
                "missing Linux icons".to_string()
            }
        }
        Platform::Windows => {
            if has_icon_with_suffix(&icon_dir, ".png") && has_icon_with_suffix(&icon_dir, ".ico") {
                "ready (.png + .ico)".to_string()
            } else {
                "missing Windows icons".to_string()
            }
        }
    }
}

fn desktop_host_bundle_status(app: &str, product_name: &str, crate_name: &str) -> &'static str {
    if built_app_candidates(app, product_name, crate_name)
        .into_iter()
        .any(|path| path.exists())
    {
        "present"
    } else {
        "missing"
    }
}

fn render_desktop_status_for_platform(platform: Platform) -> String {
    let mut lines = Vec::new();
    lines.push(String::new());
    lines.push(format!("platform: {}", platform.as_str()));
    lines.push(format!(
        "  runtime scaffold: {}",
        desktop_runtime_stage_status(platform)
    ));

    for (app, product_name, crate_name) in [
        ("hub-gui", "Kyuubiki Hub", "kyuubiki-hub-gui"),
        (
            "installer-gui",
            "Kyuubiki Installer",
            "kyuubiki-installer-gui",
        ),
        (
            "workbench-gui",
            "Kyuubiki Workbench",
            "kyuubiki-workbench-gui",
        ),
    ] {
        lines.push(format!(
            "  {:<16} manifest={:<8} icons={}",
            format!("{app}:"),
            desktop_manifest_status(platform, app),
            desktop_icon_status(platform, app)
        ));

        if platform == Platform::current() {
            lines.push(format!(
                "  {:<16} host-bundle={}",
                format!("{app}:"),
                desktop_host_bundle_status(app, product_name, crate_name)
            ));
        }
    }

    lines.push(format!(
        "  verification: {}",
        if verify_desktop_platform(platform).is_ok() {
            "ready"
        } else {
            "needs attention"
        }
    ));

    lines.join("\n")
}

fn desktop_status_text(target: Option<String>) -> String {
    let host = Platform::current();
    let normalized = target.unwrap_or_else(|| host.as_str().to_string());
    let mut lines = vec![
        "desktop packaging status".to_string(),
        format!("  host platform: {}", host.as_str()),
        format!("  dist root: {}", workspace_root().join("dist").display()),
    ];

    if normalized == "all" {
        for platform in [Platform::Macos, Platform::Linux, Platform::Windows] {
            lines.push(render_desktop_status_for_platform(platform));
        }
        lines.push(String::new());
        lines.push("next steps:".to_string());
        lines.push("  - Stage every platform scaffold: zsh ./scripts/kyuubiki desktop-stage all".to_string());
        lines.push("  - Build this host's desktop bundles: zsh ./scripts/kyuubiki desktop-build-host".to_string());
        lines.push("  - Verify manifests and icon inputs: zsh ./scripts/kyuubiki desktop-verify all".to_string());
        return lines.join("\n");
    }

    let platform = parse_platform(Some(normalized.clone()));
    lines.push(render_desktop_status_for_platform(platform));
    lines.push(String::new());
    lines.push("next steps:".to_string());

    if desktop_runtime_stage_status(platform) == "missing" {
        lines.push(format!(
            "  - Stage runtime + desktop manifests: zsh ./scripts/kyuubiki desktop-stage {}",
            platform.as_str()
        ));
    }

    if platform == host {
        lines.push("  - Build host-native Tauri bundles: zsh ./scripts/kyuubiki desktop-build-host".to_string());
        lines.push(format!(
            "  - Run the full host release pass: zsh ./scripts/kyuubiki desktop-release {}",
            platform.as_str()
        ));
    } else {
        lines.push(format!(
            "  - This host only stages {} manifests; build native bundles on a {} machine",
            platform.as_str(),
            platform.as_str()
        ));
        lines.push(format!(
            "  - Verify staged rollout descriptors: zsh ./scripts/kyuubiki desktop-verify {}",
            platform.as_str()
        ));
    }

    lines.join("\n")
}

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

fn launch_desktop_dev_app(app_dir: &str, label: &str) -> Result<String, String> {
    let root = workspace_root();
    let dir = root.join("apps").join(app_dir);

    let command = if cfg!(target_os = "windows") {
        let mut cmd = Command::new("cmd");
        cmd.arg("/C")
            .arg("npm run tauri:dev")
            .current_dir(&dir);
        cmd
    } else {
        let mut cmd = Command::new("zsh");
        cmd.arg("-lc")
            .arg("npm run tauri:dev")
            .current_dir(&dir);
        cmd
    };

    spawn_background_command(
        command,
        &format!("launch {} dev shell from {}", label, dir.display()),
    )?;

    Ok(format!("launched {} dev shell from {}", label, dir.display()))
}

fn current_platform_binary_name(crate_name: &str) -> String {
    if cfg!(target_os = "windows") {
        format!("{crate_name}.exe")
    } else {
        crate_name.to_string()
    }
}

fn built_app_candidates(app_dir: &str, product_name: &str, crate_name: &str) -> Vec<PathBuf> {
    let root = workspace_root();
    let tauri_target = root.join("apps").join(app_dir).join("src-tauri").join("target");
    let binary_name = current_platform_binary_name(crate_name);

    if cfg!(target_os = "macos") {
        vec![
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
        ]
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
        Err(_) => launch_desktop_dev_app(app_dir, label),
    }
}

fn node_command() -> &'static str {
    if cfg!(target_os = "windows") {
        "node.exe"
    } else {
        "node"
    }
}

fn run_project_cli(command: &str, input_path: &str) -> Result<String, String> {
    if input_path.trim().is_empty() {
        return Err("project bundle path is required".to_string());
    }

    let root = workspace_root();
    let script = root.join("apps").join("frontend").join("scripts").join("kyuubiki-cli.mjs");
    let output = Command::new(node_command())
        .current_dir(&root)
        .arg(script)
        .arg("project")
        .arg(command)
        .arg(input_path.trim())
        .arg("--json")
        .output()
        .map_err(|error| format!("failed to run project {}: {error}", command))?;

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();

    if output.status.success() {
        Ok(stdout)
    } else {
        Err(if stderr.is_empty() { stdout } else { stderr })
    }
}

fn run_project_cli_with_output(command: &str, input_path: &str, output_path: &str) -> Result<String, String> {
    if input_path.trim().is_empty() {
        return Err("project bundle path is required".to_string());
    }

    if output_path.trim().is_empty() {
        return Err("output path is required".to_string());
    }

    let root = workspace_root();
    let script = root.join("apps").join("frontend").join("scripts").join("kyuubiki-cli.mjs");
    let output = Command::new(node_command())
        .current_dir(&root)
        .arg(script)
        .arg("project")
        .arg(command)
        .arg(input_path.trim())
        .arg("--out")
        .arg(output_path.trim())
        .output()
        .map_err(|error| format!("failed to run project {}: {error}", command))?;

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();

    if output.status.success() {
        Ok(stdout)
    } else {
        Err(if stderr.is_empty() { stdout } else { stderr })
    }
}

fn run_project_cli_compare(command: &str, left_path: &str, right_path: &str) -> Result<String, String> {
    if left_path.trim().is_empty() || right_path.trim().is_empty() {
        return Err("both project bundle paths are required".to_string());
    }

    let root = workspace_root();
    let script = root.join("apps").join("frontend").join("scripts").join("kyuubiki-cli.mjs");
    let output = Command::new(node_command())
        .current_dir(&root)
        .arg(script)
        .arg("project")
        .arg(command)
        .arg(left_path.trim())
        .arg(right_path.trim())
        .arg("--json")
        .output()
        .map_err(|error| format!("failed to run project {}: {error}", command))?;

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();

    if output.status.success() {
        Ok(stdout)
    } else {
        Err(if stderr.is_empty() { stdout } else { stderr })
    }
}

#[tauri::command]
fn service_status() -> Result<ServiceStatusPayload, String> {
    Ok(ServiceStatusPayload {
        rendered: desktop_service_status()?,
    })
}

#[tauri::command]
fn service_start(payload: ServicePayload) -> Result<String, String> {
    desktop_service_start(resolve_service_mode(payload.mode.as_deref()))
}

#[tauri::command]
fn service_restart(payload: ServicePayload) -> Result<String, String> {
    desktop_service_restart(resolve_service_mode(payload.mode.as_deref()))
}

#[tauri::command]
fn service_stop() -> Result<String, String> {
    desktop_service_stop()
}

#[tauri::command]
fn hot_service_status() -> Result<ServiceStatusPayload, String> {
    Ok(ServiceStatusPayload {
        rendered: desktop_hot_service_status()?,
    })
}

#[tauri::command]
fn hot_service_start(payload: ServicePayload) -> Result<String, String> {
    desktop_hot_service_start(resolve_hot_service_mode(payload.mode.as_deref()))
}

#[tauri::command]
fn hot_service_stop() -> Result<String, String> {
    desktop_hot_service_stop()
}

#[tauri::command]
fn read_runtime_log(payload: LogPayload) -> Result<RuntimeLogPayload, String> {
    Ok(RuntimeLogPayload {
        service: payload.service.clone(),
        rendered: read_shared_runtime_log(&payload.service, 180)?,
    })
}

#[tauri::command]
fn doctor_report() -> Result<ServiceStatusPayload, String> {
    Ok(ServiceStatusPayload {
        rendered: build_doctor_report().render(),
    })
}

#[tauri::command]
fn validate_env() -> Result<String, String> {
    validate_env_file()
}

#[tauri::command]
fn desktop_stage(payload: PlatformPayload) -> Result<String, String> {
    let platform = parse_platform(payload.platform);
    stage_release(platform, None)
}

#[tauri::command]
fn desktop_verify(payload: PlatformPayload) -> Result<String, String> {
    let platform = parse_platform(payload.platform);
    verify_desktop_platform(platform)
}

#[tauri::command]
fn desktop_build_host() -> Result<String, String> {
    build_host_desktop_bundles()
}

#[tauri::command]
fn desktop_status(payload: PlatformPayload) -> Result<String, String> {
    Ok(desktop_status_text(payload.platform))
}

#[tauri::command]
fn project_bundle_inspect(payload: ProjectBundlePayload) -> Result<String, String> {
    run_project_cli("inspect", &payload.path)
}

#[tauri::command]
fn project_bundle_validate(payload: ProjectBundlePayload) -> Result<String, String> {
    run_project_cli("validate", &payload.path)
}

#[tauri::command]
fn project_bundle_normalize(payload: ProjectBundleOutputPayload) -> Result<String, String> {
    run_project_cli_with_output("normalize", &payload.path, &payload.out)
}

#[tauri::command]
fn project_bundle_unpack(payload: ProjectBundleOutputPayload) -> Result<String, String> {
    run_project_cli_with_output("unpack", &payload.path, &payload.out)
}

#[tauri::command]
fn project_bundle_pack(payload: ProjectBundleOutputPayload) -> Result<String, String> {
    run_project_cli_with_output("pack", &payload.path, &payload.out)
}

#[tauri::command]
fn project_bundle_diff(payload: ProjectBundleComparePayload) -> Result<String, String> {
    run_project_cli_compare("diff", &payload.left_path, &payload.right_path)
}

#[tauri::command]
fn launch_workbench_gui() -> Result<String, String> {
    launch_desktop_app_with_fallback(
        "workbench-gui",
        "workbench-gui",
        "Kyuubiki Workbench",
        "kyuubiki-workbench-gui",
    )
}

#[tauri::command]
fn launch_installer_gui() -> Result<String, String> {
    launch_desktop_app_with_fallback(
        "installer-gui",
        "installer-gui",
        "Kyuubiki Installer",
        "kyuubiki-installer-gui",
    )
}

#[tauri::command]
fn hub_environment() -> HubEnvironmentPayload {
    HubEnvironmentPayload {
        hub_role: "desktop-orchestration-shell".to_string(),
        workbench_url: "http://127.0.0.1:3000".to_string(),
        orchestrator_url: "http://127.0.0.1:4000".to_string(),
        deployment_mode: std::env::var("KYUUBIKI_DEPLOYMENT_MODE")
            .unwrap_or_else(|_| "local".to_string()),
        host_platform: Platform::current().as_str().to_string(),
        installer_gui_hint: "Use installer-gui for bootstrap and heavier deployment flows."
            .to_string(),
        workbench_gui_hint: "Use workbench-gui for focused modeling and analysis."
            .to_string(),
    }
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            service_status,
            service_start,
            service_restart,
            service_stop,
            hot_service_status,
            hot_service_start,
            hot_service_stop,
            read_runtime_log,
            doctor_report,
            validate_env,
            desktop_status,
            desktop_stage,
            desktop_verify,
            desktop_build_host,
            project_bundle_inspect,
            project_bundle_validate,
            project_bundle_normalize,
            project_bundle_unpack,
            project_bundle_pack,
            project_bundle_diff,
            launch_workbench_gui,
            launch_installer_gui,
            hub_environment
        ])
        .run(tauri::generate_context!())
        .expect("failed to run kyuubiki hub gui");
}
