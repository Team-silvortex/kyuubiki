use std::fs;
use std::path::PathBuf;
use std::process::Command;

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

pub fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .expect("failed to resolve workspace root")
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
    run_workspace_command(&["zsh", "./scripts/kyuubiki", "status"])
}

pub fn service_start(mode: ServiceMode) -> Result<String, String> {
    run_workspace_command(&["zsh", "./scripts/kyuubiki", mode.start_command()])
}

pub fn service_restart(mode: ServiceMode) -> Result<String, String> {
    run_workspace_command(&["zsh", "./scripts/kyuubiki", mode.restart_command()])
}

pub fn service_stop() -> Result<String, String> {
    run_workspace_command(&["zsh", "./scripts/kyuubiki", "stop"])
}

pub fn log_path_for(service: &str) -> Result<PathBuf, String> {
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

pub fn read_runtime_log(service: &str, max_lines: usize) -> Result<String, String> {
    let log_path = log_path_for(service)?;
    let contents = fs::read_to_string(&log_path)
        .map_err(|error| format!("failed to read {}: {error}", log_path.display()))?;
    let lines: Vec<&str> = contents.lines().collect();
    let start = lines.len().saturating_sub(max_lines);
    Ok(lines[start..].join("\n"))
}
