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
struct HubEnvironmentPayload {
    hub_role: String,
    workbench_url: String,
    orchestrator_url: String,
    deployment_mode: String,
    installer_gui_hint: String,
    workbench_gui_hint: String,
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

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct LogPayload {
    service: String,
}

fn resolve_service_mode(mode: Option<&str>) -> ServiceMode {
    match mode {
        Some("cloud") => ServiceMode::Cloud,
        Some("distributed") => ServiceMode::Distributed,
        Some("default") => ServiceMode::Default,
        _ => ServiceMode::Local,
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
fn read_runtime_log(payload: LogPayload) -> Result<RuntimeLogPayload, String> {
    Ok(RuntimeLogPayload {
        service: payload.service.clone(),
        rendered: read_shared_runtime_log(&payload.service, 180)?,
    })
}

#[tauri::command]
fn hub_environment() -> HubEnvironmentPayload {
    HubEnvironmentPayload {
        hub_role: "desktop-orchestration-shell".to_string(),
        workbench_url: "http://127.0.0.1:3000".to_string(),
        orchestrator_url: "http://127.0.0.1:4000".to_string(),
        deployment_mode: std::env::var("KYUUBIKI_DEPLOYMENT_MODE")
            .unwrap_or_else(|_| "local".to_string()),
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
            read_runtime_log,
            hub_environment
        ])
        .run(tauri::generate_context!())
        .expect("failed to run kyuubiki hub gui");
}
