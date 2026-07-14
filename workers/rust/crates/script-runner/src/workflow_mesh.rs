use crate::native_time::utc_timestamp_slug;
use std::env;
use std::ffi::OsString;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

type RunnerResult<T> = Result<T, String>;

pub(crate) fn run_workflow_mesh_regression(root: &Path) -> RunnerResult<u8> {
    let node_bin = env::var("NODE_BIN").unwrap_or_else(|_| "node".to_string());
    let output_slug = env::var("OUTPUT_SLUG")
        .unwrap_or_else(|_| format!("workflow-mesh-{}", utc_timestamp_slug()));
    let output_dir = env_path_or(
        "OUTPUT_DIR",
        root.join("tmp/workflow-mesh-regression").join(output_slug),
    );
    let log_path = env_path_or("LOG_PATH", output_dir.join("run.log"));
    let test_files = [
        "tests/integration/workflow-distributed-smoke.test.mjs",
        "tests/integration/workflow-offline-mesh-smoke.test.mjs",
        "tests/integration/workflow-offline-mesh-branch-diagnostics-smoke.test.mjs",
    ];

    std::fs::create_dir_all(&output_dir)
        .map_err(|error| format!("failed to create {}: {error}", output_dir.display()))?;
    let mut log = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(&log_path)
        .map_err(|error| format!("failed to open {}: {error}", log_path.display()))?;

    for test_file in test_files {
        log_line(&mut log, &format!("==> running {test_file}"))?;
        let status = run_logged_command(
            root,
            &node_bin,
            [OsString::from("--test"), OsString::from(test_file)],
            &mut log,
        )?;
        if status != 0 {
            return Ok(status);
        }
    }

    log_line(&mut log, "workflow mesh regression completed")?;
    let summary = crate::workflow_mesh_summary::run_build_workflow_mesh_regression_summary(
        root,
        vec![
            OsString::from("--log"),
            log_path.clone().into_os_string(),
            OsString::from("--output-dir"),
            output_dir.into_os_string(),
        ],
    )?;
    if summary != 0 {
        return Ok(summary);
    }
    let index = crate::workflow_mesh_index::run_build_workflow_mesh_regression_index(
        root,
        vec![
            OsString::from("--root"),
            root.join("tmp/workflow-mesh-regression").into_os_string(),
        ],
    )?;
    if index != 0 {
        return Ok(index);
    }
    println!("workflow mesh regression log: {}", log_path.display());
    Ok(0)
}

fn run_logged_command<I>(
    cwd: &Path,
    program: &str,
    args: I,
    log: &mut std::fs::File,
) -> RunnerResult<u8>
where
    I: IntoIterator<Item = OsString>,
{
    let output = Command::new(program)
        .args(args)
        .current_dir(cwd)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|error| format!("failed to run {program}: {error}"))?;
    log.write_all(&output.stdout)
        .map_err(|error| format!("failed to write command stdout: {error}"))?;
    log.write_all(&output.stderr)
        .map_err(|error| format!("failed to write command stderr: {error}"))?;
    std::io::stdout()
        .write_all(&output.stdout)
        .map_err(|error| format!("failed to write stdout: {error}"))?;
    std::io::stderr()
        .write_all(&output.stderr)
        .map_err(|error| format!("failed to write stderr: {error}"))?;
    Ok(output.status.code().unwrap_or(1) as u8)
}

fn env_path_or(name: &str, fallback: PathBuf) -> PathBuf {
    env::var_os(name).map(PathBuf::from).unwrap_or(fallback)
}

fn log_line(log: &mut std::fs::File, line: &str) -> RunnerResult<()> {
    println!("{line}");
    writeln!(log, "{line}").map_err(|error| format!("failed to write log: {error}"))
}
