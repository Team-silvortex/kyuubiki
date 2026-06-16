use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

mod diagnostics;
mod remote;

use diagnostics::{
    doctor_report, installation_integrity_report, latest_applied_update_record,
    latest_downloaded_update_record, latest_staged_update_record, unified_update_plan,
    unified_update_preview, update_source_config,
};
use kyuubiki_desktop_runtime::{
    ServiceMode, ServiceStatusSummary, append_desktop_audit_line as desktop_append_audit_line,
    log_path_for, read_global_language_preference as desktop_read_global_language_preference,
    read_runtime_log as read_shared_runtime_log, service_restart as desktop_service_restart,
    service_start as desktop_service_start, service_status as desktop_service_status,
    service_stop as desktop_service_stop,
    summarize_service_status as desktop_summarize_service_status,
    write_global_language_preference as desktop_write_global_language_preference,
};
use kyuubiki_installer::{
    apply_downloaded_update as installer_apply_downloaded_update,
    download_update as installer_download_update, export_launch_config,
    init_env as installer_init_env, parse_platform, prepare_layout as installer_prepare_layout,
    prepare_staged_update as installer_prepare_staged_update,
    repair_installation as installer_repair_installation, stage_release as installer_stage_release,
    validate_env_file, workspace_root,
    write_update_source_config as installer_write_update_source_config,
};
use remote::{RemoteAgentPayload, RemoteBootstrapPayload, remote_bootstrap, remote_start_agent};
use serde::Serialize;
use serde_json::json;
use tauri::{AppHandle, Emitter};

#[derive(Serialize)]
struct EnvFormPayload {
    deployment_mode: String,
    agent_discovery: String,
    agent_manifest_path: String,
    storage_backend: String,
    sqlite_database_path: String,
    database_url: String,
    database_url_configured: bool,
    agent_endpoints: String,
    kyuubiki_api_token: String,
    kyuubiki_api_token_configured: bool,
    kyuubiki_cluster_api_token: String,
    kyuubiki_cluster_api_token_configured: bool,
    kyuubiki_cluster_allowed_agent_ids: String,
    kyuubiki_cluster_allowed_cluster_ids: String,
    kyuubiki_cluster_require_fingerprint: bool,
    kyuubiki_cluster_timestamp_window_ms: String,
    kyuubiki_protect_reads: bool,
    kyuubiki_direct_mesh_enabled: bool,
    kyuubiki_direct_mesh_token: String,
    kyuubiki_direct_mesh_token_configured: bool,
}

#[derive(Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct WriteEnvPayload {
    deployment_mode: String,
    agent_discovery: String,
    agent_manifest_path: String,
    storage_backend: String,
    sqlite_database_path: String,
    database_url: String,
    database_url_configured: bool,
    agent_endpoints: String,
    kyuubiki_api_token: String,
    kyuubiki_api_token_configured: bool,
    kyuubiki_cluster_api_token: String,
    kyuubiki_cluster_api_token_configured: bool,
    kyuubiki_cluster_allowed_agent_ids: String,
    kyuubiki_cluster_allowed_cluster_ids: String,
    kyuubiki_cluster_require_fingerprint: bool,
    kyuubiki_cluster_timestamp_window_ms: String,
    kyuubiki_protect_reads: bool,
    kyuubiki_direct_mesh_enabled: bool,
    kyuubiki_direct_mesh_token: String,
    kyuubiki_direct_mesh_token_configured: bool,
}

#[derive(Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct ReleasePayload {
    platform: String,
    target_dir: Option<String>,
}
#[derive(Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct PlatformPayload {
    platform: String,
}
#[derive(Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct LogPayload {
    service: String,
}
#[derive(Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct DesktopPreferencesInputPayload {
    language: String,
}
#[derive(Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct BuildPayload {
    bundle_mode: Option<String>,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct InstallerGuardedMutationPayload {
    action: String,
    channel: Option<String>,
    catalog_path: Option<String>,
    artifact_root: Option<String>,
    download_dir: Option<String>,
    mode: Option<String>,
    force: Option<bool>,
    platform: Option<String>,
    target_dir: Option<String>,
    bundle_mode: Option<String>,
    env_payload: Option<WriteEnvPayload>,
    remote_bootstrap: Option<RemoteBootstrapPayload>,
    remote_agent: Option<RemoteAgentPayload>,
}

#[derive(Serialize)]
struct ServiceStatusPayload {
    rendered: String,
    summary: ServiceStatusSummary,
}
#[derive(Serialize)]
struct DesktopPreferencesPayload {
    language: String,
}
#[derive(Clone, Serialize)]
struct RuntimeLogPayload {
    service: String,
    rendered: String,
}

static LOG_STREAMS: OnceLock<Mutex<HashMap<String, Arc<AtomicBool>>>> = OnceLock::new();

const INSTALLER_GUARDED_MUTATION_AUDIT_FILE: &str = "installer-guarded-mutations.jsonl";

fn log_streams() -> &'static Mutex<HashMap<String, Arc<AtomicBool>>> {
    LOG_STREAMS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn audit_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

fn append_installer_guarded_mutation_audit(
    payload: &InstallerGuardedMutationPayload,
    status: &str,
    detail: &str,
) {
    let record = json!({
        "ts": audit_timestamp(),
        "action": payload.action,
        "mode": payload.mode,
        "platform": payload.platform,
        "channel": payload.channel,
        "catalogPath": payload.catalog_path,
        "bundleMode": payload.bundle_mode,
        "status": status,
        "detail": detail,
    });
    let _ = desktop_append_audit_line(INSTALLER_GUARDED_MUTATION_AUDIT_FILE, &record.to_string());
}

fn parse_env_lines(contents: &str) -> HashMap<String, String> {
    let mut entries = HashMap::new();

    for raw_line in contents.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        if let Some((key, value)) = line.split_once('=') {
            entries.insert(key.trim().to_string(), value.trim().to_string());
        }
    }

    entries
}

fn resolve_sensitive_env_value(
    payload_value: &str,
    preserve_existing: bool,
    existing_entries: &HashMap<String, String>,
    key: &str,
) -> Option<String> {
    let trimmed = payload_value.trim();
    if !trimmed.is_empty() {
        return Some(trimmed.to_string());
    }

    if preserve_existing {
        return existing_entries
            .get(key)
            .cloned()
            .filter(|value| !value.trim().is_empty());
    }

    None
}

#[tauri::command]
fn export_launch(payload: PlatformPayload) -> Result<String, String> {
    Ok(export_launch_config(parse_platform(Some(payload.platform))))
}

#[tauri::command]
fn stage_release(payload: ReleasePayload) -> Result<String, String> {
    let target_dir = payload
        .target_dir
        .filter(|value| !value.trim().is_empty())
        .map(PathBuf::from);
    installer_stage_release(parse_platform(Some(payload.platform)), target_dir)
}

#[tauri::command]
fn read_env_file() -> Result<EnvFormPayload, String> {
    let path = workspace_root().join(".env.local");
    let contents = fs::read_to_string(&path)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;

    let mut deployment_mode = "local".to_string();
    let mut agent_discovery = "static".to_string();
    let mut agent_manifest_path = String::new();
    let mut storage_backend = "sqlite".to_string();
    let mut sqlite_database_path = String::new();
    let mut database_url_configured = false;
    let mut agent_endpoints = "127.0.0.1:5001,127.0.0.1:5002".to_string();
    let mut kyuubiki_api_token_configured = false;
    let mut kyuubiki_cluster_api_token_configured = false;
    let mut kyuubiki_cluster_allowed_agent_ids = String::new();
    let mut kyuubiki_cluster_allowed_cluster_ids = String::new();
    let mut kyuubiki_cluster_require_fingerprint = false;
    let mut kyuubiki_cluster_timestamp_window_ms = "30000".to_string();
    let mut kyuubiki_protect_reads = false;
    let mut kyuubiki_direct_mesh_enabled = true;
    let mut kyuubiki_direct_mesh_token_configured = false;

    for (key, value) in parse_env_lines(&contents) {
        match key.as_str() {
            "KYUUBIKI_DEPLOYMENT_MODE" => deployment_mode = value,
            "KYUUBIKI_AGENT_DISCOVERY" => agent_discovery = value,
            "KYUUBIKI_AGENT_MANIFEST_PATH" => agent_manifest_path = value,
            "KYUUBIKI_STORAGE_BACKEND" => storage_backend = value,
            "SQLITE_DATABASE_PATH" => sqlite_database_path = value,
            "DATABASE_URL" => database_url_configured = !value.is_empty(),
            "KYUUBIKI_AGENT_ENDPOINTS" => agent_endpoints = value,
            "KYUUBIKI_API_TOKEN" => kyuubiki_api_token_configured = !value.is_empty(),
            "KYUUBIKI_CLUSTER_API_TOKEN" => {
                kyuubiki_cluster_api_token_configured = !value.is_empty()
            }
            "KYUUBIKI_CLUSTER_ALLOWED_AGENT_IDS" => kyuubiki_cluster_allowed_agent_ids = value,
            "KYUUBIKI_CLUSTER_ALLOWED_CLUSTER_IDS" => kyuubiki_cluster_allowed_cluster_ids = value,
            "KYUUBIKI_CLUSTER_REQUIRE_FINGERPRINT" => {
                kyuubiki_cluster_require_fingerprint = value == "true"
            }
            "KYUUBIKI_CLUSTER_TIMESTAMP_WINDOW_MS" => kyuubiki_cluster_timestamp_window_ms = value,
            "KYUUBIKI_PROTECT_READS" => kyuubiki_protect_reads = value == "true",
            "KYUUBIKI_DIRECT_MESH_ENABLED" => kyuubiki_direct_mesh_enabled = value != "false",
            "KYUUBIKI_DIRECT_MESH_TOKEN" => {
                kyuubiki_direct_mesh_token_configured = !value.is_empty()
            }
            _ => {}
        }
    }

    Ok(EnvFormPayload {
        deployment_mode,
        agent_discovery,
        agent_manifest_path,
        storage_backend,
        sqlite_database_path,
        database_url: String::new(),
        database_url_configured,
        agent_endpoints,
        kyuubiki_api_token: String::new(),
        kyuubiki_api_token_configured,
        kyuubiki_cluster_api_token: String::new(),
        kyuubiki_cluster_api_token_configured,
        kyuubiki_cluster_allowed_agent_ids,
        kyuubiki_cluster_allowed_cluster_ids,
        kyuubiki_cluster_require_fingerprint,
        kyuubiki_cluster_timestamp_window_ms,
        kyuubiki_protect_reads,
        kyuubiki_direct_mesh_enabled,
        kyuubiki_direct_mesh_token: String::new(),
        kyuubiki_direct_mesh_token_configured,
    })
}

#[tauri::command]
fn write_env_file(payload: WriteEnvPayload) -> Result<String, String> {
    let root = workspace_root();
    let path = root.join(".env.local");
    let existing_entries = fs::read_to_string(&path)
        .ok()
        .map(|contents| parse_env_lines(&contents))
        .unwrap_or_default();
    let mut lines = vec![
        format!("KYUUBIKI_DEPLOYMENT_MODE={}", payload.deployment_mode),
        format!("KYUUBIKI_AGENT_DISCOVERY={}", payload.agent_discovery),
        format!("KYUUBIKI_STORAGE_BACKEND={}", payload.storage_backend),
    ];

    if !payload.agent_manifest_path.trim().is_empty() {
        lines.push(format!(
            "KYUUBIKI_AGENT_MANIFEST_PATH={}",
            payload.agent_manifest_path.trim()
        ));
    }

    if !payload.sqlite_database_path.trim().is_empty() {
        lines.push(format!(
            "SQLITE_DATABASE_PATH={}",
            payload.sqlite_database_path.trim()
        ));
    }

    if let Some(database_url) = resolve_sensitive_env_value(
        &payload.database_url,
        payload.database_url_configured,
        &existing_entries,
        "DATABASE_URL",
    ) {
        lines.push(format!("DATABASE_URL={database_url}"));
    }

    if !payload.agent_endpoints.trim().is_empty() {
        lines.push(format!(
            "KYUUBIKI_AGENT_ENDPOINTS={}",
            payload.agent_endpoints.trim()
        ));
    }

    if let Some(api_token) = resolve_sensitive_env_value(
        &payload.kyuubiki_api_token,
        payload.kyuubiki_api_token_configured,
        &existing_entries,
        "KYUUBIKI_API_TOKEN",
    ) {
        lines.push(format!("KYUUBIKI_API_TOKEN={api_token}"));
    }

    if let Some(cluster_api_token) = resolve_sensitive_env_value(
        &payload.kyuubiki_cluster_api_token,
        payload.kyuubiki_cluster_api_token_configured,
        &existing_entries,
        "KYUUBIKI_CLUSTER_API_TOKEN",
    ) {
        lines.push(format!("KYUUBIKI_CLUSTER_API_TOKEN={cluster_api_token}"));
    }

    if !payload.kyuubiki_cluster_allowed_agent_ids.trim().is_empty() {
        lines.push(format!(
            "KYUUBIKI_CLUSTER_ALLOWED_AGENT_IDS={}",
            payload.kyuubiki_cluster_allowed_agent_ids.trim()
        ));
    }

    if !payload
        .kyuubiki_cluster_allowed_cluster_ids
        .trim()
        .is_empty()
    {
        lines.push(format!(
            "KYUUBIKI_CLUSTER_ALLOWED_CLUSTER_IDS={}",
            payload.kyuubiki_cluster_allowed_cluster_ids.trim()
        ));
    }

    lines.push(format!(
        "KYUUBIKI_CLUSTER_REQUIRE_FINGERPRINT={}",
        if payload.kyuubiki_cluster_require_fingerprint {
            "true"
        } else {
            "false"
        }
    ));

    if !payload
        .kyuubiki_cluster_timestamp_window_ms
        .trim()
        .is_empty()
    {
        lines.push(format!(
            "KYUUBIKI_CLUSTER_TIMESTAMP_WINDOW_MS={}",
            payload.kyuubiki_cluster_timestamp_window_ms.trim()
        ));
    }

    lines.push(format!(
        "KYUUBIKI_PROTECT_READS={}",
        if payload.kyuubiki_protect_reads {
            "true"
        } else {
            "false"
        }
    ));

    lines.push(format!(
        "KYUUBIKI_DIRECT_MESH_ENABLED={}",
        if payload.kyuubiki_direct_mesh_enabled {
            "true"
        } else {
            "false"
        }
    ));

    if let Some(direct_mesh_token) = resolve_sensitive_env_value(
        &payload.kyuubiki_direct_mesh_token,
        payload.kyuubiki_direct_mesh_token_configured,
        &existing_entries,
        "KYUUBIKI_DIRECT_MESH_TOKEN",
    ) {
        lines.push(format!("KYUUBIKI_DIRECT_MESH_TOKEN={direct_mesh_token}"));
    }

    fs::write(&path, format!("{}\n", lines.join("\n")))
        .map_err(|error| format!("failed to write {}: {error}", path.display()))?;

    validate_env_file()?;

    Ok(format!("wrote {}", path.display()))
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
fn set_global_language_preference(
    payload: DesktopPreferencesInputPayload,
) -> Result<DesktopPreferencesPayload, String> {
    Ok(DesktopPreferencesPayload {
        language: desktop_write_global_language_preference(&payload.language)?,
    })
}

fn parse_service_mode(mode: Option<&str>) -> ServiceMode {
    match mode {
        Some("local") => ServiceMode::Local,
        Some("cloud") => ServiceMode::Cloud,
        Some("distributed") => ServiceMode::Distributed,
        _ => ServiceMode::Default,
    }
}

#[tauri::command]
fn read_runtime_log(payload: LogPayload) -> Result<RuntimeLogPayload, String> {
    Ok(RuntimeLogPayload {
        service: payload.service.clone(),
        rendered: read_shared_runtime_log(&payload.service, 160)?,
    })
}

#[tauri::command]
fn start_log_stream(app: AppHandle, payload: LogPayload) -> Result<String, String> {
    let log_path = log_path_for(&payload.service)?;
    let service = payload.service.clone();

    stop_log_stream(LogPayload {
        service: service.clone(),
    })
    .ok();

    let stop_flag = Arc::new(AtomicBool::new(false));
    log_streams()
        .lock()
        .map_err(|_| "failed to lock log stream registry".to_string())?
        .insert(service.clone(), Arc::clone(&stop_flag));

    thread::spawn(move || {
        let mut last_snapshot = String::new();

        loop {
            if stop_flag.load(Ordering::Relaxed) {
                break;
            }

            let rendered = match fs::read_to_string(&log_path) {
                Ok(contents) => {
                    let lines: Vec<&str> = contents.lines().collect();
                    let start = lines.len().saturating_sub(160);
                    lines[start..].join("\n")
                }
                Err(error) => format!("failed to read {}: {error}", log_path.display()),
            };

            if rendered != last_snapshot {
                last_snapshot = rendered.clone();
                let _ = app.emit(
                    "runtime-log-update",
                    RuntimeLogPayload {
                        service: service.clone(),
                        rendered,
                    },
                );
            }

            thread::sleep(Duration::from_millis(900));
        }
    });

    Ok(format!(
        "started runtime log stream for {}",
        payload.service
    ))
}

#[tauri::command]
fn stop_log_stream(payload: LogPayload) -> Result<String, String> {
    let mut streams = log_streams()
        .lock()
        .map_err(|_| "failed to lock log stream registry".to_string())?;

    if let Some(flag) = streams.remove(&payload.service) {
        flag.store(true, Ordering::Relaxed);
        Ok(format!(
            "stopped runtime log stream for {}",
            payload.service
        ))
    } else {
        Ok(format!(
            "no active runtime log stream for {}",
            payload.service
        ))
    }
}

#[tauri::command]
fn build_installer_bundle(payload: BuildPayload) -> Result<String, String> {
    let installer_gui_dir = workspace_root().join("apps").join("installer-gui");
    let bundle_mode = payload
        .bundle_mode
        .unwrap_or_else(|| "debug-check".to_string());
    let extra_args: Vec<&str> = match bundle_mode.as_str() {
        "release-bundle" => vec!["run", "tauri:build"],
        "release-no-bundle" => vec!["run", "tauri:build", "--", "--no-bundle"],
        "debug-check" => vec!["run", "tauri:build", "--", "--debug", "--no-bundle"],
        other => return Err(format!("unknown build mode: {other}")),
    };

    let output = Command::new("npm")
        .args(&extra_args)
        .current_dir(&installer_gui_dir)
        .output()
        .map_err(|error| format!("failed to run installer build: {error}"))?;

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();

    if output.status.success() {
        Ok(if stdout.is_empty() {
            format!("installer build completed ({bundle_mode})")
        } else {
            stdout
        })
    } else {
        let detail = if stderr.is_empty() { stdout } else { stderr };
        Err(if detail.is_empty() {
            "installer build failed".to_string()
        } else {
            detail
        })
    }
}

#[tauri::command]
fn guarded_mutation_action(payload: InstallerGuardedMutationPayload) -> Result<String, String> {
    let result = match payload.action.as_str() {
        "validate_env" => validate_env_file(),
        "init_env" => installer_init_env(payload.force.unwrap_or(false)),
        "prepare_layout" => installer_prepare_layout(),
        "repair_installation" => installer_repair_installation(),
        "bootstrap" => {
            installer_prepare_layout()?;
            installer_init_env(false)?;
            validate_env_file()
        }
        "write_env_file" => {
            let env = payload
                .env_payload
                .clone()
                .ok_or_else(|| "env payload is required".to_string())?;
            write_env_file(env)
        }
        "service_start" => desktop_service_start(parse_service_mode(payload.mode.as_deref())),
        "service_restart" => desktop_service_restart(parse_service_mode(payload.mode.as_deref())),
        "service_stop" => desktop_service_stop(),
        "remote_bootstrap" => {
            let remote = payload
                .remote_bootstrap
                .clone()
                .ok_or_else(|| "remote bootstrap payload is required".to_string())?;
            remote_bootstrap(remote)
        }
        "remote_start_agent" => {
            let remote = payload
                .remote_agent
                .clone()
                .ok_or_else(|| "remote agent payload is required".to_string())?;
            remote_start_agent(remote)
        }
        "stage_release" => {
            let platform = payload
                .platform
                .clone()
                .ok_or_else(|| "platform is required".to_string())?;
            stage_release(ReleasePayload {
                platform,
                target_dir: payload.target_dir.clone(),
            })
        }
        "prepare_staged_update" => {
            let platform = payload
                .platform
                .clone()
                .ok_or_else(|| "platform is required".to_string())?;
            installer_prepare_staged_update(
                payload.channel.clone(),
                parse_platform(Some(platform)),
                payload.target_dir.clone().map(PathBuf::from),
            )
            .map(|report| report.render())
        }
        "write_update_source_config" => installer_write_update_source_config(
            payload.catalog_path.clone().unwrap_or_default(),
            payload.artifact_root.clone().unwrap_or_default(),
            payload.download_dir.clone().unwrap_or_default(),
        ),
        "download_update" => {
            let platform = payload
                .platform
                .clone()
                .ok_or_else(|| "platform is required".to_string())?;
            installer_download_update(payload.channel.clone(), parse_platform(Some(platform)))
                .map(|record| record.render())
        }
        "apply_downloaded_update" => {
            installer_apply_downloaded_update().map(|record| record.render())
        }
        "build_installer_bundle" => build_installer_bundle(BuildPayload {
            bundle_mode: payload.bundle_mode.clone(),
        }),
        other => Err(format!("unsupported guarded installer action: {other}")),
    };

    match &result {
        Ok(detail) => append_installer_guarded_mutation_audit(&payload, "ok", detail),
        Err(detail) => append_installer_guarded_mutation_audit(&payload, "failed", detail),
    }

    result
}
fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            doctor_report,
            installation_integrity_report,
            latest_applied_update_record,
            latest_downloaded_update_record,
            latest_staged_update_record,
            update_source_config,
            unified_update_plan,
            unified_update_preview,
            export_launch,
            read_env_file,
            service_status,
            get_global_language_preference,
            set_global_language_preference,
            read_runtime_log,
            start_log_stream,
            stop_log_stream,
            guarded_mutation_action
        ])
        .run(tauri::generate_context!())
        .expect("failed to run kyuubiki installer gui");
}
