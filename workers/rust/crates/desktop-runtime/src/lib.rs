use kyuubiki_platform::desktop_preferences_dir as shared_desktop_preferences_dir;
use serde::Serialize;
use serde_json::json;
use std::fs;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};

mod audit_log;
mod runtime_control;
mod runtime_layout;
mod runtime_options;

pub use audit_log::{
    DesktopAuditLedgerStatus, append_desktop_provenance_record, desktop_provenance_status,
    prepare_desktop_provenance_ledger,
};

const GLOBAL_LANGUAGE_FILE: &str = "desktop-language.txt";
const PACKAGED_BOOT_RECEIPT_ENV: &str = "KYUUBIKI_PACKAGED_BOOT_RECEIPT";
const PACKAGED_BOOT_SCHEMA: &str = "kyuubiki.packaged-desktop-boot-receipt/v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ServiceEndpointSummary {
    pub label: String,
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ServiceStatusSummary {
    pub deployment_mode: String,
    pub control_mode: String,
    pub authority_mode: String,
    pub orchestrator_status: String,
    pub frontend_status: String,
    pub agent_count: usize,
    pub active_agent_count: usize,
    pub agents: Vec<ServiceEndpointSummary>,
}

#[derive(Clone, Copy)]
pub enum ServiceMode {
    Default,
    Local,
    Cloud,
    Distributed,
}

impl ServiceMode {
    pub fn start_command(self) -> &'static str {
        match self {
            ServiceMode::Default => "start",
            ServiceMode::Local => "start-local",
            ServiceMode::Cloud => "start-cloud",
            ServiceMode::Distributed => "start-distributed",
        }
    }

    pub fn restart_command(self) -> &'static str {
        match self {
            ServiceMode::Default => "restart",
            ServiceMode::Local => "restart-local",
            ServiceMode::Cloud => "restart-cloud",
            ServiceMode::Distributed => "restart-distributed",
        }
    }
}

#[derive(Clone, Copy)]
pub enum HotServiceMode {
    Local,
    Cloud,
    Distributed,
}

impl HotServiceMode {
    pub fn start_command(self) -> &'static str {
        match self {
            HotServiceMode::Local => "hot-start-local",
            HotServiceMode::Cloud => "hot-start-cloud",
            HotServiceMode::Distributed => "hot-start-distributed",
        }
    }
}

pub fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../..")
        .canonicalize()
        .expect("failed to resolve workspace root")
}

fn normalize_language(value: &str) -> Option<String> {
    let language = value.trim();
    if !language.is_empty()
        && language.len() <= 32
        && language
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_')
    {
        Some(language.to_string())
    } else {
        None
    }
}

fn desktop_preferences_dir() -> Result<PathBuf, String> {
    shared_desktop_preferences_dir("kyuubiki")
}

fn global_language_path() -> Result<PathBuf, String> {
    Ok(desktop_preferences_dir()?.join(GLOBAL_LANGUAGE_FILE))
}

fn desktop_audit_path(file_name: &str) -> Result<PathBuf, String> {
    Ok(desktop_preferences_dir()?.join(file_name))
}

pub fn read_global_language_preference() -> Option<String> {
    let path = global_language_path().ok()?;
    let raw = fs::read_to_string(path).ok()?;
    normalize_language(raw.trim())
}

pub fn write_global_language_preference(language: &str) -> Result<String, String> {
    let normalized = normalize_language(language)
        .ok_or_else(|| format!("unsupported language preference: {language}"))?;
    let directory = desktop_preferences_dir()?;
    fs::create_dir_all(&directory)
        .map_err(|error| format!("failed to create {}: {error}", directory.display()))?;

    let path = directory.join(GLOBAL_LANGUAGE_FILE);
    fs::write(&path, normalized.as_bytes())
        .map_err(|error| format!("failed to write {}: {error}", path.display()))?;
    Ok(normalized.to_string())
}

pub fn report_packaged_boot_ready(surface: &str) -> Result<String, String> {
    if !matches!(surface, "hub" | "installer" | "workbench") {
        return Err(format!("unsupported packaged desktop surface: {surface}"));
    }
    let Some(value) = std::env::var_os(PACKAGED_BOOT_RECEIPT_ENV) else {
        return Ok("packaged boot receipt is not armed".to_string());
    };
    let path = validated_boot_receipt_path(Path::new(&value), &std::env::temp_dir())?;
    let receipt = json!({
        "schema_version": PACKAGED_BOOT_SCHEMA,
        "surface": surface,
        "version": env!("CARGO_PKG_VERSION"),
        "pid": std::process::id(),
    });
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .map_err(|error| format!("failed to create {}: {error}", path.display()))?;
    writeln!(
        file,
        "{}",
        serde_json::to_string(&receipt)
            .map_err(|error| format!("failed to serialize boot receipt: {error}"))?
    )
    .map_err(|error| format!("failed to write {}: {error}", path.display()))?;
    file.sync_all()
        .map_err(|error| format!("failed to sync {}: {error}", path.display()))?;
    Ok(format!("packaged boot ready: {surface}"))
}

fn validated_boot_receipt_path(path: &Path, temp_root: &Path) -> Result<PathBuf, String> {
    if !path.is_absolute() || path.extension().and_then(|value| value.to_str()) != Some("json") {
        return Err("packaged boot receipt must be an absolute .json path".to_string());
    }
    let parent = path
        .parent()
        .ok_or_else(|| "packaged boot receipt has no parent".to_string())?;
    let temp_root = temp_root
        .canonicalize()
        .map_err(|error| format!("failed to resolve temp directory: {error}"))?;
    let parent = parent
        .canonicalize()
        .map_err(|error| format!("failed to resolve receipt directory: {error}"))?;
    if !parent.starts_with(&temp_root) {
        return Err("packaged boot receipt must stay under the system temp directory".to_string());
    }
    Ok(parent.join(
        path.file_name()
            .ok_or_else(|| "packaged boot receipt has no file name".to_string())?,
    ))
}

pub fn service_status() -> Result<String, String> {
    runtime_control::service_status()
}

pub fn service_status_summary() -> Result<ServiceStatusSummary, String> {
    Ok(summarize_service_status(&service_status()?))
}

pub fn summarize_service_status(rendered: &str) -> ServiceStatusSummary {
    parse_service_status_summary(rendered)
}

pub fn service_start(mode: ServiceMode) -> Result<String, String> {
    runtime_control::service_start(mode)
}

pub fn service_restart(mode: ServiceMode) -> Result<String, String> {
    runtime_control::service_restart(mode)
}

pub fn service_stop() -> Result<String, String> {
    runtime_control::service_stop()
}

pub fn hot_service_status() -> Result<String, String> {
    runtime_control::hot_service_status()
}

pub fn hot_service_start(mode: HotServiceMode) -> Result<String, String> {
    runtime_control::hot_service_start(mode)
}

pub fn hot_service_stop() -> Result<String, String> {
    runtime_control::hot_service_stop()
}

pub fn export_database(url: Option<&str>) -> Result<String, String> {
    runtime_control::export_database(url)
}

pub fn log_path_for(service: &str) -> Result<PathBuf, String> {
    let paths = runtime_layout::runtime_paths()?;
    let filename = match service {
        "frontend" => "frontend.log",
        "orchestrator" => "orchestrator.log",
        "agent-5001" => "agent-5001.log",
        "agent-5002" => "agent-5002.log",
        "hot-stack" => {
            return Ok(paths.hot.join("stack.console.log"));
        }
        "hot-web" => {
            return Ok(paths.hot.join("web-4000.log"));
        }
        "hot-frontend" => {
            return Ok(paths.hot.join("frontend-3000.log"));
        }
        "hot-agent-5001" => {
            return Ok(paths.hot.join("agent-5001.log"));
        }
        "hot-agent-5002" => {
            return Ok(paths.hot.join("agent-5002.log"));
        }
        other => return Err(format!("unknown service log: {other}")),
    };

    Ok(paths.run.join(filename))
}

pub fn read_runtime_log(service: &str, max_lines: usize) -> Result<String, String> {
    let log_path = log_path_for(service)?;
    let contents = fs::read_to_string(&log_path)
        .map_err(|error| format!("failed to read {service} log: {error}"))?;
    let lines: Vec<&str> = contents.lines().collect();
    let start = lines.len().saturating_sub(max_lines);
    Ok(lines[start..].join("\n"))
}

fn parse_service_status_summary(rendered: &str) -> ServiceStatusSummary {
    let mut summary = ServiceStatusSummary {
        deployment_mode: "local".to_string(),
        control_mode: "standalone".to_string(),
        authority_mode: "self_directed".to_string(),
        orchestrator_status: "unknown".to_string(),
        frontend_status: "unknown".to_string(),
        agent_count: 0,
        active_agent_count: 0,
        agents: Vec::new(),
    };

    for line in rendered
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
    {
        if let Some(value) = line.strip_prefix("deployment-mode:") {
            summary.deployment_mode = value.trim().to_string();
            continue;
        }

        if let Some(value) = line.strip_prefix("control-mode:") {
            summary.control_mode = value.trim().to_string();
            continue;
        }

        if let Some(value) = line.strip_prefix("authority-mode:") {
            summary.authority_mode = value.trim().to_string();
            continue;
        }

        if let Some(status) = parse_named_status(line, "orchestrator") {
            summary.orchestrator_status = status;
            continue;
        }

        if let Some(status) = parse_named_status(line, "frontend") {
            summary.frontend_status = status;
            continue;
        }

        if let Some(status) = parse_agent_status(line) {
            summary.agent_count += 1;
            if status.status == "running" {
                summary.active_agent_count += 1;
            }
            summary.agents.push(status);
        }
    }

    summary
}

fn parse_named_status(line: &str, name: &str) -> Option<String> {
    let prefix = format!("{name}:");
    let value = line.strip_prefix(&prefix)?.trim();
    Some(
        if value.starts_with("running") || value.starts_with("listening") {
            "running".to_string()
        } else if value.starts_with("stopped") {
            "stopped".to_string()
        } else {
            "unknown".to_string()
        },
    )
}

fn parse_agent_status(line: &str) -> Option<ServiceEndpointSummary> {
    let (label, value) = line.split_once(':')?;
    if !label.starts_with("agent[") {
        return None;
    }

    let status = if value.trim().starts_with("running") || value.trim().starts_with("listening") {
        "running"
    } else if value.trim().starts_with("stopped") {
        "stopped"
    } else {
        "unknown"
    };

    Some(ServiceEndpointSummary {
        label: label.trim().to_string(),
        status: status.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::{
        ServiceEndpointSummary, ServiceStatusSummary, normalize_language,
        parse_service_status_summary, validated_boot_receipt_path, workspace_root,
    };

    #[test]
    fn workspace_root_points_to_repo_root() {
        let root = workspace_root();
        assert!(
            root.join("scripts").join("kyuubiki").is_file(),
            "workspace root should resolve to repo root, got {}",
            root.display()
        );
    }

    #[test]
    fn packaged_boot_receipt_rejects_paths_outside_temp_root() {
        let temp = std::env::temp_dir();
        let outside = workspace_root().join("receipt.json");
        assert!(validated_boot_receipt_path(&outside, &temp).is_err());
    }

    #[test]
    fn parses_service_status_summary() {
        let rendered = [
            "deployment-mode: distributed",
            "control-mode: orch_managed",
            "authority-mode: single_orchestrator",
            "orchestrator: running on http://127.0.0.1:4000 (pid 100)",
            "frontend: stopped",
            "agent[5001]: running on tcp://127.0.0.1:5001 (pid 101)",
            "agent[5002]: stopped",
        ]
        .join("\n");

        assert_eq!(
            parse_service_status_summary(&rendered),
            ServiceStatusSummary {
                deployment_mode: "distributed".to_string(),
                control_mode: "orch_managed".to_string(),
                authority_mode: "single_orchestrator".to_string(),
                orchestrator_status: "running".to_string(),
                frontend_status: "stopped".to_string(),
                agent_count: 2,
                active_agent_count: 1,
                agents: vec![
                    ServiceEndpointSummary {
                        label: "agent[5001]".to_string(),
                        status: "running".to_string(),
                    },
                    ServiceEndpointSummary {
                        label: "agent[5002]".to_string(),
                        status: "stopped".to_string(),
                    },
                ],
            }
        );
    }

    #[test]
    fn accepts_language_pack_locale_codes() {
        assert_eq!(normalize_language("fr-CA"), Some("fr-CA".to_string()));
        assert_eq!(
            normalize_language("ko_custom"),
            Some("ko_custom".to_string())
        );
        assert_eq!(normalize_language(""), None);
        assert_eq!(normalize_language("../fr"), None);
    }
}
