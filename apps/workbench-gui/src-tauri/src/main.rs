use kyuubiki_desktop_runtime::{
    read_runtime_log as read_shared_runtime_log, service_restart as desktop_service_restart,
    service_start as desktop_service_start, service_status as desktop_service_status,
    service_stop as desktop_service_stop, ServiceMode,
};
use serde::Serialize;

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

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct ServicePayload {
    mode: Option<String>,
}

#[tauri::command]
fn service_status() -> Result<ServiceStatusPayload, String> {
    Ok(ServiceStatusPayload {
        rendered: desktop_service_status()?,
    })
}

#[tauri::command]
fn service_start(payload: ServicePayload) -> Result<String, String> {
    let mode = match payload.mode.as_deref() {
        Some("cloud") => ServiceMode::Cloud,
        Some("distributed") => ServiceMode::Distributed,
        Some("default") => ServiceMode::Default,
        _ => ServiceMode::Local,
    };

    desktop_service_start(mode)
}

#[tauri::command]
fn service_restart(payload: ServicePayload) -> Result<String, String> {
    let mode = match payload.mode.as_deref() {
        Some("cloud") => ServiceMode::Cloud,
        Some("distributed") => ServiceMode::Distributed,
        Some("default") => ServiceMode::Default,
        _ => ServiceMode::Local,
    };

    desktop_service_restart(mode)
}

#[tauri::command]
fn service_stop() -> Result<String, String> {
    desktop_service_stop()
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct LogPayload {
    service: String,
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

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            service_status,
            service_start,
            service_restart,
            service_stop,
            read_runtime_log,
            workbench_environment
        ])
        .run(tauri::generate_context!())
        .expect("failed to run kyuubiki workbench gui");
}
