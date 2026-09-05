use super::support::{
    RuntimeGuard, WORKFLOW_ID, canonical_dir, ensure_qualification_host, execution_result,
    headless_command, path_text, pid_residue, read_json, render_digests, reserve_ports,
    run_headless_workflow, run_success, runtime_env_for_platform, validate_fetch, validate_status,
    verify_installation_for_platform, write_json,
};
use serde_json::json;
use std::ffi::OsString;
use std::fs;
use std::path::PathBuf;

type RunnerResult<T> = Result<T, String>;

pub(super) fn run(args: Vec<OsString>) -> RunnerResult<u8> {
    let options = Options::parse(args)?;
    ensure_qualification_host(
        &options.platform,
        &options.architecture,
        &options.execution_host_role,
    )?;
    capture(&options)?;
    println!(
        "installed Runtime host capture passed: {}",
        options.output.display()
    );
    Ok(0)
}

struct Options {
    managed_root: PathBuf,
    runtime_root: PathBuf,
    detached_source_root: PathBuf,
    output: PathBuf,
    package_version: String,
    platform: String,
    architecture: String,
    execution_host_role: String,
}

impl Options {
    fn parse(args: Vec<OsString>) -> RunnerResult<Self> {
        let mut managed_root = None;
        let mut runtime_root = None;
        let mut detached_source_root = None;
        let mut output = None;
        let mut package_version = None;
        let mut platform = None;
        let mut architecture = None;
        let mut execution_host_role = None;
        let mut args = args.into_iter();
        while let Some(arg) = args.next() {
            match arg.to_string_lossy().as_ref() {
                "--managed-root" => managed_root = Some(next_path(&mut args, "--managed-root")?),
                "--runtime-root" => runtime_root = Some(next_path(&mut args, "--runtime-root")?),
                "--detached-source-root" => {
                    detached_source_root = Some(next_path(&mut args, "--detached-source-root")?)
                }
                "--out" => output = Some(next_path(&mut args, "--out")?),
                "--package-version" => {
                    package_version = Some(next_string(&mut args, "--package-version")?)
                }
                "--platform" => platform = Some(next_string(&mut args, "--platform")?),
                "--architecture" => architecture = Some(next_string(&mut args, "--architecture")?),
                "--execution-host-role" => {
                    execution_host_role = Some(next_string(&mut args, "--execution-host-role")?)
                }
                other => return Err(format!("unknown installed Runtime host option: {other}")),
            }
        }
        let options = Self {
            managed_root: managed_root.ok_or("--managed-root is required")?,
            runtime_root: runtime_root.ok_or("--runtime-root is required")?,
            detached_source_root: detached_source_root
                .ok_or("--detached-source-root is required")?,
            output: output.ok_or("--out is required")?,
            package_version: package_version.ok_or("--package-version is required")?,
            platform: platform.ok_or("--platform is required")?,
            architecture: architecture.ok_or("--architecture is required")?,
            execution_host_role: execution_host_role.ok_or("--execution-host-role is required")?,
        };
        for (label, path) in [
            ("--managed-root", &options.managed_root),
            ("--runtime-root", &options.runtime_root),
            ("--detached-source-root", &options.detached_source_root),
            ("--out", &options.output),
        ] {
            if !path.is_absolute() {
                return Err(format!("{label} requires an absolute path"));
            }
        }
        if !super::valid_version(&options.package_version) {
            return Err("--package-version is invalid".to_string());
        }
        Ok(options)
    }
}

fn capture(options: &Options) -> RunnerResult<()> {
    if options.detached_source_root.exists() {
        return Err("source tree must be removed before installed capture".to_string());
    }
    let managed = canonical_dir(&options.managed_root, "managed root")?;
    let runtime = canonical_dir(&options.runtime_root, "installed Runtime")?;
    if !runtime.starts_with(&managed) {
        return Err("installed Runtime escapes the managed root".to_string());
    }
    let detached_parent = options
        .detached_source_root
        .parent()
        .ok_or("detached source path has no parent")?;
    if canonical_dir(detached_parent, "detached source parent")? != managed {
        return Err("detached source path escapes the managed root".to_string());
    }
    let output_parent = options
        .output
        .parent()
        .ok_or("host capture output has no parent")?;
    if !canonical_dir(output_parent, "host capture output parent")?.starts_with(&managed) {
        return Err("host capture output must stay inside the managed root".to_string());
    }
    let installation =
        verify_installation_for_platform(&runtime, &options.package_version, &options.platform)?;
    let ports = reserve_ports()?;
    let state_root = managed.join("runtime-state");
    let work_root = managed.join("isolated-work");
    let home = managed.join("home");
    fs::create_dir_all(&options.output)
        .and_then(|()| fs::create_dir_all(&work_root))
        .and_then(|()| fs::create_dir_all(&home))
        .map_err(|error| format!("failed to create capture roots: {error}"))?;

    let runtime_binary = runtime.join("bin/kyuubiki-runtime");
    let headless_binary = runtime.join("bin/kyuubiki-headless");
    let env = runtime_env_for_platform(&runtime, &state_root, &home, ports, &options.platform);
    let mut guard = RuntimeGuard::new(runtime_binary, env.clone(), ports);
    guard.command("start-local")?;
    validate_status(&guard.command("status")?, ports)?;

    let workflow = work_root.join("workflow.json");
    run_success(
        "initialize installed Headless workflow",
        headless_command(
            &headless_binary,
            &env,
            &work_root,
            [
                "init",
                "--template",
                "direct_bar_1d",
                "--workflow-id",
                WORKFLOW_ID,
                "--out",
                path_text(&workflow)?,
                "--json",
            ],
        ),
    )?;
    let solve_path = options.output.join("solve.json");
    run_headless_workflow(
        &headless_binary,
        &env,
        &work_root,
        &workflow,
        &solve_path,
        ports.orchestrator,
    )?;
    let (job_id, tip, stress) = execution_result(&read_json(&solve_path)?)?;

    let fetch_workflow = work_root.join("fetch-workflow.json");
    write_json(
        &fetch_workflow,
        &json!({
            "schema_version": "kyuubiki.headless-workflow/v1",
            "exported_at": "1970-01-01T00:00:00.000Z",
            "language": "en-US",
            "workflow": {
                "id": format!("{WORKFLOW_ID}.fetch"),
                "steps": [{"action": "result_fetch", "payload": {"job_id": job_id}}]
            }
        }),
    )?;
    guard.command("restart-local")?;
    let restart_path = options.output.join("fetch-report.json");
    run_headless_workflow(
        &headless_binary,
        &env,
        &work_root,
        &fetch_workflow,
        &restart_path,
        ports.orchestrator,
    )?;
    validate_fetch(&read_json(&restart_path)?, &job_id, tip, stress)?;

    guard.command("restart-local")?;
    if options.detached_source_root.exists() {
        return Err("source tree reappeared during installed restart".to_string());
    }
    let detached_path = options.output.join("detached-fetch-report.json");
    run_headless_workflow(
        &headless_binary,
        &env,
        &work_root,
        &fetch_workflow,
        &detached_path,
        ports.orchestrator,
    )?;
    validate_fetch(&read_json(&detached_path)?, &job_id, tip, stress)?;
    let detached_status = guard.command("status")?;
    validate_status(&detached_status, ports)?;
    fs::write(
        options.output.join("detached-status.txt"),
        detached_status.as_bytes(),
    )
    .map_err(|error| format!("failed to write detached status: {error}"))?;
    guard.stop()?;
    if ports.any_listening() || pid_residue(&state_root)? != 0 {
        return Err("installed Runtime cleanup left a process or PID residue".to_string());
    }
    let post_run_installation =
        verify_installation_for_platform(&runtime, &options.package_version, &options.platform)?;
    if post_run_installation != installation {
        return Err("installed Runtime payload changed during operational qualification".into());
    }
    fs::write(
        options.output.join("installed-digests.txt"),
        render_digests(&post_run_installation),
    )
    .map_err(|error| format!("failed to write installed digests: {error}"))?;
    Ok(())
}

fn next_string(args: &mut impl Iterator<Item = OsString>, option: &str) -> RunnerResult<String> {
    args.next()
        .and_then(|value| value.into_string().ok())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("{option} requires a UTF-8 value"))
}

fn next_path(args: &mut impl Iterator<Item = OsString>, option: &str) -> RunnerResult<PathBuf> {
    Ok(PathBuf::from(next_string(args, option)?))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_options_require_absolute_managed_paths() {
        let error = Options::parse(vec![
            "--managed-root".into(),
            "relative".into(),
            "--runtime-root".into(),
            "/tmp/runtime".into(),
            "--detached-source-root".into(),
            "/tmp/source".into(),
            "--out".into(),
            "/tmp/output".into(),
            "--package-version".into(),
            "2.19.0".into(),
            "--platform".into(),
            "linux".into(),
            "--architecture".into(),
            "x86_64".into(),
            "--execution-host-role".into(),
            "remote-linux-qualification-host".into(),
        ])
        .err()
        .expect("relative path must fail");
        assert!(error.contains("--managed-root"));
    }
}
