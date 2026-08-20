use kyuubiki_desktop_runtime::{
    append_desktop_provenance_record,
    prepare_desktop_provenance_ledger,
    read_global_language_preference as desktop_read_global_language_preference,
    report_packaged_boot_ready as desktop_report_packaged_boot_ready,
    read_runtime_log as read_shared_runtime_log, service_restart as desktop_service_restart,
    service_start as desktop_service_start, service_status as desktop_service_status,
    service_stop as desktop_service_stop, summarize_service_status as desktop_summarize_service_status,
    write_global_language_preference as desktop_write_global_language_preference, ServiceStatusSummary,
    ServiceMode,
};
use serde::Serialize;
use serde_json::json;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Serialize)]
struct ServiceStatusPayload {
    rendered: String,
    summary: ServiceStatusSummary,
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
    let rendered = desktop_service_status()?;
    Ok(ServiceStatusPayload {
        summary: desktop_summarize_service_status(&rendered),
        rendered,
    })
}

#[tauri::command]
fn packaged_boot_ready() -> Result<String, String> {
    desktop_report_packaged_boot_ready("workbench")
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
) -> Result<(), String> {
    let record = json!({
        "schema_version": "kyuubiki.workbench-guarded-mutation-provenance/v1",
        "ts": audit_timestamp(),
        "action": payload.action,
        "mode": payload.mode,
        "status": status,
        "detail": detail,
    });
    append_desktop_provenance_record(WORKBENCH_GUARDED_MUTATION_AUDIT_FILE, &record).map(|_| ())
}

#[tauri::command]
fn guarded_mutation_action(payload: WorkbenchGuardedMutationPayload) -> Result<String, String> {
    prepare_desktop_provenance_ledger(WORKBENCH_GUARDED_MUTATION_AUDIT_FILE).map_err(|error| {
        format!("guarded Workbench action blocked by invalid provenance ledger: {error}")
    })?;
    let result = match payload.action.as_str() {
        "service_start" => desktop_service_start(parse_service_mode(payload.mode.as_deref())),
        "service_restart" => desktop_service_restart(parse_service_mode(payload.mode.as_deref())),
        "service_stop" => desktop_service_stop(),
        other => Err(format!("unsupported guarded workbench action: {other}")),
    };

    let audit_result = match &result {
        Ok(detail) => append_workbench_guarded_mutation_audit(&payload, "ok", detail),
        Err(detail) => append_workbench_guarded_mutation_audit(&payload, "failed", detail),
    };
    match (result, audit_result) {
        (Ok(detail), Ok(())) => Ok(detail),
        (Err(detail), Ok(())) => Err(detail),
        (Ok(detail), Err(audit_error)) => Err(format!(
            "Workbench action completed but provenance persistence failed; inspect state before retry: {audit_error}; action detail: {detail}"
        )),
        (Err(detail), Err(audit_error)) => Err(format!(
            "{detail}; failed action provenance could not be persisted: {audit_error}"
        )),
    }
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
            packaged_boot_ready,
            read_runtime_log,
            workbench_environment,
            get_global_language_preference,
            set_global_language_preference,
            guarded_mutation_action
        ])
        .run(tauri::generate_context!())
        .expect("failed to run kyuubiki workbench gui");
}
