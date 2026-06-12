use std::fs;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;

const GLOBAL_LANGUAGE_FILE: &str = "desktop-language.txt";

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

fn normalize_language(value: &str) -> Option<&'static str> {
    match value.trim() {
        "en" => Some("en"),
        "zh" => Some("zh"),
        "ja" => Some("ja"),
        "es" => Some("es"),
        _ => None,
    }
}

fn desktop_preferences_dir() -> Result<PathBuf, String> {
    #[cfg(target_os = "macos")]
    {
        let home = std::env::var_os("HOME")
            .map(PathBuf::from)
            .ok_or_else(|| "HOME is not available".to_string())?;
        return Ok(home
            .join("Library")
            .join("Application Support")
            .join("kyuubiki"));
    }

    #[cfg(target_os = "windows")]
    {
        let appdata = std::env::var_os("APPDATA")
            .map(PathBuf::from)
            .ok_or_else(|| "APPDATA is not available".to_string())?;
        return Ok(appdata.join("kyuubiki"));
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        if let Some(config_home) = std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from) {
            return Ok(config_home.join("kyuubiki"));
        }

        let home = std::env::var_os("HOME")
            .map(PathBuf::from)
            .ok_or_else(|| "HOME is not available".to_string())?;
        Ok(home.join(".config").join("kyuubiki"))
    }
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
    normalize_language(raw.trim()).map(str::to_string)
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

pub fn append_desktop_audit_line(file_name: &str, line: &str) -> Result<(), String> {
    let directory = desktop_preferences_dir()?;
    fs::create_dir_all(&directory)
        .map_err(|error| format!("failed to create {}: {error}", directory.display()))?;

    let path = desktop_audit_path(file_name)?;
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .map_err(|error| format!("failed to open {}: {error}", path.display()))?;
    writeln!(file, "{line}")
        .map_err(|error| format!("failed to append {}: {error}", path.display()))?;
    Ok(())
}

pub fn run_workspace_command(args: &[&str]) -> Result<String, String> {
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

pub fn service_status() -> Result<String, String> {
    run_workspace_command(&["node", "./scripts/kyuubiki-runtime.mjs", "status"])
}

pub fn service_start(mode: ServiceMode) -> Result<String, String> {
    run_workspace_command(&[
        "node",
        "./scripts/kyuubiki-runtime.mjs",
        mode.start_command(),
    ])
}

pub fn service_restart(mode: ServiceMode) -> Result<String, String> {
    run_workspace_command(&[
        "node",
        "./scripts/kyuubiki-runtime.mjs",
        mode.restart_command(),
    ])
}

pub fn service_stop() -> Result<String, String> {
    run_workspace_command(&["node", "./scripts/kyuubiki-runtime.mjs", "stop"])
}

pub fn hot_service_status() -> Result<String, String> {
    run_workspace_command(&["node", "./scripts/kyuubiki-runtime.mjs", "hot-status"])
}

pub fn hot_service_start(mode: HotServiceMode) -> Result<String, String> {
    run_workspace_command(&[
        "node",
        "./scripts/kyuubiki-runtime.mjs",
        mode.start_command(),
    ])
}

pub fn hot_service_stop() -> Result<String, String> {
    run_workspace_command(&["node", "./scripts/kyuubiki-runtime.mjs", "hot-stop"])
}

pub fn log_path_for(service: &str) -> Result<PathBuf, String> {
    let root = workspace_root();
    let filename = match service {
        "frontend" => "frontend.log",
        "orchestrator" => "orchestrator.log",
        "agent-5001" => "agent-5001.log",
        "agent-5002" => "agent-5002.log",
        "hot-stack" => {
            return Ok(root
                .join("tmp")
                .join("run")
                .join("hot")
                .join("stack.console.log"));
        }
        "hot-web" => {
            return Ok(root
                .join("tmp")
                .join("run")
                .join("hot")
                .join("web-4000.log"));
        }
        "hot-frontend" => {
            return Ok(root
                .join("tmp")
                .join("run")
                .join("hot")
                .join("frontend-3000.log"));
        }
        "hot-agent-5001" => {
            return Ok(root
                .join("tmp")
                .join("run")
                .join("hot")
                .join("agent-5001.log"));
        }
        "hot-agent-5002" => {
            return Ok(root
                .join("tmp")
                .join("run")
                .join("hot")
                .join("agent-5002.log"));
        }
        other => return Err(format!("unknown service log: {other}")),
    };

    Ok(root.join("tmp").join("run").join(filename))
}

pub fn read_runtime_log(service: &str, max_lines: usize) -> Result<String, String> {
    let log_path = log_path_for(service)?;
    let contents = fs::read_to_string(&log_path)
        .map_err(|error| format!("failed to read {} log: {error}", service))?;
    let lines: Vec<&str> = contents.lines().collect();
    let start = lines.len().saturating_sub(max_lines);
    Ok(lines[start..].join("\n"))
}

#[cfg(test)]
mod tests {
    use super::workspace_root;

    #[test]
    fn workspace_root_points_to_repo_root() {
        let root = workspace_root();
        assert!(
            root.join("scripts").join("kyuubiki").is_file(),
            "workspace root should resolve to repo root, got {}",
            root.display()
        );
    }
}
