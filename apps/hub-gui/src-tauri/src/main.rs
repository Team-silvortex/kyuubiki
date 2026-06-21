use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::process::Stdio;

use kyuubiki_desktop_runtime::{
    append_desktop_audit_line as desktop_append_audit_line,
    hot_service_start as desktop_hot_service_start,
    hot_service_status as desktop_hot_service_status,
    hot_service_stop as desktop_hot_service_stop,
    read_global_language_preference as desktop_read_global_language_preference,
    read_runtime_log as read_shared_runtime_log,
    service_restart as desktop_service_restart,
    service_start as desktop_service_start,
    service_status as desktop_service_status,
    service_stop as desktop_service_stop,
    summarize_service_status as desktop_summarize_service_status,
    write_global_language_preference as desktop_write_global_language_preference,
    ServiceStatusSummary,
    HotServiceMode,
    ServiceMode,
};
use kyuubiki_installer::{
    doctor_report as build_doctor_report, parse_platform, stage_release, validate_env_file, Platform,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::time::{SystemTime, UNIX_EPOCH};

const HUB_GUARDED_MUTATION_AUDIT_FILE: &str = "hub-guarded-mutations.jsonl";

#[derive(Serialize)]
struct ServiceStatusPayload {
    rendered: String,
    summary: ServiceStatusSummary,
}

#[derive(Serialize)]
struct TextReportPayload {
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
struct DirectMeshRegressionSnapshotPayload {
    baseline_path: String,
    output_root: String,
    baseline_mean_elapsed_s: f64,
    baseline_mean_rss_kib: f64,
    repeat: u64,
    docker_run_network: String,
    latest_exists: bool,
    latest_generated_at: Option<String>,
    latest_mean_elapsed_s: Option<f64>,
    latest_mean_rss_kib: Option<f64>,
    elapsed_delta_pct: Option<f64>,
    rss_delta_pct: Option<f64>,
    status: String,
}

#[derive(Deserialize, Serialize)]
struct RegressionGateLanePayload {
    id: String,
    title: String,
    category: String,
    status: String,
    gate_status: String,
    gate_reasons: Vec<String>,
    generated_at_unix_s: u64,
    links: Vec<String>,
}

#[derive(Deserialize, Serialize)]
struct RegressionGateReportPayload {
    schema_version: String,
    generated_at_unix_s: u64,
    catalog_path: String,
    overall_gate_status: String,
    failing_lane_count: usize,
    warning_lane_count: usize,
    lanes: Vec<RegressionGateLanePayload>,
    #[serde(default)]
    rendered: String,
}

#[derive(Serialize)]
struct RuntimeLogPayload {
    service: String,
    rendered: String,
}

#[derive(Serialize)]
struct DesktopPreferencesPayload {
    language: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct LogPayload {
    service: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct DesktopPreferencesInputPayload {
    language: String,
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
struct ProjectBundleComparePayload {
    left_path: String,
    right_path: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct GuardedMutationPayload {
    action: String,
    mode: Option<String>,
    platform: Option<String>,
    path: Option<String>,
    out: Option<String>,
    left_path: Option<String>,
    right_path: Option<String>,
}

fn audit_timestamp() -> String {
    match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(duration) => format!("{}.{}", duration.as_secs(), duration.subsec_nanos()),
        Err(_) => "0.0".to_string(),
    }
}

fn append_guarded_mutation_audit(
    payload: &GuardedMutationPayload,
    status: &str,
    detail: &str,
) -> Result<(), String> {
    let line = json!({
        "timestamp": audit_timestamp(),
        "action": payload.action,
        "status": status,
        "detail": detail,
        "mode": payload.mode,
        "platform": payload.platform,
        "path": payload.path,
        "out": payload.out,
        "left_path": payload.left_path,
        "right_path": payload.right_path,
    })
    .to_string();
    desktop_append_audit_line(HUB_GUARDED_MUTATION_AUDIT_FILE, &line)
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
        lines.push("  - Stage every platform scaffold: ./scripts/kyuubiki desktop-stage all".to_string());
        lines.push("  - Build this host's desktop bundles: ./scripts/kyuubiki desktop-build-host".to_string());
        lines.push("  - Verify manifests and icon inputs: ./scripts/kyuubiki desktop-verify all".to_string());
        lines.push("  - Review staged bundle manifests under: dist/<host>/desktop/*/artifacts.json".to_string());
        return lines.join("\n");
    }

    let platform = parse_platform(Some(normalized.clone()));
    lines.push(render_desktop_status_for_platform(platform));
    lines.push(String::new());
    lines.push("next steps:".to_string());

    if desktop_runtime_stage_status(platform) == "missing" {
        lines.push(format!(
            "  - Stage runtime + desktop manifests: ./scripts/kyuubiki desktop-stage {}",
            platform.as_str()
        ));
    }

    if platform == host {
        lines.push("  - Build host-native Tauri bundles: ./scripts/kyuubiki desktop-build-host".to_string());
        lines.push(format!(
            "  - Run the full host release pass: ./scripts/kyuubiki desktop-release {}",
            platform.as_str()
        ));
        lines.push(format!(
            "  - Review staged bundle manifests under: dist/{}/desktop/*/artifacts.json",
            host.as_str()
        ));
    } else {
        lines.push(format!(
            "  - This host only stages {} manifests; build native bundles on a {} machine",
            platform.as_str(),
            platform.as_str()
        ));
        lines.push(format!(
            "  - Verify staged rollout descriptors: ./scripts/kyuubiki desktop-verify {}",
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
        let mut cmd = Command::new("sh");
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

fn hub_docs_file(relative: &str) -> PathBuf {
    workspace_root()
        .join("apps")
        .join("hub-gui")
        .join("ui")
        .join("docs")
        .join(relative)
}

fn direct_mesh_baseline_file() -> PathBuf {
    workspace_root()
        .join("tests")
        .join("integration")
        .join("benchmarks")
        .join("direct-mesh-docker-baseline.json")
}

fn direct_mesh_output_root() -> PathBuf {
    workspace_root()
        .join("tmp")
        .join("direct-mesh-benchmark-container")
        .join("latest")
}

fn json_number_field(value: &serde_json::Value, path: &[&str]) -> Result<f64, String> {
    let mut current = value;
    for key in path {
        current = current
            .get(*key)
            .ok_or_else(|| format!("missing json field {}", path.join(".")))?;
    }
    current
        .as_f64()
        .ok_or_else(|| format!("json field {} is not a number", path.join(".")))
}

fn json_u64_field(value: &serde_json::Value, path: &[&str]) -> Result<u64, String> {
    let mut current = value;
    for key in path {
        current = current
            .get(*key)
            .ok_or_else(|| format!("missing json field {}", path.join(".")))?;
    }
    current
        .as_u64()
        .ok_or_else(|| format!("json field {} is not an unsigned integer", path.join(".")))
}

fn json_string_field(value: &serde_json::Value, path: &[&str]) -> Result<String, String> {
    let mut current = value;
    for key in path {
        current = current
            .get(*key)
            .ok_or_else(|| format!("missing json field {}", path.join(".")))?;
    }
    current
        .as_str()
        .map(ToOwned::to_owned)
        .ok_or_else(|| format!("json field {} is not a string", path.join(".")))
}

fn read_json_file(path: &Path) -> Result<serde_json::Value, String> {
    let raw = fs::read_to_string(path)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    serde_json::from_str(&raw)
        .map_err(|error| format!("failed to parse {}: {error}", path.display()))
}

fn direct_mesh_regression_snapshot() -> Result<DirectMeshRegressionSnapshotPayload, String> {
    let baseline_path = direct_mesh_baseline_file();
    let output_root = direct_mesh_output_root();
    let latest_summary_path = output_root.join("summary.json");

    let baseline = read_json_file(&baseline_path)?;
    let baseline_mean_elapsed_s =
        json_number_field(&baseline, &["aggregate", "elapsed_s", "mean"])?;
    let baseline_mean_rss_kib =
        json_number_field(&baseline, &["aggregate", "max_rss_kib", "mean"])?;
    let repeat = json_u64_field(&baseline, &["source", "repeat"])?;
    let docker_run_network = json_string_field(&baseline, &["source", "docker_run_network"])?;

    if !latest_summary_path.is_file() {
        return Ok(DirectMeshRegressionSnapshotPayload {
            baseline_path: baseline_path.display().to_string(),
            output_root: output_root.display().to_string(),
            baseline_mean_elapsed_s,
            baseline_mean_rss_kib,
            repeat,
            docker_run_network,
            latest_exists: false,
            latest_generated_at: None,
            latest_mean_elapsed_s: None,
            latest_mean_rss_kib: None,
            elapsed_delta_pct: None,
            rss_delta_pct: None,
            status: "baseline_only".to_string(),
        });
    }

    let latest = read_json_file(&latest_summary_path)?;
    let latest_mean_elapsed_s = json_number_field(&latest, &["aggregate", "elapsed_s", "mean"])?;
    let latest_mean_rss_kib = json_number_field(&latest, &["aggregate", "max_rss_kib", "mean"])?;
    let latest_generated_at = latest
        .get("generated_at")
        .and_then(|value| value.as_str())
        .map(ToOwned::to_owned);
    let elapsed_delta_pct =
        ((latest_mean_elapsed_s - baseline_mean_elapsed_s) / baseline_mean_elapsed_s) * 100.0;
    let rss_delta_pct =
        ((latest_mean_rss_kib - baseline_mean_rss_kib) / baseline_mean_rss_kib) * 100.0;
    let status = if elapsed_delta_pct > 5.0 || rss_delta_pct > 10.0 {
        "regressed"
    } else {
        "within_baseline"
    };

    Ok(DirectMeshRegressionSnapshotPayload {
        baseline_path: baseline_path.display().to_string(),
        output_root: output_root.display().to_string(),
        baseline_mean_elapsed_s,
        baseline_mean_rss_kib,
        repeat,
        docker_run_network,
        latest_exists: true,
        latest_generated_at,
        latest_mean_elapsed_s: Some(latest_mean_elapsed_s),
        latest_mean_rss_kib: Some(latest_mean_rss_kib),
        elapsed_delta_pct: Some(elapsed_delta_pct),
        rss_delta_pct: Some(rss_delta_pct),
        status: status.to_string(),
    })
}

fn node_command() -> &'static str {
    if cfg!(target_os = "windows") {
        "node.exe"
    } else {
        "node"
    }
}

fn nonempty_trimmed_path<'a>(value: &'a str, label: &str) -> Result<&'a str, String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        Err(format!("{label} is required"))
    } else {
        Ok(trimmed)
    }
}

fn normalize_existing_path(value: &str, label: &str) -> Result<PathBuf, String> {
    let trimmed = nonempty_trimmed_path(value, label)?;
    let candidate = PathBuf::from(trimmed);
    if !candidate.exists() {
        return Err(format!("{label} does not exist: {}", candidate.display()));
    }
    candidate
        .canonicalize()
        .map_err(|error| format!("failed to resolve {label} {}: {error}", candidate.display()))
}

fn path_has_extension(path: &Path, extension: &str) -> bool {
    path.extension()
        .and_then(|value| value.to_str())
        .map(|value| value.eq_ignore_ascii_case(extension))
        .unwrap_or(false)
}

fn normalize_existing_bundle_path(value: &str, label: &str) -> Result<PathBuf, String> {
    let path = normalize_existing_path(value, label)?;
    if !path.is_file() {
        return Err(format!("{label} must point to a project bundle file"));
    }
    if !path_has_extension(&path, "kyuubiki") {
        return Err(format!("{label} must end with .kyuubiki"));
    }
    Ok(path)
}

fn normalize_existing_directory_path(value: &str, label: &str) -> Result<PathBuf, String> {
    let path = normalize_existing_path(value, label)?;
    if !path.is_dir() {
        return Err(format!("{label} must point to a directory"));
    }
    Ok(path)
}

fn normalize_output_path(value: &str, label: &str) -> Result<PathBuf, String> {
    let trimmed = nonempty_trimmed_path(value, label)?;
    let candidate = PathBuf::from(trimmed);
    let parent = candidate
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| workspace_root());
    if !parent.exists() {
        return Err(format!(
            "{label} parent directory does not exist: {}",
            parent.display()
        ));
    }
    parent
        .canonicalize()
        .map_err(|error| format!("failed to resolve {label} parent {}: {error}", parent.display()))?;
    Ok(candidate)
}

fn ensure_distinct_paths(left: &Path, right: &Path, message: &str) -> Result<(), String> {
    if left == right {
        Err(message.to_string())
    } else {
        Ok(())
    }
}

fn run_project_cli(command: &str, input_path: &str) -> Result<String, String> {
    let normalized_input = normalize_existing_bundle_path(input_path, "project bundle path")?;

    let root = workspace_root();
    let script = root.join("apps").join("frontend").join("scripts").join("kyuubiki-cli.mjs");
    let output = Command::new(node_command())
        .current_dir(&root)
        .arg(script)
        .arg("project")
        .arg(command)
        .arg(&normalized_input)
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
    let normalized_input = match command {
        "pack" => normalize_existing_directory_path(input_path, "project directory path")?,
        _ => normalize_existing_bundle_path(input_path, "project bundle path")?,
    };
    let normalized_output = normalize_output_path(output_path, "output path")?;

    match command {
        "normalize" => {
            if !path_has_extension(&normalized_output, "kyuubiki") {
                return Err("output path for project normalize must end with .kyuubiki".to_string());
            }
            ensure_distinct_paths(
                &normalized_input,
                &normalized_output,
                "output path must be different from the input bundle path",
            )?;
        }
        "unpack" => {
            if path_has_extension(&normalized_output, "kyuubiki") {
                return Err("output path for project unpack must be a directory path, not a .kyuubiki bundle".to_string());
            }
        }
        "pack" => {
            if !path_has_extension(&normalized_output, "kyuubiki") {
                return Err("output path for project pack must end with .kyuubiki".to_string());
            }
        }
        _ => {}
    }

    let root = workspace_root();
    let script = root.join("apps").join("frontend").join("scripts").join("kyuubiki-cli.mjs");
    let output = Command::new(node_command())
        .current_dir(&root)
        .arg(script)
        .arg("project")
        .arg(command)
        .arg(&normalized_input)
        .arg("--out")
        .arg(&normalized_output)
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
    let normalized_left = normalize_existing_bundle_path(left_path, "left project bundle path")?;
    let normalized_right = normalize_existing_bundle_path(right_path, "right project bundle path")?;
    ensure_distinct_paths(
        &normalized_left,
        &normalized_right,
        "left and right project bundle paths must be different",
    )?;

    let root = workspace_root();
    let script = root.join("apps").join("frontend").join("scripts").join("kyuubiki-cli.mjs");
    let output = Command::new(node_command())
        .current_dir(&root)
        .arg(script)
        .arg("project")
        .arg(command)
        .arg(&normalized_left)
        .arg(&normalized_right)
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
    let rendered = desktop_service_status()?;
    Ok(ServiceStatusPayload {
        summary: desktop_summarize_service_status(&rendered),
        rendered,
    })
}

#[tauri::command]
fn get_global_language_preference() -> DesktopPreferencesPayload {
    DesktopPreferencesPayload {
        language: desktop_read_global_language_preference().unwrap_or_else(|| "en".to_string()),
    }
}

#[tauri::command]
fn set_global_language_preference(payload: DesktopPreferencesInputPayload) -> Result<DesktopPreferencesPayload, String> {
    Ok(DesktopPreferencesPayload {
        language: desktop_write_global_language_preference(&payload.language)?,
    })
}

#[tauri::command]
fn hot_service_status() -> Result<TextReportPayload, String> {
    Ok(TextReportPayload {
        rendered: desktop_hot_service_status()?,
    })
}

#[tauri::command]
fn read_runtime_log(payload: LogPayload) -> Result<RuntimeLogPayload, String> {
    Ok(RuntimeLogPayload {
        service: payload.service.clone(),
        rendered: read_shared_runtime_log(&payload.service, 180)?,
    })
}

#[tauri::command]
fn doctor_report() -> Result<TextReportPayload, String> {
    Ok(TextReportPayload {
        rendered: build_doctor_report().render(),
    })
}

#[tauri::command]
fn guarded_mutation_action(payload: GuardedMutationPayload) -> Result<String, String> {
    let result = match payload.action.as_str() {
        "service_start" => desktop_service_start(resolve_service_mode(payload.mode.as_deref())),
        "service_restart" => desktop_service_restart(resolve_service_mode(payload.mode.as_deref())),
        "service_stop" => desktop_service_stop(),
        "hot_service_start" => desktop_hot_service_start(resolve_hot_service_mode(payload.mode.as_deref())),
        "hot_service_stop" => desktop_hot_service_stop(),
        "validate_env" => validate_env_file(),
        "desktop_stage" => stage_release(parse_platform(payload.platform.clone()), None),
        "desktop_verify" => verify_desktop_platform(parse_platform(payload.platform.clone())),
        "desktop_build_host" => build_host_desktop_bundles(),
        "project_bundle_normalize" => run_project_cli_with_output(
            "normalize",
            payload.path.as_deref().unwrap_or(""),
            payload.out.as_deref().unwrap_or(""),
        ),
        "project_bundle_unpack" => run_project_cli_with_output(
            "unpack",
            payload.path.as_deref().unwrap_or(""),
            payload.out.as_deref().unwrap_or(""),
        ),
        "project_bundle_pack" => run_project_cli_with_output(
            "pack",
            payload.path.as_deref().unwrap_or(""),
            payload.out.as_deref().unwrap_or(""),
        ),
        _ => Err(format!("unsupported guarded mutation action: {}", payload.action)),
    };

    match &result {
        Ok(message) => {
            let _ = append_guarded_mutation_audit(&payload, "ok", message);
        }
        Err(error) => {
            let _ = append_guarded_mutation_audit(&payload, "failed", error);
        }
    }

    result
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
fn open_docs_index() -> Result<String, String> {
    open_host_path(&hub_docs_file("index.html"))
}

#[tauri::command]
fn open_current_line_doc() -> Result<String, String> {
    open_host_path(&hub_docs_file("current-line.html"))
}

#[tauri::command]
fn open_operations_doc() -> Result<String, String> {
    open_host_path(&hub_docs_file("operations.html"))
}

#[tauri::command]
fn open_troubleshooting_doc() -> Result<String, String> {
    open_host_path(&hub_docs_file("troubleshooting.html"))
}

#[tauri::command]
fn open_accuracy_plan_doc() -> Result<String, String> {
    open_host_path(&hub_docs_file("accuracy-plan.html"))
}

#[tauri::command]
fn open_accuracy_baselines_doc() -> Result<String, String> {
    open_host_path(&hub_docs_file("accuracy-baselines.html"))
}

#[tauri::command]
fn open_testing_and_ci_doc() -> Result<String, String> {
    open_host_path(&hub_docs_file("testing-and-ci.html"))
}

#[tauri::command]
fn open_direct_mesh_baseline() -> Result<String, String> {
    open_host_path(
        &workspace_root()
            .join("tests")
            .join("integration")
            .join("benchmarks")
            .join("direct-mesh-docker-baseline.json"),
    )
}

#[tauri::command]
fn open_direct_mesh_output_dir() -> Result<String, String> {
    open_host_path(&direct_mesh_output_root())
}

#[tauri::command]
fn hub_direct_mesh_regression_snapshot() -> Result<DirectMeshRegressionSnapshotPayload, String> {
    direct_mesh_regression_snapshot()
}

#[tauri::command]
fn hub_regression_gate_report() -> Result<RegressionGateReportPayload, String> {
    let report_path = workspace_root().join("tmp").join("regression-gate-report.json");
    let content = fs::read_to_string(&report_path)
        .map_err(|error| format!("failed to read regression gate report: {error}"))?;
    let mut payload: RegressionGateReportPayload =
        serde_json::from_str(&content).map_err(|error| format!("invalid regression gate report: {error}"))?;
    payload.rendered = format!(
        "overall gate: {} | failing lanes: {} | warning lanes: {}",
        payload.overall_gate_status, payload.failing_lane_count, payload.warning_lane_count
    );
    Ok(payload)
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
            hot_service_status,
            read_runtime_log,
            doctor_report,
            desktop_status,
            guarded_mutation_action,
            project_bundle_inspect,
            project_bundle_validate,
            project_bundle_diff,
            launch_workbench_gui,
            launch_installer_gui,
            get_global_language_preference,
            set_global_language_preference,
            open_docs_index,
            open_current_line_doc,
            open_operations_doc,
            open_troubleshooting_doc,
            open_accuracy_plan_doc,
            open_accuracy_baselines_doc,
            open_testing_and_ci_doc,
            open_direct_mesh_baseline,
            open_direct_mesh_output_dir,
            hub_direct_mesh_regression_snapshot,
            hub_regression_gate_report,
            hub_environment
        ])
        .run(tauri::generate_context!())
        .expect("failed to run kyuubiki hub gui");
}
