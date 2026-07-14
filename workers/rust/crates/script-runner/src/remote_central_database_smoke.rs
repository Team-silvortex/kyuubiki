use crate::remote_host::shell_escape;
use std::env;
use std::ffi::OsString;
use std::path::Path;
use std::process::Command;

type RunnerResult<T> = Result<T, String>;

const DEFAULT_REMOTE_DIR: &str = ".kyuubiki-remote-runs/central-database-smoke";
const DEFAULT_DRY_RUN_DATABASE_URL: &str = "ecto://postgres:postgres@127.0.0.1:5432/kyuubiki_dev";

pub(crate) fn run_remote_central_database_smoke(
    root: &Path,
    args: Vec<OsString>,
) -> RunnerResult<u8> {
    let options = Options::from_env_and_args(args)?;
    let database_url = database_url(&options);
    print_plan(&options, &database_url);
    let database_url = database_url.ok_or_else(|| {
        "DATABASE_URL is required when RUN_DB_SMOKE=1 or --run is used".to_string()
    })?;
    if options.plan_only {
        println!("plan-only: remote sync and smoke execution skipped");
        return Ok(0);
    }
    sync_sources(root, &options)?;
    run_status(
        root,
        "ssh",
        ssh_args(&options.host, remote_command(&options, &database_url)),
    )
}

#[derive(Debug, Eq, PartialEq)]
struct Options {
    backend: String,
    host: String,
    mode: String,
    plan_only: bool,
    remote_dir: String,
    run_smoke: bool,
}

impl Options {
    fn from_env_and_args(args: Vec<OsString>) -> RunnerResult<Self> {
        let mut options = Self {
            backend: env::var("BACKEND").unwrap_or_else(|_| "postgres".to_string()),
            host: env::var("KYUUBIKI_LAB_HOST")
                .or_else(|_| env::var("REMOTE"))
                .unwrap_or_else(|_| "kyuubiki-lab".to_string()),
            mode: env::var("MODE").unwrap_or_else(|_| "cloud".to_string()),
            plan_only: env::var("PLAN_ONLY").is_ok_and(|value| value == "1"),
            remote_dir: env::var("KYUUBIKI_REMOTE_CENTRAL_DB_DIR")
                .unwrap_or_else(|_| DEFAULT_REMOTE_DIR.to_string()),
            run_smoke: env::var("RUN_DB_SMOKE").is_ok_and(|value| value == "1"),
        };
        let mut iter = args.into_iter();
        while let Some(arg) = iter.next() {
            match arg.to_string_lossy().as_ref() {
                "--help" | "-h" => {
                    print_usage();
                    return Ok(options);
                }
                "--host" => options.host = string_arg(&mut iter, "--host")?,
                "--remote-dir" => options.remote_dir = string_arg(&mut iter, "--remote-dir")?,
                "--mode" => options.mode = string_arg(&mut iter, "--mode")?,
                "--backend" => options.backend = string_arg(&mut iter, "--backend")?,
                "--run" => options.run_smoke = true,
                "--plan-only" => options.plan_only = true,
                other => return Err(format!("unknown or incomplete argument: {other}")),
            }
        }
        validate_remote_dir(&options.remote_dir)?;
        Ok(options)
    }
}

fn database_url(options: &Options) -> Option<String> {
    let value = env::var("DATABASE_URL")
        .ok()
        .or_else(|| (!options.run_smoke).then(|| DEFAULT_DRY_RUN_DATABASE_URL.to_string()))
        .unwrap_or_default();
    if value.is_empty() {
        return None;
    }
    Some(value)
}

fn print_plan(options: &Options, database_url: &Option<String>) {
    println!("remote host: {}", options.host);
    println!("remote dir: {}", options.remote_dir);
    println!("mode/backend: {}/{}", options.mode, options.backend);
    println!(
        "execute smoke: {}",
        if options.run_smoke {
            "yes"
        } else {
            "no (dry-run)"
        }
    );
    println!(
        "database url: {}",
        database_url
            .as_deref()
            .map(redact_database_url)
            .unwrap_or_else(|| "missing".to_string())
    );
}

fn sync_sources(root: &Path, options: &Options) -> RunnerResult<()> {
    require_ok(
        "remote mkdir",
        run_status(
            root,
            "ssh",
            ssh_args(
                &options.host,
                format!("mkdir -p {}", shell_escape(&options.remote_dir)),
            ),
        )?,
    )?;
    let excludes = [
        ".git/",
        ".DS_Store",
        "_build/",
        ".next/",
        "deps/",
        "dist/",
        "node_modules/",
        "target/",
        "tmp/",
    ];
    let mut args = vec![
        OsString::from("-az"),
        OsString::from("--delete"),
        OsString::from("-e"),
        OsString::from("ssh -o BatchMode=yes -o ConnectTimeout=10"),
    ];
    for exclude in excludes {
        args.push(OsString::from(format!("--exclude={exclude}")));
    }
    args.push(OsString::from(format!("{}/", root.display())));
    args.push(OsString::from(format!(
        "{}:{}/",
        options.host, options.remote_dir
    )));
    require_ok("rsync source sync", run_status(root, "rsync", args)?)?;
    Ok(())
}

fn remote_command(options: &Options, database_url: &str) -> String {
    [
        "set -euo pipefail".to_string(),
        "export PATH=\"$HOME/.kyuubiki-toolchains/node-v20.19.2-linux-x64/bin:$HOME/.elixir-install/installs/elixir/1.20.1-otp-28/bin:$HOME/.elixir-install/installs/otp/28.4/bin:$PATH\"".to_string(),
        format!("cd {}", shell_escape(&options.remote_dir)),
        format!("export MODE={}", shell_escape(&options.mode)),
        format!("export BACKEND={}", shell_escape(&options.backend)),
        format!("export RUN_DB_SMOKE={}", shell_escape(if options.run_smoke { "1" } else { "0" })),
        format!("export DATABASE_URL={}", shell_escape(database_url)),
        "./scripts/kyuubiki check-central-database-readiness --mode \"$MODE\" --backend \"$BACKEND\" --json".to_string(),
        "./scripts/kyuubiki central-database-smoke --mode \"$MODE\" --backend \"$BACKEND\"".to_string(),
    ]
    .join("; ")
}

fn ssh_args(host: &str, command: String) -> Vec<OsString> {
    ["-o", "BatchMode=yes", "-o", "ConnectTimeout=10"]
        .into_iter()
        .map(OsString::from)
        .chain([OsString::from(host), OsString::from(command)])
        .collect()
}

fn validate_remote_dir(remote_dir: &str) -> RunnerResult<()> {
    if remote_dir.is_empty() || remote_dir.starts_with('/') || remote_dir.contains("..") {
        return Err("remote dir must be a relative scratch path without '..'".to_string());
    }
    if !remote_dir
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '-' | '/'))
    {
        return Err(
            "remote dir may only contain letters, numbers, '.', '_', '-', and '/'".to_string(),
        );
    }
    Ok(())
}

fn string_arg(iter: &mut impl Iterator<Item = OsString>, flag: &str) -> RunnerResult<String> {
    iter.next()
        .map(|value| value.to_string_lossy().into_owned())
        .ok_or_else(|| format!("unknown or incomplete argument: {flag}"))
}

fn run_status<I>(root: &Path, program: &str, args: I) -> RunnerResult<u8>
where
    I: IntoIterator<Item = OsString>,
{
    let status = Command::new(program)
        .args(args)
        .current_dir(root)
        .status()
        .map_err(|error| format!("failed to run {program}: {error}"))?;
    Ok(status.code().unwrap_or(1) as u8)
}

fn require_ok(label: &str, status: u8) -> RunnerResult<()> {
    if status == 0 {
        Ok(())
    } else {
        Err(format!("{label} failed with exit code {status}"))
    }
}

fn redact_database_url(value: &str) -> String {
    let Some(scheme_end) = value.find("//") else {
        return value.to_string();
    };
    let auth_start = scheme_end + 2;
    let Some(relative_at) = value[auth_start..].find('@') else {
        return value.to_string();
    };
    let at = auth_start + relative_at;
    let auth = &value[auth_start..at];
    let Some(colon) = auth.find(':') else {
        return value.to_string();
    };
    format!(
        "{}{}:***{}",
        &value[..auth_start],
        &auth[..colon],
        &value[at..]
    )
}

fn print_usage() {
    println!(
        "Usage:\n  ./scripts/kyuubiki remote-central-database-smoke [options]\n\n\
Options:\n  --host <ssh-host>       Default: KYUUBIKI_LAB_HOST, REMOTE, or kyuubiki-lab\n  \
--remote-dir <path>     Relative remote scratch dir, default: .kyuubiki-remote-runs/central-database-smoke\n  \
--mode <mode>           Default: MODE or cloud\n  \
--backend <backend>     Default: BACKEND or postgres\n  \
--run                   Execute DB-backed tests; otherwise dry-run only\n  \
--plan-only             Print the safe execution plan without SSH/rsync\n\n\
The runner never stores credentials or server config in the repository. It uses\n\
the existing SSH key/config and forwards DATABASE_URL only through the process\n\
environment for the current run."
    );
}
