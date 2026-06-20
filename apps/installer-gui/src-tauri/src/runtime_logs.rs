use std::collections::HashMap;
use std::fs;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::thread;
use std::time::Duration;

use kyuubiki_desktop_runtime::{log_path_for, read_runtime_log as read_shared_runtime_log};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LogPayload {
    pub service: String,
}

#[derive(Clone, Serialize)]
pub struct RuntimeLogPayload {
    pub service: String,
    pub rendered: String,
}

static LOG_STREAMS: OnceLock<Mutex<HashMap<String, Arc<AtomicBool>>>> = OnceLock::new();

fn log_streams() -> &'static Mutex<HashMap<String, Arc<AtomicBool>>> {
    LOG_STREAMS.get_or_init(|| Mutex::new(HashMap::new()))
}

#[tauri::command]
pub fn read_runtime_log(payload: LogPayload) -> Result<RuntimeLogPayload, String> {
    Ok(RuntimeLogPayload {
        service: payload.service.clone(),
        rendered: read_shared_runtime_log(&payload.service, 160)?,
    })
}

#[tauri::command]
pub fn start_log_stream(app: AppHandle, payload: LogPayload) -> Result<String, String> {
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

    Ok(format!("started runtime log stream for {}", payload.service))
}

#[tauri::command]
pub fn stop_log_stream(payload: LogPayload) -> Result<String, String> {
    let mut streams = log_streams()
        .lock()
        .map_err(|_| "failed to lock log stream registry".to_string())?;
    if let Some(flag) = streams.remove(&payload.service) {
        flag.store(true, Ordering::Relaxed);
        Ok(format!("stopped runtime log stream for {}", payload.service))
    } else {
        Ok(format!("no active runtime log stream for {}", payload.service))
    }
}
