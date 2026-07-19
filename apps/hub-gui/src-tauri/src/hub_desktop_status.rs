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
