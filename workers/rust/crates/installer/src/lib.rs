use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DoctorCheck {
    pub label: String,
    pub ok: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DoctorReport {
    pub platform: String,
    pub workspace: String,
    pub checks: Vec<DoctorCheck>,
}

impl DoctorReport {
    pub fn render(&self) -> String {
        let mut lines = vec![
            "kyuubiki installer doctor".to_string(),
            format!("platform: {}", self.platform),
            format!("workspace: {}", self.workspace),
        ];

        for check in &self.checks {
            lines.push(format!(
                "[{}] {}",
                if check.ok { "ok" } else { "missing" },
                check.label
            ));
        }

        lines.join("\n")
    }
}

pub fn doctor_report() -> DoctorReport {
    let root = workspace_root();
    let platform = Platform::current();
    let postgres_ok = command_exists("psql") || command_exists("pg_isready");
    let background_runner_ok = if platform == Platform::Windows {
        command_exists("powershell")
    } else {
        command_exists("screen") || command_exists("tmux")
    };
    let env_file = root.join(".env.local");

    let mut checks = Vec::new();
    for command in ["node", "npm", "cargo", "mix"] {
        checks.push(DoctorCheck {
            label: command.to_string(),
            ok: command_exists(command),
        });
    }

    checks.push(DoctorCheck {
        label: "postgres-client".to_string(),
        ok: postgres_ok,
    });
    checks.push(DoctorCheck {
        label: "background-runner".to_string(),
        ok: background_runner_ok,
    });
    checks.push(DoctorCheck {
        label: ".env.local".to_string(),
        ok: env_file.exists(),
    });
    checks.push(DoctorCheck {
        label: "env-config".to_string(),
        ok: validate_env_file().is_ok(),
    });

    DoctorReport {
        platform: platform.as_str().to_string(),
        workspace: root.display().to_string(),
        checks,
    }
}

pub fn run_doctor() {
    println!("{}", doctor_report().render());
}

pub fn print_help() {
    println!(
        concat!(
            "kyuubiki-installer\n\n",
            "Commands:\n",
            "  help             Show this help\n",
            "  doctor           Check local prerequisites for the current platform\n",
            "  validate-env     Validate required environment variables from .env.local\n",
            "  init-env         Create .env.local from .env.example when missing\n",
            "  prepare-layout   Create repo-local runtime folders\n",
            "  export-launch    Print a cross-platform launch manifest as JSON\n",
            "  stage-release    Create a portable release directory layout under dist/\n",
            "  bootstrap        Run doctor + prepare-layout + init-env\n\n",
            "Examples:\n",
            "  cargo run -p kyuubiki-installer -- doctor\n",
            "  cargo run -p kyuubiki-installer -- stage-release\n",
            "  cargo run -p kyuubiki-installer -- stage-release windows ./dist/windows-preview\n",
        )
    );
}

pub fn init_env(force: bool) -> Result<String, String> {
    let root = workspace_root();
    let env_file = root.join(".env.local");
    let example = root.join(".env.example");

    if env_file.exists() && !force {
        return Ok(format!("env already exists at {}", env_file.display()));
    }

    let contents = fs::read_to_string(&example)
        .map_err(|error| format!("failed to read {}: {error}", example.display()))?;
    fs::write(&env_file, contents)
        .map_err(|error| format!("failed to write {}: {error}", env_file.display()))?;
    Ok(format!("wrote {}", env_file.display()))
}

pub fn prepare_layout() -> Result<String, String> {
    let root = workspace_root();
    let mut prepared = Vec::new();

    for relative in ["tmp/run", "tmp/data", "dist"] {
        let path = root.join(relative);
        fs::create_dir_all(&path)
            .map_err(|error| format!("failed to create {}: {error}", path.display()))?;
        prepared.push(path.display().to_string());
    }

    Ok(format!("prepared {}", prepared.join(", ")))
}

pub fn export_launch_config(platform: Platform) -> String {
    let root = workspace_root();
    let shell = platform.default_shell();
    let entry = platform.entrypoint_command();
    format!(
        concat!(
            "{{\n",
            "  \"schema_version\": \"kyuubiki.launch/v1\",\n",
            "  \"platform\": \"{platform}\",\n",
            "  \"shell\": \"{shell}\",\n",
            "  \"workspace\": \"{workspace}\",\n",
            "  \"services\": [\n",
            "    {{\"name\": \"frontend\", \"command\": \"{entry} frontend\"}},\n",
            "    {{\"name\": \"orchestrator\", \"command\": \"{entry} orchestrator\"}},\n",
            "    {{\"name\": \"agents\", \"command\": \"{entry} start\"}}\n",
            "  ]\n",
            "}}\n"
        ),
        platform = platform.as_str(),
        shell = shell,
        workspace = escape_json(&root.display().to_string()),
        entry = escape_json(entry),
    )
}

pub fn stage_release(platform: Platform, target_dir: Option<PathBuf>) -> Result<String, String> {
    let root = workspace_root();
    validate_env_file()?;
    let release_dir = target_dir.unwrap_or_else(|| root.join("dist").join(platform.as_str()));

    for relative in [
        "bin",
        "config",
        "data",
        "desktop/installer-gui",
        "desktop/workbench-gui",
        "logs",
        "manifests",
        "scripts",
        "exports",
    ] {
        fs::create_dir_all(release_dir.join(relative))
            .map_err(|error| format!("failed to create {}: {error}", release_dir.join(relative).display()))?;
    }

    let manifest_path = release_dir.join("manifests").join("release-manifest.json");
    let launch_path = release_dir.join("manifests").join("launch.json");
    let readme_path = release_dir.join("README.txt");
    let env_example_target = release_dir.join("config").join(".env.example");
    let desktop_readme_path = release_dir.join("desktop").join("README.txt");
    let installer_gui_manifest_path =
        release_dir.join("desktop").join("installer-gui").join("manifest.json");
    let workbench_gui_manifest_path =
        release_dir.join("desktop").join("workbench-gui").join("manifest.json");

    fs::write(&manifest_path, build_release_manifest(&root, &release_dir, platform))
        .map_err(|error| format!("failed to write {}: {error}", manifest_path.display()))?;
    fs::write(&launch_path, build_launch_manifest(&root, platform))
        .map_err(|error| format!("failed to write {}: {error}", launch_path.display()))?;
    fs::write(&readme_path, build_release_readme(platform))
        .map_err(|error| format!("failed to write {}: {error}", readme_path.display()))?;
    fs::write(&desktop_readme_path, build_desktop_readme())
        .map_err(|error| format!("failed to write {}: {error}", desktop_readme_path.display()))?;
    fs::write(
        &installer_gui_manifest_path,
        build_desktop_app_manifest("installer-gui", platform),
    )
    .map_err(|error| format!("failed to write {}: {error}", installer_gui_manifest_path.display()))?;
    fs::write(
        &workbench_gui_manifest_path,
        build_desktop_app_manifest("workbench-gui", platform),
    )
    .map_err(|error| format!("failed to write {}: {error}", workbench_gui_manifest_path.display()))?;

    let env_example = root.join(".env.example");
    if env_example.exists() {
        fs::copy(&env_example, &env_example_target)
            .map_err(|error| format!("failed to copy {}: {error}", env_example.display()))?;
    }

    write_release_scripts(&release_dir, platform)?;

    Ok(format!("staged release layout at {}", release_dir.display()))
}

fn write_release_scripts(release_dir: &Path, platform: Platform) -> Result<(), String> {
    let scripts_dir = release_dir.join("scripts");

    if platform == Platform::Windows {
        write_text_file(
            &scripts_dir.join("start.cmd"),
            "@echo off\r\ncd /d %~dp0\\..\\..\r\nzsh ./scripts/kyuubiki start\r\n",
        )?;
        write_text_file(
            &scripts_dir.join("stop.cmd"),
            "@echo off\r\ncd /d %~dp0\\..\\..\r\nzsh ./scripts/kyuubiki stop\r\n",
        )?;
        write_text_file(
            &scripts_dir.join("status.cmd"),
            "@echo off\r\ncd /d %~dp0\\..\\..\r\nzsh ./scripts/kyuubiki status\r\n",
        )?;
        write_text_file(
            &scripts_dir.join("export-db.cmd"),
            "@echo off\r\ncd /d %~dp0\\..\\..\r\nzsh ./scripts/kyuubiki export-db > .\\exports\\kyuubiki-database.json\r\n",
        )?;
    } else {
        write_text_file(
            &scripts_dir.join("start.sh"),
            "#!/bin/zsh\nset -e\ncd \"$(dirname \"$0\")/../..\"\nzsh ./scripts/kyuubiki start\n",
        )?;
        write_text_file(
            &scripts_dir.join("stop.sh"),
            "#!/bin/zsh\nset -e\ncd \"$(dirname \"$0\")/../..\"\nzsh ./scripts/kyuubiki stop\n",
        )?;
        write_text_file(
            &scripts_dir.join("status.sh"),
            "#!/bin/zsh\nset -e\ncd \"$(dirname \"$0\")/../..\"\nzsh ./scripts/kyuubiki status\n",
        )?;
        write_text_file(
            &scripts_dir.join("export-db.sh"),
            "#!/bin/zsh\nset -e\ncd \"$(dirname \"$0\")/../..\"\nzsh ./scripts/kyuubiki export-db > ./dist/exports/kyuubiki-database.json\n",
        )?;
    }

    Ok(())
}

fn build_release_manifest(root: &Path, release_dir: &Path, platform: Platform) -> String {
    format!(
        concat!(
            "{{\n",
            "  \"schema_version\": \"kyuubiki.release/v1\",\n",
            "  \"platform\": \"{platform}\",\n",
            "  \"release_dir\": \"{release_dir}\",\n",
            "  \"workspace\": \"{workspace}\",\n",
            "  \"layout\": [\n",
            "    \"bin\",\n",
            "    \"config\",\n",
            "    \"data\",\n",
            "    \"desktop\",\n",
            "    \"logs\",\n",
            "    \"exports\",\n",
            "    \"manifests\",\n",
            "    \"scripts\"\n",
            "  ],\n",
            "  \"recommended_flow\": [\n",
            "    \"scripts/start\",\n",
            "    \"scripts/status\",\n",
            "    \"scripts/export-db\"\n",
            "  ]\n",
            "}}\n"
        ),
        platform = platform.as_str(),
        release_dir = escape_json(&release_dir.display().to_string()),
        workspace = escape_json(&root.display().to_string()),
    )
}

fn build_launch_manifest(root: &Path, platform: Platform) -> String {
    format!(
        concat!(
            "{{\n",
            "  \"schema_version\": \"kyuubiki.launch/v1\",\n",
            "  \"platform\": \"{platform}\",\n",
            "  \"shell\": \"{shell}\",\n",
            "  \"workspace\": \"{workspace}\",\n",
            "  \"entrypoint\": \"{entry}\",\n",
            "  \"deployment_profiles\": [\"local\", \"cloud\", \"distributed\"],\n",
            "  \"agent_discovery\": [\"static\", \"manifest\"],\n",
            "  \"services\": [\n",
            "    {{\"name\": \"frontend\", \"port\": 3000}},\n",
            "    {{\"name\": \"orchestrator\", \"port\": 4000}},\n",
            "    {{\"name\": \"agent-1\", \"port\": 5001}},\n",
            "    {{\"name\": \"agent-2\", \"port\": 5002}}\n",
            "  ]\n",
            "}}\n"
        ),
        platform = platform.as_str(),
        shell = platform.default_shell(),
        workspace = escape_json(&root.display().to_string()),
        entry = escape_json(platform.entrypoint_command()),
    )
}

fn build_release_readme(platform: Platform) -> String {
    format!(
        concat!(
            "Kyuubiki portable release scaffold\n\n",
            "Platform: {platform}\n\n",
            "This directory is a repo-local deployment scaffold.\n",
            "It is intentionally lightweight and does not install host-level packages.\n\n",
            "Directory shape:\n",
            "- bin/        component binaries or launch helpers\n",
            "- config/     environment and deployment configuration\n",
            "- data/       local runtime state\n",
            "- desktop/    desktop-shell packaging placeholders\n",
            "- logs/       runtime logs\n",
            "- manifests/  release and launch manifests\n",
            "- scripts/    operator entry points\n",
            "- exports/    snapshots and operator exports\n\n",
            "Suggested flow:\n",
            "1. copy config/.env.example to config/.env.local when packaging externally\n",
            "2. use scripts/start to launch services\n",
            "3. use scripts/export-db to snapshot persisted data\n"
        ),
        platform = platform.as_str()
    )
}

fn build_desktop_readme() -> String {
    concat!(
        "Desktop packaging placeholders\n\n",
        "installer-gui/\n",
        "  Reserved for the Tauri installer GUI build output or packaged bundle references.\n",
        "  Contains a manifest.json packaging descriptor.\n\n",
        "workbench-gui/\n",
        "  Reserved for the Tauri desktop workbench shell build output or packaged bundle references.\n",
        "  Contains a manifest.json packaging descriptor.\n"
    )
    .to_string()
}

fn build_desktop_app_manifest(app: &str, platform: Platform) -> String {
    let (product_name, source_dir, target_dir, build_command) = match app {
        "installer-gui" => (
            "Kyuubiki Installer",
            "apps/installer-gui",
            "apps/installer-gui/src-tauri/target",
            "make build-installer-gui",
        ),
        _ => (
            "Kyuubiki Workbench",
            "apps/workbench-gui",
            "apps/workbench-gui/src-tauri/target",
            "make build-workbench-gui",
        ),
    };

    format!(
        concat!(
            "{{\n",
            "  \"schema_version\": \"kyuubiki.desktop-package/v1\",\n",
            "  \"app\": \"{app}\",\n",
            "  \"product_name\": \"{product_name}\",\n",
            "  \"platform\": \"{platform}\",\n",
            "  \"expected_bundle_kinds\": [{bundle_kinds}],\n",
            "  \"source_dir\": \"{source_dir}\",\n",
            "  \"tauri_target_dir\": \"{target_dir}\",\n",
            "  \"build_command\": \"{build_command}\",\n",
            "  \"notes\": \"Use the platform-scoped desktop packaging flow to populate this placeholder.\"\n",
            "}}\n"
        ),
        app = app,
        product_name = product_name,
        platform = platform.as_str(),
        bundle_kinds = platform.desktop_bundle_kinds_json(),
        source_dir = source_dir,
        target_dir = target_dir,
        build_command = build_command,
    )
}

fn write_text_file(path: &Path, contents: &str) -> Result<(), String> {
    fs::write(path, contents).map_err(|error| format!("failed to write {}: {error}", path.display()))
}

pub fn validate_env_file() -> Result<String, String> {
    let root = workspace_root();
    let env_file = root.join(".env.local");
    let env_map = parse_env_file(&env_file)?;
    let deployment_mode = env_map
        .get("KYUUBIKI_DEPLOYMENT_MODE")
        .map(String::as_str)
        .unwrap_or("local");
    let agent_discovery = env_map
        .get("KYUUBIKI_AGENT_DISCOVERY")
        .map(String::as_str)
        .unwrap_or("static");

    let storage_backend = env_map
        .get("KYUUBIKI_STORAGE_BACKEND")
        .ok_or_else(|| missing_env("KYUUBIKI_STORAGE_BACKEND"))?;

    if !matches!(deployment_mode, "local" | "cloud" | "distributed") {
        return Err(format!(
            "invalid KYUUBIKI_DEPLOYMENT_MODE: {deployment_mode} (expected local, cloud, or distributed)"
        ));
    }

    if !matches!(storage_backend.as_str(), "postgres" | "sqlite" | "memory" | "json") {
        return Err(format!(
            "invalid KYUUBIKI_STORAGE_BACKEND: {storage_backend} (expected postgres, sqlite, memory, or json)"
        ));
    }

    if storage_backend == "postgres" {
        let database_url = env_map
            .get("DATABASE_URL")
            .ok_or_else(|| missing_env("DATABASE_URL"))?;

        if !(database_url.starts_with("ecto://") || database_url.starts_with("postgres://")) {
            return Err(
                "invalid DATABASE_URL: expected ecto://... or postgres://... when using postgres storage".to_string(),
            );
        }
    }

    if storage_backend == "sqlite" {
        let sqlite_path = env_map
            .get("SQLITE_DATABASE_PATH")
            .map(String::as_str)
            .unwrap_or("./tmp/data/kyuubiki_dev.sqlite3");

        if !sqlite_path.ends_with(".sqlite3") && !sqlite_path.ends_with(".db") {
            return Err(
                "invalid SQLITE_DATABASE_PATH: expected a .sqlite3 or .db file path when using sqlite storage"
                    .to_string(),
            );
        }
    }

    let endpoints = match agent_discovery {
        "static" => {
            let agent_endpoints = env_map
                .get("KYUUBIKI_AGENT_ENDPOINTS")
                .map(String::as_str)
                .unwrap_or("127.0.0.1:5001,127.0.0.1:5002");

            parse_agent_endpoints(agent_endpoints)?
        }
        "manifest" => {
            let manifest_path = env_map
                .get("KYUUBIKI_AGENT_MANIFEST_PATH")
                .ok_or_else(|| missing_env("KYUUBIKI_AGENT_MANIFEST_PATH"))?;

            validate_agent_manifest_path(manifest_path)?;
            parse_agent_manifest(Path::new(manifest_path))?
        }
        "registry" => vec![("runtime-registry".to_string(), 0)],
        _ => {
            return Err(format!(
                "invalid KYUUBIKI_AGENT_DISCOVERY: {agent_discovery} (expected static, manifest, or registry)"
            ))
        }
    };

    Ok(format!(
        "validated .env.local (deployment={}, storage={}, discovery={}, agents={})",
        deployment_mode,
        storage_backend,
        agent_discovery,
        endpoints.len()
    ))
}

pub fn parse_env_file(path: &Path) -> Result<BTreeMap<String, String>, String> {
    let contents =
        fs::read_to_string(path).map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    let mut values = BTreeMap::new();

    for raw_line in contents.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let Some((key, value)) = line.split_once('=') else {
            return Err(format!("invalid env line in {}: {}", path.display(), raw_line));
        };

        values.insert(key.trim().to_string(), value.trim().to_string());
    }

    Ok(values)
}

pub fn parse_agent_endpoints(value: &str) -> Result<Vec<(String, u16)>, String> {
    let mut parsed = Vec::new();

    for endpoint in value.split(',').map(str::trim).filter(|item| !item.is_empty()) {
        let Some((host, port)) = endpoint.rsplit_once(':') else {
            return Err(format!("invalid KYUUBIKI_AGENT_ENDPOINTS entry: {endpoint}"));
        };

        let port_number = port
            .parse::<u16>()
            .map_err(|_| format!("invalid agent port in KYUUBIKI_AGENT_ENDPOINTS: {endpoint}"))?;

        if host.trim().is_empty() {
            return Err(format!("invalid agent host in KYUUBIKI_AGENT_ENDPOINTS: {endpoint}"));
        }

        parsed.push((host.trim().to_string(), port_number));
    }

    if parsed.is_empty() {
        return Err("KYUUBIKI_AGENT_ENDPOINTS must contain at least one host:port pair".to_string());
    }

    Ok(parsed)
}

fn validate_agent_manifest_path(path: &str) -> Result<(), String> {
    if !(path.ends_with(".json")) {
        return Err(
            "invalid KYUUBIKI_AGENT_MANIFEST_PATH: expected a .json file path when using manifest discovery"
                .to_string(),
        );
    }

    Ok(())
}

fn parse_agent_manifest(path: &Path) -> Result<Vec<(String, u16)>, String> {
    let contents =
        fs::read_to_string(path).map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    let payload: serde_json::Value =
        serde_json::from_str(&contents).map_err(|error| format!("invalid JSON in {}: {error}", path.display()))?;
    let agents = payload
        .get("agents")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| format!("invalid agent manifest {}: missing agents array", path.display()))?;

    let mut parsed = Vec::new();

    for agent in agents {
        let host = agent
            .get("host")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| format!("invalid agent manifest {}: agent host missing", path.display()))?;
        let port = agent
            .get("port")
            .and_then(serde_json::Value::as_u64)
            .ok_or_else(|| format!("invalid agent manifest {}: agent port missing", path.display()))?;

        if !(1..=u16::MAX as u64).contains(&port) {
            return Err(format!("invalid agent manifest {}: agent port out of range", path.display()));
        }

        parsed.push((host.to_string(), port as u16));
    }

    if parsed.is_empty() {
        return Err(format!("invalid agent manifest {}: no agents found", path.display()));
    }

    Ok(parsed)
}

fn missing_env(key: &str) -> String {
    format!("missing required env var: {key}")
}

pub fn exit_on_err(result: Result<String, String>) {
    match result {
        Ok(message) => println!("{message}"),
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(1);
        }
    }
}

fn command_exists(command: &str) -> bool {
    let checker = if Platform::current() == Platform::Windows {
        "where"
    } else {
        "which"
    };

    Command::new(checker)
        .arg(command)
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

pub fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../..")
        .canonicalize()
        .unwrap_or_else(|_| Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../.."))
}

pub fn parse_platform(value: Option<String>) -> Platform {
    match value.as_deref() {
        Some("macos") => Platform::Macos,
        Some("linux") => Platform::Linux,
        Some("windows") => Platform::Windows,
        Some(_) | None => Platform::current(),
    }
}

fn escape_json(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Platform {
    Macos,
    Linux,
    Windows,
}

impl Platform {
    pub fn current() -> Self {
        match env::consts::OS {
            "macos" => Self::Macos,
            "windows" => Self::Windows,
            _ => Self::Linux,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Macos => "macos",
            Self::Linux => "linux",
            Self::Windows => "windows",
        }
    }

    fn default_shell(self) -> &'static str {
        match self {
            Self::Windows => "powershell",
            Self::Macos | Self::Linux => "zsh",
        }
    }

    fn entrypoint_command(self) -> &'static str {
        match self {
            Self::Windows => "zsh ./scripts/kyuubiki",
            Self::Macos | Self::Linux => "zsh ./scripts/kyuubiki",
        }
    }

    fn desktop_bundle_kinds_json(self) -> &'static str {
        match self {
            Self::Macos => "\"app\", \"dmg\"",
            Self::Linux => "\"appimage\", \"deb\", \"rpm\"",
            Self::Windows => "\"msi\", \"nsis\"",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_unknown_platform_to_current() {
        assert_eq!(parse_platform(Some("unknown".to_string())), Platform::current());
    }

    #[test]
    fn release_manifest_contains_expected_schema() {
        let manifest =
            build_release_manifest(Path::new("/tmp/workspace"), Path::new("/tmp/dist/macos"), Platform::Macos);
        assert!(manifest.contains("\"schema_version\": \"kyuubiki.release/v1\""));
        assert!(manifest.contains("\"platform\": \"macos\""));
    }

    #[test]
    fn parses_agent_endpoint_list() {
        let parsed = parse_agent_endpoints("127.0.0.1:5001,solver.local:5002").unwrap();
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].0, "127.0.0.1");
        assert_eq!(parsed[1].1, 5002);
    }

    #[test]
    fn rejects_invalid_agent_endpoint_list() {
        assert!(parse_agent_endpoints("127.0.0.1").is_err());
        assert!(parse_agent_endpoints("127.0.0.1:abc").is_err());
    }

    #[test]
    fn parses_agent_manifest_file() {
        let path = workspace_root().join("tmp").join("installer-agent-manifest-test.json");
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(
            &path,
            r#"{
              "schema_version": "kyuubiki.agent-manifest/v1",
              "deployment_mode": "distributed",
              "agents": [
                {"id": "solver-a", "host": "10.0.0.11", "port": 6101},
                {"id": "solver-b", "host": "10.0.0.12", "port": 6102}
              ]
            }"#,
        )
        .unwrap();

        let parsed = parse_agent_manifest(&path).unwrap();
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].0, "10.0.0.11");
        assert_eq!(parsed[1].1, 6102);

        fs::remove_file(path).unwrap();
    }
}
