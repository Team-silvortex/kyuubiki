use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::thread;
use std::time::Duration;

use kyuubiki_installer::{
    doctor_report as build_doctor_report, export_launch_config, init_env as installer_init_env,
    parse_platform, prepare_layout as installer_prepare_layout, stage_release as installer_stage_release,
    validate_env_file, workspace_root,
};
use serde::Serialize;
use tauri::{AppHandle, Emitter};

#[derive(Serialize)]
struct DoctorReportPayload {
    platform: String,
    workspace: String,
    checks: Vec<DoctorCheckPayload>,
    rendered: String,
}

#[derive(Serialize)]
struct DoctorCheckPayload {
    label: String,
    ok: bool,
}

#[derive(Serialize)]
struct EnvFormPayload {
    deployment_mode: String,
    agent_discovery: String,
    agent_manifest_path: String,
    storage_backend: String,
    sqlite_database_path: String,
    database_url: String,
    agent_endpoints: String,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct WriteEnvPayload {
    deployment_mode: String,
    agent_discovery: String,
    agent_manifest_path: String,
    storage_backend: String,
    sqlite_database_path: String,
    database_url: String,
    agent_endpoints: String,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct ReleasePayload {
    platform: String,
    target_dir: Option<String>,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct ServicePayload {
    mode: Option<String>,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct PlatformPayload {
    platform: String,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct LogPayload {
    service: String,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct BuildPayload {
    bundle_mode: Option<String>,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct RemoteBootstrapPayload {
    target_host: String,
    ssh_user: String,
    remote_workspace: String,
    ssh_port: Option<u16>,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct RemoteAgentPayload {
    target_host: String,
    ssh_user: String,
    remote_workspace: String,
    orchestrator_url: String,
    agent_id: String,
    advertise_host: String,
    agent_port: u16,
    ssh_port: Option<u16>,
}

#[derive(Serialize)]
struct ServiceStatusPayload {
    rendered: String,
}

#[derive(Clone, Serialize)]
struct RuntimeLogPayload {
    service: String,
    rendered: String,
}

static LOG_STREAMS: OnceLock<Mutex<HashMap<String, Arc<AtomicBool>>>> = OnceLock::new();

fn log_streams() -> &'static Mutex<HashMap<String, Arc<AtomicBool>>> {
    LOG_STREAMS.get_or_init(|| Mutex::new(HashMap::new()))
}

#[tauri::command]
fn doctor_report() -> Result<DoctorReportPayload, String> {
    let report = build_doctor_report();
    Ok(DoctorReportPayload {
        rendered: report.render(),
        platform: report.platform,
        workspace: report.workspace,
        checks: report
            .checks
            .into_iter()
            .map(|check| DoctorCheckPayload {
                label: check.label,
                ok: check.ok,
            })
            .collect(),
    })
}

#[tauri::command]
fn validate_env() -> Result<String, String> {
    validate_env_file()
}

#[tauri::command]
fn init_env(force: bool) -> Result<String, String> {
    installer_init_env(force)
}

#[tauri::command]
fn prepare_layout() -> Result<String, String> {
    installer_prepare_layout()
}

#[tauri::command]
fn bootstrap() -> Result<String, String> {
    installer_prepare_layout()?;
    installer_init_env(false)?;
    validate_env_file()
}

#[tauri::command]
fn export_launch(payload: PlatformPayload) -> Result<String, String> {
    Ok(export_launch_config(parse_platform(Some(payload.platform))))
}

#[tauri::command]
fn stage_release(payload: ReleasePayload) -> Result<String, String> {
    let target_dir = payload.target_dir.filter(|value| !value.trim().is_empty()).map(PathBuf::from);
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
    let mut database_url = String::new();
    let mut agent_endpoints = "127.0.0.1:5001,127.0.0.1:5002".to_string();

    for raw_line in contents.lines() {
      let line = raw_line.trim();
      if line.is_empty() || line.starts_with('#') {
        continue;
      }

      if let Some((key, value)) = line.split_once('=') {
        match key.trim() {
          "KYUUBIKI_DEPLOYMENT_MODE" => deployment_mode = value.trim().to_string(),
          "KYUUBIKI_AGENT_DISCOVERY" => agent_discovery = value.trim().to_string(),
          "KYUUBIKI_AGENT_MANIFEST_PATH" => agent_manifest_path = value.trim().to_string(),
          "KYUUBIKI_STORAGE_BACKEND" => storage_backend = value.trim().to_string(),
          "SQLITE_DATABASE_PATH" => sqlite_database_path = value.trim().to_string(),
          "DATABASE_URL" => database_url = value.trim().to_string(),
          "KYUUBIKI_AGENT_ENDPOINTS" => agent_endpoints = value.trim().to_string(),
          _ => {}
        }
      }
    }

    Ok(EnvFormPayload {
      deployment_mode,
      agent_discovery,
      agent_manifest_path,
      storage_backend,
      sqlite_database_path,
      database_url,
      agent_endpoints,
    })
}

#[tauri::command]
fn write_env_file(payload: WriteEnvPayload) -> Result<String, String> {
    let root = workspace_root();
    let path = root.join(".env.local");
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

    if !payload.database_url.trim().is_empty() {
        lines.push(format!("DATABASE_URL={}", payload.database_url.trim()));
    }

    if !payload.agent_endpoints.trim().is_empty() {
        lines.push(format!(
            "KYUUBIKI_AGENT_ENDPOINTS={}",
            payload.agent_endpoints.trim()
        ));
    }

    fs::write(&path, format!("{}\n", lines.join("\n")))
        .map_err(|error| format!("failed to write {}: {error}", path.display()))?;

    validate_env_file()?;

    Ok(format!("wrote {}", path.display()))
}

#[tauri::command]
fn service_status() -> Result<ServiceStatusPayload, String> {
    Ok(ServiceStatusPayload {
        rendered: run_workspace_command(&["zsh", "./scripts/kyuubiki", "status"])?,
    })
}

#[tauri::command]
fn service_start(payload: ServicePayload) -> Result<String, String> {
    let command = match payload.mode.as_deref() {
        Some("local") => "start-local",
        Some("cloud") => "start-cloud",
        Some("distributed") => "start-distributed",
        _ => "start",
    };

    run_workspace_command(&["zsh", "./scripts/kyuubiki", command])
}

#[tauri::command]
fn service_restart(payload: ServicePayload) -> Result<String, String> {
    let command = match payload.mode.as_deref() {
        Some("local") => "restart-local",
        Some("cloud") => "restart-cloud",
        Some("distributed") => "restart-distributed",
        _ => "restart",
    };

    run_workspace_command(&["zsh", "./scripts/kyuubiki", command])
}

#[tauri::command]
fn remote_bootstrap(payload: RemoteBootstrapPayload) -> Result<String, String> {
    let target = format!("{}@{}", payload.ssh_user.trim(), payload.target_host.trim());
    let remote_command = format!(
        "cd {} && zsh ./scripts/kyuubiki install bootstrap",
        shell_escape(&payload.remote_workspace)
    );

    run_remote_ssh(payload.ssh_port, &target, &remote_command)
}

#[tauri::command]
fn remote_start_agent(payload: RemoteAgentPayload) -> Result<String, String> {
    let target = format!("{}@{}", payload.ssh_user.trim(), payload.target_host.trim());
    let screen_name = format!("kyuubiki_remote_agent_{}", payload.agent_port);
    let remote_command = format!(
        "cd {workspace} && screen -S {screen} -X quit >/dev/null 2>&1 || true && screen -dmS {screen} zsh -lc 'cd workers/rust && KYUUBIKI_ORCHESTRATOR_URL={orchestrator} KYUUBIKI_AGENT_ID={agent_id} KYUUBIKI_AGENT_ADVERTISE_HOST={advertise_host} cargo run -p kyuubiki-cli -- agent --host 0.0.0.0 --port {port} --agent-id {agent_id} --advertise-host {advertise_host} --orchestrator-url {orchestrator}'",
        workspace = shell_escape(&payload.remote_workspace),
        screen = shell_escape(&screen_name),
        orchestrator = shell_escape(&payload.orchestrator_url),
        agent_id = shell_escape(&payload.agent_id),
        advertise_host = shell_escape(&payload.advertise_host),
        port = payload.agent_port
    );

    run_remote_ssh(payload.ssh_port, &target, &remote_command)
}

#[tauri::command]
fn service_stop() -> Result<String, String> {
    run_workspace_command(&["zsh", "./scripts/kyuubiki", "stop"])
}

fn log_path_for(service: &str) -> Result<PathBuf, String> {
    let root = workspace_root();
    let filename = match service {
        "frontend" => "frontend.log",
        "orchestrator" => "orchestrator.log",
        "agent-5001" => "agent-5001.log",
        "agent-5002" => "agent-5002.log",
        other => return Err(format!("unknown service log: {other}")),
    };

    Ok(root.join("tmp").join("run").join(filename))
}

#[tauri::command]
fn read_runtime_log(payload: LogPayload) -> Result<RuntimeLogPayload, String> {
    let log_path = log_path_for(&payload.service)?;

    let contents = fs::read_to_string(&log_path)
        .map_err(|error| format!("failed to read {}: {error}", log_path.display()))?;

    let lines: Vec<&str> = contents.lines().collect();
    let start = lines.len().saturating_sub(160);

    Ok(RuntimeLogPayload {
        service: payload.service,
        rendered: lines[start..].join("\n"),
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

    Ok(format!("started runtime log stream for {}", payload.service))
}

#[tauri::command]
fn stop_log_stream(payload: LogPayload) -> Result<String, String> {
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

#[tauri::command]
fn build_installer_bundle(payload: BuildPayload) -> Result<String, String> {
    let installer_gui_dir = workspace_root().join("apps").join("installer-gui");
    let bundle_mode = payload.bundle_mode.unwrap_or_else(|| "debug-check".to_string());
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

fn run_workspace_command(args: &[&str]) -> Result<String, String> {
    let root = workspace_root();
    let (program, tail) = args
        .split_first()
        .ok_or_else(|| "missing process command".to_string())?;

    let output = Command::new(program)
        .args(tail)
        .current_dir(&root)
        .output()
        .map_err(|error| format!("failed to run {}: {error}", args.join(" ")))?;

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();

    if output.status.success() {
        Ok(if stdout.is_empty() {
            "command completed".to_string()
        } else {
            stdout
        })
    } else {
        let detail = if stderr.is_empty() { stdout } else { stderr };
        Err(if detail.is_empty() {
            format!("command failed: {}", args.join(" "))
        } else {
            detail
        })
    }
}

fn run_remote_ssh(ssh_port: Option<u16>, target: &str, remote_command: &str) -> Result<String, String> {
    let mut command = Command::new("ssh");
    if let Some(port) = ssh_port {
        command.arg("-p").arg(port.to_string());
    }

    let output = command
        .arg(target)
        .arg(remote_command)
        .output()
        .map_err(|error| format!("failed to run ssh command: {error}"))?;

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();

    if output.status.success() {
        Ok(if stdout.is_empty() {
            format!("remote command completed on {}", target)
        } else {
            stdout
        })
    } else {
        Err(if stderr.is_empty() { stdout } else { stderr })
    }
}

fn shell_escape(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\"'\"'"))
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            doctor_report,
            validate_env,
            init_env,
            prepare_layout,
            bootstrap,
            export_launch,
            stage_release,
            read_env_file,
            write_env_file,
            service_status,
            service_start,
            service_restart,
            service_stop,
            remote_bootstrap,
            remote_start_agent,
            read_runtime_log,
            start_log_stream,
            stop_log_stream,
            build_installer_bundle
        ])
        .run(tauri::generate_context!())
        .expect("failed to run kyuubiki installer gui");
}
