use crate::native_time::utc_timestamp_slug;
use crate::remote_host::shell_escape;
use std::env;
use std::ffi::OsString;
use std::fs;
use std::path::Path;
use std::process::Command;

const DEFAULT_REMOTE_DIR: &str = ".kyuubiki-remote-runs/material-research-example";

type RunnerResult<T> = Result<T, String>;

pub(crate) fn run_remote_material_research_example(
    root: &Path,
    args: Vec<OsString>,
) -> RunnerResult<u8> {
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        print_usage();
        return Ok(0);
    }
    if args.iter().any(|arg| arg == "--self-test") {
        return run_self_test();
    }

    let options = Options::from_env_and_args(args)?;
    let local_dir = root
        .join("tmp")
        .join("remote-material-research")
        .join(&options.output_slug);

    println!("remote host: {}", options.host);
    println!("remote dir: {}", options.remote_dir);
    println!(
        "benchmark: profile={} matrix={} repeat={} solver_preconditioner={} case={}",
        options.profile,
        options.matrix,
        options.repeat,
        options.solver_preconditioner,
        options.case_filter.as_deref().unwrap_or("all")
    );

    sync_sources(root, &options)?;
    require_ok(
        "remote material research run",
        ssh_status(root, &options.host, remote_command(&options))?,
    )?;
    pull_evidence(root, &options, &local_dir)?;
    require_ok(
        "remote material benchmark summary",
        run_status(
            root,
            "./scripts/kyuubiki",
            [OsString::from("build-remote-material-benchmark-summary")],
        )?,
    )?;
    println!("remote material research evidence: {}", local_dir.display());
    Ok(0)
}

#[derive(Debug, Eq, PartialEq)]
struct Options {
    case_filter: Option<String>,
    host: String,
    matrix: String,
    output_slug: String,
    profile: String,
    remote_dir: String,
    repeat: String,
    solver_preconditioner: String,
}

impl Options {
    fn from_env_and_args(args: Vec<OsString>) -> RunnerResult<Self> {
        let mut options = Self {
            case_filter: env_value("CASE_FILTER"),
            host: env::var("KYUUBIKI_LAB_HOST").unwrap_or_else(|_| "kyuubiki-lab".to_string()),
            matrix: env::var("MATRIX").unwrap_or_else(|_| "compound-core".to_string()),
            output_slug: env::var("OUTPUT_SLUG")
                .unwrap_or_else(|_| format!("material-research-remote-{}", utc_timestamp_slug())),
            profile: env::var("PROFILE").unwrap_or_else(|_| "100k".to_string()),
            remote_dir: env::var("KYUUBIKI_REMOTE_MATERIAL_DIR")
                .unwrap_or_else(|_| DEFAULT_REMOTE_DIR.to_string()),
            repeat: env::var("REPEAT").unwrap_or_else(|_| "1".to_string()),
            solver_preconditioner: env::var("SOLVER_PRECONDITIONER")
                .unwrap_or_else(|_| "auto".to_string()),
        };
        let mut iter = args.into_iter();
        while let Some(arg) = iter.next() {
            match arg.to_string_lossy().as_ref() {
                "--host" => options.host = required_value(&mut iter, "--host")?,
                "--case" => options.case_filter = Some(required_value(&mut iter, "--case")?),
                "--remote-dir" => options.remote_dir = required_value(&mut iter, "--remote-dir")?,
                "--profile" => options.profile = required_value(&mut iter, "--profile")?,
                "--matrix" => options.matrix = required_value(&mut iter, "--matrix")?,
                "--repeat" => options.repeat = required_value(&mut iter, "--repeat")?,
                "--output-slug" => {
                    options.output_slug = required_value(&mut iter, "--output-slug")?
                }
                "--solver-preconditioner" => {
                    options.solver_preconditioner =
                        required_value(&mut iter, "--solver-preconditioner")?
                }
                other => return Err(format!("unknown or incomplete argument: {other}")),
            }
        }
        validate_remote_dir(&options.remote_dir)?;
        Ok(options)
    }
}

fn sync_sources(root: &Path, options: &Options) -> RunnerResult<()> {
    require_ok(
        "remote mkdir",
        ssh_status(
            root,
            &options.host,
            format!("mkdir -p {}", shell_escape(&options.remote_dir)),
        )?,
    )?;
    let excludes = [
        ".git/",
        "tmp/",
        "target/",
        "node_modules/",
        "deps/",
        "_build/",
        "dist/",
        ".next/",
        ".DS_Store",
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

fn pull_evidence(root: &Path, options: &Options, local_dir: &Path) -> RunnerResult<()> {
    fs::create_dir_all(local_dir)
        .map_err(|error| format!("failed to create {}: {error}", local_dir.display()))?;
    for artifact in [
        "tmp/material-research-example.json",
        "tmp/remote-material-research-benchmark.json",
    ] {
        let destination = local_dir.join(
            Path::new(artifact)
                .file_name()
                .ok_or_else(|| format!("invalid artifact path: {artifact}"))?,
        );
        let source = format!("{}:{}/{}", options.host, options.remote_dir, artifact);
        require_ok(
            &format!("pull {artifact}"),
            run_status(
                root,
                "scp",
                [
                    OsString::from("-o"),
                    OsString::from("BatchMode=yes"),
                    OsString::from(source),
                    destination.into_os_string(),
                ],
            )?,
        )?;
    }
    Ok(())
}

fn remote_command(options: &Options) -> String {
    let case_args = options
        .case_filter
        .as_deref()
        .map(|case| format!(" --case {}", shell_escape(case)))
        .unwrap_or_default();
    [
        "set -euo pipefail".to_string(),
        format!("cd {}", shell_escape(&options.remote_dir)),
        "mkdir -p tmp".to_string(),
        "make verify-material-research-example OUT=tmp/material-research-example.json".to_string(),
        "cd workers/rust".to_string(),
        "cargo test -p kyuubiki-cli material_report".to_string(),
        "cargo test -p kyuubiki-cli --bin kyuubiki-material-explore".to_string(),
        format!(
            "cargo run --release -q -p kyuubiki-benchmark -- --profile {} --matrix {} --repeat {} --format json --solver-preconditioner {}{} > ../../tmp/remote-material-research-benchmark.json",
            shell_escape(&options.profile),
            shell_escape(&options.matrix),
            shell_escape(&options.repeat),
            shell_escape(&options.solver_preconditioner),
            case_args
        ),
    ]
    .join("; ")
}

fn ssh_status(root: &Path, host: &str, remote_command: String) -> RunnerResult<u8> {
    run_status(
        root,
        "ssh",
        [
            OsString::from("-o"),
            OsString::from("BatchMode=yes"),
            OsString::from("-o"),
            OsString::from("ConnectTimeout=10"),
            OsString::from(host),
            OsString::from(remote_command),
        ],
    )
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

fn validate_remote_dir(remote_dir: &str) -> RunnerResult<()> {
    if remote_dir.is_empty() || remote_dir.starts_with('/') || remote_dir.contains("..") {
        return Err("remote dir must be a relative path without '..'".to_string());
    }
    if !remote_dir
        .chars()
        .all(|item| item.is_ascii_alphanumeric() || matches!(item, '.' | '_' | '-' | '/'))
    {
        return Err(
            "remote dir may only contain letters, numbers, '.', '_', '-', and '/'".to_string(),
        );
    }
    Ok(())
}

fn required_value(iter: &mut impl Iterator<Item = OsString>, flag: &str) -> RunnerResult<String> {
    iter.next()
        .map(|value| value.to_string_lossy().to_string())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("unknown or incomplete argument: {flag}"))
}

fn env_value(name: &str) -> Option<String> {
    env::var(name)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn run_self_test() -> RunnerResult<u8> {
    validate_remote_dir(DEFAULT_REMOTE_DIR)?;
    validate_remote_dir("runs/material_500k")?;
    if validate_remote_dir("/tmp/nope").is_ok() || validate_remote_dir("bad/../path").is_ok() {
        return Err("remote dir validation accepted an unsafe path".to_string());
    }
    let command = remote_command(&Options {
        case_filter: Some("beam".to_string()),
        host: "kyuubiki-lab".to_string(),
        matrix: "compound-core".to_string(),
        output_slug: "fixture".to_string(),
        profile: "100k".to_string(),
        remote_dir: DEFAULT_REMOTE_DIR.to_string(),
        repeat: "1".to_string(),
        solver_preconditioner: "auto".to_string(),
    });
    for token in [
        "make verify-material-research-example",
        "cargo test -p kyuubiki-cli material_report",
        "--solver-preconditioner 'auto' --case 'beam'",
    ] {
        if !command.contains(token) {
            return Err(format!("self-test command missing token: {token}"));
        }
    }
    println!("remote material research example self-test passed");
    Ok(0)
}

fn print_usage() {
    println!(
        "Usage:\n  ./scripts/kyuubiki remote-material-research-example [options]\n\n\
Options:\n  --host <ssh-host>       Default: KYUUBIKI_LAB_HOST or kyuubiki-lab\n  \
--case <substring>      Optional benchmark case filter, default: CASE_FILTER\n  \
--remote-dir <path>     Relative remote scratch dir, default: {DEFAULT_REMOTE_DIR}\n  \
--profile <profile>     Benchmark profile, default: PROFILE or 100k\n  \
--matrix <matrix>       Benchmark matrix, default: MATRIX or compound-core\n  \
--repeat <count>        Benchmark repeat count, default: REPEAT or 1\n  \
--output-slug <slug>    Local tmp output slug\n  \
--solver-preconditioner <name>\n                          Default: SOLVER_PRECONDITIONER or auto\n  \
--self-test             Validate native command assembly without SSH\n\n\
This runner never stores credentials. It requires existing SSH key/config and\n\
uses rsync --delete only inside the selected remote scratch directory."
    );
}

#[cfg(test)]
mod tests {
    use super::{DEFAULT_REMOTE_DIR, Options, remote_command, validate_remote_dir};

    #[test]
    fn validates_relative_remote_dirs_only() {
        assert!(validate_remote_dir(DEFAULT_REMOTE_DIR).is_ok());
        assert!(validate_remote_dir("nested/run_001").is_ok());
        assert!(validate_remote_dir("/tmp/kyuubiki").is_err());
        assert!(validate_remote_dir("nested/../run").is_err());
        assert!(validate_remote_dir("nested/run with space").is_err());
    }

    #[test]
    fn remote_command_keeps_material_evidence_steps() {
        let command = remote_command(&Options {
            case_filter: Some("panel core".to_string()),
            host: "kyuubiki-lab".to_string(),
            matrix: "compound-core".to_string(),
            output_slug: "fixture".to_string(),
            profile: "100k".to_string(),
            remote_dir: DEFAULT_REMOTE_DIR.to_string(),
            repeat: "1".to_string(),
            solver_preconditioner: "auto".to_string(),
        });
        assert!(command.contains("make verify-material-research-example"));
        assert!(command.contains("cargo test -p kyuubiki-cli --bin kyuubiki-material-explore"));
        assert!(command.contains("--case 'panel core'"));
    }
}
