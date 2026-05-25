use kyuubiki_desktop_runtime::{
    append_desktop_audit_line as desktop_append_audit_line,
    read_global_language_preference as desktop_read_global_language_preference,
    read_runtime_log as read_shared_runtime_log, service_restart as desktop_service_restart,
    service_start as desktop_service_start, service_status as desktop_service_status,
    service_stop as desktop_service_stop, write_global_language_preference as desktop_write_global_language_preference,
    ServiceMode,
};
use serde::Serialize;
use serde_json::json;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Serialize)]
struct ServiceStatusPayload {
    rendered: String,
}

#[derive(Serialize)]
struct WorkbenchEnvironmentPayload {
    workbench_url: String,
    orchestrator_url: String,
    deployment_mode: String,
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

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct WorkbenchGuardedMutationPayload {
    action: String,
    mode: Option<String>,
}

#[tauri::command]
fn service_status() -> Result<ServiceStatusPayload, String> {
    Ok(ServiceStatusPayload {
        rendered: desktop_service_status()?,
    })
}

fn parse_service_mode(mode: Option<&str>) -> ServiceMode {
    match mode {
        Some("cloud") => ServiceMode::Cloud,
        Some("distributed") => ServiceMode::Distributed,
        Some("default") => ServiceMode::Default,
        _ => ServiceMode::Local,
    }
}

const WORKBENCH_GUARDED_MUTATION_AUDIT_FILE: &str = "workbench-guarded-mutations.jsonl";

fn audit_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

fn append_workbench_guarded_mutation_audit(
    payload: &WorkbenchGuardedMutationPayload,
    status: &str,
    detail: &str,
) {
    let record = json!({
        "ts": audit_timestamp(),
        "action": payload.action,
        "mode": payload.mode,
        "status": status,
        "detail": detail,
    });
    let _ = desktop_append_audit_line(WORKBENCH_GUARDED_MUTATION_AUDIT_FILE, &record.to_string());
}

#[tauri::command]
fn guarded_mutation_action(payload: WorkbenchGuardedMutationPayload) -> Result<String, String> {
    let result = match payload.action.as_str() {
        "service_start" => desktop_service_start(parse_service_mode(payload.mode.as_deref())),
        "service_restart" => desktop_service_restart(parse_service_mode(payload.mode.as_deref())),
        "service_stop" => desktop_service_stop(),
        other => Err(format!("unsupported guarded workbench action: {other}")),
    };

    match &result {
        Ok(detail) => append_workbench_guarded_mutation_audit(&payload, "ok", detail),
        Err(detail) => append_workbench_guarded_mutation_audit(&payload, "failed", detail),
    }

    result
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct LogPayload {
    service: String,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct DesktopPreferencesInputPayload {
    language: String,
}

#[tauri::command]
fn read_runtime_log(payload: LogPayload) -> Result<RuntimeLogPayload, String> {
    Ok(RuntimeLogPayload {
        service: payload.service.clone(),
        rendered: read_shared_runtime_log(&payload.service, 180)?,
    })
}

#[tauri::command]
fn workbench_environment() -> WorkbenchEnvironmentPayload {
    WorkbenchEnvironmentPayload {
        workbench_url: "http://127.0.0.1:3000".to_string(),
        orchestrator_url: "http://127.0.0.1:4000".to_string(),
        deployment_mode: std::env::var("KYUUBIKI_DEPLOYMENT_MODE").unwrap_or_else(|_| "local".to_string()),
    }
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

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            service_status,
            read_runtime_log,
            workbench_environment,
            get_global_language_preference,
            set_global_language_preference,
            guarded_mutation_action
        ])
        .run(tauri::generate_context!())
        .expect("failed to run kyuubiki workbench gui");
}
