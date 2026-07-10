use std::env;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

mod agent_registry_sync;
mod benchmark_profile_remote;
mod benchmark_profile_remote_summary;
mod desktop;
mod desktop_icon_variants;
mod desktop_linux_remote;
mod desktop_release_upload_remote;
mod direct_mesh_container;
mod direct_mesh_remote;
mod lab;
mod native_script_audit;
mod native_time;
mod remote_host;
mod remote_ssh_fixture;
mod standard_benchmark_remote;
mod workflow_catalog_remote;
mod workflow_mesh;
mod workflow_mesh_remote;

type RunnerResult<T> = Result<T, String>;

use desktop::{
    DesktopApp, host_platform, run_desktop_build, run_desktop_build_host, run_desktop_dev,
    run_desktop_release, run_desktop_stage, run_desktop_status, run_desktop_verify,
    run_package_desktop,
};

struct RepoPaths {
    root: PathBuf,
    frontend: PathBuf,
    rust: PathBuf,
    web: PathBuf,
    hub_gui: PathBuf,
    installer_gui: PathBuf,
    workbench_gui: PathBuf,
}

fn main() -> ExitCode {
    match run() {
        Ok(code) => ExitCode::from(code),
        Err(message) => {
            eprintln!("{message}");
            ExitCode::from(1)
        }
    }
}

fn run() -> RunnerResult<u8> {
    let paths = RepoPaths::discover()?;
    let mut args = env::args_os().skip(1);
    let command = args
        .next()
        .and_then(|value| value.into_string().ok())
        .unwrap_or_else(|| "help".to_string());
    let rest: Vec<OsString> = args.collect();

    match command.as_str() {
        "help" | "--help" | "-h" => {
            print_help();
            Ok(0)
        }
        "native-script-audit" => native_script_audit::run_native_script_audit(
            &paths.root,
            host_platform().as_str(),
            rest,
        ),
        "status"
        | "start"
        | "start-local"
        | "start-cloud"
        | "start-distributed"
        | "restart"
        | "restart-local"
        | "restart-cloud"
        | "restart-distributed"
        | "stop"
        | "export-db"
        | "hot-status"
        | "hot-start-local"
        | "hot-start-cloud"
        | "hot-start-distributed"
        | "hot-stop" => {
            let mut runtime_args = vec![
                paths
                    .root
                    .join("scripts/kyuubiki-runtime.mjs")
                    .into_os_string(),
                OsString::from(&command),
            ];
            runtime_args.extend(rest);
            run_command(&paths.root, "node", runtime_args)
        }
        "doctor" => run_installer(&paths, "doctor", rest),
        "validate-env" => run_installer(&paths, "validate-env", rest),
        "cross-platform-audit" => run_installer(&paths, "cross-platform-audit", rest),
        "operator-package-preflight" => run_installer(&paths, "operator-package-preflight", rest),
        "install" => run_command(&paths.rust, "cargo", cargo_run("kyuubiki-installer", rest)),
        "package" | "package-runtime" => {
            let platform = host_platform().as_str();
            run_installer(&paths, "stage-release", prepend(platform, rest))
        }
        "project" | "macro" => run_command(
            &paths.frontend,
            "node",
            [
                OsString::from("./scripts/kyuubiki-cli.mjs"),
                OsString::from(&command),
            ]
            .into_iter()
            .chain(rest),
        ),
        "build-frontend" => run_command(
            &paths.frontend,
            "npm",
            prepend("run", prepend("build", rest)),
        ),
        "build-orchestrator" => run_with_env(
            &paths.web,
            "mix",
            prepend("compile", rest),
            &[("MIX_ENV", "prod")],
        ),
        "build-agent" => run_command(
            &paths.rust,
            "cargo",
            ["build", "-p", "kyuubiki-cli", "--release"]
                .into_iter()
                .map(OsString::from)
                .chain(rest),
        ),
        "build-hub-gui" => run_desktop_build(&paths, DesktopApp::Hub, rest),
        "build-installer-gui" => run_desktop_build(&paths, DesktopApp::Installer, rest),
        "build-workbench-gui" => run_desktop_build(&paths, DesktopApp::Workbench, rest),
        "package-desktop" => run_package_desktop(&paths, rest),
        "desktop-status" => run_desktop_status(&paths, rest),
        "desktop-stage" => run_desktop_stage(&paths, rest),
        "desktop-build-host" => run_desktop_build_host(&paths),
        "desktop-release" => run_desktop_release(&paths, rest),
        "desktop-verify" => run_desktop_verify(&paths, rest),
        "desktop-linux-remote" => desktop_linux_remote::run_desktop_linux_remote(&paths.root, rest),
        "desktop-upload-remote" | "desktop-release-upload-remote" => {
            desktop_release_upload_remote::run_desktop_release_upload_remote(&paths.root, rest)
        }
        "generate-desktop-icon-variants" => {
            desktop_icon_variants::run_generate_desktop_icon_variants(&paths.root, rest)
        }
        "agent-registry-sync" => agent_registry_sync::run_agent_registry_sync(&paths.root, rest),
        "lab" => lab::run_lab(&paths.root, rest),
        "remote-ssh-fixture" => remote_ssh_fixture::run_remote_ssh_fixture(&paths.root, rest),
        "web-test" => run_command(&paths.web, "mix", prepend("test", rest)),
        "rust-test" => {
            let status = run_command(&paths.rust, "cargo", prepend("test", rest.clone()))?;
            if status != 0 {
                return Ok(status);
            }
            run_node_script(&paths.root, "audit-rust-line-counts.mjs", Vec::new())
        }
        "rust-line-audit" => run_node_script(&paths.root, "audit-rust-line-counts.mjs", rest),
        "frontend-test" => {
            let typecheck = run_command(
                &paths.frontend,
                "npm",
                ["run", "typecheck"].map(OsString::from),
            )?;
            if typecheck != 0 {
                return Ok(typecheck);
            }
            run_command(&paths.frontend, "npm", ["run", "build"].map(OsString::from))
        }
        "headless-test" => run_command(
            &paths.frontend,
            "npm",
            ["run", "test:unit:headless"].map(OsString::from),
        ),
        "headless-live-test" => run_command(
            &paths.frontend,
            "npm",
            ["run", "test:unit:headless-live"].map(OsString::from),
        ),
        "headless-rust-live-test" => run_command(
            &paths.rust,
            "cargo",
            [
                "test",
                "-p",
                "kyuubiki-cli",
                "--test",
                "headless_live",
                "--",
                "--test-threads=1",
            ]
            .map(OsString::from),
        ),
        "sdk-smoke" => run_sdk_smoke(&paths),
        "workflow-preflight" => run_command(
            &paths.frontend,
            "npm",
            ["run", "check:workflow-preflight"].map(OsString::from),
        ),
        "benchmark-profile-remote" => {
            benchmark_profile_remote::run_benchmark_profile_remote(&paths.root, rest)
        }
        "direct-mesh-benchmark-container" => {
            direct_mesh_container::run_direct_mesh_benchmark_container(&paths.root, rest)
        }
        "direct-mesh-benchmark-regression" => {
            direct_mesh_remote::run_direct_mesh_benchmark_regression(&paths.root, rest)
        }
        "standard-benchmark-regression" => {
            standard_benchmark_remote::run_standard_benchmark_regression(&paths.root, rest)
        }
        "workflow-catalog-benchmark-regression" => {
            workflow_catalog_remote::run_workflow_catalog_remote(&paths.root, rest)
        }
        "workflow-mesh-regression-remote" => {
            workflow_mesh_remote::run_workflow_mesh_remote(&paths.root, rest)
        }
        "workflow-mesh-regression" => workflow_mesh::run_workflow_mesh_regression(&paths.root),
        "agent-capability-smoke" => {
            run_python_script(&paths.root, "agent-capability-smoke.py", rest)
        }
        "worker" => run_command(&paths.rust, "cargo", cargo_run("kyuubiki-cli", rest)),
        "benchmark" => run_command(&paths.rust, "cargo", cargo_run("kyuubiki-benchmark", rest)),
        "agent" => {
            let agent_args = if rest.is_empty() {
                vec![
                    OsString::from("agent"),
                    OsString::from("--port"),
                    OsString::from("5001"),
                ]
            } else {
                prepend("agent", rest)
            };
            run_command(&paths.rust, "cargo", cargo_run("kyuubiki-cli", agent_args))
        }
        "frontend" => run_command(&paths.frontend, "npm", ["run", "dev"].map(OsString::from)),
        "format" => {
            let mix = run_command(&paths.web, "mix", ["format"].map(OsString::from))?;
            if mix != 0 {
                return Ok(mix);
            }
            run_command(&paths.rust, "cargo", ["fmt"].map(OsString::from))
        }
        "hub-gui-dev" => run_desktop_dev(&paths, DesktopApp::Hub),
        "installer-gui-dev" => run_desktop_dev(&paths, DesktopApp::Installer),
        "workbench-gui-dev" => run_desktop_dev(&paths, DesktopApp::Workbench),
        _ => {
            eprintln!("unknown native command: {command}");
            eprintln!("run `./scripts/kyuubiki help` for supported native commands");
            Ok(2)
        }
    }
}

impl RepoPaths {
    fn discover() -> RunnerResult<Self> {
        let exe =
            env::current_exe().map_err(|error| format!("failed to resolve executable: {error}"))?;
        let mut current = exe.parent().unwrap_or_else(|| Path::new(".")).to_path_buf();
        loop {
            if current.join("workers/rust/Cargo.toml").is_file() && current.join("scripts").is_dir()
            {
                return Ok(Self {
                    frontend: current.join("apps/frontend"),
                    rust: current.join("workers/rust"),
                    web: current.join("apps/web"),
                    hub_gui: current.join("apps/hub-gui"),
                    installer_gui: current.join("apps/installer-gui"),
                    workbench_gui: current.join("apps/workbench-gui"),
                    root: current,
                });
            }
            if !current.pop() {
                break;
            }
        }
        let cwd = env::current_dir().map_err(|error| format!("failed to resolve cwd: {error}"))?;
        Ok(Self {
            frontend: cwd.join("apps/frontend"),
            rust: cwd.join("workers/rust"),
            web: cwd.join("apps/web"),
            hub_gui: cwd.join("apps/hub-gui"),
            installer_gui: cwd.join("apps/installer-gui"),
            workbench_gui: cwd.join("apps/workbench-gui"),
            root: cwd,
        })
    }
}

fn run_installer(paths: &RepoPaths, subcommand: &str, rest: Vec<OsString>) -> RunnerResult<u8> {
    run_command(
        &paths.rust,
        "cargo",
        cargo_run("kyuubiki-installer", prepend(subcommand, rest)),
    )
}

fn run_node_script(root: &Path, script: &str, rest: Vec<OsString>) -> RunnerResult<u8> {
    run_command(
        root,
        "node",
        [root.join("scripts").join(script).into_os_string()]
            .into_iter()
            .chain(rest),
    )
}

fn run_python_script(root: &Path, script: &str, rest: Vec<OsString>) -> RunnerResult<u8> {
    run_with_env(
        root,
        "python3",
        [root.join("scripts").join(script).into_os_string()]
            .into_iter()
            .chain(rest),
        &[("PYTHONPATH", "sdks/python")],
    )
}

fn run_sdk_smoke(paths: &RepoPaths) -> RunnerResult<u8> {
    let python = run_with_env(
        &paths.root,
        "python3",
        ["-m", "unittest", "discover", "-s", "sdks/python/tests"].map(OsString::from),
        &[("PYTHONPATH", "sdks/python")],
    )?;
    if python != 0 {
        return Ok(python);
    }

    let elixir = run_command(
        &paths.root.join("sdks/elixir"),
        "mix",
        ["test"].map(OsString::from),
    )?;
    if elixir != 0 {
        return Ok(elixir);
    }

    run_command(
        &paths.root,
        "cargo",
        ["test", "--manifest-path", "sdks/rust/Cargo.toml"].map(OsString::from),
    )
}

fn run_command<I>(cwd: &Path, program: &str, args: I) -> RunnerResult<u8>
where
    I: IntoIterator<Item = OsString>,
{
    run_with_env(cwd, program, args, &[])
}

fn run_with_env<I>(cwd: &Path, program: &str, args: I, envs: &[(&str, &str)]) -> RunnerResult<u8>
where
    I: IntoIterator<Item = OsString>,
{
    let status = Command::new(program)
        .args(args)
        .current_dir(cwd)
        .envs(envs.iter().copied())
        .status()
        .map_err(|error| format!("failed to run {program}: {error}"))?;
    Ok(status.code().unwrap_or(1) as u8)
}

fn cargo_run(package: &str, rest: Vec<OsString>) -> impl Iterator<Item = OsString> {
    ["run", "-p", package, "--"]
        .into_iter()
        .map(OsString::from)
        .chain(rest)
}

fn prepend(value: &str, rest: Vec<OsString>) -> Vec<OsString> {
    std::iter::once(OsString::from(value)).chain(rest).collect()
}

fn print_help() {
    println!(
        "Kyuubiki native script runner\n\n\
Native commands:\n  \
status/start/stop/restart/export-db/hot-status\n  \
doctor validate-env install package cross-platform-audit\n  \
operator-package-preflight\n  \
project macro build-frontend build-orchestrator build-agent\n  \
build-hub-gui build-installer-gui build-workbench-gui\n  \
package-desktop desktop-status desktop-stage desktop-build-host\n  \
desktop-release desktop-verify\n  \
desktop-linux-remote\n  \
desktop-upload-remote desktop-release-upload-remote\n  \
generate-desktop-icon-variants\n  \
lab remote-ssh-fixture\n  \
web-test rust-test rust-line-audit frontend-test headless-test\n  \
  headless-live-test headless-rust-live-test sdk-smoke workflow-preflight\n  \
  benchmark-profile-remote\n  \
  direct-mesh-benchmark-container\n  \
  direct-mesh-benchmark-regression\n  \
  standard-benchmark-regression\n  \
  workflow-catalog-benchmark-regression\n  \
  workflow-mesh-regression-remote\n  \
  workflow-mesh-regression\n  \
  agent-capability-smoke\n  \
worker benchmark agent frontend format\n  \
hub-gui-dev installer-gui-dev workbench-gui-dev\n  \
native-script-audit\n"
    );
}
