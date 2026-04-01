use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    let mut args = env::args().skip(1);
    let command = args.next().unwrap_or_else(|| "help".to_string());

    match command.as_str() {
        "help" | "--help" | "-h" => print_help(),
        "doctor" => run_doctor(),
        "init-env" => {
            let force = args.any(|arg| arg == "--force");
            match init_env(force) {
                Ok(message) => println!("{message}"),
                Err(error) => {
                    eprintln!("{error}");
                    std::process::exit(1);
                }
            }
        }
        "prepare-layout" => {
            if let Err(error) = prepare_layout() {
                eprintln!("{error}");
                std::process::exit(1);
            }
        }
        "export-launch" => export_launch_config(),
        "bootstrap" => {
            run_doctor();
            if let Err(error) = prepare_layout() {
                eprintln!("{error}");
                std::process::exit(1);
            }
            match init_env(false) {
                Ok(message) => println!("{message}"),
                Err(error) => {
                    eprintln!("{error}");
                    std::process::exit(1);
                }
            }
        }
        other => {
            eprintln!("unknown command: {other}");
            print_help();
            std::process::exit(1);
        }
    }
}

fn print_help() {
    println!(
        "kyuubiki-installer\n\nCommands:\n  help             Show this help\n  doctor           Check local prerequisites for the current platform\n  init-env         Create .env.local from .env.example when missing\n  prepare-layout   Create repo-local runtime folders\n  export-launch    Print a cross-platform launch manifest as JSON\n  bootstrap        Run doctor + prepare-layout + init-env"
    );
}

fn run_doctor() {
    let root = workspace_root();
    let platform = env::consts::OS;
    println!("kyuubiki installer doctor");
    println!("platform: {platform}");
    println!("workspace: {}", root.display());

    for command in ["node", "npm", "cargo", "mix"] {
        print_check(command, command_exists(command));
    }

    let postgres_ok = command_exists("psql") || command_exists("pg_isready");
    print_check("postgres-client", postgres_ok);

    let screen_ok = if platform == "windows" {
        command_exists("powershell")
    } else {
        command_exists("screen") || command_exists("tmux")
    };
    print_check("background-runner", screen_ok);

    let env_file = root.join(".env.local");
    print_check(".env.local", env_file.exists());
}

fn init_env(force: bool) -> Result<String, String> {
    let root = workspace_root();
    let env_file = root.join(".env.local");
    let example = root.join(".env.example");

    if env_file.exists() && !force {
        return Ok(format!("env already exists at {}", env_file.display()));
    }

    let contents =
        fs::read_to_string(&example).map_err(|error| format!("failed to read {}: {error}", example.display()))?;
    fs::write(&env_file, contents)
        .map_err(|error| format!("failed to write {}: {error}", env_file.display()))?;
    Ok(format!("wrote {}", env_file.display()))
}

fn prepare_layout() -> Result<(), String> {
    let root = workspace_root();
    for relative in ["tmp/run", "tmp/data"] {
        let path = root.join(relative);
        fs::create_dir_all(&path)
            .map_err(|error| format!("failed to create {}: {error}", path.display()))?;
        println!("prepared {}", path.display());
    }
    Ok(())
}

fn export_launch_config() {
    let root = workspace_root();
    let shell = if env::consts::OS == "windows" { "powershell" } else { "zsh" };
    let launch = format!(
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
        platform = env::consts::OS,
        shell = shell,
        workspace = escape_json(&root.display().to_string()),
        entry = escape_json(&format!("{shell} ./scripts/kyuubiki")),
    );
    print!("{launch}");
}

fn print_check(label: &str, ok: bool) {
    println!("[{}] {}", if ok { "ok" } else { "missing" }, label);
}

fn command_exists(command: &str) -> bool {
    let checker = if env::consts::OS == "windows" { "where" } else { "which" };
    Command::new(checker)
        .arg(command)
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../..")
        .canonicalize()
        .unwrap_or_else(|_| Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../.."))
}

fn escape_json(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}
