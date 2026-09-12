use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde_json::Value;

use crate::frontend_launch;
use crate::runtime_layout::{
    RuntimePaths, resolve_development_command, runtime_bin_dirs, runtime_paths,
};
use crate::runtime_lifecycle_lock::RuntimeLifecycleLock;
use crate::runtime_options::{DEFAULT_ORCHESTRATOR_PORT, RuntimeOptions};
use crate::runtime_process::{
    ManagedProcess, already_running, service_line, spawn_managed, stop_managed,
};
use crate::runtime_process_record::read_pid;
use crate::runtime_support::{remove_file_if_present, service_mode_name};
use crate::{HotServiceMode, ServiceMode};

const DEFAULT_AGENT_ENDPOINTS: &str = "127.0.0.1:5001,127.0.0.1:5002";

struct StartedProcess {
    label: String,
    pid: PathBuf,
    port: u16,
}

pub(super) fn service_status() -> Result<String, String> {
    let paths = runtime_paths()?;
    let env = runtime_env(&paths.root);
    let mut options = RuntimeOptions::from_env(&env)?;
    options.frontend_disabled |= paths.is_headless();
    let mode = read_runtime_mode(&paths, &env, options);
    let mut lines = vec![
        format!("deployment-mode: {mode}"),
        format!(
            "control-mode: {}",
            if mode == "local" {
                "standalone"
            } else {
                "orch_managed"
            }
        ),
        format!(
            "authority-mode: {}",
            if mode == "local" {
                "self_directed"
            } else {
                "single_orchestrator"
            }
        ),
        format!("runtime-policy: {}", paths.origin_label()),
        "lifecycle-policy: explicit-background (GUI exit does not stop services)".to_string(),
        "lifecycle-ownership: process-incarnation; unmanaged processes are never adopted"
            .to_string(),
        format!(
            "lifecycle-lock: {}",
            paths.run.join("lifecycle.lock").display()
        ),
    ];
    if paths.is_development() {
        for command in ["mix", "cargo"] {
            lines.push(match resolve_development_command(&paths.root, command) {
                Ok(path) => format!(
                    "runtime-command[{command}]: development -> {}",
                    path.display()
                ),
                Err(error) => format!("runtime-command[{command}]: missing ({error})"),
            });
        }
        lines.push(match resolve_development_command(&paths.root, "npm") {
            Ok(path) => format!(
                "frontend-build-command[npm]: development-only -> {}",
                path.display()
            ),
            Err(error) => format!("frontend-build-command[npm]: missing ({error})"),
        });
    } else {
        lines.push(format!("runtime-root: {}", paths.root.display()));
        lines.push(format!("runtime-state: {}", paths.state.display()));
        for service in ["agent", "orchestrator", "frontend"] {
            lines.push(
                match paths.service(service, &[("port", "5001".to_string())]) {
                    Ok(spec) => format!(
                        "runtime-service[{service}]: installed -> {}",
                        spec.command.display()
                    ),
                    Err(error) => format!("runtime-service[{service}]: blocked ({error})"),
                },
            );
        }
    }
    lines.push(service_line(
        "orchestrator",
        &options.orchestrator_pid(&paths.run),
        options.orchestrator_port,
        "http",
    ));
    lines.push(if options.frontend_disabled {
        "frontend: disabled by runtime configuration".to_string()
    } else {
        service_line(
            "frontend",
            &options.frontend_pid(&paths.run),
            options.frontend_port,
            "http",
        )
    });
    for port in agent_ports(&paths.root, &env) {
        lines.push(service_line(
            &format!("agent[{port}]"),
            &paths.run.join(format!("agent-{port}.pid")),
            port,
            "tcp",
        ));
    }
    Ok(lines.join("\n"))
}

pub(super) fn service_start(mode: ServiceMode) -> Result<String, String> {
    let paths = runtime_paths()?;
    let _lock = RuntimeLifecycleLock::acquire(&paths.run)?;
    start_services(&paths, service_mode_name(mode))
}

pub(super) fn service_restart(mode: ServiceMode) -> Result<String, String> {
    let paths = runtime_paths()?;
    let _lock = RuntimeLifecycleLock::acquire(&paths.run)?;
    let mut lines = vec![stop_services(&paths)?];
    lines.push(start_services(&paths, service_mode_name(mode))?);
    lines.push("restart complete".to_string());
    Ok(lines.join("\n"))
}

pub(super) fn service_stop() -> Result<String, String> {
    let paths = runtime_paths()?;
    let _lock = RuntimeLifecycleLock::acquire(&paths.run)?;
    stop_services(&paths)
}

fn stop_services(paths: &RuntimePaths) -> Result<String, String> {
    let env = runtime_env(&paths.root);
    let mut options = RuntimeOptions::from_env(&env)?;
    options.frontend_disabled |= paths.is_headless();
    let mut lines = Vec::new();
    let mut targets = Vec::new();
    if !options.orchestrator_only && !options.frontend_disabled {
        targets.push((
            options.frontend_pid(&paths.run),
            "frontend".to_string(),
            options.frontend_port,
        ));
    }
    targets.push((
        options.orchestrator_pid(&paths.run),
        "orchestrator".to_string(),
        options.orchestrator_port,
    ));
    if !options.orchestrator_only && read_runtime_mode(paths, &env, options) != "distributed" {
        for port in agent_ports(&paths.root, &env).into_iter().rev() {
            targets.push((
                paths.run.join(format!("agent-{port}.pid")),
                format!("agent[{port}]"),
                port,
            ));
        }
    }
    let mut failed = false;
    for (pid, label, port) in targets {
        match stop_managed(&pid, &label, Some(port)) {
            Ok(line) => lines.push(line),
            Err(error) => {
                failed = true;
                lines.push(error);
            }
        }
    }
    if failed {
        return Err(format!("runtime stop incomplete:\n{}", lines.join("\n")));
    }
    if !options.orchestrator_only {
        remove_file_if_present(&options.runtime_mode(&paths.run))?;
    }
    if !options.orchestrator_only && options.orchestrator_port == DEFAULT_ORCHESTRATOR_PORT {
        remove_file_if_present(&paths.hot.join("native-mode.txt"))?;
    }
    Ok(lines.join("\n"))
}

pub(super) fn hot_service_status() -> Result<String, String> {
    let paths = runtime_paths()?;
    let active = paths.hot.join("native-mode.txt").is_file();
    let status = service_status()?;
    let mut lines = vec![format!(
        "hot-loop: {} (native runtime control)",
        hot_runtime_state(active, &status)
    )];
    lines.push(status);
    lines.push(format!("hot-logs: {}", paths.hot.display()));
    Ok(lines.join("\n"))
}

fn hot_runtime_state(configured: bool, rendered: &str) -> &'static str {
    if !configured {
        return "stopped";
    }
    let summary = crate::summarize_service_status(rendered);
    let states = std::iter::once(summary.orchestrator_status.as_str())
        .chain(std::iter::once(summary.frontend_status.as_str()))
        .chain(
            summary
                .agents
                .iter()
                .filter(|_| summary.deployment_mode != "distributed")
                .map(|agent| agent.status.as_str()),
        )
        .collect::<Vec<_>>();
    if states.contains(&"blocked") {
        "blocked"
    } else if states.contains(&"starting") {
        "starting"
    } else if states
        .iter()
        .all(|state| matches!(*state, "running" | "disabled"))
    {
        "running"
    } else {
        "degraded"
    }
}

pub(super) fn hot_service_start(mode: HotServiceMode) -> Result<String, String> {
    let paths = runtime_paths()?;
    if !paths.is_development() {
        return Err(
            "hot runtime controls are available only in explicit desktop source mode".to_string(),
        );
    }
    let _lock = RuntimeLifecycleLock::acquire(&paths.run)?;
    ensure_runtime_dirs(&paths)?;
    let mode = match mode {
        HotServiceMode::Local => "local",
        HotServiceMode::Cloud => "cloud",
        HotServiceMode::Distributed => "distributed",
    };
    let rendered = start_services(&paths, mode)?;
    fs::write(paths.hot.join("native-mode.txt"), format!("{mode}\n"))
        .map_err(|error| format!("failed to write native hot runtime state: {error}"))?;
    Ok(format!(
        "{rendered}\nstarted native development runtime ({mode})"
    ))
}

pub(super) fn hot_service_stop() -> Result<String, String> {
    service_stop()
}

fn start_services(paths: &RuntimePaths, requested_mode: &str) -> Result<String, String> {
    ensure_runtime_dirs(&paths)?;
    let mut env = runtime_env(&paths.root);
    paths.apply_writable_state_env(&mut env)?;
    let mode = resolve_mode(requested_mode, &env)?;
    let existing_mode = read_runtime_mode(paths, &env, RuntimeOptions::from_env(&env)?);
    if mode == "local" {
        env.entry("SQLITE_DATABASE_PATH".to_string())
            .or_insert_with(|| {
                paths
                    .data
                    .join(if paths.is_development() {
                        "kyuubiki_dev.sqlite3"
                    } else {
                        "kyuubiki.sqlite3"
                    })
                    .display()
                    .to_string()
            });
    }
    apply_mode_env(&mut env, &mode)?;
    let mut options = RuntimeOptions::from_env(&env)?;
    options.frontend_disabled |= paths.is_headless();
    env.entry("KYUUBIKI_ORCHESTRATOR_URL".to_string())
        .or_insert_with(|| options.orchestrator_url());
    let endpoints = agent_endpoints(&env);
    env.insert("KYUUBIKI_AGENT_ENDPOINTS".to_string(), endpoints.clone());
    env.entry("KYUUBIKI_AGENT_DISCOVERY".to_string())
        .or_insert_with(|| "static".to_string());
    augment_path(&paths, &mut env);

    let mut reused = false;
    if mode != "distributed" && !options.orchestrator_only {
        for port in agent_ports(&paths.root, &env) {
            reused |= already_running(
                &paths.run.join(format!("agent-{port}.pid")),
                &format!("agent[{port}]"),
                port,
            )?;
        }
    }
    reused |= already_running(
        &options.orchestrator_pid(&paths.run),
        "orchestrator",
        options.orchestrator_port,
    )?;
    if !options.orchestrator_only && !options.frontend_disabled {
        reused |= already_running(
            &options.frontend_pid(&paths.run),
            "frontend",
            options.frontend_port,
        )?;
    }
    if reused && existing_mode != mode {
        return Err(
            "running runtime uses a different deployment mode; use explicit restart".to_string(),
        );
    }

    let mut lines = Vec::new();
    let mut started = Vec::new();
    if mode != "distributed" && !options.orchestrator_only {
        for port in agent_ports(&paths.root, &env) {
            let pid = paths.run.join(format!("agent-{port}.pid"));
            let previous_pid = read_pid(&pid);
            let result = start_agent(&paths, port, &env);
            record_started(
                &mut started,
                &pid,
                &format!("agent[{port}]"),
                port,
                previous_pid,
            );
            lines.push(rollback_on_error(result, &mut started)?);
        }
    }
    let orchestrator_pid = options.orchestrator_pid(&paths.run);
    let previous_pid = read_pid(&orchestrator_pid);
    let result = start_orchestrator(&paths, &env, &mode, options);
    record_started(
        &mut started,
        &orchestrator_pid,
        "orchestrator",
        options.orchestrator_port,
        previous_pid,
    );
    lines.push(rollback_on_error(result, &mut started)?);
    if !options.orchestrator_only && !options.frontend_disabled {
        let pid = options.frontend_pid(&paths.run);
        let previous_pid = read_pid(&pid);
        let result = start_frontend(&paths, &env, options);
        record_started(
            &mut started,
            &pid,
            "frontend",
            options.frontend_port,
            previous_pid,
        );
        lines.push(rollback_on_error(result, &mut started)?);
    }
    let persisted = fs::write(options.runtime_mode(&paths.run), format!("{mode}\n"))
        .map_err(|error| format!("failed to persist runtime mode: {error}"));
    rollback_on_error(persisted, &mut started)?;
    Ok(lines.join("\n"))
}

fn record_started(
    started: &mut Vec<StartedProcess>,
    pid: &Path,
    label: &str,
    port: u16,
    previous_pid: Option<u32>,
) {
    if read_pid(pid).is_some_and(|current| Some(current) != previous_pid) {
        started.push(StartedProcess {
            label: label.to_string(),
            pid: pid.to_path_buf(),
            port,
        });
    }
}

fn rollback_on_error<T>(
    result: Result<T, String>,
    started: &mut Vec<StartedProcess>,
) -> Result<T, String> {
    result.map_err(|error| {
        let cleanup = started
            .drain(..)
            .rev()
            .map(|process| {
                stop_managed(&process.pid, &process.label, Some(process.port))
                    .unwrap_or_else(|rollback_error| format!("{}: {rollback_error}", process.label))
            })
            .collect::<Vec<_>>()
            .join("; ");
        format!("{error}; startup rollback: {cleanup}")
    })
}

fn start_agent(
    paths: &RuntimePaths,
    port: u16,
    env: &HashMap<String, String>,
) -> Result<String, String> {
    if already_running(
        &paths.run.join(format!("agent-{port}.pid")),
        &format!("agent[{port}]"),
        port,
    )? {
        return Ok(format!(
            "Rust FEM agent already running at tcp://127.0.0.1:{port}"
        ));
    }
    let (command, args, cwd) = if paths.is_development() {
        (
            resolve_development_command(&paths.root, "cargo")?,
            development_agent_args(port, env),
            paths.root.join("workers/rust"),
        )
    } else {
        let spec = paths.service("agent", &[("port", port.to_string())])?;
        (spec.command, spec.args, spec.cwd)
    };
    let process = ManagedProcess {
        label: format!("agent[{port}]"),
        command,
        args,
        cwd,
        pid: paths.run.join(format!("agent-{port}.pid")),
        log: paths.run.join(format!("agent-{port}.log")),
        port: Some(port),
        env: env.clone(),
    };
    spawn_managed(process, Duration::from_secs(60))?;
    Ok(format!("started Rust FEM agent at tcp://127.0.0.1:{port}"))
}

fn development_agent_args(port: u16, env: &HashMap<String, String>) -> Vec<String> {
    let mut args = vec!["run".into()];
    if env.get("KYUUBIKI_AGENT_BUILD_PROFILE").map(String::as_str) == Some("release") {
        args.push("--release".into());
    }
    args.extend([
        "-p".into(),
        "kyuubiki-cli".into(),
        "--bin".into(),
        "kyuubiki-cli".into(),
    ]);
    args.extend([
        "--".into(),
        "agent".into(),
        "--port".into(),
        port.to_string(),
    ]);
    args
}

fn start_orchestrator(
    paths: &RuntimePaths,
    env: &HashMap<String, String>,
    mode: &str,
    options: RuntimeOptions,
) -> Result<String, String> {
    let url = options.orchestrator_url();
    if already_running(
        &options.orchestrator_pid(&paths.run),
        "orchestrator",
        options.orchestrator_port,
    )? {
        return Ok(format!("orchestrator already running at {url}"));
    }
    let mut process_env = env.clone();
    process_env.insert("PORT".to_string(), options.orchestrator_port.to_string());
    process_env.insert("RELEASE_DISTRIBUTION".to_string(), "none".to_string());
    let (command, args, cwd) = if paths.is_development() {
        (
            resolve_development_command(&paths.root, "mix")?,
            vec!["run".into(), "--no-halt".into()],
            paths.root.join("apps/web"),
        )
    } else {
        let spec = paths.service("orchestrator", &[])?;
        (spec.command, spec.args, spec.cwd)
    };
    let process = ManagedProcess {
        label: "orchestrator".to_string(),
        command,
        args,
        cwd,
        pid: options.orchestrator_pid(&paths.run),
        log: options.orchestrator_log(&paths.run),
        port: Some(options.orchestrator_port),
        env: process_env,
    };
    spawn_managed(process, Duration::from_secs(120))?;
    Ok(format!("started orchestrator API at {url} ({mode})"))
}

fn start_frontend(
    paths: &RuntimePaths,
    env: &HashMap<String, String>,
    options: RuntimeOptions,
) -> Result<String, String> {
    let port = options.frontend_port;
    if already_running(&options.frontend_pid(&paths.run), "frontend", port)? {
        return Ok(format!(
            "frontend already running at http://127.0.0.1:{port}"
        ));
    }
    let mut process_env = env.clone();
    process_env.insert("HOSTNAME".to_string(), "127.0.0.1".to_string());
    process_env.insert("PORT".to_string(), port.to_string());
    let frontend = frontend_launch::resolve(paths, port)?;
    let process = ManagedProcess {
        label: "frontend".to_string(),
        command: frontend.command,
        args: frontend.args,
        cwd: frontend.cwd,
        pid: options.frontend_pid(&paths.run),
        log: options.frontend_log(&paths.run),
        port: Some(port),
        env: process_env,
    };
    spawn_managed(process, Duration::from_secs(60))?;
    Ok(format!(
        "started {} at http://127.0.0.1:{port}",
        frontend.label
    ))
}

fn ensure_runtime_dirs(paths: &RuntimePaths) -> Result<(), String> {
    fs::create_dir_all(&paths.hot)
        .map_err(|error| format!("failed to create {}: {error}", paths.hot.display()))?;
    fs::create_dir_all(&paths.data)
        .map_err(|error| format!("failed to create {}: {error}", paths.data.display()))
}

pub(super) fn runtime_env(root: &Path) -> HashMap<String, String> {
    let mut values = HashMap::new();
    for path in [
        root.join("config/.env.example"),
        root.join("config/.env.local"),
        root.join(".env.example"),
        root.join(".env.local"),
    ] {
        load_env_file(&path, &mut values);
    }
    values.extend(env::vars());
    values
}

fn load_env_file(path: &Path, values: &mut HashMap<String, String>) {
    let Ok(contents) = fs::read_to_string(path) else {
        return;
    };
    for line in contents.lines().map(str::trim) {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((key, value)) = line.split_once('=') {
            values.insert(key.trim().to_string(), value.trim().to_string());
        }
    }
}

fn resolve_mode(requested: &str, env: &HashMap<String, String>) -> Result<String, String> {
    let mode = match requested {
        "local" | "cloud" | "distributed" => requested,
        _ => env
            .get("KYUUBIKI_DEPLOYMENT_MODE")
            .map(String::as_str)
            .unwrap_or("local"),
    };
    if matches!(mode, "local" | "cloud" | "distributed") {
        Ok(mode.to_string())
    } else {
        Err(format!("unsupported deployment mode: {mode}"))
    }
}

fn apply_mode_env(env: &mut HashMap<String, String>, mode: &str) -> Result<(), String> {
    if mode == "local" {
        env.insert("KYUUBIKI_STORAGE_BACKEND".into(), "sqlite".into());
        env.entry("SQLITE_DATABASE_PATH".into())
            .or_insert_with(|| "./tmp/data/kyuubiki_dev.sqlite3".into());
    } else {
        if env.get("DATABASE_URL").is_none_or(String::is_empty) {
            return Err(format!("DATABASE_URL is required for {mode} mode"));
        }
        env.insert("KYUUBIKI_STORAGE_BACKEND".into(), "postgres".into());
    }
    env.insert("KYUUBIKI_DEPLOYMENT_MODE".into(), mode.into());
    Ok(())
}

fn read_runtime_mode(
    paths: &RuntimePaths,
    env: &HashMap<String, String>,
    options: RuntimeOptions,
) -> String {
    fs::read_to_string(options.runtime_mode(&paths.run))
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| matches!(value.as_str(), "local" | "cloud" | "distributed"))
        .or_else(|| env.get("KYUUBIKI_DEPLOYMENT_MODE").cloned())
        .unwrap_or_else(|| "local".to_string())
}

fn agent_endpoints(env: &HashMap<String, String>) -> String {
    env.get("KYUUBIKI_AGENT_ENDPOINTS")
        .cloned()
        .unwrap_or_else(|| DEFAULT_AGENT_ENDPOINTS.to_string())
}

fn agent_ports(root: &Path, env: &HashMap<String, String>) -> Vec<u16> {
    if env.get("KYUUBIKI_AGENT_DISCOVERY").map(String::as_str) == Some("manifest") {
        return manifest_agent_ports(root, env);
    }
    agent_endpoints(env)
        .split(',')
        .filter_map(|entry| entry.trim().rsplit(':').next()?.parse().ok())
        .collect()
}

fn manifest_agent_ports(root: &Path, env: &HashMap<String, String>) -> Vec<u16> {
    let path = env
        .get("KYUUBIKI_AGENT_MANIFEST_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("./deploy/agents.local.example.json"));
    let path = if path.is_absolute() {
        path
    } else {
        root.join(path)
    };
    fs::read_to_string(path)
        .ok()
        .and_then(|text| serde_json::from_str::<Value>(&text).ok())
        .and_then(|value| value.get("agents").and_then(Value::as_array).cloned())
        .unwrap_or_default()
        .iter()
        .filter_map(|agent| agent.get("port")?.as_u64())
        .filter_map(|port| u16::try_from(port).ok())
        .collect()
}

fn augment_path(paths: &RuntimePaths, env: &mut HashMap<String, String>) {
    let mut entries = runtime_bin_dirs(&paths.root);
    if paths.is_development() {
        if let Some(current) = env.get("PATH") {
            entries.extend(env::split_paths(current));
        }
        if let Some(home) = env.get("HOME") {
            entries.push(PathBuf::from(home).join(".cargo/bin"));
        }
        #[cfg(unix)]
        entries.extend([
            unix_rooted_path(&["opt", "homebrew", "bin"]),
            unix_rooted_path(&["usr", "local", "bin"]),
            unix_rooted_path(&["usr", "bin"]),
            unix_rooted_path(&["bin"]),
        ]);
    } else if cfg!(windows) {
        if let Some(system_root) = env.get("SystemRoot").or_else(|| env.get("SYSTEMROOT")) {
            entries.push(PathBuf::from(system_root).join("System32"));
        }
    } else {
        // Installed releases use only manifest runtimes plus the OS baseline.
        entries.extend([PathBuf::from("/usr/bin"), PathBuf::from("/bin")]);
    }
    let joined = env::join_paths(entries.iter().filter(|path| path.is_dir()))
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    env.insert("PATH".to_string(), joined);
}

#[cfg(unix)]
fn unix_rooted_path(parts: &[&str]) -> PathBuf {
    let mut path = PathBuf::from(std::path::MAIN_SEPARATOR.to_string());
    path.extend(parts);
    path
}

#[cfg(test)]
#[path = "runtime_control_tests.rs"]
mod tests;
